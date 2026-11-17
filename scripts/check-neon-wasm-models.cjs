#!/usr/bin/env node
const fs = require('node:fs');
const path = require('node:path');
const { withGateWitness, lastJsonLine } = require('./lib/playwriter-witness.cjs');

const repo = path.resolve(__dirname, '..');
const port = Number(process.env.PORT || 8092);
const cdn = process.env.NEON_CDN || '/workspace/neon-syndicates/assets/cdn';
const cache = path.join(repo, 'target', 'gltf-cache');
const witnessDir = path.join(repo, 'target', 'witness');
const witnessPath = path.join(witnessDir, 'neon-wasm-witness.json');
const batchSize = Number(process.env.BATCH || 8);
const batchTimeoutMs = Number(process.env.BATCH_TIMEOUT_MS || 300000);

function snippet(pageBase, batch) {
    return `
return await (async () => {
const batch = ${JSON.stringify(batch)};
const pageBase = ${JSON.stringify(pageBase)};
const rows = [];
for (const name of batch) {
  const model = '/target/gltf-cache/' + name;
  state.logs = [];
  state.failures = [];
  const page = await context.newPage();
  page.on('console', (message) => state.logs.push(message.type() + ': ' + message.text() + ' @ ' + (message.location().url || '(no url)')));
  page.on('pageerror', (error) => state.logs.push('pageerror: ' + error.message));
  page.on('response', (response) => { if (response.status() >= 400) state.failures.push({ url: response.url(), status: response.status() }); });
  page.on('requestfailed', (request) => state.failures.push({ url: request.url(), status: 'failed: ' + (request.failure()?.errorText ?? 'unknown') }));
  const started = Date.now();
  await page.goto(pageBase + '?model=' + encodeURIComponent(model), { waitUntil: 'domcontentloaded' });
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
  const resources = await page.evaluate(() => performance.getEntriesByType('resource').map((entry) => ({ name: decodeURIComponent(entry.name), transferSize: entry.transferSize, encodedBodySize: entry.encodedBodySize, decodedBodySize: entry.decodedBodySize })));
  const glb = resources.find((entry) => entry.name.endsWith('/target/gltf-cache/' + name));
  const wasm = resources.filter((entry) => /\.wasm$/.test(entry.name)).map((entry) => ({ name: entry.name.split('/').pop(), transferSize: entry.transferSize, encodedBodySize: entry.encodedBodySize, decodedBodySize: entry.decodedBodySize }));
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
  });
  await page.close();
}
console.log(JSON.stringify(rows));
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
        notClean: rows.filter((row) => !clean.includes(row)).map((row) => row.name),
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
    const result = { ok: false, total: names.length, cdn, batches: [] };
    const missing = names.filter((name) => !fs.existsSync(path.join(cache, name)));
    if (missing.length) {
        result.error = `decoded copies missing in ${cache}; run gltf-check on ${cdn} first: ${missing.slice(0, 5).join(', ')}`;
        return result;
    }
    fs.mkdirSync(witnessDir, { recursive: true });
    const rows = [];
    try {
        const { value, browser } = await withGateWitness({ repo, port, tabGroup: 'neon-wasm' }, async ({ pageBase, run }) => {
            for (let index = 0; index < names.length; index += batchSize) {
                const batch = names.slice(index, index + batchSize);
                const started = Date.now();
                const batchRows = lastJsonLine(run(snippet(pageBase, batch), batchTimeoutMs));
                result.batches.push({ first: batch[0], count: batch.length, wallMs: Date.now() - started });
                rows.push(...batchRows);
            }
            return rows;
        });
        result.browser = browser;
        result.summary = summarize(value, names);
        result.rows = value;
        result.ok = result.summary.clean === names.length && result.summary.missingNames.length === 0;
    } catch (error) {
        result.error = String(error.stderr || error.message || error);
        result.rows = rows;
    }
    fs.writeFileSync(witnessPath, JSON.stringify(result, null, 2));
    return result;
}

main().then((result) => {
    const { rows, ...brief } = result;
    console.log(JSON.stringify({ ...brief, witness: witnessPath }, null, 2));
    process.exit(result.ok ? 0 : 1);
});
