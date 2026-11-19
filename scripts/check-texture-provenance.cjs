#!/usr/bin/env node
const fs = require('node:fs');
const path = require('node:path');

const repo = path.resolve(__dirname, '..');
const witnessDir = path.join(repo, 'target', 'witness');
const buildMs = fs.statSync(path.join(repo, 'web', 'wasm', 'bevy-jayzee-wasm_bg.wasm')).mtimeMs;

function setMs(name) {
    const dir = path.join(witnessDir, name);
    if (!fs.existsSync(dir)) return null;
    const pngs = fs.readdirSync(dir).filter((file) => file.endsWith('.png'));
    if (!pngs.length) return null;
    return Math.min(...pngs.map((file) => fs.statSync(path.join(dir, file)).mtimeMs));
}

const visual = fs.readdirSync(witnessDir).filter((file) => /^texture-visual-.*\.json$/.test(file)).sort().map((file) => {
    const record = JSON.parse(fs.readFileSync(path.join(witnessDir, file), 'utf8'));
    const sourceMs = record.source ? setMs(record.source) : null;
    const referenceMs = record.reference ? setMs(record.reference) : null;
    const current = sourceMs !== null && sourceMs >= buildMs && referenceMs !== null && referenceMs >= buildMs;
    return { record: file, source: record.source ?? null, reference: record.reference ?? null, ok: record.ok, current };
});
const texel = { record: 'texture-texel.json', current: fs.statSync(path.join(witnessDir, 'texture-texel.json')).mtimeMs >= buildMs };
const rows = [...visual, texel];
const ok = rows.length > 0 && rows.every((row) => row.current);
console.log(JSON.stringify({ ok, buildTime: new Date(buildMs).toISOString(), rows }, null, 2));
process.exit(ok ? 0 : 1);
