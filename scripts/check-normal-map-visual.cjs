#!/usr/bin/env node
const fs = require('node:fs');
const path = require('node:path');
const { withGateWitness, lastJsonLine } = require('./lib/playwriter-witness.cjs');

const repo = path.resolve(__dirname, '..');
const port = Number(process.env.PORT || 8092);
const witnessDir = path.join(repo, 'target', 'witness');
const variants = [
    { kind: 'raw', model: '/testdata/normal-map/normal-map.glb', screenshot: path.join(witnessDir, 'normal-map-raw.png') },
    { kind: 'decoded', model: '/target/gltf-cache/normal-map.glb', screenshot: path.join(witnessDir, 'normal-map-decoded.png') },
];

function snippet(pageBase) {
    return `
return await (async () => {
const variants = ${JSON.stringify(variants)};
const pageBase = ${JSON.stringify(pageBase)};
const results = [];
for (const variant of variants) {
  state.logs = [];
  state.failures = [];
  const page = await context.newPage();
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
})();
`;
}

async function main() {
    const result = { ok: false, variants: [] };
    fs.mkdirSync(witnessDir, { recursive: true });
    try {
        const { value, browser } = await withGateWitness({ repo, port, tabGroup: 'normal-map' }, async ({ pageBase, run }) => {
            const output = run(snippet(pageBase), 420000);
            return lastJsonLine(output);
        });
        result.variants = value;
        result.browser = browser;
        result.ok = result.variants.length === variants.length && result.variants.every((variant) => variant.ready && variant.modelState === 'loaded' && variant.adapter.length > 0 && variant.tangentWarnings.length === 0 && variant.failedRequests.length === 0 && fs.existsSync(variant.screenshot));
    } catch (error) {
        result.error = String(error.stderr || error.message || error);
    }
    return result;
}

main().then((result) => {
    console.log(JSON.stringify(result, null, 2));
    process.exit(result.ok ? 0 : 1);
});
