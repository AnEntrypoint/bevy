#!/usr/bin/env node
const fs = require('node:fs');
const path = require('node:path');
const { withGateWitness } = require('./lib/playwriter-witness.cjs');

const repo = path.resolve(__dirname, '..');
const port = Number(process.env.PORT || 8092);
const cdn = process.env.NEON_CDN || '/workspace/neon-syndicates/assets/cdn';
const modes = {
    bc7: { urlDir: 'gltf-cache', decodedDir: 'gltf-cache', query: '' },
    rgba8: { urlDir: 'gltf-cache-rgba8', decodedDir: 'gltf-cache-rgba8', query: '' },
    'rgba8-software': { urlDir: 'gltf-cache-rgba8', decodedDir: 'gltf-cache-rgba8', query: '&adapter=software' },
    astc4x4: { urlDir: 'gltf-cache', decodedDir: 'gltf-cache-astc', query: '&adapter=software&disable-bc=1' },
    etc2: { urlDir: 'gltf-cache', decodedDir: 'gltf-cache-etc2', query: '&adapter=software&disable-bc=1&disable-astc=1' },
};
const texture = process.env.NEON_TEXTURE || 'bc7';
if (!modes[texture]) throw new Error(`unknown NEON_TEXTURE ${texture}; expected one of ${Object.keys(modes).join(', ')}`);
const { urlDir, decodedDir, query } = modes[texture];
const cache = path.join(repo, 'target', decodedDir);
const witnessDir = path.join(repo, 'target', 'witness');
const witnessPath = path.join(witnessDir, `neon-wasm-witness-${texture}.json`);
const rowsPath = path.join(witnessDir, `neon-rows-${texture}.json`);
const shotDir = path.join(witnessDir, `neon-shots-${texture}`);
const callTimeoutMs = Number(process.env.CALL_TIMEOUT_MS || 1200000);

function snippet(pageBase, models, outPath, shots) {
    return `
return await (async () => {
const fs = require('node:fs');
const models = ${JSON.stringify(models)};
const pageBase = ${JSON.stringify(pageBase)};
const outPath = ${JSON.stringify(outPath)};
const cacheDir = ${JSON.stringify(urlDir)};
const query = ${JSON.stringify(query)};
const shotDir = ${JSON.stringify(shots)};
fs.mkdirSync(shotDir, { recursive: true });
const rows = [];
const page = await context.newPage();
page.on('console', (message) => state.logs.push(message.type() + ': ' + message.text() + ' @ ' + (message.location().url || '(no url)')));
page.on('pageerror', (error) => state.logs.push('pageerror: ' + error.message));
page.on('response', (response) => { if (response.status() >= 400) state.failures.push({ url: response.url(), status: response.status() }); });
page.on('requestfailed', (request) => state.failures.push({ url: request.url(), status: 'failed: ' + (request.failure()?.errorText ?? 'unknown') }));
for (const name of models) {
  const model = '/target/' + cacheDir + '/' + name;
  state.logs = [];
  state.failures = [];
  const started = Date.now();
  try {
    await page.goto(pageBase + '?model=' + encodeURIComponent(model) + query, { waitUntil: 'domcontentloaded' });
  } catch (error) {
    rows.push({ name, model, ready: false, modelState: 'goto failed: ' + String(error.message).slice(0, 160), loadMs: null, adapter: 0, tangentWarnings: [], errors: ['goto failed'], failedRequests: [], glb: null, wasm: [] });
    fs.writeFileSync(outPath, JSON.stringify(rows));
    continue;
  }
  const deadline = started + 120000;
  let loadMs = null;
  let status = { ready: false, modelState: null };
  while (Date.now() < deadline) {
    status = await page.evaluate(() => ({ ready: window.__BEVY_WASM_READY === true, modelState: document.querySelector('#app-status')?.getAttribute('data-model-state') ?? null })).catch(() => ({ ready: false, modelState: null }));
    if (status.modelState === 'loaded' && loadMs === null) loadMs = Date.now() - started;
    if (status.modelState === 'loaded' || /fail|error|missing/.test(status.modelState ?? '')) break;
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  await new Promise((resolve) => setTimeout(resolve, 1500));
  const screenshot = shotDir + '/' + name + '.png';
  await page.screenshot({ path: screenshot }).catch(() => null);
  const resources = await page.evaluate(() => performance.getEntriesByType('resource').map((entry) => ({ name: decodeURIComponent(entry.name), transferSize: entry.transferSize, encodedBodySize: entry.encodedBodySize, decodedBodySize: entry.decodedBodySize })));
  const glb = resources.find((entry) => entry.name.endsWith('/target/' + cacheDir + '/' + name));
  const wasm = resources.filter((entry) => /\\.wasm$/.test(entry.name)).map((entry) => ({ name: entry.name.split('/').pop(), transferSize: entry.transferSize, encodedBodySize: entry.encodedBodySize, decodedBodySize: entry.decodedBodySize }));
  rows.push({
    name,
    model,
    ready: status.ready,
    modelState: status.modelState,
    loadMs,
    adapter: state.logs.filter((line) => /BrowserWebGpu/.test(line)).length,
    tangentWarnings: state.logs.filter((line) => /TANGENT/.test(line)),
    errors: state.logs.filter((line) => /^error|^pageerror/.test(line)).slice(-10),
    failedRequests: state.failures,
    glb: glb ? { transferSize: glb.transferSize, encodedBodySize: glb.encodedBodySize, decodedBodySize: glb.decodedBodySize } : null,
    wasm,
    screenshot: fs.existsSync(screenshot) ? screenshot : null,
  });
  fs.writeFileSync(outPath, JSON.stringify(rows));
}
await page.close();
fs.writeFileSync(outPath, JSON.stringify(rows));
return 'written ' + rows.length;
})();
`;
}

