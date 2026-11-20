const fs = require('node:fs');
const path = require('node:path');

const root = path.join(__dirname, '..');
const LOGS = ['window-gate-r116.log', 'window-gate-r117.log', 'window-gate-r118.log'];
const SECONDS_PER_READING = 30;

const runs = new Map();
const readings = [];
for (const name of LOGS) {
    const lines = fs.readFileSync(path.join(root, 'target', 'witness', name), 'utf8').split('\n').filter((line) => line.startsWith('reading'));
    lines.forEach((line, index) => {
        const time = line.split(' ')[1];
        const match = line.match(/HEAVY_JOBS busy n=\d+;(.*)$/);
        readings.push({ name, time, busy: Boolean(match) });
        if (!match) return;
        for (const item of match[1].split(';')) {
            const pm = item.match(/^(\S+) pid=(\d+)/);
            if (!pm) continue;
            const key = name + ' pid=' + pm[2];
            if (!runs.has(key)) runs.set(key, { file: name, pid: pm[2], kind: new Set(), indices: [] });
            runs.get(key).kind.add(pm[1]);
            runs.get(key).indices.push(readings.length - 1);
        }
    });
}

const byLength = new Map();
const rows = [];
for (const run of runs.values()) {
    const sorted = run.indices.slice().sort((p, q) => p - q);
    let start = sorted[0];
    let length = 1;
    const segments = [];
    for (let i = 1; i <= sorted.length; i += 1) {
        if (i < sorted.length && sorted[i] === sorted[i - 1] + 1 && readings[sorted[i]].name === readings[start].name) {
            length += 1;
        } else {
            segments.push({ start, length });
            if (i < sorted.length) {
                start = sorted[i];
                length = 1;
            }
        }
    }
    for (const seg of segments) {
        const kind = [...run.kind].join('+');
        const label = seg.length === 1 ? 'transient (one reading)' : 'sustained (' + seg.length + ' readings)';
        const key = kind + ' | ' + label;
        byLength.set(key, (byLength.get(key) || 0) + 1);
        rows.push({ file: run.file, pid: run.pid, kind, time: readings[seg.start].time, span: seg.length * SECONDS_PER_READING });
    }
}

console.log('readings ' + readings.length + ' (busy ' + readings.filter((r) => r.busy).length + ')');
for (const [key, count] of [...byLength].sort()) console.log('   ' + key + ': ' + count + ' runs');
console.log('');
for (const row of rows.filter((r) => r.span >= 60).sort((p, q) => q.span - p.span)) {
    console.log('   sustained ' + row.kind + ' pid=' + row.pid + ' from ' + row.time + ' (' + row.file + ') for about ' + row.span + ' s');
}
process.exit(0);
