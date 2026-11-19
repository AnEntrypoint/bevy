#!/usr/bin/env node
const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const repo = path.resolve(__dirname, '..');
const witnessDir = path.join(repo, 'target', 'witness');
const linkDir = path.join(witnessDir, 'cross-adapter');
const referenceOfRecord = 'hw-rgba8';
const sets = {
    'hw-rgba8': 'neon-shots-rgba8',
    'sw-rgba8': 'neon-shots-rgba8-software',
    'hw-bc7': 'neon-shots-bc7',
    'sw-bc7': 'neon-shots-bc7-software',
    'sw-astc': 'neon-shots-astc4x4',
    'sw-etc2': 'neon-shots-etc2',
};
const pairs = {
    hwBc7: [referenceOfRecord, 'hw-bc7'],
    swBc7: [referenceOfRecord, 'sw-bc7'],
    swRgba8: [referenceOfRecord, 'sw-rgba8'],
    astc4x4: [referenceOfRecord, 'sw-astc'],
    etc2: [referenceOfRecord, 'sw-etc2'],
    swBc7VsSwRgba8: ['sw-rgba8', 'sw-bc7'],
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
const astcRecord = JSON.parse(fs.readFileSync(path.join(witnessDir, 'texture-visual-astc4x4.json'), 'utf8'));
const checks = {
    referenceOfRecordIsHardwareRgba8: referenceOfRecord === 'hw-rgba8',
    hwBc7PassesAgainstReference: results.hwBc7.ok === true,
    swBc7PassesAgainstReference: results.swBc7.ok === true,
    swRgba8DifferenceStandsAgainstReference: results.swRgba8.ok === false,
    astcRecordNamesReference: astcRecord.reference === sets[referenceOfRecord],
    astcPassesAgainstReference: astcRecord.ok === true && results.astc4x4.ok === true,
    supersededComparisonReproducesDifference: results.swBc7VsSwRgba8.ok === false && Math.abs(results.swBc7VsSwRgba8.psnrMedian - results.swRgba8.psnrMedian) <= 1,
};
const summary = Object.fromEntries(Object.entries(results).map(([key, value]) => [key, { reference: value.reference, ok: value.ok, psnrMin: value.psnrMin, psnrMedian: value.psnrMedian, failing: value.failing.length }]));
const finding = {
    name: 'software-rgba8-rendering-difference',
    reference: referenceOfRecord,
    psnrMedian: results.swRgba8.psnrMedian,
    psnrMin: results.swRgba8.psnrMin,
    reopen: 'a second hardware re-run of the software RGBA8 set, or a second adapter',
};
const ok = Object.values(checks).every(Boolean);
console.log(JSON.stringify({ ok, checks, finding, summary }, null, 2));
process.exit(ok ? 0 : 1);
