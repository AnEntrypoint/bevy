export function geometry(ctx) {
  ctx.color('oklch(0.7 0.1 30)');
  ctx.quad(0, 0, 0, 1, 0, 0, 1, 0, 1, 0, 0, 1);
  ctx.tri(0, 0, 0, 1, 0, 0, 0, 1, 0);
  return null;
}
export function collider(ctx) {
  const size = ctx.params.size ?? 1;
  return [{ kind: 'box', width: size, height: size, depth: size, center: [0, size * 0.5, 0] }];
}
