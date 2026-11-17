#!/usr/bin/env node
const fs = require('node:fs');
const path = require('node:path');
const { withGateWitness } = require('./lib/playwriter-witness.cjs');

const repo = path.resolve(__dirname, '..');
const port = Number(process.env.PORT || 8092);
const cdn = process.env.NEON_CDN || '/workspace/neon-syndicates/assets/cdn';
const witnessDir = path.join(repo, 'target', 'witness');
const rowsPath = path.join(witnessDir, 'device-bc-fallback-rows.json');
const witnessPath = path.join(witnessDir, 'device-bc-fallback.json');
const callTimeoutMs = Number(process.env.CALL_TIMEOUT_MS || 1200000);
const fallbackQuery = '&disable-bc=1';

function snippet(pageBase, models, outPath) {
    return `
return await (async () => {
const fs = require('node:fs');
const models = ${JSON.stringify(models)};
const pageBase = ${JSON.stringify(pageBase)};
const outPath = ${JSON.stringify(outPath)};
const fallbackQuery = ${JSON.stringify(fallbackQuery)};
const result = { control: null, rows: [] };
const write = () => fs.writeFileSync(outPath, JSON.stringify(result));
async function visit(page, name, query) {
  const logs = [];
  const failures = [];
  const onConsole = (message) => logs.push(message.type() + ': ' + message.text());
  const onError = (error) => logs.push('pageerror: ' + error.message);
  const onResponse = (response) => { if (response.status() >= 400) failures.push({ url: response.url(), status: response.status() }); };
  page.on('console', onConsole);
  page.on('pageerror', onError);
  page.on('response', onResponse);
  const started = Date.now();
  let status = { ready: false, modelState: null };
  try {
    await page.goto(pageBase + '?model=' + encodeURIComponent('/target/gltf-cache/' + name) + query, { waitUntil: 'domcontentloaded' });
    while (Date.now() - started < 120000) {
      status = await page.evaluate(() => {
        const element = document.querySelector('#app-status');
        return {
          ready: window.__BEVY_WASM_READY === true,
          modelState: element?.getAttribute('data-model-state') ?? null,
          format: element?.getAttribute('data-texture-format') ?? null,
          modelPath: element?.getAttribute('data-model-path') ?? null,
          bcSupported: element?.getAttribute('data-bc-supported') ?? null,
        };
      }).catch(() => ({ ready: false, modelState: null }));
      if (status.modelState === 'loaded' || /fail|error|missing/.test(status.modelState ?? '')) break;
      await new Promise((resolve) => setTimeout(resolve, 250));
    }
  } catch (error) {
    status = { ready: false, modelState: 'goto failed: ' + String(error.message).slice(0, 160) };
  }
  const loadMs = status.modelState === 'loaded' ? Date.now() - started : null;
  const adapterHasBc = await page.evaluate(async () => {
    const adapter = await navigator.gpu?.requestAdapter();
    return adapter ? adapter.features.has('texture-compression-bc') : null;
  }).catch(() => null);
  const fetched = await page.evaluate(() => performance.getEntriesByType('resource').map((entry) => decodeURIComponent(entry.name))).catch(() => []);
  page.off('console', onConsole);
  page.off('pageerror', onError);
  page.off('response', onResponse);
  return {
    name,
    query,
    ...status,
    loadMs,
    adapterHasBc,
    fetchedRgba8: fetched.some((url) => url.endsWith('/target/gltf-cache-rgba8/' + name)),
    fetchedBc7: fetched.some((url) => url.endsWith('/target/gltf-cache/' + name)),
    errors: logs.filter((line) => /^error|^pageerror/.test(line)).slice(-10),
    failedRequests: failures,
  };
}
const control = await context.newPage();
result.control = await visit(control, models[0], '');
await control.close();
write();
const page = await context.newPage();
for (const name of models) {
  result.rows.push(await visit(page, name, fallbackQuery));
  write();
}
await page.close();
write();
return 'written ' + result.rows.length;
})();
`;
}

function summarize(result, names) {
    const rows = result.rows;
    const good = rows.filter((row) => row.modelState === 'loaded' && row.format === 'rgba8' && row.bcSupported === 'false' && row.fetchedRgba8 && !row.fetchedBc7 && row.errors.length === 0 && row.failedRequests.length === 0);
    const control = result.control;
    const controlOk = Boolean(control && control.modelState === 'loaded' && control.format === 'bc7' && control.bcSupported === 'true' && control.fetchedBc7 && !control.fetchedRgba8);
    const loads = rows.map((row) => row.loadMs).filter((value) => value !== null).sort((a, b) => a - b);
    return {
        total: names.length,
        rows: rows.length,
        loaded: rows.filter((row) => row.modelState === 'loaded').length,
        formatRgba8: rows.filter((row) => row.format === 'rgba8').length,
        deviceBcFalse: rows.filter((row) => row.bcSupported === 'false').length,
        fetchedFromRgba8Cache: rows.filter((row) => row.fetchedRgba8).length,
        fetchedFromBc7Cache: rows.filter((row) => row.fetchedBc7).length,
        adapterHasBc: rows.filter((row) => row.adapterHasBc === true).length,
        good: good.length,
        notGood: names.filter((name) => !good.some((row) => row.name === name)),
        loadMsMedian: loads.length ? loads[Math.floor(loads.length / 2)] : null,
        controlOk,
        control: control && { format: control.format, bcSupported: control.bcSupported, adapterHasBc: control.adapterHasBc, modelState: control.modelState, fetchedBc7: control.fetchedBc7 },
    };
}

async function main() {
    const names = fs.readdirSync(cdn).filter((file) => file.endsWith('.glb')).sort();
    const result = { ok: false, total: names.length, cdn, query: fallbackQuery };
    const missing = names.filter((name) => !fs.existsSync(path.join(repo, 'target', 'gltf-cache', name)) || !fs.existsSync(path.join(repo, 'target', 'gltf-cache-rgba8', name)));
    if (missing.length) {
        result.error = `decoded copies missing; run gltf-check on ${cdn} with and without the rgba8 target first: ${missing.slice(0, 5).join(', ')}`;
        return result;
    }
    fs.mkdirSync(witnessDir, { recursive: true });
    try {
        const { value, browser, gate } = await withGateWitness({ repo, port }, async ({ pageBase, run }) => {
            run(snippet(pageBase, names, rowsPath), callTimeoutMs);
            return JSON.parse(fs.readFileSync(rowsPath, 'utf8'));
        });
        result.browser = browser;
        result.gate = gate;
        result.summary = summarize(value, names);
        result.rows = value.rows;
        result.ok = result.summary.good === names.length && result.summary.controlOk;
    } catch (error) {
        result.error = String(error.stderr || error.message || error);
    }
    fs.writeFileSync(witnessPath, JSON.stringify(result, null, 2));
    return result;
}

main().then((result) => {
    const { rows, ...brief } = result;
    console.log(JSON.stringify({ ...brief, witness: witnessPath }, null, 2));
    process.exit(result.ok ? 0 : 1);
});
