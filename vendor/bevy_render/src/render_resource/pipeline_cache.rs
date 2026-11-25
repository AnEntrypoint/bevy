use bevy_material::descriptor::{
    BindGroupLayoutDescriptor, CachedComputePipelineId, ComputePipelineDescriptor,
    MeshPipelineDescriptor, PipelineDescriptor, RenderPipelineDescriptor,
};
use smallvec::SmallVec;

use crate::{
    render_resource::*,
    renderer::{wgpu_wrapper, RenderDevice},
    Extract,
};
use alloc::{borrow::Cow, collections::VecDeque, sync::Arc};
use bevy_platform::time::Instant;
use core::time::Duration;
use bevy_asset::{AssetEvent, AssetId, Assets, Handle};
use bevy_ecs::{
    message::MessageReader,
    resource::Resource,
    system::{Res, ResMut},
};
use bevy_log::error;
use bevy_platform::collections::{hash_map::RawEntryMut, HashMap, HashSet};
use bevy_shader::{
    CachedPipelineId, Shader, ShaderCache, ShaderCacheError, ShaderCacheSource, ShaderDefVal,
    ValidateShader,
};
use bevy_utils::default;
use core::{future::Future, mem};
use std::sync::{Mutex, PoisonError};
use wgpu::{PipelineCompilationOptions, VertexBufferLayout as RawVertexBufferLayout};

#[derive(Debug)]
pub enum Pipeline {
    RenderPipeline(RenderPipeline),
    ComputePipeline(ComputePipeline),
    MeshPipeline(RenderPipeline),
}

pub struct CachedPipeline {
    pub descriptor: PipelineDescriptor,
    pub state: CachedPipelineState,
}

#[derive(Debug)]
pub enum CachedPipelineState {
    Queued,
    Ok(Pipeline),
    Err(ShaderCacheError),
}

impl CachedPipelineState {
    pub fn unwrap(&self) -> &Pipeline {
        match self {
            CachedPipelineState::Ok(pipeline) => pipeline,
            CachedPipelineState::Queued => {
                panic!("Pipeline has not been compiled yet. It is still in the 'Queued' state.")
            }
            CachedPipelineState::Err(err) => panic!("{}", err),
        }
    }
}

const BIND_GROUP_LAYOUTS_INLINE_CAPACITY: usize = 8;

type ImmediateSize = u32;
type LayoutCacheKey = (
    SmallVec<[BindGroupLayoutId; BIND_GROUP_LAYOUTS_INLINE_CAPACITY]>,
    ImmediateSize,
);

wgpu_wrapper! {
    struct WgpuPipelineLayout(PipelineLayout);

    struct WgpuShaderModule(ShaderModule);
}

#[derive(Default)]
struct LayoutCache {
    layouts: HashMap<LayoutCacheKey, Arc<WgpuPipelineLayout>>,
}

impl LayoutCache {
    fn get(
        &mut self,
        render_device: &RenderDevice,
        bind_group_layouts: &[BindGroupLayout],
        immediate_size: u32,
    ) -> Arc<WgpuPipelineLayout> {
        let bind_group_ids = bind_group_layouts.iter().map(BindGroupLayout::id).collect();
        self.layouts
            .entry((bind_group_ids, immediate_size))
            .or_insert_with_key(|(_, immediate_size)| {
                let bind_group_layouts = bind_group_layouts
                    .iter()
                    .map(BindGroupLayout::value)
                    .map(Some)
                    .collect::<SmallVec<[_; BIND_GROUP_LAYOUTS_INLINE_CAPACITY]>>();
                Arc::new(WgpuPipelineLayout::new(
                    render_device.create_pipeline_layout(&PipelineLayoutDescriptor {
                        bind_group_layouts: &bind_group_layouts,
                        immediate_size: *immediate_size,
                        ..default()
                    }),
                ))
            })
            .clone()
    }
}

