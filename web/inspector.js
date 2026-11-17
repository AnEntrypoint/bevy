const tabs = ['Performance','GPU Passes','Frame Graph','Scene','Memory','Textures','Multiplayer','Log','Jobs','Terrain','Fields','World','ECS','Physics','Vibes','Parameters','Diagnostic'];
const colors = ['var(--flag-green-text)','var(--flag-gold-text)','var(--flag-blue-text)','var(--ui-text)','var(--ui-muted)','var(--flag-green-text)','var(--flag-gold-text)'];
const q = (id) => document.getElementById(id);
const root = q('render-inspector');
const query = new URLSearchParams(location.search);
let open = query.get('inspector') === '1' || query.has('inspector');
let activeTab = 'Performance';
const timeline = {};
const fmt = (v) => Number.isInteger(v) ? String(v) : Number(v).toFixed(Math.abs(v) > 100 ? 0 : 2);
const mibOf = (bytes) => bytes == null ? 'unsupported' : `${(bytes / 1048576).toFixed(1)} MiB`;
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
  return `<svg class="spark" viewBox="0 0 82 14" role="img" aria-label="sparkline"><polyline points="${pts}" style="fill:none;stroke:${color};stroke-width:1.5"/></svg>`;
}
function shown(snapshot, group) {
  return (snapshot.shown || []).filter(m => m.group === group);
}
function cell(m) {
  if (Number.isFinite(m.value)) return `<span class="value">${fmt(m.value)}${m.unit ? ` ${esc(m.unit)}` : ''}</span>`;
  return `<span class="value"><span class="unavailable" title="${esc(m.reason || 'no reason given')}">unsupported</span></span>`;
}
function strip(snapshot) {
  q('inspector-strip').innerHTML = shown(snapshot, 'strip').map((m, i) => {
    const series = [...(timeline[m.name] || []), Number.isFinite(m.value) ? m.value : 0].slice(-48);
    timeline[m.name] = series;
    return `<span class="bucket" data-metric="${esc(m.name)}" title="${esc(m.source)}"><b>${esc(m.name)}</b> ${cell(m)} ${sparkline(series, colors[i % colors.length])}</span>`;
  }).join('');
}
function metrics(snapshot) {
  q('inspector-metrics').innerHTML = shown(snapshot, 'metric').map(m => `<div class="metric" data-metric="${esc(m.name)}" title="${esc(m.source)}"><b>${cell(m)}</b><small>${esc(m.name)}</small></div>`).join('');
}
function bars(snapshot) {
  q('inspector-bars').innerHTML = shown(snapshot, 'bar').map(m => {
    const width = Number.isFinite(m.value) && m.max > 0 ? Math.min(100, m.value / m.max * 100) : 0;
    return `<div data-bar="${esc(m.name)}" title="${esc(m.source)}"><span>${esc(m.name)}</span><span class="bar"><i style="width:${width}%"></i></span><span>${cell(m)}</span></div>`;
  }).join('');
}
function panel(snapshot) {
  const render = snapshot.renderDiagnostics || [];
  const ecs = snapshot.ecs || {}, assets = snapshot.assets || {}, memory = snapshot.memory || {}, gpu = snapshot.gpu || {};
  const gpuNote = gpu.timestampQueries
    ? (render.length ? render.map(x => `${x.path}: ${fmt(x.value)} ms`).join('\n') : 'unsupported: timestamp queries are enabled but no elapsed_gpu spans were recorded')
    : `unsupported: ${gpu.reason || 'TIMESTAMP_QUERY is not enabled on this device'}\ndevice features: ${gpu.deviceFeatures || 'unknown'}`;
  const tabData = {
    'GPU Passes': gpuNote,
    'Frame Graph': 'unsupported: Bevy 0.20 does not expose a browser-readable graph through DiagnosticsStore',
    Scene: `cameras=${ecs.cameras} lights=${ecs.lights} meshEntities=${ecs.meshEntities} sprites=${ecs.sprites} uiNodes=${ecs.uiNodes}`,
    Memory: `wasm linear memory=${mibOf(memory.wasmBytes)}\nJS heap=${mibOf(memory.jsHeapBytes)}${memory.jsHeapBytes == null ? ' (performance.memory is Chromium-only)' : ''}`,
    Textures: `images=${assets.images} CPU image bytes=${mibOf(assets.imageBytes)}\nGPU texture memory: unsupported, Bevy exposes no GPU texture byte counter`,
    Multiplayer: 'unsupported: wasm viewer intentionally excludes Lightyear networking',
    Log: 'Bevy LogDiagnosticsPlugin is enabled; see browser console for diagnostic logs',
    Jobs: 'unsupported: no browser job diagnostics exposed by this wasm scene',
    Terrain: 'unsupported: no terrain plugin in the wasm scene',
    Fields: 'unsupported: no field diagnostics registered',
    World: 'unsupported: filesystem world runtime is native-only',
    ECS: JSON.stringify(ecs),
    Physics: 'unsupported: Avian is native-only in this build',
    Vibes: 'unsupported: native feature state is not linked into the wasm scene',
    Parameters: JSON.stringify({source: snapshot.source, plugins: snapshot.plugins, adapter: navigator.gpu ? 'WebGPU detected' : 'unsupported: navigator.gpu is absent'}),
    Diagnostic: JSON.stringify(snapshot, null, 2)
  };
  q('inspector-tab-title').textContent = activeTab;
  q('inspector-note').textContent = tabData[activeTab] || `Bevy DiagnosticsStore snapshot ${snapshot.time ?? 'unsupported'}`;
  q('inspector-json').textContent = JSON.stringify(snapshot.diagnostics || [], null, 2);
  q('inspector-source').textContent = `Source: ${snapshot.source || 'unsupported'} • Plugins: ${(snapshot.plugins || []).join(', ')} • Browser: ${navigator.gpu ? 'navigator.gpu available' : 'navigator.gpu unavailable'}`;
  bars(snapshot);
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
window.__BEVY_INSPECTOR_CHECK = () => {
  let snapshot = window.__BEVY_INSPECTOR_SNAPSHOT;
  if (typeof snapshot === 'string') { try { snapshot = JSON.parse(snapshot); } catch (_) { snapshot = null; } }
  return { open, marker: root.dataset.inspector, tabs: tabs.length, shown: snapshot?.shown || [], snapshot: snapshot || null };
};
