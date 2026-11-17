#!/usr/bin/env node
const { chromium } = require('/data/cache/npm-global/lib/node_modules/rea-agents/node_modules/playwright-core');
const url = process.argv[2] || 'http://127.0.0.1:8092/web/wasm/?inspector=1';
const out = process.argv[3] || '/tmp/bevy-inspector.png';
const flags = [
  '--no-sandbox', '--disable-gpu-sandbox', '--enable-unsafe-webgpu',
  '--enable-features=Vulkan,UseSkiaRenderer', '--use-angle=vulkan',
  '--ignore-gpu-blocklist', '--enable-webgpu-developer-features'
];
const required = {
  strip: ['FPS', 'MEMQ', 'MEMO', 'DRAW', 'SHOW', 'SIM', 'LAUNCH'],
  metric: ['DRAWS', 'SHADOW-MAP DRAWS', 'TRIANGLES', 'SHADOW TRIS', 'SHADOW PASSES', 'ENTITIES', 'MESHES', 'SPRITES', 'TEXTURES', 'TEXTURE BYTES'],
  bar: ['frame interval', 'CPU total', 'SIM', 'LAUNCH', 'DRAW', 'SHOW', 'GPU']
};
const numeric = /^-?\d+(\.\d+)?( (fps|ms|MiB|B))?$/;

function judge(snapshot, dom) {
  const failures = [];
  const shown = snapshot.shown || [];
  for (const [group, names] of Object.entries(required)) {
    for (const name of names) {
      if (!shown.some(m => m.group === group && m.name === name)) failures.push(`missing shown ${group} metric ${name}`);
    }
  }
  for (const m of shown) {
    if (m.value === null && !m.reason) failures.push(`stub: ${m.group}/${m.name} has no value and no reason (source: ${m.source})`);
    if (m.value !== null && !Number.isFinite(m.value)) failures.push(`non-finite value for ${m.group}/${m.name}`);
  }
  for (const [group, entries] of Object.entries(dom)) {
    if (entries.length !== shown.filter(m => m.group === group).length) failures.push(`DOM ${group} count ${entries.length} differs from snapshot`);
    for (const el of entries) {
      const label = `${group}/${el.name}`;
      if (/unavailable/i.test(el.text)) failures.push(`${label} still says unavailable`);
      if (el.text === 'unsupported') {
        if (!el.reason) failures.push(`${label} is unsupported without a reason`);
      } else if (!numeric.test(el.text)) {
        failures.push(`${label} shows "${el.text}", neither a number nor unsupported`);
      }
    }
  }
  return failures;
}

(async () => {
  const browser = await chromium.launch({ headless: false, executablePath: '/usr/bin/chromium', args: flags, env: { ...process.env, DISPLAY: process.env.DISPLAY || ':1.0' } });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const logs = [];
  page.on('console', m => logs.push(`${m.type()}: ${m.text()}`));
  page.on('pageerror', e => logs.push(`pageerror: ${e.message}`));
  await page.goto(url, { waitUntil: 'domcontentloaded', timeout: 30000 });
  await page.waitForTimeout(6000);
  const probe = await page.evaluate(() => {
    const read = (selector, attr) => [...document.querySelectorAll(selector)].map(el => ({
      name: el.dataset[attr],
      text: el.textContent.trim(),
      reason: el.querySelector('.unavailable')?.title || null
    }));
    return {
      inspector: document.getElementById('render-inspector')?.dataset.inspector ?? null,
      inspectorReady: document.getElementById('render-inspector')?.dataset.ready ?? null,
      appReady: window.__BEVY_WASM_READY === true,
      status: document.getElementById('app-status')?.outerHTML ?? null,
      snapshot: window.__BEVY_INSPECTOR_SNAPSHOT || null,
      dom: {
        strip: read('#inspector-strip [data-metric]', 'metric'),
        metric: read('#inspector-metrics [data-metric]', 'metric'),
        bar: read('#inspector-bars [data-bar]', 'bar')
      }
    };
  });
  await page.screenshot({ path: out });
  await browser.close();
  const snapshot = probe.snapshot || {};
  const failures = snapshot.kind === 'bevy-diagnostics' ? judge(snapshot, probe.dom) : ['no bevy-diagnostics snapshot was published'];
  const result = {
    ok: failures.length === 0,
    failures,
    inspector: probe.inspector,
    inspectorReady: probe.inspectorReady,
    appReady: probe.appReady,
    shown: (snapshot.shown || []).map(m => ({ group: m.group, name: m.name, value: m.value, unit: m.unit, source: m.source, reason: m.reason })),
    dom: probe.dom,
    unsupported: snapshot.unsupported || [],
    logs: logs.slice(-20),
    screenshot: out
  };
  console.log(JSON.stringify(result, null, 2));
  process.exit(result.ok ? 0 : 1);
})().catch(e => { console.error(e.stack || e); process.exit(1); });