fn load_module(
    render_device: &RenderDevice,
    shader_source: ShaderCacheSource,
    validate_shader: &ValidateShader,
) -> Result<WgpuShaderModule, ShaderCacheError> {
    let shader_source = match shader_source {
        #[cfg(feature = "shader_format_spirv")]
        ShaderCacheSource::SpirV(data) => wgpu::util::make_spirv(data),
        #[cfg(not(feature = "shader_format_spirv"))]
        ShaderCacheSource::SpirV(_) => {
            unimplemented!("Enable feature \"shader_format_spirv\" to use SPIR-V shaders")
        }
        ShaderCacheSource::Wgsl(src) => ShaderSource::Wgsl(Cow::Owned(src)),
    };
    let module_descriptor = ShaderModuleDescriptor {
        label: None,
        source: shader_source,
    };

    let scope = render_device
        .wgpu_device()
        .push_error_scope(wgpu::ErrorFilter::Validation);

    let shader_module = WgpuShaderModule::new(match validate_shader {
        ValidateShader::Enabled => {
            render_device.create_and_validate_shader_module(module_descriptor)
        }
        ValidateShader::Disabled => unsafe {
            render_device.create_shader_module(module_descriptor)
        },
    });

    let error = scope.pop();

    if let Some(Some(wgpu::Error::Validation { description, .. })) =
        bevy_tasks::futures::now_or_never(error)
    {
        return Err(ShaderCacheError::CreateShaderModule(description));
    }

    Ok(shader_module)
}

#[derive(Default)]
struct BindGroupLayoutCache {
    bgls: HashMap<BindGroupLayoutDescriptor, BindGroupLayout>,
}

impl BindGroupLayoutCache {
    fn get(
        &mut self,
        render_device: &RenderDevice,
        descriptor: &BindGroupLayoutDescriptor,
    ) -> BindGroupLayout {
        match self.bgls.raw_entry_mut().from_key(descriptor) {
            RawEntryMut::Occupied(entry) => entry.into_mut(),
            RawEntryMut::Vacant(slot) => {
                let created = render_device
                    .create_bind_group_layout(descriptor.label.as_ref(), &descriptor.entries);
                slot.insert(descriptor.clone(), created).1
            }
        }
        .clone()
    }
}

#[derive(Resource, Clone, Copy, Debug)]
pub struct PipelineQueuePolicy {
    pub open: bool,
    pub budget: Option<Duration>,
}

impl PipelineQueuePolicy {
    pub const UNLIMITED: Self = Self {
        open: true,
        budget: None,
    };
}

#[derive(Default, Clone, Copy)]
struct StepCost {
    prepare_ms: f64,
    create_ms: f64,
}

fn blend(previous_ms: f64, measured: Duration) -> f64 {
    let measured_ms = measured.as_secs_f64() * 1000.0;
    if previous_ms == 0.0 {
        measured_ms
    } else {
        0.5 * previous_ms + 0.5 * measured_ms
    }
}

fn fits(budget: Option<Duration>, started: Instant, next_ms: f64) -> bool {
    budget.is_none_or(|budget| {
        started.elapsed().as_secs_f64() * 1000.0 + next_ms <= budget.as_secs_f64() * 1000.0
    })
}

#[derive(Resource)]
pub struct PipelineCache {
    layout_cache: Arc<Mutex<LayoutCache>>,
    bindgroup_layout_cache: Arc<Mutex<BindGroupLayoutCache>>,
    shader_cache: Arc<Mutex<ShaderCache<WgpuShaderModule, RenderDevice>>>,
    device: RenderDevice,
    pipelines: Vec<CachedPipeline>,
    pending: VecDeque<CachedPipelineId>,
    cost: StepCost,
    new_pipelines: Mutex<Vec<CachedPipeline>>,
    global_shader_defs: Vec<ShaderDefVal>,
    needs_shader_reload: bool,
}

impl PipelineCache {
    pub fn pipelines(&self) -> impl Iterator<Item = &CachedPipeline> {
        self.pipelines.iter()
    }

