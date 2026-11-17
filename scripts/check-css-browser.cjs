#!/usr/bin/env node
const fs = require('node:fs');
const path = require('node:path');
const { withGateWitness } = require('./lib/playwriter-witness.cjs');

const repo = path.resolve(__dirname, '..');
const port = Number(process.env.PORT || 8092);
const witnessDir = path.join(repo, 'target', 'witness');
const nativePath = path.join(witnessDir, 'hud-fixture-native.json');
const rowsPath = path.join(witnessDir, 'css-browser-rows.json');
const planPath = path.join(witnessDir, 'css-browser-plan.json');
const outPath = path.join(witnessDir, 'css-agreement.json');
const tolerance = { opacity: 0.02, channel: 1.5, matrix: 0.002, pixels: 0.05 };
const transitionKeys = ['trFade', 'trHide', 'trRev'];

function plan(samples) {
    return samples.map((s, index) => {
        if (!transitionKeys.includes(s.key)) return { index, key: s.key, mode: 'animation', times: s.animationElapsed };
        if (s.transitionElapsed === null) return { index, key: s.key, mode: 'before', time: null };
        if (s.key === 'trRev' && s.reversedAt !== null) return { index, key: s.key, mode: 'reversed', time: s.transitionElapsed };
        return { index, key: s.key, mode: 'forward', time: s.transitionElapsed };
    });
}

function snippet(pageUrl, plan, out) {
    return `
return await (async () => {
const { steps, reversedAt } = JSON.parse(require('node:fs').readFileSync(${JSON.stringify(plan)}, 'utf8'));
const page = await context.newPage();
await page.goto(${JSON.stringify(pageUrl)}, { waitUntil: 'domcontentloaded' });
await page.waitForFunction(() => window.__FIXTURE_READY === true, null, { timeout: 30000 });
const result = await page.evaluate(({ steps, reversedAt }) => {
  const read = (element) => {
    const style = getComputedStyle(element);
    return { opacity: Number(style.opacity), visible: style.visibility === 'visible', color: style.color, background: style.backgroundColor, transform: style.transform };
  };
  const rows = [];
  const animated = new Map();
  for (const step of steps.filter((s) => s.mode === 'animation')) {
    const element = document.getElementById(step.key);
    if (!animated.has(step.key)) {
      const list = element.getAnimations().filter((item) => item.animationName);
      list.forEach((item) => item.pause());
      animated.set(step.key, list);
    }
    animated.get(step.key).forEach((item, i) => { item.currentTime = (step.times[i] ?? 0) * 1000; });
    rows.push({ index: step.index, ...read(element) });
  }
  for (const step of steps.filter((s) => s.mode === 'before')) rows.push({ index: step.index, ...read(document.getElementById(step.key)) });
  window.__renderStep(1);
  const firstTransitions = {};
  for (const key of ${JSON.stringify(transitionKeys)}) {
    const element = document.getElementById(key);
    getComputedStyle(element).opacity;
    const list = element.getAnimations().filter((item) => item.transitionProperty);
    list.forEach((item) => item.pause());
    firstTransitions[key] = list;
  }
  for (const step of steps.filter((s) => s.mode === 'forward')) {
    firstTransitions[step.key].forEach((item) => { item.currentTime = step.time * 1000; });
    rows.push({ index: step.index, ...read(document.getElementById(step.key)) });
  }
  const meta = { firstTransitions: Object.fromEntries(Object.entries(firstTransitions).map(([key, list]) => [key, list.map((item) => item.transitionProperty)])) };
  if (reversedAt !== null) {
    const element = document.getElementById('trRev');
    firstTransitions.trRev.forEach((item) => { item.currentTime = reversedAt * 1000; });
    meta.valueAtReversal = Number(getComputedStyle(element).opacity);
    window.__renderStep(2);
    getComputedStyle(element).opacity;
    const second = element.getAnimations().filter((item) => item.transitionProperty);
    second.forEach((item) => item.pause());
    meta.reversedTransitions = second.map((item) => ({ property: item.transitionProperty, duration: item.effect.getComputedTiming().duration }));
    for (const step of steps.filter((s) => s.mode === 'reversed')) {
      second.forEach((item) => { item.currentTime = step.time * 1000; });
      rows.push({ index: step.index, ...read(element) });
    }
  }
  return { rows, meta };
}, { steps, reversedAt });
await page.close();
require('node:fs').writeFileSync(${JSON.stringify(out)}, JSON.stringify(result));
return 'rows ' + result.rows.length;
})();
`;
}

function parseRgb(text) {
    const match = /rgba?\(([^)]+)\)/.exec(text || '');
    if (!match) return null;
    const parts = match[1].split(',').map((part) => Number(part.trim()));
    return [parts[0], parts[1], parts[2], parts.length > 3 ? parts[3] : 1];
}

