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
const pageBase = `http://127.0.0.1:${port}/web/wasm/index.html`;
const witnessDir = path.join(repo, 'target', 'witness');
const work = fs.mkdtempSync(path.join(os.tmpdir(), 'normal-map-visual-'));
const flags = [
    '--no-sandbox', '--disable-gpu-sandbox', '--enable-unsafe-webgpu',
    '--enable-features=Vulkan,UseSkiaRenderer', '--use-angle=vulkan',
    '--ignore-gpu-blocklist', '--enable-webgpu-developer-features',
];
const variants = [
    { kind: 'raw', model: '/testdata/normal-map/normal-map.glb', screenshot: path.join(witnessDir, 'normal-map-raw.png') },
    { kind: 'decoded', model: '/target/gltf-cache/normal-map.glb', screenshot: path.join(witnessDir, 'normal-map-decoded.png') },
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
    return new Set(rows.map((row) => row.trim()).filter((row) => /playwriter/.test(row) && !row.includes('check-normal-map-visual')).map((row) => Number(row.split(/\s+/)[0])));
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
const variants = ${JSON.stringify(variants)};
const pageBase = ${JSON.stringify(pageBase)};
const results = [];
for (const variant of variants) {
  state.logs = [];
  const page = await context.newPage();
  state.failures = [];
  page.on('console', (message) => state.logs.push(message.type() + ': ' + message.text() + ' @ ' + (message.location().url || '(no url)')));
  page.on('pageerror', (error) => state.logs.push('pageerror: ' + error.message));
  page.on('response', (response) => { if (response.status() >= 400) state.failures.push({ url: response.url(), status: response.status() }); });
  page.on('requestfailed', (request) => state.failures.push({ url: request.url(), status: 'failed: ' + (request.failure()?.errorText ?? 'unknown') }));
  await page.goto(pageBase + '?model=' + encodeURIComponent(variant.model), { waitUntil: 'domcontentloaded' });
  const deadline = Date.now() + 120000;
  let status = { ready: false, modelState: null };
  while (Date.now() < deadline) {
    status = await page.evaluate(() => ({ ready: window.__BEVY_WASM_READY === true, modelState: document.querySelector('#app-status')?.getAttribute('data-model-state') ?? null })).catch(() => ({ ready: false, modelState: null }));
    if (status.ready && status.modelState) break;
    await new Promise((resolve) => setTimeout(resolve, 500));
  }
  await new Promise((resolve) => setTimeout(resolve, 3000));
  const finalUrl = await page.evaluate(() => location.href);
  await page.screenshot({ path: variant.screenshot });
  results.push({
    kind: variant.kind,
    model: variant.model,
    finalUrl,
    ready: status.ready,
    modelState: status.modelState,
    adapter: state.logs.filter((line) => /BrowserWebGpu/.test(line)),
    tangentWarnings: state.logs.filter((line) => /TANGENT/.test(line)),
    errors: state.logs.filter((line) => /^error|^pageerror/.test(line)).slice(-10),
    failedRequests: state.failures,
    screenshot: variant.screenshot,
  });
  await page.close();
}
console.log(JSON.stringify(results));
`;

async function main() {
    const result = { ok: false, pageBase, variants: [] };
    if (await listening(pageBase)) {
        result.error = `port ${port} already serves ${pageBase}; refusing to run a second server`;
        return result;
    }
    fs.mkdirSync(witnessDir, { recursive: true });
    const playwriterBefore = playwriterPids();
    const server = spawn('bash', [path.join(repo, 'scripts/serve-wasm.sh')], { cwd: repo, env: { ...process.env, PORT: String(port) }, stdio: 'ignore' });
    let browser = null;
    let session = null;
    try {
        result.serverPid = server.pid;
        if (!(await waitFor(() => listening(pageBase), 20000))) {
            result.error = `server did not answer ${pageBase} within 20 s`;
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
        const created = execFileSync(playwriter, ['session', 'new', '--direct', wsUrl, '--tab-group', 'normal-map'], { encoding: 'utf8', timeout: 30000 });
        session = /Session (\d+) created/.exec(created)?.[1];
        if (!session) throw new Error(`no session id in: ${created.trim()}`);
        result.session = session;
        const output = execFileSync(playwriter, ['-s', session, '--timeout', '420000', '-e', snippet], { encoding: 'utf8', timeout: 420000, maxBuffer: 64 * 1024 * 1024 });
        const line = output.trim().split('\n').reverse().map((text) => text.replace(/^\[log\] /, '').trim()).find((text) => text.startsWith('['));
        result.variants = JSON.parse(line);
        result.ok = result.variants.length === variants.length && result.variants.every((variant) => variant.ready && variant.modelState === 'loaded' && variant.adapter.length > 0 && variant.tangentWarnings.length === 0 && variant.failedRequests.length === 0 && fs.existsSync(variant.screenshot));
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
