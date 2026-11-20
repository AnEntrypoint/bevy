#!/usr/bin/env node
const { checkPairs } = require('./lib/astc-load-ratio.cjs');

const TOLERANCE = 1.1;
const run = (window, arm, startS, loadMs) => ({ window, arm, startS, loadMs, loadStart: 1 });

const cases = [
    { name: 'one interleaved pair within the tolerance passes', runs: [run('w1', 'astc', 1000, 1045), run('w1', 'reference', 1400, 1048)], check: (r) => r.interleaved && r.ok && r.pairs.length === 1 },
    { name: 'two interleaved pairs, both within the tolerance, pass', runs: [run('w1', 'astc', 1000, 1045), run('w1', 'reference', 1400, 1048), run('w1', 'astc', 1800, 1040), run('w1', 'reference', 2200, 1050)], check: (r) => r.interleaved && r.ok && r.pairs.length === 2 },
    { name: 'arms that run back to back are not interleaved', runs: [run('w1', 'astc', 1000, 1045), run('w1', 'astc', 1400, 1040), run('w1', 'reference', 1800, 1048), run('w1', 'reference', 2200, 1050)], check: (r) => !r.interleaved && !r.ok },
    { name: 'a pair split across two windows is not interleaved', runs: [run('w1', 'astc', 1000, 1045), run('w2', 'reference', 1400, 1048)], check: (r) => !r.interleaved && !r.ok },
    { name: 'an odd number of runs in a window is not interleaved', runs: [run('w1', 'astc', 1000, 1045), run('w1', 'reference', 1400, 1048), run('w1', 'astc', 1800, 1040)], check: (r) => !r.interleaved && !r.ok },
    { name: 'a pair ratio above the tolerance fails (1267 against 1048)', runs: [run('w1', 'astc', 1000, 1267), run('w1', 'reference', 1400, 1048)], check: (r) => r.interleaved && !r.ok && r.reason === 'a pair ratio is above the tolerance' },
    { name: 'one pair above the tolerance fails the whole check', runs: [run('w1', 'astc', 1000, 1045), run('w1', 'reference', 1400, 1048), run('w1', 'astc', 1800, 1267), run('w1', 'reference', 2200, 1048)], check: (r) => r.interleaved && !r.ok },
    { name: 'a pair at exactly the tolerance passes', runs: [run('w1', 'astc', 1000, 1100), run('w1', 'reference', 1400, 1000)], check: (r) => r.ok },
    { name: 'out-of-order input is sorted by start time', runs: [run('w1', 'reference', 1400, 1048), run('w1', 'astc', 1000, 1045)], check: (r) => r.interleaved && r.ok && r.pairs[0].order === 'astc,reference' },
    { name: 'no runs is not a pass', runs: [], check: (r) => !r.ok },
    { name: 'one interleaved ETC2 pair within the tolerance passes (955 against 1018)', runs: [run('w1', 'etc2', 1000, 955), run('w1', 'reference', 1400, 1018)], check: (r) => r.interleaved && r.ok && r.pairs[0].order === 'etc2,reference' },
    { name: 'ETC2 arms that run back to back are not interleaved', runs: [run('w1', 'etc2', 1000, 955), run('w1', 'etc2', 1400, 960), run('w1', 'reference', 1800, 1018), run('w1', 'reference', 2200, 1020)], check: (r) => !r.interleaved && !r.ok },
    { name: 'an ETC2 pair split across two windows is not interleaved', runs: [run('w1', 'etc2', 1000, 955), run('w2', 'reference', 1400, 1018)], check: (r) => !r.interleaved && !r.ok },
    { name: 'an ETC2 run with no reference in its window fails', runs: [run('w1', 'etc2', 1000, 955), run('w1', 'etc2', 1400, 960)], check: (r) => !r.ok && r.reason === 'window w1: a pair lacks one arm' },
    { name: 'an ETC2 pair ratio above the tolerance fails (1200 against 1018)', runs: [run('w1', 'etc2', 1000, 1200), run('w1', 'reference', 1400, 1018)], check: (r) => r.interleaved && !r.ok && r.reason === 'a pair ratio is above the tolerance' },
    { name: 'an ETC2 pair at exactly the tolerance passes', runs: [run('w1', 'etc2', 1000, 1100), run('w1', 'reference', 1400, 1000)], check: (r) => r.ok },
];

let fail = 0;
for (const testCase of cases) {
    const result = checkPairs(testCase.runs, TOLERANCE);
    const ok = testCase.check(result);
    console.log(`${ok ? 'ok  ' : 'FAIL'} ${testCase.name}`);
    if (!ok) fail = 1;
}
process.exit(fail);