function summarize(rows, names) {
    const clean = rows.filter((row) => row.ready && row.modelState === 'loaded' && row.adapter > 0 && row.tangentWarnings.length === 0 && row.errors.length === 0 && row.failedRequests.length === 0);
    const loads = rows.map((row) => row.loadMs).filter((value) => value !== null).sort((a, b) => a - b);
    const glbBytes = rows.map((row) => row.glb?.encodedBodySize ?? 0);
    const median = (values) => (values.length ? values[Math.floor(values.length / 2)] : null);
    return {
        total: names.length,
        rows: rows.length,
        clean: clean.length,
        notClean: names.filter((name) => !clean.some((row) => row.name === name)),
        missingNames: names.filter((name) => !rows.some((row) => row.name === name)),
        glbEncodedBytesTotal: glbBytes.reduce((sum, value) => sum + value, 0),
        glbEncodedBytesMax: Math.max(0, ...glbBytes),
        loadMsMedian: median(loads),
        loadMsMax: loads.length ? loads[loads.length - 1] : null,
        wasm: rows.find((row) => row.wasm.length)?.wasm ?? [],
    };
}

async function main() {
    const names = fs.readdirSync(cdn).filter((file) => file.endsWith('.glb')).sort();
    const result = { ok: false, texture, query, total: names.length, cdn };
    const missing = names.filter((name) => !fs.existsSync(path.join(cache, name)));
    if (missing.length) {
        result.error = `decoded copies missing in ${cache}; run gltf-check on ${cdn} for ${decodedDir} first: ${missing.slice(0, 5).join(', ')}`;
        return result;
    }
    fs.mkdirSync(witnessDir, { recursive: true });
    try {
        const { value, browser, gate } = await withGateWitness({ repo, port }, async ({ pageBase, run }) => {
            const started = Date.now();
            run(snippet(pageBase, names, rowsPath, shotDir), callTimeoutMs);
            result.wallMs = Date.now() - started;
            return JSON.parse(fs.readFileSync(rowsPath, 'utf8'));
        });
        result.browser = browser;
        result.gate = gate;
        result.summary = summarize(value, names);
        result.rows = value;
        result.ok = result.summary.clean === names.length && result.summary.missingNames.length === 0;
    } catch (error) {
        result.error = String(error.stderr || error.message || error);
        result.rows = fs.existsSync(rowsPath) ? JSON.parse(fs.readFileSync(rowsPath, 'utf8')) : [];
    }
    fs.writeFileSync(witnessPath, JSON.stringify(result, null, 2));
    return result;
}

main().then((result) => {
    const { rows, ...brief } = result;
    console.log(JSON.stringify({ ...brief, witness: witnessPath }, null, 2));
    process.exit(result.ok ? 0 : 1);
});
