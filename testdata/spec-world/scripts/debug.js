export function update(ctx) {
  const ticks = (ctx.self.state.debugTicks = (ctx.self.state.debugTicks ?? 0) + 1);
  if (ticks === 1) {
    ctx.log('debug-parent', { parent: ctx.self.parent, mixer: ctx.self.mixer, bones: ctx.self.bones, tag: ctx.self.tags });
    globalThis.__host('setInput', JSON.stringify({ axes: { moveX: 1 }, pressed: ['jump'] }));
  }
  if (ticks === 3) ctx.log('debug-pos', { x: ctx.self.feetPosition.x, sleeping: ctx.self.sleeping });
}