function parseMatrix(text) {
    if (!text || text === 'none') return [1, 0, 0, 1, 0, 0];
    const match = /matrix\(([^)]+)\)/.exec(text);
    return match ? match[1].split(',').map((part) => Number(part.trim())) : null;
}

function nativeMatrix(t) {
    const [tx, ty, sx, sy, rotation] = t;
    const cos = Math.cos(rotation);
    const sin = Math.sin(rotation);
    return [sx * cos, sx * sin, -sy * sin, sy * cos, tx, ty];
}

function compare(sample, row) {
    const problems = [];
    if (!row) return ['no browser row'];
    if (Math.abs(sample.opacity - row.opacity) > tolerance.opacity) problems.push(`opacity ${sample.opacity} vs ${row.opacity}`);
    if (sample.visible !== row.visible) problems.push(`visible ${sample.visible} vs ${row.visible}`);
    if (sample.color) {
        const browser = parseRgb(row.color);
        const native = sample.color.map((v, i) => (i < 3 ? v * 255 : v));
        if (!browser || native.slice(0, 3).some((v, i) => Math.abs(v - browser[i]) > tolerance.channel) || Math.abs(native[3] - browser[3]) > 0.02) problems.push(`color ${JSON.stringify(native)} vs ${row.color}`);
    }
    if (sample.background) {
        const browser = parseRgb(row.background);
        const native = sample.background.map((v, i) => (i < 3 ? v * 255 : v));
        if (!browser || native.slice(0, 3).some((v, i) => Math.abs(v - browser[i]) > tolerance.channel) || Math.abs(native[3] - browser[3]) > 0.02) problems.push(`background ${JSON.stringify(native)} vs ${row.background}`);
    }
    if (sample.transform) {
        const browser = parseMatrix(row.transform);
        const native = nativeMatrix(sample.transform);
        if (!browser || native.some((v, i) => Math.abs(v - browser[i]) > (i < 4 ? tolerance.matrix : tolerance.pixels))) problems.push(`transform ${JSON.stringify(native.map((v) => Number(v.toFixed(4))))} vs ${row.transform}`);
    }
    return problems;
}

async function main() {
    const result = { ok: false, tolerance };
    if (!fs.existsSync(nativePath)) {
        result.error = `run scripts/check-hud-fixture.cjs first; ${nativePath} is missing`;
        return result;
    }
    const native = JSON.parse(fs.readFileSync(nativePath, 'utf8'));
    const samples = native.samples || [];
    const steps = plan(samples);
    const reversedAt = samples.find((s) => s.key === 'trRev' && s.reversedAt !== null)?.reversedAt ?? null;
    try {
        const { value, browser, gate } = await withGateWitness({ repo, port }, async ({ pageBase, run }) => {
            const pageUrl = new URL('/testdata/hud-fixture/browser.html', pageBase).toString();
            fs.writeFileSync(planPath, JSON.stringify({ steps, reversedAt }));
            run(snippet(pageUrl, planPath, rowsPath), 300000);
            return JSON.parse(fs.readFileSync(rowsPath, 'utf8'));
        });
        result.browser = browser;
        result.gate = gate;
        const rows = new Map(value.rows.map((row) => [row.index, row]));
        const compared = samples.map((sample, index) => ({ key: sample.key, mode: steps[index].mode, problems: compare(sample, rows.get(index)) }));
        result.byKey = {};
        for (const row of compared) {
            const entry = (result.byKey[row.key] ||= { samples: 0, disagreements: 0, modes: [] });
            entry.samples += 1;
            if (row.problems.length) entry.disagreements += 1;
            if (!entry.modes.includes(row.mode)) entry.modes.push(row.mode);
        }
        result.samples = compared.length;
        result.disagreements = compared.filter((row) => row.problems.length).length;
        result.firstDisagreements = compared.filter((row) => row.problems.length).slice(0, 12);
        result.checked = {
            colour: samples.filter((s) => s.color).length,
            background: samples.filter((s) => s.background).length,
            transform: samples.filter((s) => s.transform).length,
        };
        result.meta = value.meta;
        result.nativeReversal = { reversedAt, shortenedDuration: null };
        const reversedTrack = samples.filter((s) => s.key === 'trRev' && s.reversedAt !== null);
        result.nativeReversal.samples = reversedTrack.length;
        const expectedKeys = ['dzNEXUS', 'lhAnim', 'lhPaused', 'trFade', 'trHide', 'anTransform', 'anCompose', 'anColor', 'dirNormal', 'dirReverse', 'dirAlternate', 'dirAltReverse', 'ppMixed', 'trRev'];
        const allKeys = expectedKeys.every((key) => result.byKey[key]?.samples > 0);
        result.ok = result.disagreements === 0 && allKeys && result.byKey.trRev.modes.includes('reversed') && result.checked.transform > 0 && result.checked.background > 0;
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
