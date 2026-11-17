#!/usr/bin/env node
const fs = require('node:fs');
const path = require('node:path');
const { withGateWitness } = require('./lib/playwriter-witness.cjs');
const { measure } = require('./lib/texture-cache.cjs');

const repo = path.resolve(__dirname, '..');
const port = Number(process.env.PORT || 8092);
const cdn = process.env.NEON_CDN || '/workspace/neon-syndicates/assets/cdn';
const witnessDir = path.join(repo, 'target', 'witness');
const rowsPath = path.join(witnessDir, 'device-format-fallback-rows.json');
const witnessPath = path.join(witnessDir, 'device-format-fallback.json');
const callTimeoutMs = Number(process.env.CALL_TIMEOUT_MS || 1200000);
const fallbackQuery = '&adapter=software&disable-bc=1';
const compressed = {
    astc4x4: { feature: 'texture-compression-astc', cache: 'target/gltf-cache-astc' },
    etc2: { feature: 'texture-compression-etc2', cache: 'target/gltf-cache-etc2' },
};
const cacheDirs = { rgba8: 'target/gltf-cache-rgba8', bc7: 'target/gltf-cache', ...Object.fromEntries(Object.entries(compressed).map(([key, value]) => [key, value.cache])) };

function snippet(pageBase, models, outPath) {
    return `
return await (async () => {
const fs = require('node:fs');
const models = ${JSON.stringify(models)};
const pageBase = ${JSON.stringify(pageBase)};
const outPath = ${JSON.stringify(outPath)};
const fallbackQuery = ${JSON.stringify(fallbackQuery)};
const compressed = ${JSON.stringify(compressed)};
const cacheDirs = ${JSON.stringify(cacheDirs)};
const result = { adapters: null, expected: null, control: null, rows: [] };
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
          compressedFormats: element?.getAttribute('data-compressed-formats') ?? null,
          adapter: element?.getAttribute('data-adapter') ?? null,
        };
      }).catch(() => ({ ready: false, modelState: null }));
      if (status.modelState === 'loaded' || /fail|error|missing/.test(status.modelState ?? '')) break;
      await new Promise((resolve) => setTimeout(resolve, 250));
    }
  } catch (error) {
    status = { ready: false, modelState: 'goto failed: ' + String(error.message).slice(0, 160) };
  }
  const loadMs = status.modelState === 'loaded' ? Date.now() - started : null;
  const fetched = await page.evaluate(() => performance.getEntriesByType('resource').map((entry) => decodeURIComponent(entry.name))).catch(() => []);
  page.off('console', onConsole);
  page.off('pageerror', onError);
  page.off('response', onResponse);
  const fetchedFrom = Object.fromEntries(Object.entries(cacheDirs).map(([key, dir]) => [key, fetched.some((url) => url.endsWith('/' + dir + '/' + name))]));
  return {
    name,
    query,
    ...status,
    loadMs,
    fetchedFrom,
    errors: logs.filter((line) => /^error|^pageerror/.test(line)).slice(-10),
    failedRequests: failures,
  };
}
const control = await context.newPage();
result.control = await visit(control, models[0], '');
result.adapters = await control.evaluate(async () => {
  const describe = async (options) => {
    const adapter = await navigator.gpu?.requestAdapter(options).catch(() => null);
    if (!adapter) return null;
    const info = adapter.info ?? {};
    return { features: [...adapter.features].sort(), isFallbackAdapter: adapter.isFallbackAdapter ?? null, vendor: info.vendor ?? null, architecture: info.architecture ?? null, description: info.description ?? null };
  };
  return { hardware: await describe({}), software: await describe({ forceFallbackAdapter: true }) };
}).catch((error) => ({ error: String(error.message).slice(0, 160) }));
await control.close();
const software = result.adapters && result.adapters.software;
result.expected = Object.keys(compressed).find((key) => software && software.features.includes(compressed[key].feature)) ?? 'rgba8';
write();
if (result.expected !== 'rgba8') {
  const page = await context.newPage();
  for (const name of models) {
    result.rows.push(await visit(page, name, fallbackQuery));
    write();
  }
  await page.close();
}
write();
return 'written ' + result.rows.length;
})();
`;
}

