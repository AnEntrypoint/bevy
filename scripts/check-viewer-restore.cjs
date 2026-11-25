const fs = require('node:fs');
const { execFileSync } = require('node:child_process');
const { chromium } = require(process.env.PLAYWRIGHT_CORE || '/config/.gmnew/pw/node_modules/@xmorse/playwright-core');
const { readPng } = require('./lib/png-read.cjs');
const { detect } = require('./lib/streak-detect.cjs');

const url = process.env.VIEWER_URL || 'http://127.0.0.1:8093/web/wasm/index.html';
const cdp = process.env.CDP_URL || 'http://127.0.0.1:10232';
const out = process.env.OUT;
const mode = process.env.RESTORE_MODE || 'lifecycle';
const before = Number(process.env.BEFORE || 6);
const after = Number(process.env.AFTER || 12);
const hideS = Number(process.env.HIDE_S || 15);
const display = process.env.DISPLAY_FOR_WINDOW || ':1';
const minRun = 40;
const step = 24;

const recorder = () => {
  window.__events = [];
  const log = (entry) => window.__events.push({ t: Math.round(performance.now()), ...entry });
  const requestDevice = GPUAdapter.prototype.requestDevice;
  GPUAdapter.prototype.requestDevice = function (...args) {
    return requestDevice.apply(this, args).then((device) => {
      device.lost.then((info) => log({ ev: 'device-lost', reason: info.reason, message: String(info.message).slice(0, 160) }));
      device.addEventListener('uncapturederror', (e) => log({ ev: 'uncaptured', message: String(e.error && e.error.message).slice(0, 160) }));
      log({ ev: 'device-created' });
      return device;
    });
  };
  const observeCanvas = () => {
    const canvas = document.querySelector('canvas');
    if (!canvas) return setTimeout(observeCanvas, 100);
    new ResizeObserver((entries) => {
      for (const entry of entries) log({ ev: 'resize', w: Math.round(entry.contentRect.width), h: Math.round(entry.contentRect.height) });
    }).observe(canvas);
  };
  observeCanvas();
  document.addEventListener('visibilitychange', () => log({ ev: 'visibility', state: document.visibilityState }));
  window.__raf = [];
  const tick = (t) => {
    window.__raf.push(Math.round(t));
    requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);
};

function sampleOf(buffer) {
  const image = readPng(buffer);
  const runs = detect(image.width, image.height, image.rgb, 1, step);
  const longest = runs.reduce((max, run) => Math.max(max, run.len), 0);
  const over = runs.filter((run) => run.len >= minRun).length;
  return { w: image.width, h: image.height, longest, over40: over };
}