    pub fn waiting_pipelines(&self) -> impl Iterator<Item = CachedPipelineId> + '_ {
        self.pending.iter().copied()
    }

    pub fn new(device: RenderDevice) -> Self {
        let mut global_shader_defs = Vec::new();
        #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
        {
            global_shader_defs.push("NO_ARRAY_TEXTURES_SUPPORT".into());
            global_shader_defs.push("NO_CUBE_ARRAY_TEXTURES_SUPPORT".into());
            global_shader_defs.push("SIXTEEN_BYTE_ALIGNMENT".into());
        }

        if cfg!(target_abi = "sim") {
            global_shader_defs.push("NO_CUBE_ARRAY_TEXTURES_SUPPORT".into());
        }

        let available_storage_buffer_bindings =
            device.limits().max_storage_buffers_per_shader_stage;
        global_shader_defs.push(ShaderDefVal::UInt(
            "AVAILABLE_STORAGE_BUFFER_BINDINGS".into(),
            available_storage_buffer_bindings,
        ));
        global_shader_defs.push(ShaderDefVal::Bool(
            "AVAILABLE_STORAGE_BUFFER_BINDINGS__GE_3".into(),
            available_storage_buffer_bindings >= 3,
        ));
        global_shader_defs.push(ShaderDefVal::Bool(
            "AVAILABLE_STORAGE_BUFFER_BINDINGS__GE_6".into(),
            available_storage_buffer_bindings >= 6,
        ));

        Self {
            shader_cache: Arc::new(Mutex::new(ShaderCache::new(device.clone(), load_module))),
            device,
            layout_cache: default(),
            bindgroup_layout_cache: default(),
            pending: VecDeque::new(),
            cost: StepCost::default(),
            new_pipelines: default(),
            pipelines: default(),
            global_shader_defs,
            needs_shader_reload: true,
        }
    }

    #[inline]
    pub fn get_render_pipeline_state(&self, id: CachedRenderPipelineId) -> &CachedPipelineState {
        self.pipelines
            .get(id.id())
            .map_or(&CachedPipelineState::Queued, |pipeline| &pipeline.state)
    }

    #[inline]
    pub fn get_compute_pipeline_state(&self, id: CachedComputePipelineId) -> &CachedPipelineState {
        self.pipelines
            .get(id.id())
            .map_or(&CachedPipelineState::Queued, |pipeline| &pipeline.state)
    }

    #[inline]
    pub fn get_render_pipeline_descriptor(
        &self,
        id: CachedRenderPipelineId,
    ) -> &RenderPipelineDescriptor {
        match &self.pipelines[id.id()].descriptor {
            PipelineDescriptor::RenderPipelineDescriptor(descriptor) => descriptor,
            PipelineDescriptor::ComputePipelineDescriptor(_)
            | PipelineDescriptor::MeshPipelineDescriptor(_) => unreachable!(),
        }
    }

    #[inline]
    pub fn get_compute_pipeline_descriptor(
        &self,
        id: CachedComputePipelineId,
    ) -> &ComputePipelineDescriptor {
        match &self.pipelines[id.id()].descriptor {
            PipelineDescriptor::RenderPipelineDescriptor(_)
            | PipelineDescriptor::MeshPipelineDescriptor(_) => unreachable!(),
            PipelineDescriptor::ComputePipelineDescriptor(descriptor) => descriptor,
        }
    }

    #[inline]
    pub fn get_mesh_pipeline_descriptor(
        &self,
        id: CachedRenderPipelineId,
    ) -> &MeshPipelineDescriptor {
        match &self.pipelines[id.id()].descriptor {
            PipelineDescriptor::RenderPipelineDescriptor(_)
            | PipelineDescriptor::ComputePipelineDescriptor(_) => unreachable!(),
            PipelineDescriptor::MeshPipelineDescriptor(descriptor) => descriptor,
        }
    }

    #[inline]
    pub fn get_render_pipeline(&self, id: CachedRenderPipelineId) -> Option<&RenderPipeline> {
        if let CachedPipelineState::Ok(
            Pipeline::RenderPipeline(pipeline) | Pipeline::MeshPipeline(pipeline),
        ) = &self.pipelines.get(id.id())?.state
        {
            Some(pipeline)
        } else {
            None
        }
    }

    #[inline]
    pub fn get_compute_pipeline(&self, id: CachedComputePipelineId) -> Option<&ComputePipeline> {
        if let CachedPipelineState::Ok(Pipeline::ComputePipeline(pipeline)) =
            &self.pipelines.get(id.id())?.state
        {
            Some(pipeline)
        } else {
            None
        }
    }

    pub fn queue_render_pipeline(
        &self,
        descriptor: RenderPipelineDescriptor,
    ) -> CachedRenderPipelineId {
        let mut new_pipelines = self
            .new_pipelines
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let id = CachedRenderPipelineId::new(self.pipelines.len() + new_pipelines.len());
        new_pipelines.push(CachedPipeline {
            descriptor: PipelineDescriptor::RenderPipelineDescriptor(Box::new(descriptor)),
            state: CachedPipelineState::Queued,
        });
        id
    }

    pub fn queue_compute_pipeline(
        &self,
        descriptor: ComputePipelineDescriptor,
    ) -> CachedComputePipelineId {
        let mut new_pipelines = self
            .new_pipelines
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let id = CachedComputePipelineId::new(self.pipelines.len() + new_pipelines.len());
        new_pipelines.push(CachedPipeline {
            descriptor: PipelineDescriptor::ComputePipelineDescriptor(Box::new(descriptor)),
            state: CachedPipelineState::Queued,
        });
        id
    }

    pub fn queue_mesh_pipeline(
        &self,
        descriptor: MeshPipelineDescriptor,
    ) -> CachedRenderPipelineId {
        let mut new_pipelines = self
            .new_pipelines
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let id = CachedRenderPipelineId::new(self.pipelines.len() + new_pipelines.len());
        new_pipelines.push(CachedPipeline {
            descriptor: PipelineDescriptor::MeshPipelineDescriptor(Box::new(descriptor)),
            state: CachedPipelineState::Queued,
        });
        id
    }

    pub fn get_bind_group_layout(
        &self,
        bind_group_layout_descriptor: &BindGroupLayoutDescriptor,
    ) -> BindGroupLayout {
        self.bindgroup_layout_cache
            .lock()
            .unwrap()
            .get(&self.device, bind_group_layout_descriptor)
    }

    pub fn set_shader(&mut self, id: AssetId<Shader>, shader: Shader) {
        let pipelines_to_queue: Vec<CachedPipelineId> = self.shader_cache.lock().unwrap().set_shader(id, shader).into_iter().collect();
        for cached_pipeline in pipelines_to_queue {
            self.pipelines[cached_pipeline].state = CachedPipelineState::Queued;
            self.enqueue(cached_pipeline);
        }
    }

    pub fn remove_shader(&mut self, shader: AssetId<Shader>) {
        let pipelines_to_queue: Vec<CachedPipelineId> = self.shader_cache.lock().unwrap().remove(shader).into_iter().collect();
        for cached_pipeline in pipelines_to_queue {
            self.pipelines[cached_pipeline].state = CachedPipelineState::Queued;
            self.enqueue(cached_pipeline);
        }
    }

    fn start_create_render_pipeline(
        &mut self,
        id: CachedPipelineId,
        descriptor: RenderPipelineDescriptor,
    ) -> CachedPipelineState {
        let device = self.device.clone();
        let shader_cache = self.shader_cache.clone();
        let layout_cache = self.layout_cache.clone();
        let mut bindgroup_layout_cache = self.bindgroup_layout_cache.lock().unwrap();
        let bind_group_layout = descriptor
            .layout
            .iter()
            .map(|bind_group_layout_descriptor| {
                bindgroup_layout_cache.get(&self.device, bind_group_layout_descriptor)
            })
            .collect::<SmallVec<[_; BIND_GROUP_LAYOUTS_INLINE_CAPACITY]>>();

        create_pipeline_task(
            async move {
                let mut shader_cache = shader_cache.lock().unwrap();
                let mut layout_cache = layout_cache.lock().unwrap();

                let vertex_module = match shader_cache.get(
                    id,
                    descriptor.vertex.shader.id(),
                    &descriptor.vertex.shader_defs,
                ) {
                    Ok(module) => module,
                    Err(err) => return Err(err),
                };

                let fragment_module = match &descriptor.fragment {
                    Some(fragment) => {
                        match shader_cache.get(id, fragment.shader.id(), &fragment.shader_defs) {
                            Ok(module) => Some(module),
                            Err(err) => return Err(err),
                        }
                    }
                    None => None,
                };

                let layout = if descriptor.layout.is_empty() && descriptor.immediate_size == 0 {
                    None
                } else {
                    Some(layout_cache.get(&device, &bind_group_layout, descriptor.immediate_size))
                };

                drop((shader_cache, layout_cache));

                let vertex_buffer_layouts = descriptor
                    .vertex
                    .buffers
                    .iter()
                    .map(|layout| {
                        Some(RawVertexBufferLayout {
                            array_stride: layout.array_stride,
                            attributes: &layout.attributes,
                            step_mode: layout.step_mode,
                        })
                    })
                    .collect::<Vec<_>>();

                let fragment_data = descriptor.fragment.as_ref().map(|fragment| {
                    (
                        fragment_module.unwrap(),
                        fragment.entry_point.as_deref(),
                        fragment.targets.as_slice(),
                        fragment
                            .constants
                            .iter()
                            .map(|(k, v)| (k.as_ref(), *v))
                            .collect::<Vec<_>>(),
                    )
                });

                let vertex_constants: Vec<(&str, f64)> = descriptor
                    .vertex
                    .constants
                    .iter()
                    .map(|(k, v)| (k.as_ref(), *v))
                    .collect();

                let descriptor = RawRenderPipelineDescriptor {
                    multiview_mask: None,
                    depth_stencil: descriptor.depth_stencil.clone(),
                    label: descriptor.label.as_deref(),
                    layout: layout.as_ref().map(|layout| -> &PipelineLayout { layout }),
                    multisample: descriptor.multisample,
                    primitive: descriptor.primitive,
                    vertex: RawVertexState {
                        buffers: &vertex_buffer_layouts,
                        entry_point: descriptor.vertex.entry_point.as_deref(),
                        module: &vertex_module,
                        compilation_options: PipelineCompilationOptions {
                            constants: &vertex_constants,
                            zero_initialize_workgroup_memory: descriptor
                                .zero_initialize_workgroup_memory,
                        },
                    },
                    fragment: fragment_data.as_ref().map(
                        |(module, entry_point, targets, constants)| RawFragmentState {
                            entry_point: entry_point.as_deref(),
                            module,
                            targets,
                            compilation_options: PipelineCompilationOptions {
                                constants,
                                zero_initialize_workgroup_memory: descriptor
                                    .zero_initialize_workgroup_memory,
                            },
                        },
                    ),
                    cache: None,
                };

                Ok(Pipeline::RenderPipeline(
                    device.create_render_pipeline(&descriptor),
                ))
            },
        )
    }

    fn start_create_compute_pipeline(
        &mut self,
        id: CachedPipelineId,
        descriptor: ComputePipelineDescriptor,
    ) -> CachedPipelineState {
        let device = self.device.clone();
        let shader_cache = self.shader_cache.clone();
        let layout_cache = self.layout_cache.clone();
        let mut bindgroup_layout_cache = self.bindgroup_layout_cache.lock().unwrap();
        let bind_group_layout = descriptor
            .layout
            .iter()
            .map(|bind_group_layout_descriptor| {
                bindgroup_layout_cache.get(&self.device, bind_group_layout_descriptor)
            })
            .collect::<SmallVec<[_; BIND_GROUP_LAYOUTS_INLINE_CAPACITY]>>();

        create_pipeline_task(
            async move {
                let mut shader_cache = shader_cache.lock().unwrap();
                let mut layout_cache = layout_cache.lock().unwrap();

                let compute_module =
                    match shader_cache.get(id, descriptor.shader.id(), &descriptor.shader_defs) {
                        Ok(module) => module,
                        Err(err) => return Err(err),
                    };

                let layout = if descriptor.layout.is_empty() && descriptor.immediate_size == 0 {
                    None
                } else {
                    Some(layout_cache.get(&device, &bind_group_layout, descriptor.immediate_size))
                };

                drop((shader_cache, layout_cache));

                let constants: Vec<(&str, f64)> = descriptor
                    .constants
                    .iter()
                    .map(|(k, v)| (k.as_ref(), *v))
                    .collect();
                let descriptor = RawComputePipelineDescriptor {
                    label: descriptor.label.as_deref(),
                    layout: layout.as_ref().map(|layout| -> &PipelineLayout { layout }),
                    module: &compute_module,
                    entry_point: descriptor.entry_point.as_deref(),
                    compilation_options: PipelineCompilationOptions {
                        constants: &constants,
                        zero_initialize_workgroup_memory: descriptor
                            .zero_initialize_workgroup_memory,
                    },
                    cache: None,
                };

                Ok(Pipeline::ComputePipeline(
                    device.create_compute_pipeline(&descriptor),
                ))
            },
        )
    }

    fn start_create_mesh_pipeline(
        &mut self,
        id: CachedPipelineId,
        descriptor: MeshPipelineDescriptor,
    ) -> CachedPipelineState {
        let device = self.device.clone();
        let shader_cache = self.shader_cache.clone();
        let layout_cache = self.layout_cache.clone();
        let mut bindgroup_layout_cache = self.bindgroup_layout_cache.lock().unwrap();
        let bind_group_layout = descriptor
            .layout
            .iter()
            .map(|bind_group_layout_descriptor| {
                bindgroup_layout_cache.get(&self.device, bind_group_layout_descriptor)
            })
            .collect::<SmallVec<[_; BIND_GROUP_LAYOUTS_INLINE_CAPACITY]>>();

        create_pipeline_task(
            async move {
                let mut shader_cache = shader_cache.lock().unwrap();
                let mut layout_cache = layout_cache.lock().unwrap();

                let mesh_module = match shader_cache.get(
                    id,
                    descriptor.mesh.shader.id(),
                    &descriptor.mesh.shader_defs,
                ) {
                    Ok(module) => module,
                    Err(err) => return Err(err),
                };

                let fragment_module = match &descriptor.fragment {
                    Some(fragment) => {
                        match shader_cache.get(id, fragment.shader.id(), &fragment.shader_defs) {
                            Ok(module) => Some(module),
                            Err(err) => return Err(err),
                        }
                    }
                    None => None,
                };

                let task_module = match &descriptor.task {
                    Some(task) => match shader_cache.get(id, task.shader.id(), &task.shader_defs) {
                        Ok(module) => Some(module),
                        Err(err) => return Err(err),
                    },
                    None => None,
                };

                let layout = if descriptor.layout.is_empty() && descriptor.immediate_size == 0 {
                    None
                } else {
                    Some(layout_cache.get(&device, &bind_group_layout, descriptor.immediate_size))
                };

                drop((shader_cache, layout_cache));

                let fragment_data = descriptor.fragment.as_ref().map(|fragment| {
                    (
                        fragment_module.unwrap(),
                        fragment.entry_point.as_deref(),
                        fragment.targets.as_slice(),
                        fragment
                            .constants
                            .iter()
                            .map(|(k, v)| (k.as_ref(), *v))
                            .collect::<Vec<_>>(),
                    )
                });

                let mesh_constants: Vec<(&str, f64)> = descriptor
                    .mesh
                    .constants
                    .iter()
                    .map(|(k, v)| (k.as_ref(), *v))
                    .collect();

                let task_data = descriptor.task.as_ref().map(|task| {
                    (
                        task_module.unwrap(),
                        task.entry_point.as_deref(),
                        task.constants
                            .iter()
                            .map(|(k, v)| (k.as_ref(), *v))
                            .collect::<Vec<_>>(),
                    )
                });

                let descriptor = RawMeshPipelineDescriptor {
                    multiview: None,
                    depth_stencil: descriptor.depth_stencil.clone(),
                    label: descriptor.label.as_deref(),
                    layout: layout.as_ref().map(|layout| -> &PipelineLayout { layout }),
                    multisample: descriptor.multisample,
                    primitive: descriptor.primitive,
                    task: task_data
                        .as_ref()
                        .map(|(module, entry_point, constants)| RawTaskState {
                            entry_point: entry_point.as_deref(),
                            module,
                            compilation_options: PipelineCompilationOptions {
                                constants,
                                zero_initialize_workgroup_memory: descriptor
                                    .zero_initialize_workgroup_memory,
                            },
                        }),
                    mesh: RawMeshState {
                        module: &mesh_module,
                        entry_point: descriptor.mesh.entry_point.as_deref(),
                        compilation_options: PipelineCompilationOptions {
                            constants: &mesh_constants,
                            zero_initialize_workgroup_memory: descriptor
                                .zero_initialize_workgroup_memory,
                        },
                    },
                    fragment: fragment_data.as_ref().map(
                        |(module, entry_point, targets, constants)| RawFragmentState {
                            entry_point: entry_point.as_deref(),
                            module,
                            targets,
                            compilation_options: PipelineCompilationOptions {
                                constants,
                                zero_initialize_workgroup_memory: descriptor
                                    .zero_initialize_workgroup_memory,
                            },
                        },
                    ),
                    cache: None,
                };

                Ok(Pipeline::MeshPipeline(
                    device.create_mesh_pipeline(&descriptor),
                ))
            },
        )
    }
    pub fn process_queue_under(&mut self, policy: PipelineQueuePolicy) {
        self.admit_new_pipelines();
        if !policy.open {
            return;
        }
        let started = Instant::now();
        let mut ran_step = false;
        let mut attempts = self.pending.len();
        while attempts > 0 {
            attempts -= 1;
            let Some(&id) = self.pending.front() else {
                break;
            };
            if ran_step && !fits(policy.budget, started, self.cost.prepare_ms) {
                break;
            }
            let began = Instant::now();
            let prepared = self.prepare_shaders(id);
            self.cost.prepare_ms = blend(self.cost.prepare_ms, began.elapsed());
            ran_step = true;
            if let Err(err) = prepared {
                self.pending.pop_front();
                self.settle_failed(id, err);
                continue;
            }
            if !fits(policy.budget, started, self.cost.create_ms) {
                break;
            }
            self.pending.pop_front();
            let began = Instant::now();
            self.create_pipeline(id);
            self.cost.create_ms = blend(self.cost.create_ms, began.elapsed());
        }
    }

    fn admit_new_pipelines(&mut self) {
        let mut new_pipelines = self
            .new_pipelines
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        for new_pipeline in new_pipelines.drain(..) {
            let id = self.pipelines.len();
            self.pipelines.push(new_pipeline);
            self.pending.push_back(id);
        }
    }

    fn enqueue(&mut self, id: CachedPipelineId) {
        if !self.pending.contains(&id) {
            self.pending.push_back(id);
        }
    }

    fn prepare_shaders(&self, id: CachedPipelineId) -> Result<(), ShaderCacheError> {
        let mut shader_cache = self.shader_cache.lock().unwrap();
        match &self.pipelines[id].descriptor {
            PipelineDescriptor::RenderPipelineDescriptor(descriptor) => {
                shader_cache.get(id, descriptor.vertex.shader.id(), &descriptor.vertex.shader_defs)?;
                if let Some(fragment) = &descriptor.fragment {
                    shader_cache.get(id, fragment.shader.id(), &fragment.shader_defs)?;
                }
            }
            PipelineDescriptor::ComputePipelineDescriptor(descriptor) => {
                shader_cache.get(id, descriptor.shader.id(), &descriptor.shader_defs)?;
            }
            PipelineDescriptor::MeshPipelineDescriptor(descriptor) => {
                if let Some(task) = &descriptor.task {
                    shader_cache.get(id, task.shader.id(), &task.shader_defs)?;
                }
                shader_cache.get(id, descriptor.mesh.shader.id(), &descriptor.mesh.shader_defs)?;
                if let Some(fragment) = &descriptor.fragment {
                    shader_cache.get(id, fragment.shader.id(), &fragment.shader_defs)?;
                }
            }
        }
        Ok(())
    }

    fn create_pipeline(&mut self, id: CachedPipelineId) {
        let state = match &self.pipelines[id].descriptor {
            PipelineDescriptor::RenderPipelineDescriptor(descriptor) => {
                let descriptor = descriptor.clone();
                self.start_create_render_pipeline(id, *descriptor)
            }
            PipelineDescriptor::ComputePipelineDescriptor(descriptor) => {
                let descriptor = descriptor.clone();
                self.start_create_compute_pipeline(id, *descriptor)
            }
            PipelineDescriptor::MeshPipelineDescriptor(descriptor) => {
                let descriptor = descriptor.clone();
                self.start_create_mesh_pipeline(id, *descriptor)
            }
        };
        match state {
            CachedPipelineState::Err(err) => self.settle_failed(id, err),
            state => self.pipelines[id].state = state,
        }
    }

    fn settle_failed(&mut self, id: CachedPipelineId, err: ShaderCacheError) {
        match &err {
            ShaderCacheError::ShaderNotLoaded(_) | ShaderCacheError::ShaderImportNotYetAvailable => {
                bevy_log::debug!("retry processing pipeline {id}: {err}");
                self.pipelines[id].state = CachedPipelineState::Queued;
                self.enqueue(id);
            }
            ShaderCacheError::ProcessShaderError(error_detail) => {
                if std::env::var("VERBOSE_SHADER_ERROR")
                    .is_ok_and(|v| !(v.is_empty() || v == "0" || v == "false"))
                {
                    error!("{}", pipeline_error_context(&self.pipelines[id]));
                }
                error!("failed to process shader error:\n{}", error_detail);
                self.pipelines[id].state = CachedPipelineState::Err(err);
            }
            ShaderCacheError::CreateShaderModule(description) => {
                error!("failed to create shader module: {}", description);
                self.pipelines[id].state = CachedPipelineState::Err(err);
            }
        }
    }

    pub(crate) fn process_pipeline_queue_system(
        mut cache: ResMut<Self>,
        policy: Option<Res<PipelineQueuePolicy>>,
    ) {
        let policy = policy.map_or(PipelineQueuePolicy::UNLIMITED, |policy| *policy);
        cache.process_queue_under(policy);
    }

    pub(crate) fn extract_shaders(
        mut cache: ResMut<Self>,
        shaders: Extract<Res<Assets<Shader>>>,
        mut events: Extract<MessageReader<AssetEvent<Shader>>>,
    ) {
        if cache.needs_shader_reload {
            cache.needs_shader_reload = false;
            for (id, shader) in shaders.iter() {
                let mut shader = shader.clone();
                shader
                    .shader_defs
                    .splice(0..0, cache.global_shader_defs.iter().cloned());
                cache.set_shader(id, shader);
            }
            for _ in events.read() {}
            return;
        }

        for event in events.read() {
            #[expect(
                clippy::match_same_arms,
                reason = "LoadedWithDependencies is marked as a TODO, so it's likely this will no longer lint soon."
            )]
            match event {
                AssetEvent::Added { id } | AssetEvent::Modified { id } => {
                    if let Some(shader) = shaders.get(*id) {
                        let mut shader = shader.clone();
                        shader
                            .shader_defs
                            .splice(0..0, cache.global_shader_defs.iter().cloned());

                        cache.set_shader(*id, shader);
                    }
                }
                AssetEvent::Removed { id } => cache.remove_shader(*id),
                AssetEvent::Unused { .. } => {}
                AssetEvent::LoadedWithDependencies { .. } => {
                }
            }
        }
    }
}

