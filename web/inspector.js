const tabs = ['Performance','GPU Passes','Frame Graph','Scene','Memory','Textures','Multiplayer','Log','Jobs','Terrain','Fields','World','ECS','Physics','Vibes','Parameters','Diagnostic'];
const metricNames = ['DRAWS','SHADOW-MAP DRAWS','TRIANGLES','SHADOW TRIS','SHADOW PASSES','CASCADE SKIPS','ENTITIES','MESHES','SPRITES','TERRAIN'];
const stripNames = ['FPS','MEMQ','MEMO','DRAW','SHOW','SIM','LAUNCH'];
const colors = ['#22d3ee','#a3e635','#f472b6','#fbbf24','#60a5fa','#c084fc','#34d399'];
const q = (id) => document.getElementById(id);
const root = q('render-inspector');
const query = new URLSearchParams(location.search);
let open = query.get('inspector') === '1' || query.has('inspector');
let activeTab = 'Performance';
let history = Object.fromEntries(stripNames.map(n => [n, []]));
const fmt = (v, suffix='') => v == null || !Number.isFinite(v) ? 'unavailable' : `${Number(v).toFixed(v > 100 ? 0 : 2)}${suffix}`;
const esc = (s) => String(s).replace(/[&<>"]/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;'}[c]));
function setOpen(value) {
  open = value;
  root.dataset.inspector = open ? 'open' : 'closed';
  window.__BEVY_INSPECTOR_ENABLED = open;
  q('inspector-toggle').setAttribute('aria-pressed', String(open));
  if (open) render({});
}
function sparkline(values, color) {
  const max = Math.max(1, ...values.map(v => Math.abs(v || 0)));
  const pts = values.map((v, i) => `${(i / Math.max(1, values.length - 1)) * 82},${13 - Math.min(13, Math.abs(v || 0) / max * 12)}`).join(' ');
  return `<svg class="spark" viewBox="0 0 82 14" role="img" aria-label="sparkline"><polyline points="${pts}" fill="none" stroke="${color}" stroke-width="1.5"/></svg>`;
}
function strip(snapshot) {
  const fps = value(snapshot, 'fps');
  const frameTime = value(snapshot, 'frame_time');
  const values = {
    FPS: fps, MEMQ: unavailableNumber(snapshot, 'memory'), MEMO: unavailableNumber(snapshot, 'memory'),
    DRAW: unavailableNumber(snapshot, 'draw'), SHOW: unavailableNumber(snapshot, 'show'),
    SIM: unavailableNumber(snapshot, 'sim'), LAUNCH: frameTime
  };
  q('inspector-strip').innerHTML = stripNames.map((name, i) => {
    const v = values[name];
    history[name].push(Number.isFinite(v) ? v : 0);
    history[name] = history[name].slice(-48);
    const range = name === 'FPS' ? '0–144' : '0–16.7ms';
    return `<span class="bucket" data-metric="${name}"><b>${name}</b> <span>${Number.isFinite(v) ? fmt(v) : 'unavailable'}</span> ${sparkline(history[name], colors[i])}<small>${range}</small></span>`;
  }).join('');
}
function value(snapshot, name) {
  const d = (snapshot.diagnostics || []).find(x => x.name === name);
  return d && Number.isFinite(d.value) ? d.value : null;
}
function unavailableNumber(_snapshot, _name) { return null; }
function metrics(snapshot) {
  const d = Object.fromEntries((snapshot.diagnostics || []).map(x => [x.name, x.value]));
  const ecs = snapshot.ecs || {}, assets = snapshot.assets || {};
  const values = {
    DRAWS: null, 'SHADOW-MAP DRAWS': null, TRIANGLES: null, 'SHADOW TRIS': null,
    'SHADOW PASSES': null, 'CASCADE SKIPS': null,
    ENTITIES: d.entity_count, MESHES: assets.meshes ?? ecs.meshEntities,
    SPRITES: ecs.sprites, TERRAIN: null
  };
  q('inspector-metrics').innerHTML = metricNames.map(name => {
    const v = values[name];
    return `<div class="metric" data-metric="${esc(name)}"><b>${Number.isFinite(v) ? fmt(v) : '<span class="unavailable">unavailable</span>'}</b><small>${esc(name)}</small></div>`;
  }).join('');
}
function panel(snapshot) {
  const render = snapshot.renderDiagnostics || [];
  const ecs = snapshot.ecs || {}, assets = snapshot.assets || {};
  const tabData = {
    'GPU Passes': render.length ? render.map(x => `${x.path}: ${fmt(x.value, 'ms')}`).join('\n') : 'Render pass timestamps unavailable: Bevy RenderDiagnosticsPlugin is Tracy-gated; WebGPU reports CPU spans only when enabled.',
    'Frame Graph': 'Frame graph unavailable: Bevy 0.20 does not expose a browser-readable graph through DiagnosticsStore.',
    Scene: `cameras=${ecs.cameras ?? 'unavailable'} lights=${ecs.lights ?? 'unavailable'} meshEntities=${ecs.meshEntities ?? 'unavailable'}`,
    Memory: `process memory=${performance.memory ? Math.round(performance.memory.usedJSHeapSize/1048576)+' MiB browser JS heap' : 'unavailable'}`,
    Textures: `image assets=${assets.images ?? 'unavailable'}; texture bytes unavailable`,
    Multiplayer: 'unavailable: wasm viewer intentionally excludes Lightyear networking',
    Log: 'Bevy LogDiagnosticsPlugin is enabled; see browser console for diagnostic logs',
    Jobs: 'unavailable: no browser job diagnostics exposed by this wasm scene',
    Terrain: 'unavailable: no terrain plugin in the wasm scene',
    Fields: 'unavailable: no field diagnostics registered',
    World: 'unavailable: filesystem world runtime is native-only',
    ECS: JSON.stringify(ecs),
    Physics: 'unavailable: Avian is native-only in this build',
    Vibes: 'unavailable: native feature state is not linked into the wasm scene',
    Parameters: JSON.stringify({source: snapshot.source, plugins: snapshot.plugins, adapter: navigator.gpu ? 'WebGPU detected' : 'unavailable'}),
    Diagnostic: JSON.stringify(snapshot, null, 2)
  };
  q('inspector-tab-title').textContent = activeTab;
  q('inspector-note').textContent = tabData[activeTab] || `Bevy DiagnosticsStore snapshot ${snapshot.time ?? 'unavailable'}`;
  q('inspector-json').textContent = JSON.stringify(snapshot.diagnostics || [], null, 2);
  q('inspector-source').textContent = `Source: ${snapshot.source || 'unavailable'} • Plugins: ${(snapshot.plugins || []).join(', ')} • Browser: ${navigator.gpu ? 'navigator.gpu available' : 'navigator.gpu unavailable'}`;
  const fps = value(snapshot, 'fps') ?? 0, frame = value(snapshot, 'frame_time') ?? 0;
  const bars = [
    ['frame interval', frame, 16.7, 'ms'], ['CPU total', frame, 16.7, 'ms'], ['FPS', fps, 144, ''],
    ['adapter CPU', null, 1, ''], ['Lume GPU renderer', null, 1, ''], ['process/queue', null, 1, ''],
    ['on-demand timing', null, 1, '']
  ];
  q('inspector-bars').innerHTML = bars.map(([name, v, max, unit]) => `<div><span>${name}</span><span class="bar"><i style="width:${Number.isFinite(v) ? Math.min(100, v/max*100) : 0}%"></i></span><span>${Number.isFinite(v) ? fmt(v, unit) : '<span class="unavailable">unavailable</span>'}</span></div>`).join('');
}
function render(snapshot) {
  strip(snapshot); metrics(snapshot); panel(snapshot);
  q('inspector-tabs').innerHTML = tabs.map(t => `<button role="tab" data-tab="${esc(t)}" aria-selected="${t === activeTab}">${esc(t)}</button>`).join('');
  q('inspector-tabs').querySelectorAll('button').forEach(b => b.onclick = () => { activeTab = b.dataset.tab; render(window.__BEVY_INSPECTOR_SNAPSHOT || {}); });
  root.dataset.ready = snapshot.kind === 'bevy-diagnostics' ? 'true' : 'false';
}
q('inspector-tabs').innerHTML = tabs.map(t => `<button role="tab" data-tab="${esc(t)}" aria-selected="${t === activeTab}">${esc(t)}</button>`).join('');
q('inspector-tabs').onclick = (e) => { const b = e.target.closest('button'); if (b) { activeTab = b.dataset.tab; render(window.__BEVY_INSPECTOR_SNAPSHOT || {}); } };
q('inspector-toggle').onclick = () => setOpen(!open);
q('inspector-close').onclick = () => setOpen(false);
q('inspector-collapse').onclick = () => { root.dataset.collapsed = root.dataset.collapsed === 'true' ? 'false' : 'true'; };
window.addEventListener('keydown', (e) => { if (e.key.toLowerCase() === 'i' && !/input|textarea/i.test(document.activeElement.tagName)) setOpen(!open); });
window.addEventListener('bevy-inspector', (e) => render(e.detail || {}));
setOpen(open);
setInterval(() => {
  let snapshot = window.__BEVY_INSPECTOR_SNAPSHOT;
  if (typeof snapshot === 'string') { try { snapshot = JSON.parse(snapshot); } catch (_) { snapshot = {}; } }
  window.__BEVY_INSPECTOR_SNAPSHOT = snapshot || {};
  if (open) render(window.__BEVY_INSPECTOR_SNAPSHOT);
}, 250);
window.__BEVY_INSPECTOR_CHECK = () => ({
  open, marker: root.dataset.inspector, tabs: tabs.length,
  metricNames, stripNames,
  snapshot: window.__BEVY_INSPECTOR_SNAPSHOT || null,
  domMetrics: [...document.querySelectorAll('#inspector-metrics [data-metric]')].map(x => x.dataset.metric)
});
