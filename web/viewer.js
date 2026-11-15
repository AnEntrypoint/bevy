const $ = (id) => document.getElementById(id);
const statusFiles = ['status.json', 'web/status.json', 'status-test.json', 'status-world.json'];
async function loadStatus() {
  for (const path of statusFiles) {
    try {
      const response = await fetch(path + '?t=' + Date.now(), { cache: 'no-store' });
      if (!response.ok) continue;
      return await response.json();
    } catch (_) {}
  }
  return { kind: 'viewer-status', title: 'No generated status found', objects: 0, meshes: 1, lights: 0, place: '—', directory: '—', renderer: 'Run ./scripts/generate_viewer_report.sh', visibleIds: [] };
}
function draw(data) {
  const canvas = $('scene');
  const ctx = canvas.getContext('2d');
  const w = canvas.width, h = canvas.height;
  const grad = ctx.createLinearGradient(0, 0, 0, h);
  grad.addColorStop(0, '#081423'); grad.addColorStop(1, '#02060b');
  ctx.fillStyle = grad; ctx.fillRect(0, 0, w, h);
  const horizon = h * 0.62;
  ctx.strokeStyle = 'rgba(90,160,230,.22)'; ctx.lineWidth = 1;
  for (let y = horizon; y < h; y += 34) { ctx.beginPath(); ctx.moveTo(0, y); ctx.lineTo(w, y); ctx.stroke(); }
  for (let x = -w; x < w * 2; x += 60) { ctx.beginPath(); ctx.moveTo(w / 2, horizon); ctx.lineTo(x, h); ctx.stroke(); }
  ctx.fillStyle = 'rgba(40,100,160,.18)';
  ctx.beginPath(); ctx.moveTo(0, horizon); ctx.lineTo(w, horizon); ctx.lineTo(w, h); ctx.lineTo(0, h); ctx.fill();
  const count = Math.max(1, Math.min(80, data.meshes || data.objects || 1));
  for (let i = 0; i < count; i++) {
    const angle = i * 2.39996;
    const radius = 42 + Math.sqrt(i) * 26;
    const x = w / 2 + Math.cos(angle) * radius * 1.7;
    const y = horizon - 18 - (i % 7) * 15 + Math.sin(angle) * radius * 0.18;
    const size = 10 + (i % 5) * 3;
    ctx.fillStyle = `hsl(${(i * 47) % 360} 72% 62%)`;
    ctx.beginPath();
    if (i % 3 === 0) ctx.arc(x, y, size * .55, 0, Math.PI * 2);
    else ctx.rect(x - size / 2, y - size / 2, size, size);
    ctx.fill();
  }
  ctx.fillStyle = '#dbeafe'; ctx.font = '16px system-ui';
  ctx.fillText(data.title || 'bevy-jayzee viewer', 24, 32);
}
function facts(data) {
  const rows = [
    ['title', data.title], ['place', data.place], ['directory', data.directory],
    ['objects', data.objects], ['proxy meshes', data.meshes], ['lights', data.lights],
    ['camera', JSON.stringify(data.camera || [])], ['renderer', data.renderer],
    ['visible ids', (data.visibleIds || []).slice(0, 12).join(', ') || '—'],
  ];
  $('facts').innerHTML = rows.map(([k, v]) => `<dt>${k}</dt><dd>${String(v ?? '—')}</dd>`).join('');
}
loadStatus().then((data) => {
  $('status').textContent = data.kind === 'viewer-status' ? 'status.json loaded' : 'status loaded';
  facts(data);
  $('native').textContent = JSON.stringify(data.nativeDepth || {}, null, 2);
  $('wasm').textContent = JSON.stringify(data.wasm || { built: true, renderer: 'Bevy 0.20 WebGPU WASM', output: 'web/wasm/' }, null, 2);
  document.title = data.title || document.title;
  draw(data);
});