fn pipeline_error_context(cached_pipeline: &CachedPipeline) -> String {
    fn format(
        shader: &Handle<Shader>,
        entry: &Option<Cow<'static, str>>,
        shader_defs: &[ShaderDefVal],
    ) -> String {
        let source = match shader.path() {
            Some(path) => path.path().to_string_lossy().to_string(),
            None => String::new(),
        };
        let entry = match entry {
            Some(entry) => entry.to_string(),
            None => String::new(),
        };
        let shader_defs = shader_defs
            .iter()
            .flat_map(|def| match def {
                ShaderDefVal::Bool(k, v) if *v => Some(k.to_string()),
                ShaderDefVal::Int(k, v) => Some(format!("{k} = {v}")),
                ShaderDefVal::UInt(k, v) => Some(format!("{k} = {v}")),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(", ");
        format!("{source}:{entry}\nshader defs: {shader_defs}")
    }
    match &cached_pipeline.descriptor {
        PipelineDescriptor::RenderPipelineDescriptor(desc) => {
            let vert = &desc.vertex;
            let vert_str = format(&vert.shader, &vert.entry_point, &vert.shader_defs);
            let Some(frag) = desc.fragment.as_ref() else {
                return vert_str;
            };
            let frag_str = format(&frag.shader, &frag.entry_point, &frag.shader_defs);
            format!("vertex {vert_str}\nfragment {frag_str}")
        }
        PipelineDescriptor::ComputePipelineDescriptor(desc) => {
            format(&desc.shader, &desc.entry_point, &desc.shader_defs)
        }
        PipelineDescriptor::MeshPipelineDescriptor(desc) => {
            let mesh = &desc.mesh;
            let mesh_str = format(&mesh.shader, &mesh.entry_point, &mesh.shader_defs);

            let task = &desc.task;
            let task_str = task
                .as_ref()
                .map(|task| format(&task.shader, &task.entry_point, &task.shader_defs));

            let frag = &desc.fragment;
            let frag_str = frag
                .as_ref()
                .map(|frag| format(&frag.shader, &frag.entry_point, &frag.shader_defs));

            match (task_str, frag_str) {
                (None, None) => format!("mesh {mesh_str}"),
                (None, Some(frag_str)) => format!("mesh {mesh_str}\nfragment {frag_str}"),
                (Some(task_str), None) => format!("task {task_str}\nmesh {mesh_str}"),
                (Some(task_str), Some(frag_str)) => {
                    format!("task {task_str}\nmesh {mesh_str}\nfragment {frag_str}")
                }
            }
        }
    }
}

fn create_pipeline_task(task: impl Future<Output = Result<Pipeline, ShaderCacheError>>) -> CachedPipelineState {
    match bevy_tasks::block_on(task) {
        Ok(pipeline) => CachedPipelineState::Ok(pipeline),
        Err(err) => CachedPipelineState::Err(err),
    }
}
