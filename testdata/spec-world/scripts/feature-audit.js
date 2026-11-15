import { createWorld, raycast as physicsRaycast, overlapPoint } from 'builtin/physics';
import { animate } from 'builtin/tween';
import { splineLength, sampleSpline, projectToSpline } from 'builtin/spline';
import { lerp, shortestArc } from 'builtin/math';
import { rotate, eulerFromRotation } from 'builtin/vec3';
import { easeInOutCubic } from 'builtin/easing';
import { findPath } from 'builtin/nav';
import { createPath } from 'builtin/path';
import { solveShot, elevation } from 'builtin/ballistics';
import { shore } from 'builtin/terrain';
import { simplex2, voronoi2, makeSeededRng } from 'builtin/noise';
import { part, material, checker, random } from 'builtin/voxels';
import { note, s, seq, arrange, scale, progression, arp, chord, above } from 'builtin/vibe';
import { pop, disc, sprite, rgb, gravity, orbit } from 'builtin/fx';
import { grade, vignette, bloom, pixelate } from 'builtin/postfx';
import { MeshStandardNodeMaterial, StorageTexture, Plane, SRGBColorSpace, FloatType, DataTexture, Vector3 } from 'builtin/three';
import { float, uv, Fn, textureStore, atomicAdd, instancedArray } from 'builtin/tsl';
import { fogColor, lightsAt } from 'builtin/lighting';
import { waterFresnel, waterFoam } from 'builtin/water';
import { envAt } from 'builtin/env';

export function onJayzee(ctx) {
  const world = createWorld({ gravity: { x: 0, y: -9.81, z: 0 } });
  const body = world.add({ collider: 'sphere', radius: 0.5, position: { x: 0, y: 2, z: 0 } });
  const bytes = world.save();
  world.step(1 / 64);
  const restored = world.restore(bytes);
  const vox = part();
  vox.set({ x: 2, y: 1, z: 0, material: material('stone') });
  vox.hollowBox({ from: [0, 0, 0], to: [2, 2, 2], material: checker('stone', 'air') });
  const serialized = vox.serialize();
  const path = createPath([[0, 0], [3, 0], [3, 4]], { closed: true, spacing: 1 });
  const rng = makeSeededRng('native-feature-audit');
  const shader = new MeshStandardNodeMaterial({ color: float(0.5) });
  shader.positionNode = uv();
  const storage = new StorageTexture(8, 8);
  const data = new DataTexture([], 2, 2);
  const vector = new Vector3(1, 2, 3);
  const buffer = instancedArray(4);
  atomicAdd(buffer.element(0), 1);
  textureStore(storage, uv(), float(1));
  const receipt = ctx.trade('apple', { with: 'local-player', for: 'orange' });
  ctx.notifyDm('native feature audit');
  ctx.job('local:probe', { seed: rng.int32() }, 'nativeJobDone', { from: ctx.self.id });
  ctx.pulseFx('native-fx', 'burst', { data: { n: 3 } });
  ctx.log('native-features', {
    physics: { world: Boolean(world.id), body: Boolean(body.id ?? body.__nativeBody !== undefined), bytes: bytes.length, restored },
    worldRay: Boolean(physicsRaycast(world, { x: 0, y: 4, z: 0 }, { x: 0, y: -1, z: 0 }, 5)),
    overlap: overlapPoint(world, { x: 0, y: 2, z: 0 }).hits.length,
    spline: { length: splineLength(ctx, 'rail'), sample: Boolean(sampleSpline(ctx, 'rail', 0.5)), project: Boolean(projectToSpline(ctx, 'rail', { x: 0, y: 0, z: -6 })) },
    math: { lerp: lerp(0, 10, 0.25), arc: shortestArc(350, 10), ease: easeInOutCubic(0.5) },
    vec: { rotated: rotate({ yaw: 90 }, { x: 1, y: 0, z: 0 }), euler: eulerFromRotation({ x: 0, y: 0, z: 0, w: 1 }) },
    nav: findPath(ctx, { x: 0, y: 0, z: 0 }, { x: 4, y: 0, z: 4 }, { budget: 32 }),
    path: { length: path.length, nearest: Boolean(path.nearest(1, 1, 10)), at: Boolean(path.at(1)) },
    ballistics: { shot: Boolean(solveShot({ x: 0, y: 0, z: 0 }, { x: 5, y: 0, z: 0 }, 10, 9.81)), angle: elevation(10, 9.81, 5, 0) },
    terrain: shore(-1, 0),
    noise: { simplex: simplex2({ x: 1, z: 2, seed: 3 }), voronoi: voronoi2({ x: 1, z: 2, seed: 3 }).cell, rng: rng.double() },
    voxels: { version: serialized.version, palette: serialized.palette.length, cells: serialized.voxels.length, brush: Boolean(random(['a', 'b'])) },
    vibe: { events: seq(s('bd'), note(60)).events.length, arranged: arrange([1, note(64)]).length, scale: scale('C', [0, 2, 4]).length, progression: progression(['I', 'V'], 'C').length, arp: arp([chord('C')], 'up').length, shape: above(0.5, 0.75) },
    fx: { pop: Boolean(pop('spark', disc({ r: 1, jitter: 0.1 }))), sprite: Boolean(sprite({ texture: 'cdn/spark.png' })), color: rgb(1, 0, 0), gravity: Boolean(gravity()), orbit: Boolean(orbit({ radius: 1 })), state: Boolean(ctx.getFxState('native-fx')) },
    postfx: { grade: grade(ctx.scene, {}).__postfx, vignette: vignette(ctx.scene, 0.2, {}).__postfx, bloom: bloom(ctx.scene).__postfx, pixelate: pixelate(ctx.scene, 4).__postfx },
    three: { material: shader.constructor.name, storage: storage.constructor.name, data: data.constructor.name, plane: Plane, colorSpace: SRGBColorSpace, type: FloatType, vector: vector.args.length },
    tsl: { fn: typeof Fn(() => float(1)), uv: Boolean(uv()), buffer: Boolean(buffer.element(0)) },
    lighting: { fog: fogColor({ x: 0, y: 0, z: -1 }), lights: lightsAt({ x: 0, y: 0, z: 0 }) },
    water: { fresnel: waterFresnel(0.5, 0.02), foam: waterFoam({ x: 1, z: 2 }, 1, { x: 0, y: 0, z: 0 }, 1) },
    env: envAt(ctx, [1, 2, 3]),
    trade: receipt,
    timing: { dtMs: ctx.dtMs, gapMs: ctx.gapMs, constants: typeof ctx.constants, portal: ctx.portal },
    random: ctx.random(),
    fxState: ctx.getFxState('native-fx'),
  });
  animate(ctx, ctx.self.id, { 'state.auditTween': 1 }, { duration: 0.1, easing: 'easeInOutCubic' });
  ctx.self.clip = { near: 0.1, far: 100 };
  ctx.self.minPitch = -80;
  ctx.self.maxPitch = 80;
  ctx.self.projection = 'perspective';
  ctx.self.pixelSnap = true;
  ctx.self.pointerLock = 'drag';
  ctx.self.orientation = null;
  ctx.self.sensitivity = 1;
  ctx.self.hideLocalPlayer = false;
}

export function update(ctx, dt) {
  if (ctx.self.state.auditTicks === undefined) ctx.self.state.auditTicks = 0;
  ctx.self.state.auditTicks += 1;
}

export function nativeJobDone(ctx, result) {
  ctx.log('native-job-done', result);
}
