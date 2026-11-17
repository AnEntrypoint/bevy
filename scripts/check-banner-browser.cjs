#!/usr/bin/env node
const fs = require('node:fs');
const path = require('node:path');
const { withGateWitness } = require('./lib/playwriter-witness.cjs');

const repo = path.resolve(__dirname, '..');
const port = Number(process.env.PORT || 8092);
const witnessDir = path.join(repo, 'target', 'witness');
const nativePath = path.join(witnessDir, 'hud-fixture-native.json');
const rowsPath = path.join(witnessDir, 'banner-browser-rows.json');
const outPath = path.join(witnessDir, 'banner-agreement.json');
const tolerance = Number(process.env.OPACITY_TOLERANCE || 0.02);

function snippet(pageUrl, times, out) {
    return `
return await (async () => {
const page = await context.newPage();
await page.goto(${JSON.stringify(pageUrl)}, { waitUntil: 'domcontentloaded' });
await page.waitForFunction(() => window.__FIXTURE_READY === true, null, { timeout: 30000 });
const rows = await page.evaluate((times) => {
  const element = document.getElementById('dzNEXUS');
  const animation = element.getAnimations()[0];
  animation.pause();
  return times.map((elapsed) => {
    animation.currentTime = elapsed * 1000;
    const style = getComputedStyle(element);
    return { elapsed, opacity: Number(style.opacity), visible: style.visibility === 'visible', name: animation.animationName };
  });
}, ${JSON.stringify(times)});
await page.close();
require('node:fs').writeFileSync(${JSON.stringify(out)}, JSON.stringify(rows));
return 'rows ' + rows.length;
})();
`;
}

async function main() {
    const result = { ok: false, tolerance };
    if (!fs.existsSync(nativePath)) {
        result.error = `run scripts/check-hud-fixture.cjs first; ${nativePath} is missing`;
        return result;
    }
    const native = JSON.parse(fs.readFileSync(nativePath, 'utf8'));
    const samples = native.samples || [];
    const times = [...samples.map((s) => s.elapsed), 0, 2, 4, 6];
    try {
        const { value, browser, gate } = await withGateWitness({ repo, port }, async ({ pageBase, run }) => {
            const pageUrl = new URL('/testdata/hud-fixture/browser.html', pageBase).toString();
            run(snippet(pageUrl, times, rowsPath), 120000);
            return JSON.parse(fs.readFileSync(rowsPath, 'utf8'));
        });
        result.browser = browser;
        result.gate = gate;
        const browserAt = (elapsed) => value.find((row) => row.elapsed === elapsed);
        const compared = samples.map((s) => {
            const b = browserAt(s.elapsed);
            return { elapsed: s.elapsed, native: s.opacity, browser: b?.opacity ?? null, nativeVisible: s.visible, browserVisible: b?.visible ?? null };
        });
        const disagree = compared.filter((row) => row.browser === null || Math.abs(row.native - row.browser) > tolerance || row.nativeVisible !== row.browserVisible);
        const fixed = Object.fromEntries([0, 2, 4, 6].map((t) => [t, browserAt(t)]));
        result.samples = compared.length;
        result.maxDifference = Math.max(0, ...compared.filter((row) => row.browser !== null).map((row) => Math.abs(row.native - row.browser)));
        result.disagreements = disagree.length;
        result.firstDisagreements = disagree.slice(0, 10);
        result.browserAtFixedTimes = fixed;
        result.nativeFirst = samples[0] || null;
        result.ok = compared.length > 0 && disagree.length === 0 && fixed[4]?.opacity === 0 && fixed[4]?.visible === false && fixed[6]?.opacity === 0;
        fs.writeFileSync(path.join(witnessDir, 'banner-compared.json'), JSON.stringify(compared, null, 2));
    } catch (error) {
        result.error = String(error.stderr || error.message || error);
    }
    fs.writeFileSync(outPath, JSON.stringify(result, null, 2));
    return result;
}

main().then((result) => {
    console.log(JSON.stringify({ ...result, witness: outPath }, null, 2));
    process.exit(result.ok ? 0 : 1);
});
