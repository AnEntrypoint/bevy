import { clamp, stepAngle } from 'builtin/math';
import { add, normalize } from 'builtin/vec3';
import { createPath } from 'builtin/path';
import { fbm2 } from 'builtin/noise';
import { raycast, overlapPoint } from 'builtin/physics';
import { splineLength, sampleSpline } from 'builtin/spline';
import { solveShot } from 'builtin/ballistics';
import { shore } from 'builtin/terrain';
import { part } from 'builtin/voxels';
import { note, s, stack } from 'builtin/vibe';
import { grade } from 'builtin/postfx';
import { MeshStandardNodeMaterial } from 'builtin/three';
import { vec3 as tvec3 } from 'builtin/tsl';
import { sunDirection } from 'builtin/lighting';

export function onJayzee(ctx) {
  ctx.self.state.count = (ctx.self.state.count ?? 0) + 1;
  ctx.self.session.started = true;
  ctx.self.anim.main = { clip: 'cdn/clip-idle.glb', loop: true, duration: 0.5 };
  ctx.self.vel = ctx.self.velocity;
  const path = createPath([[0, 0], [10, 0], [10, 10]], { closed: false });
  const vox = part().fill({ from: [0, 0, 0], to: [1, 1, 1], material: 'stone' });
  const material = new MeshStandardNodeMaterial({ color: tvec3(0.2, 0.4, 0.6) });
  ctx.log('spec-onJayzee', {
    id: ctx.self.id,
    place: ctx.place.name,
    mode: ctx.world.mode,
    gravity: ctx.place.gravity.y,
    clamp: clamp(3, 0, 2),
    angle: Math.round(stepAngle(0, 90, 0.016)),
    vec: normalize(add({ x: 0, y: 1, z: 0 }, { x: 1, y: 0, z: 0 })),
    pathLength: path.length,
    pathAt: path.at(5).x,
    noise: fbm2({ x: 0.5, z: 0.5, octaves: 2 }),
    shot: solveShot({ x: 0, y: 0, z: 0 }, { x: 10, y: 0, z: 0 }, 20, 9.81) !== null,
    shore: shore(-2, 0),
    voxels: vox.serialize().voxels.length,
    vibe: stack(s('bd'), note(60)).events.length,
    grade: grade(ctx.scene, { exposure: 1 }).__postfx,
    material: material.constructor.name,
    sun: sunDirection.y > 0,
    field: ctx.readField('terrain', 0, 0),
    water: ctx.getWaterLevelAt(0, 0),
    normal: ctx.getTerrainNormal(0, 0),
  });
  ctx.self.forces.fan = { force: { x: 2, y: 0, z: 0 } };
  ctx.self.joints.hinge = { to: 'anchor', type: 'revolute', axis: { x: 0, y: 1, z: 0 }, limits: [-1, 1] };
  ctx.on('ping', (event, payload) => { ctx.log('spec-event', payload.n); });
  ctx.after(0.2, 'later', { value: 7 });
  ctx.emit('ping', { n: 5 });
}

export function later(ctx, payload) {
  ctx.log('spec-timer', payload.value, ctx.self.state.count);
}

export function update(ctx, dt) {
  const state = ctx.self.state;
  state.ticks = (state.ticks ?? 0) + 1;
  if (state.ticks === 1) {
    const ground = raycast(ctx, ctx.self.feetPosition, { x: 0, y: -1, z: 0 }, { distance: 4 });
    const overlap = overlapPoint(ctx, ctx.self.feetPosition);
    const hit = ctx.query({ tags: ['world'] });
    ctx.log('spec-update', {
      grounded: ctx.self.grounded,
      feet: ctx.self.feetPosition,
      velocity: ctx.self.velocity,
      ground: ground?.id ?? null,
      groundY: ground?.point?.y ?? null,
      overlap: overlap?.hits?.length ?? null,
      query: hit.length,
      other: ctx.getObject('ground')?.tags,
      worldPlayers: ctx.world.players.length,
      anim: ctx.self.anim.main,
      bounds: ctx.self.bounds ? Object.keys(ctx.self.bounds) : null,
    });
  }
  if (state.ticks === 3) ctx.log('spec-changed', ctx.changed('*').slice(0, 4));
  if (state.ticks === 20) {
    ctx.log('spec-anim', ctx.self.anim.main);
    ctx.log('spec-joints', ctx.getObject('ball').joints);
    ctx.self.joints.hinge = null;
  }
  if (state.ticks === 10) {
    ctx.jayzee({ id: 'spawned', primitive: { kind: 'sphere', radius: 0.3 }, feetPosition: { x: 1, y: 1, z: 1 }, tags: ['spawned'], lifetime: 0.1, behavior: 'scripts/destroy-hook.js' });
    ctx.jayzee({ id: 'child', primitive: { kind: 'box', width: 0.5, height: 0.5, depth: 0.5 }, feetPosition: { x: 0, y: 0, z: 0 } });
    ctx.attach('child', 'ball', { position: { x: 0, y: 2, z: 0 } });
    ctx.log('spec-spawn', { spawned: Boolean(ctx.getObject('spawned')), parent: ctx.getObject('child').parent?.id, children: ctx.getObject('ball').children.length });
  }
  if (state.ticks === 20) ctx.log('spec-destroy', { spawned: ctx.getObject('spawned'), child: Boolean(ctx.getObject('child')) });
  if (state.ticks === 22) ctx.destroy('child');
  if (state.ticks === 24) ctx.log('spec-destroyed-child', { child: ctx.getObject('child') });
  if (state.ticks === 30) {
    const landed = ctx.enterPlace({ place: 'two' });
    ctx.log('spec-enter', { landed, world: ctx.world.defaultPlace, mode: ctx.world.mode, placeMode: ctx.place.mode });
  }
  if (state.ticks === 35) ctx.log('spec-two', { objects: ctx.query({ place: 'two' }).length, stone: ctx.getObject('stone')?.tags, gravity: ctx.place.gravity.y });
  if (state.ticks === 40) {
    ctx.log('spec-final', { ticks: state.ticks, pos: ctx.self.feetPosition, joints: ctx.self.joints, mode: ctx.place.mode });
  }
  if (state.ticks === 60) {
    ctx.log('spec-spline', { length: splineLength(ctx, 'rail'), sample: sampleSpline(ctx, 'rail', 0.5) });
    ctx.self.spline = { id: 'rail', at: 0.5, orient: true };
  }
  if (state.ticks === 70) ctx.log('spec-spline-pos', ctx.self.feetPosition);
  const v = ctx.self.velocity;
  if (v.y > -20) ctx.self.velocity = { x: state.ticks === 1 ? 5 : v.x, y: v.y - 9.81 * dt, z: v.z };
  if (state.ticks === 100) ctx.log('spec-ground', { grounded: ctx.self.grounded, touching: ctx.self.touching, y: ctx.self.feetPosition.y, x: ctx.self.feetPosition.x });
}
export function onDestroy(ctx) {
  ctx.log('spec-onDestroy', ctx.self.id);
}
export function onCollide(ctx, other, contact) {
  ctx.log('spec-collide', { other: other.id, normal: contact?.normal, point: contact?.point });
}
export function onTriggerEnter(ctx, other) {
  ctx.log('spec-trigger', { other: other.id, x: ctx.self.feetPosition.x });
}
export function onTriggerExit(ctx, other) {
  ctx.log('spec-trigger-exit', { other: other.id });
}

