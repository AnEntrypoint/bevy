#!/usr/bin/env node
const fs = require('node:fs');
const path = require('node:path');
const { withGateWitness } = require('./lib/playwriter-witness.cjs');

const repo = path.resolve(__dirname, '..');
const port = Number(process.env.PORT || 8092);
const witnessDir = path.join(repo, 'target', 'witness');
const nativePath = path.join(witnessDir, 'hud-fixture-native.json');
const rowsPath = path.join(witnessDir, 'css-browser-rows.json');
const outPath = path.join(witnessDir, 'css-agreement.json');
const tolerance = Number(process.env.OPACITY_TOLERANCE || 0.02);
const seekedAnimations = ['dzNEXUS', 'lhAnim'];
const transitions = ['trFade', 'trHide'];

function snippet(pageUrl, plan, out) {
    return `
return await (async () => {
const page = await context.newPage();
await page.goto(${JSON.stringify(pageUrl)}, { waitUntil: 'domcontentloaded' });
await page.waitForFunction(() => window.__FIXTURE_READY === true, null, { timeout: 30000 });
const rows = await page.evaluate((plan) => {
  const read = (element) => {
    const style = getComputedStyle(element);
    return { opacity: Number(style.opacity), visible: style.visibility === 'visible' };
  };
  const rows = [];
  for (const key of plan.seeked) {
    const element = document.getElementById(key);
    const animation = element.getAnimations().find((item) => item.animationName);
    animation.pause();
    for (const elapsed of plan.times[key] || []) {
      animation.currentTime = elapsed * 1000;
      rows.push({ key, mode: 'seek-animation', time: elapsed, ...read(element) });
    }
  }
  const paused = document.getElementById('lhPaused');
  const pausedAnimation = paused.getAnimations()[0];
  rows.push({ key: 'lhPaused', mode: 'as-is', time: null, playState: pausedAnimation ? pausedAnimation.playState : null, currentTime: pausedAnimation ? pausedAnimation.currentTime : null, ...read(paused) });
  for (const key of plan.transitions) rows.push({ key, mode: 'before-change', time: null, ...read(document.getElementById(key)) });
  window.__renderStep(1);
  for (const key of plan.transitions) {
    const element = document.getElementById(key);
    getComputedStyle(element).opacity;
    const running = element.getAnimations().filter((item) => item.transitionProperty);
    running.forEach((item) => item.pause());
    rows.push({ key, mode: 'transitions', time: null, properties: running.map((item) => item.transitionProperty) });
    for (const elapsed of plan.transitionTimes[key] || []) {
      running.forEach((item) => { item.currentTime = elapsed * 1000; });
      rows.push({ key, mode: 'seek-transition', time: elapsed, ...read(element) });
    }
  }
  return rows;
}, ${JSON.stringify(plan)});
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
    const of = (key) => samples.filter((s) => s.key === key);
    const plan = {
        seeked: seekedAnimations,
        transitions,
        times: Object.fromEntries(seekedAnimations.map((key) => [key, [...new Set([...of(key).map((s) => s.elapsed), 0, 2, 4, 6])]])),
        transitionTimes: Object.fromEntries(transitions.map((key) => [key, [...new Set(of(key).filter((s) => s.transitionElapsed !== null).map((s) => s.transitionElapsed))]])),
    };
    try {
        const { value, browser, gate } = await withGateWitness({ repo, port }, async ({ pageBase, run }) => {
            const pageUrl = new URL('/testdata/hud-fixture/browser.html', pageBase).toString();
            run(snippet(pageUrl, plan, rowsPath), 180000);
            return JSON.parse(fs.readFileSync(rowsPath, 'utf8'));
        });
        result.browser = browser;
        result.gate = gate;
        const find = (key, mode, time) => value.find((row) => row.key === key && row.mode === mode && (time === null || row.time === time));
        const compared = samples.map((s) => {
            let mode = 'seek-animation';
            let time = s.elapsed;
            if (s.key === 'lhPaused') {
                mode = 'as-is';
                time = null;
            } else if (transitions.includes(s.key)) {
                mode = s.transitionElapsed === null ? 'before-change' : 'seek-transition';
                time = s.transitionElapsed;
            }
            const b = find(s.key, mode, time);
            return { key: s.key, mode, time, native: s.opacity, browser: b?.opacity ?? null, nativeVisible: s.visible, browserVisible: b?.visible ?? null };
        });
        const disagree = compared.filter((row) => row.browser === null || Math.abs(row.native - row.browser) > tolerance || row.nativeVisible !== row.browserVisible);
        result.byKey = Object.fromEntries([...seekedAnimations, 'lhPaused', ...transitions].map((key) => {
            const rows = compared.filter((row) => row.key === key);
            const bad = rows.filter((row) => disagree.includes(row));
            return [key, { samples: rows.length, disagreements: bad.length, maxDifference: Math.max(0, ...rows.filter((row) => row.browser !== null).map((row) => Math.abs(row.native - row.browser))), modes: [...new Set(rows.map((row) => row.mode))] }];
        }));
        result.browserTransitions = value.filter((row) => row.mode === 'transitions').map((row) => ({ key: row.key, properties: row.properties }));
        result.browserPaused = value.find((row) => row.key === 'lhPaused');
        result.bannerFixed = Object.fromEntries([0, 2, 4, 6].map((t) => [t, find('dzNEXUS', 'seek-animation', t)]));
        result.samples = compared.length;
        result.disagreements = disagree.length;
        result.firstDisagreements = disagree.slice(0, 12);
        const everyKeySampled = Object.values(result.byKey).every((entry) => entry.samples > 0);
        const transitionsSeen = result.browserTransitions.every((row) => row.properties.length > 0);
        result.ok = compared.length > 0 && disagree.length === 0 && everyKeySampled && transitionsSeen && result.byKey.trFade.modes.includes('seek-transition') && result.byKey.trHide.modes.includes('seek-transition');
        fs.writeFileSync(path.join(witnessDir, 'css-compared.json'), JSON.stringify(compared, null, 2));
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
