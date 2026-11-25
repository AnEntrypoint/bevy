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
      (window.__devices ||= []).push(device);
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
  const configure = GPUCanvasContext.prototype.configure;
  GPUCanvasContext.prototype.configure = function (config) {
    log({ ev: 'configure', w: config.width ?? this.canvas.width, h: config.height ?? this.canvas.height, format: config.format, dpr: window.devicePixelRatio });
    return configure.call(this, config);
  };
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
  const top = runs.reduce((best, run) => (run.len > best.len ? run : best), { x: -1, y0: -1, len: 0 });
  return { w: image.width, h: image.height, longest, over40: over, top: [top.x, top.y0, top.len] };
}

async function attachPage(context, targetId) {
  for (let attempt = 0; attempt < 100; attempt++) {
    for (const candidate of context.pages()) {
      const session = await candidate.context().newCDPSession(candidate);
      const { targetInfo } = await session.send('Target.getTargetInfo');
      if (targetInfo.targetId === targetId) return candidate;
    }
    await new Promise((resolve) => setTimeout(resolve, 200));
  }
  throw new Error('the viewer target was not found');
}

async function main() {
  fs.mkdirSync(out, { recursive: true });
  const browser = await chromium.connectOverCDP(cdp);
  const context = browser.contexts()[0];
  const bcdp = await browser.newBrowserCDPSession();
  const created = mode === 'activate' ? await bcdp.send('Target.createTarget', { url: 'about:blank', newWindow: false }) : null;
  const page = created ? await attachPage(context, created.targetId) : await context.newPage();
  let other = null;
  const consoleErrors = [];
  page.on('console', (msg) => { if (msg.type() === 'error' || msg.type() === 'warning') consoleErrors.push(msg.type() + ': ' + msg.text().slice(0, 160)); });
  page.on('pageerror', (err) => consoleErrors.push('pageerror: ' + String(err.message).slice(0, 160)));
  await page.addInitScript(recorder);
  await page.setViewportSize({ width: 1600, height: 900 });
  await page.goto(url, { waitUntil: 'load', timeout: 120000 });
  await page.waitForFunction(() => document.getElementById('app-status')?.dataset.appReady === 'true', null, { timeout: 240000, polling: 200 });
  const samples = [];
  const shot = async (phase, index, timeoutMs = 30000) => {
    const meta = await page.evaluate(() => {
      const c = document.querySelector('canvas');
      const r = c.getBoundingClientRect();
      return { vis: document.visibilityState, dpr: window.devicePixelRatio, backing: [c.width, c.height], clip: { x: r.left, y: r.top, width: r.width, height: r.height } };
    });
    const png = await page.screenshot({ type: 'png', clip: meta.clip, timeout: timeoutMs });
    samples.push({ phase, index, t: Date.now(), vis: meta.vis, dpr: meta.dpr, canvas: meta.backing, css: [meta.clip.width, meta.clip.height], ...sampleOf(png) });
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
  if (mode === 'wm-size') {
    const { targetInfo } = await cdpSession.send('Target.getTargetInfo');
    const title = await page.title();
    const ids = execFileSync('bash', ['-c', `DISPLAY=${display} xdotool search --class chromium`]).toString().trim().split('\n').filter(Boolean);
    const named = ids.map((id) => ({ id, name: execFileSync('bash', ['-c', `DISPLAY=${display} xdotool getwindowname ${id} 2>/dev/null || true`]).toString().trim() }));
    const target = (named.find((w) => title && w.name.includes(title)) || named.find((w) => /bevy/.test(w.name)) || { id: named[0] && named[0].id }).id;
    console.log('wm window ' + target + ' target ' + targetInfo.targetId);
    for (const [label, size] of [['held-817x325', [817, 325]], ['held-1600x900', [1600, 900]]]) {
      execFileSync('bash', ['-c', `DISPLAY=${display} xdotool windowsize ${target} ${size[0]} ${size[1]}`]);
      await page.waitForTimeout(3000);
      for (let k = 0; k < 8; k++) await shot(label, k, 15000).catch((error) => samples.push({ phase: label, index: k, error: String(error.message).slice(0, 120) }));
    }
  } else if (mode === 'window-width') {
    const { targetInfo } = await cdpSession.send('Target.getTargetInfo');
    const windowInfo = await bcdp.send('Browser.getWindowForTarget', { targetId: targetInfo.targetId });
    const original = windowInfo.bounds;
    console.log('window ' + windowInfo.windowId + ' original bounds ' + JSON.stringify(original));
    for (let cycle = 0; cycle < 2; cycle++) {
      await bcdp.send('Browser.setWindowBounds', { windowId: windowInfo.windowId, bounds: { width: 817, height: 600 } });
      for (let k = 0; k < 8; k++) await shot('width-narrow', cycle * 100 + k, 15000).catch((error) => samples.push({ phase: 'width-narrow', index: k, error: String(error.message).slice(0, 120) }));
      await page.waitForTimeout(3000);
      for (let k = 0; k < 2; k++) await shot('width-narrow-steady', cycle * 100 + k, 15000).catch((error) => samples.push({ phase: 'width-narrow-steady', index: k, error: String(error.message).slice(0, 120) }));
      await bcdp.send('Browser.setWindowBounds', { windowId: windowInfo.windowId, bounds: { width: original.width, height: original.height } });
      for (let k = 0; k < 8; k++) await shot('width-restore', cycle * 100 + k, 15000).catch((error) => samples.push({ phase: 'width-restore', index: k, error: String(error.message).slice(0, 120) }));
      await page.waitForTimeout(3000);
      for (let k = 0; k < 2; k++) await shot('width-restore-steady', cycle * 100 + k, 15000).catch((error) => samples.push({ phase: 'width-restore-steady', index: k, error: String(error.message).slice(0, 120) }));
    }
  } else if (mode === 'viewport-transient') {
    for (let cycle = 0; cycle < 2; cycle++) {
      await page.setViewportSize({ width: 1280, height: 720 });
      for (let k = 0; k < 8; k++) await shot('vp-small', cycle * 100 + k, 15000).catch((error) => samples.push({ phase: 'vp-small', index: k, error: String(error.message).slice(0, 120) }));
      await page.waitForTimeout(3000);
      for (let k = 0; k < 2; k++) await shot('vp-small-steady', cycle * 100 + k, 15000).catch((error) => samples.push({ phase: 'vp-small-steady', index: k, error: String(error.message).slice(0, 120) }));
      await page.setViewportSize({ width: 1600, height: 900 });
      for (let k = 0; k < 8; k++) await shot('vp-big', cycle * 100 + k, 15000).catch((error) => samples.push({ phase: 'vp-big', index: k, error: String(error.message).slice(0, 120) }));
      await page.waitForTimeout(3000);
      for (let k = 0; k < 2; k++) await shot('vp-big-steady', cycle * 100 + k, 15000).catch((error) => samples.push({ phase: 'vp-big-steady', index: k, error: String(error.message).slice(0, 120) }));
    }
  } else if (mode === 'transient') {
    for (let cycle = 0; cycle < 3; cycle++) {
      await cdpSession.send('Emulation.setDeviceMetricsOverride', { width: 1600, height: 900, deviceScaleFactor: 2, mobile: false });
      await page.waitForTimeout(2000);
      for (let k = 0; k < 2; k++) await shot('dpr2', cycle * 100 + k, 15000).catch((error) => samples.push({ phase: 'dpr2', index: k, error: String(error.message).slice(0, 120) }));
      console.log('cycle ' + cycle + ' clear override ' + Date.now());
      await cdpSession.send('Emulation.clearDeviceMetricsOverride');
      for (let k = 0; k < 10; k++) await shot('transient', cycle * 100 + k, 15000).catch((error) => samples.push({ phase: 'transient', index: k, error: String(error.message).slice(0, 120) }));
      await page.waitForTimeout(3000);
      for (let k = 0; k < 2; k++) await shot('steady', cycle * 100 + k, 15000).catch((error) => samples.push({ phase: 'steady', index: k, error: String(error.message).slice(0, 120) }));
    }
  } else if (mode === 'swapchain') {
    const steps = [
      ['resize-small', () => page.setViewportSize({ width: 1280, height: 720 })],
      ['resize-back', () => page.setViewportSize({ width: 1600, height: 900 })],
      ['dpr-2', () => cdpSession.send('Emulation.setDeviceMetricsOverride', { width: 1600, height: 900, deviceScaleFactor: 2, mobile: false })],
      ['dpr-back', () => cdpSession.send('Emulation.clearDeviceMetricsOverride')],
    ];
    for (const [phase, change] of steps) {
      await change();
      console.log('step ' + phase);
      await page.waitForTimeout(2000);
      for (let k = 0; k < 3; k++) {
        await shot(phase, k, 15000).catch((error) => samples.push({ phase, index: k, error: String(error.message).slice(0, 120) }));
        await page.waitForTimeout(1000);
      }
    }
  } else if (mode === 'deviceloss') {
    const contexts = await page.evaluate(() => {
      const c = document.querySelector('canvas');
      return { webgl2: Boolean(c.getContext('webgl2')), webgl: Boolean(c.getContext('webgl')), webgpu: Boolean(c.getContext('webgpu')), devicesRecorded: (window.__devices || []).length };
    });
    console.log('contexts ' + JSON.stringify(contexts));
    const destroyed = await page.evaluate(() => {
      const device = window.__devices && window.__devices[0];
      if (!device) return 'no device recorded';
      device.destroy();
      return 'device destroyed';
    });
    console.log(destroyed);
    for (let k = 0; k < Math.floor(hideS / 3); k++) {
      await page.waitForTimeout(3000);
      await shot('lost', k, 15000).catch((error) => samples.push({ phase: 'lost', index: k, error: String(error.message).slice(0, 120) }));
    }
  } else if (mode === 'activate') {
    other = await bcdp.send('Target.createTarget', { url: 'about:blank', background: true, newWindow: false });
    const viewerWindow = await bcdp.send('Browser.getWindowForTarget', { targetId: created.targetId });
    const otherWindow = await bcdp.send('Browser.getWindowForTarget', { targetId: other.targetId });
    console.log(`activate viewer window ${viewerWindow.windowId} other window ${otherWindow.windowId}`);
    await bcdp.send('Target.activateTarget', { targetId: other.targetId });
    const otherPage = await attachPage(context, other.targetId);
    console.log('other target visibility ' + (await otherPage.evaluate(() => document.visibilityState)) + ' viewer visibility ' + (await page.evaluate(() => document.visibilityState)));
    for (let k = 0; k < Math.floor(hideS / 3); k++) {
      await page.waitForTimeout(3000);
      await shot('hidden', k, 15000).catch((error) => samples.push({ phase: 'hidden', index: k, error: String(error.message).slice(0, 120) }));
    }
  } else {
    await page.waitForTimeout(hideS * 1000);
  }
  const hiddenState = await page.evaluate(() => document.visibilityState).catch((error) => 'unreachable: ' + String(error.message).slice(0, 80));
  samples.push({ phase: 'hidden', index: 0, t: Date.now(), vis: hiddenState, canvas: null, css: null, w: 0, h: 0, longest: 0, over40: 0 });
  console.log('hidden state ' + hiddenState);
  if (mode === 'lifecycle') await cdpSession.send('Page.setWebLifecycleState', { state: 'active' });
  if (mode === 'tab') { await page.bringToFront(); }
  if (mode === 'window') execFileSync('bash', ['-c', `DISPLAY=${display} xdotool windowmap ${window}`]);
  if (mode === 'minimize') await minimize.browserSession.send('Browser.setWindowBounds', { windowId: minimize.windowId, bounds: { windowState: 'normal' } });
  if (mode === 'activate') await bcdp.send('Target.activateTarget', { targetId: created.targetId });
  for (let i = 0; i < after; i++) { await shot('after', i); await page.waitForTimeout(500); }
  if (other) await bcdp.send('Target.closeTarget', { targetId: other.targetId }).catch(() => {});
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
