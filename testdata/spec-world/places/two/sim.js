export const cadence = '1s';

export function tick(ctx) {
  ctx.place.state.ticks = (ctx.place.state.ticks ?? 0) + 1;
  if (ctx.place.state.ticks === 1) ctx.log('spec-two', { objects: ctx.query({ place: 'two' }).length, stone: ctx.getObject('stone')?.tags, gravity: ctx.place.gravity.y });
  if (ctx.place.state.ticks === 2) ctx.enterPlace({ place: 'test' });
}
