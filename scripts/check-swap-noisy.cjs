#!/usr/bin/env node
const { classify } = require('./lib/swap-noisy.cjs');

const oneFile = [
    ['jobs12-onefile', 3.6, 6.6, 0, false],
    ['jobs12-onefile', 4.0, 1.4, 0, false],
    ['q40-onefile', 3.4, 3.6, 0, false],
    ['v40-onefile-3', 3.1, 1.8, 0, false],
    ['final-onefile', 8.4, 13, 0, true],
    ['r40-onefile', 8.7, 1.3, 142.4, true],
    ['v40-onefile-2', 14.1, 25.8, 0, true],
    ['r40b-onefile', 10.8, 15.6, 1.4, true],
    ['r41-onefile', 17.2, 27.1, 493.9, true],
    ['v40-onefile-1', 16.3, 29.2, 716.5, true],
    ['p40-onefile', 10.7, 5.1, 638.4, true],
];
const failures = [];
for (const [label, wall, swapInMiB, swapOutMiB, expected] of oneFile) {
    if (wall <= 4.0 && expected) failures.push(`${label}: quiet-band wall ${wall} classified noisy`);
    if (wall >= 8.4 && !expected) failures.push(`${label}: elevated wall ${wall} classified quiet`);
    const { swapNoisy } = classify({ crates: 1, swapInMiB, swapOutMiB });
    if (swapNoisy !== expected) failures.push(`${label}: expected swapNoisy ${expected}, got ${swapNoisy}`);
}
for (const [label, crates] of [['r43-on-1', 452], ['r43-off-3', 452], ['r42-b-release', 1]]) {
    if (crates > 1 && classify({ crates, swapInMiB: 1000, swapOutMiB: 1000 }).swapNoisy !== null) failures.push(`${label}: multi-crate build must be null`);
}
if (classify({ crates: 0, swapInMiB: 9, swapOutMiB: 0 }).swapNoisy !== false) failures.push('no-change build at 9 MiB must be quiet');
if (classify({ crates: 0, swapInMiB: 6, swapOutMiB: 5 }).swapNoisy !== true) failures.push('no-change build at 11 MiB must be noisy');
const ok = failures.length === 0;
console.log(JSON.stringify({ ok, checked: oneFile.length + 5, failures }, null, 2));
process.exit(ok ? 0 : 1);
