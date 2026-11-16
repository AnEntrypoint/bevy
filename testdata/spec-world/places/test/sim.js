export const cadence = '1s';

export function tick(ctx) {
  ctx.place.state.ticks = (ctx.place.state.ticks ?? 0) + 1;
  if (ctx.place.state.ticks !== 6) return;
  const landed = ctx.enterPlace({ place: 'two' });
  ctx.log('spec-enter', { landed, world: ctx.world.defaultPlace, mode: ctx.world.mode, placeMode: ctx.place.mode });
}
