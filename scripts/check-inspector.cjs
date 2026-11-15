#!/usr/bin/env node
const { chromium } = require('/data/cache/npm-global/lib/node_modules/rea-agents/node_modules/playwright-core');
const url = process.argv[2] || 'http://127.0.0.1:8092/web/wasm/?inspector=1';
const out = process.argv[3] || '/tmp/bevy-inspector.png';
const flags = [
  '--no-sandbox', '--disable-gpu-sandbox', '--enable-unsafe-webgpu',
  '--enable-features=Vulkan,UseSkiaRenderer', '--use-angle=vulkan',
  '--ignore-gpu-blocklist', '--enable-webgpu-developer-features'
];
(async () => {
  const browser = await chromium.launch({ headless: false, executablePath: '/usr/bin/chromium', args: flags, env: { ...process.env, DISPLAY: process.env.DISPLAY || ':1.0' } });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const logs = [];
  page.on('console', m => logs.push(`${m.type()}: ${m.text()}`));
  page.on('pageerror', e => logs.push(`pageerror: ${e.message}`));
  await page.goto(url, { waitUntil: 'domcontentloaded', timeout: 30000 });
  await page.waitForTimeout(6000);
  const result = await page.evaluate(() => ({
    title: document.title,
    inspector: document.getElementById('render-inspector')?.dataset.inspector,
    inspectorReady: document.getElementById('render-inspector')?.dataset.ready,
    markers: {
      dataInspector: document.querySelector('[data-inspector]')?.getAttribute('data-inspector'),
      tabs: [...document.querySelectorAll('#inspector-tabs [data-tab]')].map(b => b.dataset.tab),
      metrics: [...document.querySelectorAll('#inspector-metrics [data-metric]')].map(b => b.dataset.metric),
      strip: [...document.querySelectorAll('#inspector-strip [data-metric]')].map(b => b.dataset.metric)
    },
    check: window.__BEVY_INSPECTOR_CHECK ? window.__BEVY_INSPECTOR_CHECK() : null,
    appReady: window.__BEVY_WASM_READY === true,
    status: document.getElementById('app-status')?.outerHTML,
    snapshotKind: window.__BEVY_INSPECTOR_SNAPSHOT?.kind || null,
    diagnostics: window.__BEVY_INSPECTOR_SNAPSHOT?.diagnostics || [],
    unavailable: window.__BEVY_INSPECTOR_SNAPSHOT?.unavailable || []
  }));
  await page.screenshot({ path: out });
  console.log(JSON.stringify({ result, logs: logs.slice(-20), screenshot: out }, null, 2));
  await browser.close();
})().catch(e => { console.error(e.stack || e); process.exit(1); });
