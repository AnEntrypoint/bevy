const fs = require('node:fs');
const path = require('node:path');
const { codeMap } = require('./lib/astc-endpoint-code.cjs');
const { nearestCodeDistance, tabulateNoCodeDistances } = require('./lib/astc-endpoint-distance.cjs');

const root = path.join(__dirname, '..');
const RANGE = 19;
const byValue = codeMap(RANGE, 192);
const codeValues = [...byValue.keys()];
const records = JSON.parse(fs.readFileSync(path.join(root, 'target', 'witness', 'astc-endpoint-records.json'), 'utf8'));
const table = tabulateNoCodeDistances(records, codeValues);
const total = [...table.values()].reduce((a, b) => a + b, 0);
console.log('candidates ' + records.length + ', no-code candidates ' + total);
for (const [key, count] of [...table.entries()].sort()) console.log('   GPU value, ' + key + ': ' + count + ' (' + (100 * count / total).toFixed(1) + '%)');
const gaps = [];
for (let v = 0; v < 256; v += 1) if (!byValue.has(v)) gaps.push(v);
const runs = [];
let run = 1;
for (let i = 1; i < gaps.length; i += 1) {
    if (gaps[i] === gaps[i - 1] + 1) run += 1; else { runs.push(run); run = 1; }
}
runs.push(run);
const runHist = new Map();
for (const r of runs) runHist.set(r, (runHist.get(r) || 0) + 1);
console.log('unproduced values in 0..255: ' + gaps.length + ', runs of consecutive unproduced values by length: ' + JSON.stringify([...runHist.entries()].sort((p, q) => p[0] - q[0])));
const baseline = new Map();
for (const g of gaps) {
    const key = 'distance ' + nearestCodeDistance(g, codeValues);
    baseline.set(key, (baseline.get(key) || 0) + 1);
}
for (const [key, count] of [...baseline.entries()].sort()) console.log('   baseline, every unproduced value, ' + key + ': ' + count + ' (' + (100 * count / gaps.length).toFixed(1) + '%)');
process.exit(0);
