#!/usr/bin/env node
const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const repo = path.resolve(__dirname, '..');
const witnessDir = path.join(repo, 'target', 'witness');
const linkDir = path.join(witnessDir, 'cross-adapter');
const buildMs = fs.statSync(path.join(repo, 'web', 'wasm', 'bevy-jayzee-wasm_bg.wasm')).mtimeMs;
const referenceOfRecord = 'hw-rgba8';
const sets = {
    'hw-rgba8': 'neon-shots-rgba8',
    'sw-rgba8': 'neon-shots-rgba8-software',
    'sw-rgba8-stale': 'stale-before-r58/neon-shots-rgba8-software',
    'hw-bc7': 'neon-shots-bc7',
    'sw-bc7': 'neon-shots-bc7-software',
    'sw-astc': 'neon-shots-astc4x4',
    'sw-etc2': 'neon-shots-etc2',
    'sw-etc2-stale': 'stale-before-r58/neon-shots-etc2',
};
const pairs = {
    hwBc7: [referenceOfRecord, 'hw-bc7'],
    swBc7: [referenceOfRecord, 'sw-bc7'],
    currentSwRgba8: [referenceOfRecord, 'sw-rgba8'],
    astc4x4: [referenceOfRecord, 'sw-astc'],
    etc2: [referenceOfRecord, 'sw-etc2'],
};
function newestAndOldestMs(name) {
    const dir = path.join(witnessDir, name);
    if (!fs.existsSync(dir)) return null;
    const pngs = fs.readdirSync(dir).filter((file) => file.endsWith('.png'));
    if (!pngs.length) return null;
    const times = pngs.map((file) => fs.statSync(path.join(dir, file)).mtimeMs);
    return Math.min(...times);
}
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
const usedSets = Object.values(pairs).flatMap(([left, right]) => [left, right]);
const setsCurrent = usedSets.every((key) => {
    const ms = newestAndOldestMs(sets[key]);
    return ms !== null && ms >= buildMs;
});
const checks = {
    referenceOfRecordIsHardwareRgba8: referenceOfRecord === 'hw-rgba8',
    setsAreCurrentBuild: setsCurrent,
    hwBc7PassesAgainstReference: results.hwBc7.ok === true,
    swBc7PassesAgainstReference: results.swBc7.ok === true,
    currentSoftwareRgba8PassesAgainstReference: results.currentSwRgba8.ok === true,
    astcRecordNamesReference: results.astc4x4.reference === sets[referenceOfRecord],
    astcPassesAgainstReference: results.astc4x4.ok === true,
    etc2RecordNamesReference: results.etc2.reference === sets[referenceOfRecord],
};
const summary = Object.fromEntries(Object.entries(results).map(([key, value]) => [key, { reference: value.reference, source: value.source, ok: value.ok, psnrMin: value.psnrMin, psnrMedian: value.psnrMedian, failing: value.failing.length }]));
const ok = Object.values(checks).every(Boolean);
console.log(JSON.stringify({ ok, checks, summary }, null, 2));
process.exit(ok ? 0 : 1);
