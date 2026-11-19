#!/usr/bin/env node
const fs = require('node:fs');

const out = process.argv[2];
const loadMove = Number(process.env.LOAD_MOVE || 0.5);
const loadLimit = Number(process.env.LOAD_LIMIT || 4);
const cores = Number(process.env.CORES || 1);
const rows = fs.readFileSync(0, 'utf8').trim().split('\n').filter(Boolean).map((line) => JSON.parse(line));

const load1 = (raw) => Number(raw.split(' ')[0]);
const runnable = (raw) => Math.max(0, Number(raw.split(' ')[3].split('/')[0]) - 1);

for (const row of rows) {
    const reasons = [];
    if (Math.abs(load1(row.loadAfter) - load1(row.loadBefore)) > loadMove) reasons.push(`load1 moved by more than ${loadMove}`);
    if (Math.max(load1(row.loadBefore), load1(row.loadAfter)) > loadLimit) reasons.push(`load1 above ${loadLimit}`);
    if (Math.max(runnable(row.loadBefore), runnable(row.loadAfter)) >= cores / 2) reasons.push(`other runnable tasks reached ${cores / 2} or more`);
    row.noisy = reasons.length > 0;
    row.noisyReasons = reasons;
}

const median = (values) => {
    const sorted = [...values].sort((a, b) => a - b);
    const mid = Math.floor(sorted.length / 2);
    return sorted.length % 2 ? sorted[mid] : (sorted[mid - 1] + sorted[mid]) / 2;
};
const round = (value) => (value === null ? null : Number(value.toFixed(4)));

const table = [...new Set(rows.map((row) => row.bodies))].map((bodies) => {
    const set = rows.filter((row) => row.bodies === bodies);
    const quiet = set.filter((row) => !row.noisy);
    const refused = quiet.length === 0;
    const jayzee = (list) => round(median(list.map((row) => row.jayzee.meanMs)));
    const bevy = (list) => round(median(list.map((row) => row.bevy.meanMs)));
    return {
        bodies,
        runs: set.length,
        noisyRuns: set.length - quiet.length,
        quietRuns: quiet.length,
        refused,
        refusal: refused ? `no quiet run at this size: every run had 1-minute load above ${loadLimit} or another noise reason` : null,
        speedup: refused ? null : round(jayzee(quiet) / bevy(quiet)),
        jayzeeMedianMs: jayzee(set),
        bevyMedianMs: bevy(set),
        jayzeeQuietMedianMs: refused ? null : jayzee(quiet),
        bevyQuietMedianMs: refused ? null : bevy(quiet),
    };
});

const result = { ticks: rows[0]?.jayzee.ticks, cores, loadMove, loadLimit, table, rows };
fs.writeFileSync(out, JSON.stringify(result, null, 2));
console.log(JSON.stringify({ ticks: result.ticks, cores, loadMove, loadLimit, table }, null, 2));
