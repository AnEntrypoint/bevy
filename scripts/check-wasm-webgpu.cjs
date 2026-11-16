#!/usr/bin/env node
const { execFileSync, spawn } = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const repo = path.resolve(__dirname, '..');
const playwriter = process.env.PLAYWRITER_BIN || 'playwriter';
const port = Number(process.env.PORT || 8092);
const cdpPort = Number(process.env.CDP_PORT || 9333);
const chromium = process.env.CHROMIUM_BIN || '/usr/bin/chromium';
const display = process.env.DISPLAY_OVERRIDE || ':1';
const pageUrl = `http://127.0.0.1:${port}/web/viewer.html`;
const work = fs.mkdtempSync(path.join(os.tmpdir(), 'wasm-webgpu-'));
const flags = [
    '--no-sandbox', '--disable-gpu-sandbox', '--enable-unsafe-webgpu',
    '--enable-features=Vulkan,UseSkiaRenderer', '--use-angle=vulkan',
    '--ignore-gpu-blocklist', '--enable-webgpu-developer-features',
];

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

function children(pid) {
    const rows = execFileSync('ps', ['-eo', 'pid=,ppid='], { encoding: 'utf8' }).trim().split('\n');
    const map = new Map();
    for (const row of rows) {
        const [child, parent] = row.trim().split(/\s+/).map(Number);
        if (!map.has(parent)) map.set(parent, []);
        map.get(parent).push(child);
    }
    const found = [];
    const queue = [pid];
    while (queue.length) {
        for (const child of map.get(queue.shift()) || []) {
            found.push(child);
            queue.push(child);
        }
    }
    return found;
}

function killTree(pid) {
    for (const target of [...children(pid).reverse(), pid]) {
        try {
            process.kill(target, 'SIGTERM');
        } catch {}
    }
}

function playwriterPids() {
    const rows = execFileSync('ps', ['-eo', 'pid=,args='], { encoding: 'utf8' }).trim().split('\n');
    return new Set(rows.map((row) => row.trim()).filter((row) => /playwriter/.test(row) && !row.includes('check-wasm-webgpu')).map((row) => Number(row.split(/\s+/)[0])));
}

async function waitFor(check, limitMs) {
    const deadline = Date.now() + limitMs;
    while (Date.now() < deadline) {
        if (await check()) return true;
        await sleep(500);
    }
    return false;
}

async function listening(url) {
    try {
        const response = await fetch(url);
        return response.ok;
    } catch {
        return false;
    }
}

const snippet = `
state.logs = [];
state.page = await context.newPage();
state.page.on('console', (message) => state.logs.push(message.type() + ': ' + message.text()));
state.page.on('pageerror', (error) => state.logs.push('pageerror: ' + error.message));
await state.page.goto(${JSON.stringify(pageUrl)}, { waitUntil: 'domcontentloaded' });
const deadline = Date.now() + 90000;
let ready = false;
while (Date.now() < deadline) {
  ready = await state.page.evaluate(() => window.__BEVY_WASM_READY === true && document.querySelector('[data-app-ready="true"]') !== null).catch(() => false);
  if (ready) break;
  await new Promise((resolve) => setTimeout(resolve, 500));
}
const gpu = await state.page.evaluate(() => ({ present: navigator.gpu != null, url: location.href, appReady: document.querySelector('[data-app-ready="true"]') !== null, bevyReady: window.__BEVY_WASM_READY === true }));
await state.page.screenshot({ path: ${JSON.stringify(path.join(work, 'viewer.png'))} });
const adapter = state.logs.filter((line) => /BrowserWebGpu/.test(line));
const errors = state.logs.filter((line) => /^error|^pageerror/.test(line));
await state.page.close();
JSON.stringify({ ready, gpu, adapter, errors: errors.slice(-10), logCount: state.logs.length });
`;

async function main() {
    const result = { ok: false, pageUrl, screenshot: path.join(work, 'viewer.png') };
    if (await listening(pageUrl)) {
        result.error = `port ${port} already serves ${pageUrl}; refusing to run a second server`;
        return result;
    }
    const playwriterBefore = playwriterPids();
    const server = spawn('bash', [path.join(repo, 'scripts/serve-wasm.sh')], { cwd: repo, env: { ...process.env, PORT: String(port) }, stdio: 'ignore' });
    let browser = null;
    let session = null;
    try {
        result.serverPid = server.pid;
        if (!(await waitFor(() => listening(pageUrl), 20000))) {
            result.error = `server did not answer ${pageUrl} within 20 s`;
            return result;
        }
        browser = spawn(chromium, [...flags, `--remote-debugging-port=${cdpPort}`, `--user-data-dir=${path.join(work, 'profile')}`, '--no-first-run', 'about:blank'], { env: { ...process.env, DISPLAY: display }, stdio: 'ignore' });
        result.browserPid = browser.pid;
        let wsUrl = null;
        const versionUrl = `http://127.0.0.1:${cdpPort}/json/version`;
        if (!(await waitFor(async () => {
            try {
                wsUrl = (await (await fetch(versionUrl)).json()).webSocketDebuggerUrl;
                return Boolean(wsUrl);
            } catch {
                return false;
            }
        }, 30000))) {
            result.error = `Chromium on ${display} did not expose ${versionUrl} within 30 s`;
            return result;
        }
        session = execFileSync(playwriter, ['session', 'new', '--direct', wsUrl, '--tab-group', 'wasm'], { encoding: 'utf8', timeout: 30000 }).trim().split('\n').pop();
        result.session = session;
        const output = execFileSync(playwriter, ['-s', session, '-e', snippet], { encoding: 'utf8', timeout: 150000, maxBuffer: 16 * 1024 * 1024 });
        const line = output.trim().split('\n').reverse().find((text) => text.trim().startsWith('{'));
        const evidence = JSON.parse(line);
        result.ready = evidence.ready;
        result.navigatorGpu = evidence.gpu.present;
        result.finalUrl = evidence.gpu.url;
        result.appReady = evidence.gpu.appReady;
        result.bevyReady = evidence.gpu.bevyReady;
        result.adapterLog = evidence.adapter;
        result.errors = evidence.errors;
        result.logCount = evidence.logCount;
        result.ok = evidence.ready && evidence.gpu.present && evidence.adapter.length > 0;
        return result;
    } catch (error) {
        result.error = String(error.stderr || error.message || error);
        return result;
    } finally {
        if (session) {
            try {
                execFileSync(playwriter, ['session', 'delete', session], { encoding: 'utf8', timeout: 15000 });
            } catch {}
        }
        for (const pid of playwriterPids()) {
            if (!playwriterBefore.has(pid)) killTree(pid);
        }
        if (browser) killTree(browser.pid);
        killTree(server.pid);
        fs.rmSync(path.join(work, 'profile'), { recursive: true, force: true });
    }
}

main().then((result) => {
    console.log(JSON.stringify(result, null, 2));
    process.exit(result.ok ? 0 : 1);
});
