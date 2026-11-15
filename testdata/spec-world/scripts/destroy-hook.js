export function onDestroy(ctx) {
  ctx.log('spec-onDestroy', ctx.self.id);
}
export const updateSchedule = { every: { seconds: 10 } };
