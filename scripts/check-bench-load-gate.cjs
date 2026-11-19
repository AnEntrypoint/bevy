#!/usr/bin/env node
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const script = path.join(__dirname, 'bench-median.cjs');
const quiet = '1.20 1.10 1.00 2/300 1';
const loaded = '5.50 5.20 4.80 2/300 1';
const row = (jayzee, bevy, load) => JSON.stringify({ bodies: 500, jayzee: { meanMs: jayzee }, bevy: { meanMs: bevy }, loadBefore: load, loadAfter: load });
const cases = [
    {
        name: 'quiet rows give a speedup',
        rows: [row(0.43, 0.40, quiet), row(0.44, 0.40, quiet), row(0.43, 0.41, quiet)],
        expect: { refused: false, quietRuns: 3, speedup: 1.075 },
    },
    {
        name: 'loaded rows are refused',
        rows: [row(0.43, 0.40, loaded), row(0.44, 0.40, loaded), row(0.43, 0.41, loaded)],
        expect: { refused: true, quietRuns: 0, speedup: null },
    },
    {
        name: 'mixed rows use only the quiet ones',
        rows: [row(0.43, 0.40, quiet), row(0.44, 0.40, quiet), row(0.43, 0.41, loaded)],
        expect: { refused: false, quietRuns: 2, noisyRuns: 1, speedup: 1.0875 },
    },
];
const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'bench-gate-'));
const results = cases.map((testCase, index) => {
    const outPath = path.join(dir, `case-${index}.json`);
    const run = spawnSync('node', [script, outPath], { input: testCase.rows.join('\n') + '\n', encoding: 'utf8', env: { ...process.env, CORES: '12' } });
    if (run.status !== 0) return { name: testCase.name, ok: false, error: run.stderr };
    const size = JSON.parse(fs.readFileSync(outPath, 'utf8')).table[0];
    const failures = Object.entries(testCase.expect).filter(([key, value]) => (typeof value === 'number' && Math.abs(size[key] - value) > 0.0001) || (typeof value !== 'number' && size[key] !== value)).map(([key, value]) => `${key} expected ${value} got ${size[key]}`);
    return { name: testCase.name, ok: failures.length === 0, failures, refusal: size.refusal, speedup: size.speedup, quietRuns: size.quietRuns };
});
fs.rmSync(dir, { recursive: true, force: true });
const ok = results.every((result) => result.ok);
console.log(JSON.stringify({ loadLimit: process.env.LOAD_LIMIT || 4, ok, results }, null, 2));
process.exit(ok ? 0 : 1);
