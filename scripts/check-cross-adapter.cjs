#!/usr/bin/env node
const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const repo = path.resolve(__dirname, '..');
const witnessDir = path.join(repo, 'target', 'witness');
const linkDir = path.join(witnessDir, 'cross-adapter');
const sets = {
    'hw-rgba8': 'neon-shots-rgba8',
    'sw-rgba8': 'neon-shots-rgba8-software',
    'hw-bc7': 'neon-shots-bc7',
    'sw-bc7': 'neon-shots-bc7-software',
    'sw-astc': 'neon-shots-astc4x4',
    'sw-etc2': 'neon-shots-etc2',
};
const pairs = {
    hwBc7VsHwRgba8: ['hw-rgba8', 'hw-bc7'],
    swBc7VsHwRgba8: ['hw-rgba8', 'sw-bc7'],
    swRgba8VsHwRgba8: ['hw-rgba8', 'sw-rgba8'],
    swBc7VsSwRgba8: ['sw-rgba8', 'sw-bc7'],
    swAstcVsHwRgba8: ['hw-rgba8', 'sw-astc'],
    swAstcVsSwRgba8: ['sw-rgba8', 'sw-astc'],
    swEtc2VsHwRgba8: ['hw-rgba8', 'sw-etc2'],
    swEtc2VsSwRgba8: ['sw-rgba8', 'sw-etc2'],
};
fs.mkdirSync(linkDir, { recursive: true });
const results = {};
for (const [key, [left, right]] of Object.entries(pairs)) {
    const link = path.join(linkDir, key);
    fs.rmSync(link, { force: true });
    fs.symlinkSync(path.join(witnessDir, sets[right]), link);
    spawnSync('node', [path.join(__dirname, 'compare-screenshots.cjs')], {
        env: { ...process.env, SHOTS_LEFT: path.join(witnessDir, sets[left]), SHOTS_RIGHT: link },
        stdio: 'ignore',
    });
    results[key] = JSON.parse(fs.readFileSync(path.join(witnessDir, `texture-visual-${key}.json`), 'utf8'));
}
const checks = {
    hwBc7MatchesHwRgba8: results.hwBc7VsHwRgba8.ok === true,
    swBc7MatchesHwRgba8: results.swBc7VsHwRgba8.ok === true,
    swRgba8DeviatesFromHwRgba8: results.swRgba8VsHwRgba8.ok === false,
    swBc7FailureIsReferenceDeviation: results.swBc7VsSwRgba8.ok === false && Math.abs(results.swBc7VsSwRgba8.psnrMedian - results.swRgba8VsHwRgba8.psnrMedian) <= 1,
};
const summary = Object.fromEntries(Object.entries(results).map(([key, value]) => [key, { ok: value.ok, psnrMin: value.psnrMin, psnrMedian: value.psnrMedian, failing: value.failing.length }]));
const ok = Object.values(checks).every(Boolean);
console.log(JSON.stringify({ ok, checks, summary }, null, 2));
process.exit(ok ? 0 : 1);
