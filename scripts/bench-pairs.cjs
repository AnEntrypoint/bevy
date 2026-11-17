#!/usr/bin/env node
const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const repo = path.resolve(__dirname, '..');
const jayzee = process.env.JAYZEE || '/workspace/jayzee';
const binary = process.env.BINARY || path.join(repo, 'target', 'release', 'bevy-jayzee');
const pairs = Number(process.env.PAIRS || 8);
const ticks = Number(process.env.TICKS || 300);
const sizes = (process.env.SIZES || '500,2000,5000').split(',').map(Number);
const baseline = { 500: 0.43 / 0.40, 2000: 2.78 / 1.04, 5000: 8.96 / 1.88 };
const outPath = path.join(repo, 'target', 'witness', 'bench-pairs.json');

const load = () => Number(fs.readFileSync('/proc/loadavg', 'utf8').split(' ')[0]);

function runJayzee(bodies) {
    const before = load();
    const run = spawnSync('node', ['scripts/bench.mjs', String(bodies), String(ticks)], { cwd: jayzee, encoding: 'utf8' });
    if (run.status !== 0) throw new Error(`jayzee bench failed: ${run.stderr}`);
    const parsed = JSON.parse(run.stdout.trim().split('\n').pop());
    return { meanMs: parsed.meanMs, p95Ms: parsed.p95Ms, loadBefore: before, loadAfter: load() };
}

function runBevy(bodies) {
    const before = load();
    const run = spawnSync(binary, ['bench', String(bodies), String(ticks)], { cwd: repo, encoding: 'utf8' });
    if (run.status !== 0) throw new Error(`bevy bench failed: ${run.stderr}`);
    const line = run.stdout.split('\n').find((row) => row.includes('"kind":"bench"'));
    const parsed = JSON.parse(line);
    return { meanMs: parsed.meanMs, p95Ms: parsed.p95Ms, hash: parsed.hash, loadBefore: before, loadAfter: load() };
}

const median = (values) => {
    const sorted = [...values].sort((a, b) => a - b);
    const middle = Math.floor(sorted.length / 2);
    return sorted.length % 2 ? sorted[middle] : (sorted[middle - 1] + sorted[middle]) / 2;
};

function main() {
    const result = { pairs, ticks, node: process.version, sizes: [] };
    for (const bodies of sizes) {
        const rows = [];
        for (let index = 0; index < pairs; index += 1) {
            const bevyFirst = index % 2 === 0;
            const first = bevyFirst ? runBevy(bodies) : runJayzee(bodies);
            const second = bevyFirst ? runJayzee(bodies) : runBevy(bodies);
            const bevy = bevyFirst ? first : second;
            const jz = bevyFirst ? second : first;
            rows.push({ pair: index, order: bevyFirst ? 'bevy,jayzee' : 'jayzee,bevy', jayzeeMeanMs: jz.meanMs, bevyMeanMs: bevy.meanMs, ratio: Number((jz.meanMs / bevy.meanMs).toFixed(4)), jayzeeLoad: [jz.loadBefore, jz.loadAfter], bevyLoad: [bevy.loadBefore, bevy.loadAfter], bevyHash: bevy.hash });
        }
        const ratios = rows.map((row) => row.ratio);
        const mid = median(ratios);
        const spread = Math.max(...ratios) - Math.min(...ratios);
        const difference = Math.abs(mid - 1);
        const conclusive = spread <= difference;
        result.sizes.push({
            bodies,
            medianRatio: Number(mid.toFixed(4)),
            minRatio: Math.min(...ratios),
            maxRatio: Math.max(...ratios),
            spread: Number(spread.toFixed(4)),
            differenceFromParity: Number(difference.toFixed(4)),
            verdict: conclusive ? (mid > 1 ? 'bevy faster' : 'jayzee faster') : 'inconclusive',
            baselineRatio: Number(baseline[bodies]?.toFixed(4) ?? NaN),
            loadRange: [Math.min(...rows.flatMap((row) => [...row.jayzeeLoad, ...row.bevyLoad])), Math.max(...rows.flatMap((row) => [...row.jayzeeLoad, ...row.bevyLoad]))],
            hashes: [...new Set(rows.map((row) => row.bevyHash))],
            rows,
        });
    }
    fs.mkdirSync(path.dirname(outPath), { recursive: true });
    fs.writeFileSync(outPath, JSON.stringify(result, null, 2));
    console.log(JSON.stringify({ pairs, ticks, sizes: result.sizes.map(({ rows, ...size }) => size), witness: outPath }, null, 2));
}

main();
