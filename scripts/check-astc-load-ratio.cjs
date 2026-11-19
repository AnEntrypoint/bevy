#!/usr/bin/env node
const { decideLoad, formatLoadLine } = require('./lib/astc-load-ratio.cjs');

const cases = [
    { name: 'ASTC 1267 ms against the software RGBA8 reference 1048 ms: ratio 1.21, not usable', run: () => decideLoad({ astcMs: 1267, referenceMs: 1048, tolerance: 1.1 }), check: (d) => Math.abs(d.ratio - 1.20897) < 0.0001 && d.usable === false },
    { name: 'the stale reference 1250 ms made the same ASTC load pass', run: () => decideLoad({ astcMs: 1267, referenceMs: 1250, tolerance: 1.1 }), check: (d) => d.usable === true },
    { name: 'a load exactly at the tolerance is usable', run: () => decideLoad({ astcMs: 1100, referenceMs: 1000, tolerance: 1.1 }), check: (d) => d.usable === true },
    { name: 'one millisecond over the tolerance is not usable', run: () => decideLoad({ astcMs: 1101, referenceMs: 1000, tolerance: 1.1 }), check: (d) => d.usable === false },
    { name: 'equal loads give ratio 1 and are usable', run: () => decideLoad({ astcMs: 1048, referenceMs: 1048, tolerance: 1.1 }), check: (d) => d.ratio === 1 && d.usable === true },
    { name: 'a null ASTC load is rejected', run: () => decideLoad({ astcMs: null, referenceMs: 1048, tolerance: 1.1 }), expectThrow: true },
    { name: 'a zero reference load is rejected', run: () => decideLoad({ astcMs: 1100, referenceMs: 0, tolerance: 1.1 }), expectThrow: true },
];

let fail = 0;
for (const testCase of cases) {
    let ok = false;
    if (testCase.expectThrow) {
        try {
            testCase.run();
        } catch {
            ok = true;
        }
    } else {
        ok = testCase.check(testCase.run());
    }
    console.log(`${ok ? 'ok  ' : 'FAIL'} ${testCase.name}`);
    if (!ok) fail = 1;
}

const line = formatLoadLine({ run_ok: true, astc_ms: 1267, reference_ms: 1048, ratio: '1.209', tolerance: 1.1, load_ok: false });
const lineOk = line === 'ASTC_LOAD run_ok=true astc_ms=1267 reference_ms=1048 ratio=1.209 tolerance=1.1 load_ok=false';
console.log(`${lineOk ? 'ok  ' : 'FAIL'} the result line has the fixed field order`);
if (!lineOk) fail = 1;
process.exit(fail);