async function main() {
  fs.mkdirSync(out, { recursive: true });
  const browser = await chromium.connectOverCDP(cdp);
  const context = browser.contexts()[0];
  const page = await context.newPage();
  const consoleErrors = [];
  page.on('console', (msg) => { if (msg.type() === 'error' || msg.type() === 'warning') consoleErrors.push(msg.type() + ': ' + msg.text().slice(0, 160)); });
  page.on('pageerror', (err) => consoleErrors.push('pageerror: ' + String(err.message).slice(0, 160)));
  await page.addInitScript(recorder);
  await page.setViewportSize({ width: 1600, height: 900 });
  await page.goto(url, { waitUntil: 'load', timeout: 120000 });
  await page.waitForFunction(() => document.getElementById('app-status')?.dataset.appReady === 'true', null, { timeout: 240000, polling: 200 });
  const samples = [];
  const shot = async (phase, index) => {
    const meta = await page.evaluate(() => {
      const c = document.querySelector('canvas');
      const r = c.getBoundingClientRect();
      return { vis: document.visibilityState, backing: [c.width, c.height], clip: { x: r.left, y: r.top, width: r.width, height: r.height } };
    });
    const png = await page.screenshot({ type: 'png', clip: meta.clip });
    samples.push({ phase, index, t: Date.now(), vis: meta.vis, canvas: meta.backing, css: [meta.clip.width, meta.clip.height], ...sampleOf(png) });
  };
  for (let i = 0; i < before; i++) { await shot('before', i); await page.waitForTimeout(1000); }
  const cdpSession = await page.context().newCDPSession(page);
  let window = null;
  let minimize = null;
  if (mode === 'minimize') {
    const { targetInfo } = await cdpSession.send('Target.getTargetInfo');
    const browserSession = await browser.newBrowserCDPSession();
    const { windowId } = await browserSession.send('Browser.getWindowForTarget', { targetId: targetInfo.targetId });
    minimize = { browserSession, windowId };
  }
  if (mode === 'lifecycle') await cdpSession.send('Page.setWebLifecycleState', { state: 'frozen' });
  if (mode === 'tab') { const other = await context.newPage(); await other.bringToFront(); }
  if (mode === 'minimize') await minimize.browserSession.send('Browser.setWindowBounds', { windowId: minimize.windowId, bounds: { windowState: 'minimized' } });
  if (mode === 'window') {
    const title = await page.title();
    const ids = execFileSync('bash', ['-c', `DISPLAY=${display} xdotool search --class chromium`]).toString().trim().split('\n').filter(Boolean);
    const named = ids.map((id) => ({ id, name: execFileSync('bash', ['-c', `DISPLAY=${display} xdotool getwindowname ${id} 2>/dev/null || true`]).toString().trim() }));
    for (const w of named) console.log('window candidate ' + w.id + ' ' + JSON.stringify(w.name));
    window = (named.find((w) => w.name.includes(title) && title) || named.find((w) => /127\.0\.0\.1|bevy/.test(w.name)) || { id: named[0] && named[0].id }).id;
    console.log('window chosen ' + window + ' title ' + JSON.stringify(title));
    execFileSync('bash', ['-c', `DISPLAY=${display} xdotool windowunmap ${window}`]);
  }
  await page.waitForTimeout(hideS * 1000);
  const hiddenState = await page.evaluate(() => document.visibilityState).catch((error) => 'unreachable: ' + String(error.message).slice(0, 80));
  samples.push({ phase: 'hidden', index: 0, t: Date.now(), vis: hiddenState, canvas: null, css: null, w: 0, h: 0, longest: 0, over40: 0 });
  console.log('hidden state ' + hiddenState);
  if (mode === 'lifecycle') await cdpSession.send('Page.setWebLifecycleState', { state: 'active' });
  if (mode === 'tab') { await page.bringToFront(); }
  if (mode === 'window') execFileSync('bash', ['-c', `DISPLAY=${display} xdotool windowmap ${window}`]);
  if (mode === 'minimize') await minimize.browserSession.send('Browser.setWindowBounds', { windowId: minimize.windowId, bounds: { windowState: 'normal' } });
  for (let i = 0; i < after; i++) { await shot('after', i); await page.waitForTimeout(500); }
  const events = await page.evaluate(() => window.__events);
  const raf = await page.evaluate(() => window.__raf);
  const gaps = raf.slice(1).map((t, i) => t - raf[i]);
  const maxGap = Math.max(0, ...gaps);
  const at = gaps.indexOf(maxGap);
  console.log(`raf frames=${raf.length} maxGapMs=${maxGap} atFrame=${at} gapStartMs=${raf[at]} gapEndMs=${raf[at + 1]}`);
  fs.writeFileSync(`${out}/series.json`, JSON.stringify({ mode, hideS, samples, events, consoleErrors, raf: { frames: raf.length, maxGapMs: maxGap, atFrame: at } }, null, 1));
  for (const s of samples) console.log(`${s.phase} ${s.index} vis=${s.vis} backing=${JSON.stringify(s.canvas)} css=${JSON.stringify(s.css)} ${s.w}x${s.h} longest=${s.longest} over40=${s.over40}`);
  for (const e of events) console.log('event ' + JSON.stringify(e));
  console.log('console ' + consoleErrors.length + ' ' + consoleErrors.slice(0, 5).join(' | '));
  await page.close();
  await browser.close().catch(() => {});
}

main().catch((error) => { console.error('RESTORE_ERR ' + String(error.message).slice(0, 300)); process.exit(1); });
