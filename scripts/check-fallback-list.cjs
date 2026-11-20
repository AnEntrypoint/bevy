#!/usr/bin/env node
const fs = require('node:fs');
const path = require('node:path');
const { checkFallbackList, windowEvidence } = require('./lib/fallback-list.cjs');

const footprintPath = path.join(__dirname, '..', 'target', 'witness', 'texture-footprint.json');

const consistent = () => ({
    decision: 'bc7',
    fallbackFormats: { astc4x4: true, etc2: true },
    formats: {
        bc7: { usable: true, loadEvidence: windowEvidence('2026-11-19T20:00:00.000Z', '2026-11-19T20:01:00.000Z') },
        astc4x4: { usable: true, loadEvidence: windowEvidence('2026-11-19T23:59:22.000Z', '2026-11-20T00:04:31.000Z') },
        etc2: { usable: true, loadEvidence: windowEvidence('2026-11-19T21:41:30.000Z', '2026-11-20T00:04:31.000Z') },
    },
});

const mutate = (edit) => {
    const footprint = consistent();
    edit(footprint);
    return footprint;
};

const cases = [
    { name: 'a consistent list with window evidence passes', footprint: consistent(), expectFail: false },
    { name: 'an ETC2 entry listed false while usable fails', footprint: mutate((f) => { f.fallbackFormats.etc2 = false; }), expectFail: true },
    { name: 'an extra key in the list fails', footprint: mutate((f) => { f.fallbackFormats.rgba8 = true; }), expectFail: true },
    { name: 'missing window evidence fails', footprint: mutate((f) => { delete f.formats.astc4x4.loadEvidence; }), expectFail: true },
    { name: 'a sameWindow flag that contradicts the gap fails', footprint: mutate((f) => { f.formats.etc2.loadEvidence.sameWindow = true; }), expectFail: true },
    { name: 'a decision that does not follow bc7 usable fails', footprint: mutate((f) => { f.decision = 'rgba8'; }), expectFail: true },
];

let fail = 0;
for (const testCase of cases) {
    const failures = checkFallbackList(testCase.footprint);
    const ok = testCase.expectFail ? failures.length > 0 : failures.length === 0;
    console.log(`${ok ? 'ok  ' : 'FAIL'} ${testCase.name}${failures.length ? ': ' + failures.join('; ') : ''}`);
    if (!ok) fail = 1;
}

if (!fs.existsSync(footprintPath)) {
    console.log('FAIL live footprint missing at ' + footprintPath);
    process.exit(2);
}
const live = JSON.parse(fs.readFileSync(footprintPath, 'utf8'));
const liveFailures = checkFallbackList(live);
console.log(`${liveFailures.length ? 'FAIL' : 'ok  '} live footprint list${liveFailures.length ? ': ' + liveFailures.join('; ') : ' matches the verdicts'}`);
if (liveFailures.length) fail = 1;
process.exit(fail);