function summarize(result, names) {
    const expected = result.expected;
    const rows = result.rows;
    const others = Object.keys(cacheDirs).filter((key) => key !== expected);
    const good = rows.filter((row) => row.modelState === 'loaded' && row.format === expected && row.bcSupported === 'false' && /swiftshader/i.test(row.adapter ?? '') && row.fetchedFrom[expected] && others.every((key) => !row.fetchedFrom[key]) && row.errors.length === 0 && row.failedRequests.length === 0);
    const control = result.control;
    const controlOk = Boolean(control && control.modelState === 'loaded' && control.format === 'bc7' && control.bcSupported === 'true' && control.fetchedFrom.bc7 && !control.fetchedFrom.rgba8);
    const loads = rows.map((row) => row.loadMs).filter((value) => value !== null).sort((a, b) => a - b);
    const footprint = expected === 'rgba8' ? null : footprintSummary(names, expected);
    return {
        expected,
        softwareFeatures: result.adapters && result.adapters.software ? result.adapters.software.features : null,
        total: names.length,
        rows: rows.length,
        loaded: rows.filter((row) => row.modelState === 'loaded').length,
        formatMatches: rows.filter((row) => row.format === expected).length,
        fetchedFromExpectedCache: rows.filter((row) => row.fetchedFrom[expected]).length,
        fetchedFromOtherCache: rows.filter((row) => others.some((key) => row.fetchedFrom[key])).length,
        deviceBcFalse: rows.filter((row) => row.bcSupported === 'false').length,
        compressedFormats: [...new Set(rows.map((row) => row.compressedFormats))],
        deviceAdapters: [...new Set(rows.map((row) => row.adapter))],
        good: good.length,
        notGood: names.filter((name) => !good.some((row) => row.name === name)),
        loadMsMedian: loads.length ? loads[Math.floor(loads.length / 2)] : null,
        controlOk,
        footprint,
    };
}

function footprintSummary(names, expected) {
    const expectedDir = path.join(repo, compressed[expected].cache);
    const sums = { rgba8: 0, bc7: 0, expected: 0, rgba8Glb: 0, expectedGlb: 0 };
    let complete = true;
    for (const name of names) {
        const rgba8 = measure(path.join(repo, cacheDirs.rgba8, name));
        const bc7 = measure(path.join(repo, cacheDirs.bc7, name));
        const format = measure(path.join(expectedDir, name));
        if (!rgba8.present || !bc7.present || !format.present) {
            complete = false;
            continue;
        }
        sums.rgba8 += rgba8.textureBytes;
        sums.bc7 += bc7.textureBytes;
        sums.expected += format.textureBytes;
        sums.rgba8Glb += rgba8.glbBytes;
        sums.expectedGlb += format.glbBytes;
    }
    return {
        complete,
        models: names.length,
        rgba8TextureBytes: sums.rgba8,
        bc7TextureBytes: sums.bc7,
        expectedTextureBytes: sums.expected,
        ratioToRgba8: sums.rgba8 ? sums.expected / sums.rgba8 : null,
        ratioToBc7: sums.bc7 ? sums.expected / sums.bc7 : null,
        rgba8GlbBytes: sums.rgba8Glb,
        expectedGlbBytes: sums.expectedGlb,
    };
}

async function main() {
    const names = fs.readdirSync(cdn).filter((file) => file.endsWith('.glb')).sort();
    const result = { ok: false, total: names.length, cdn, query: fallbackQuery };
    const missing = names.filter((name) => Object.values(cacheDirs).some((dir) => !fs.existsSync(path.join(repo, dir, name))));
    if (missing.length) {
        result.error = `decoded copies missing; run gltf-check on ${cdn} for rgba8, bc7, etc2 and astc4x4 first: ${missing.slice(0, 5).join(', ')}`;
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
        result.adapters = value.adapters;
        result.expected = value.expected;
        result.summary = summarize(value, names);
        result.rows = value.rows;
        if (result.summary.expected === 'rgba8') {
            result.reason = 'the software adapter exposes neither texture-compression-astc nor texture-compression-etc2';
        } else {
            result.ok = result.summary.good === names.length && result.summary.controlOk && result.summary.footprint.complete;
        }
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
