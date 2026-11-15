export function onInput(ctx, input) {
  ctx.log('move-input', { axes: input.axes, pressed: input.pressed, moveX: input.axes.moveX });
  if (input.axes.moveX) ctx.self.velocity = { x: input.axes.moveX * 3, y: ctx.self.velocity.y, z: 0 };
  if (input.pressed.jump) ctx.log('move-jump', ctx.self.grounded);
}
export function update(ctx) {
  if (!ctx.self.velocity || ctx.self.velocity.x === 0) ctx.sleep();
}
