export const cadence = '100ms';

const anchors = [
  ['fixture-district', ['district'], { x: 0, y: 0.1, z: 0 }],
  ['fixture-event', ['event'], { x: 10, y: 0, z: 0 }],
  ['fixture-foe', ['unit'], { x: -10, y: 0.3, z: 0 }],
  ['fixture-pad', ['pad'], { x: 0, y: 0, z: 10 }],
  ['fixture-ping', ['pingmark'], { x: 0, y: 0, z: -10 }],
];

export function tick(ctx) {
  for (const [id, tags, feetPosition] of anchors) {
    if (!ctx.getObject(id)) ctx.spawn(id, { tags, feetPosition });
  }
}
