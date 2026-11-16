export function collider(ctx) {
  if (ctx.params.kind === 'sphere') return [{ kind: 'sphere', radius: 0.5, center: [0, 0.5, 0] }];
  if (ctx.params.kind === 'cylinder') return [{ kind: 'cylinder', radius: 0.4, height: 1, center: [0, 0.5, 0] }];
  if (ctx.params.kind === 'capsule') return [{ kind: 'capsule', radius: 0.3, height: 1.2, center: [0, 0.6, 0] }];
  return null;
}
