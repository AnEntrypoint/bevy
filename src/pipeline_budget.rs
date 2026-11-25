use bevy::app::App;
use bevy::prelude::*;
use bevy::render::render_resource::PipelineQueuePolicy;
use bevy::render::{Extract, ExtractSchedule, RenderApp};
use std::time::Duration;

pub const CREATE_BUDGET: Duration = Duration::from_micros(7900);

#[derive(Resource, Default)]
pub struct AppReady(pub bool);

pub struct PipelineBudgetPlugin;

impl Plugin for PipelineBudgetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AppReady>();
        let render = app.get_sub_app_mut(RenderApp).expect("the render sub-app");
        render.insert_resource(PipelineQueuePolicy { open: false, budget: Some(CREATE_BUDGET) });
        render.add_systems(ExtractSchedule, open_when_ready);
    }
}

fn open_when_ready(ready: Extract<Res<AppReady>>, mut policy: ResMut<PipelineQueuePolicy>) {
    if ready.0 {
        policy.open = true;
    }
}
