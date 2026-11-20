#!/usr/bin/env node
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { SHARED_REFERENCE, referenceFor, keepReference } = require('./lib/pair-reference.cjs');

const PRE = 'PRE';

function readState(file) {
    return JSON.parse(fs.readFileSync(file, 'utf8')).state;
}

function pairsInTurn(order) {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), 'reference-layout-'));
    try {
        const witnessDir = path.join(root, 'witness');
        fs.mkdirSync(witnessDir, { recursive: true });
        const sharedWitness = path.join(witnessDir, `neon-wasm-witness-${SHARED_REFERENCE}.json`);
        const sharedRows = path.join(witnessDir, `neon-rows-${SHARED_REFERENCE}.json`);
        fs.writeFileSync(sharedWitness, JSON.stringify({ state: PRE }));
        fs.writeFileSync(sharedRows, JSON.stringify({ state: PRE }));
        for (const format of order) {
            const beforeDir = path.join(root, 'archive', format);
            fs.mkdirSync(beforeDir, { recursive: true });
            fs.copyFileSync(sharedWitness, path.join(beforeDir, `before-neon-wasm-witness-${SHARED_REFERENCE}.json`));
            fs.copyFileSync(sharedRows, path.join(beforeDir, `before-neon-rows-${SHARED_REFERENCE}.json`));
            fs.writeFileSync(sharedWitness, JSON.stringify({ state: format }));
            fs.writeFileSync(sharedRows, JSON.stringify({ state: format }));
            keepReference({ witnessDir, format, beforeDir });
        }
        return {
            etc2: readState(path.join(witnessDir, `neon-wasm-witness-${referenceFor('etc2')}.json`)),
            astc4x4: readState(path.join(witnessDir, `neon-wasm-witness-${referenceFor('astc4x4')}.json`)),
            etc2Rows: readState(path.join(witnessDir, `neon-rows-${referenceFor('etc2')}.json`)),
            astc4x4Rows: readState(path.join(witnessDir, `neon-rows-${referenceFor('astc4x4')}.json`)),
            shared: readState(sharedWitness),
            sharedRows: readState(sharedRows),
        };
    } finally {
        fs.rmSync(root, { recursive: true, force: true });
    }
}

const runs = [
    { label: 'etc2 then astc4x4', result: pairsInTurn(['etc2', 'astc4x4']) },
    { label: 'astc4x4 then etc2', result: pairsInTurn(['astc4x4', 'etc2']) },
];

const cases = [];
for (const { label, result } of runs) {
    cases.push({ name: `${label}: the ETC2 reference is kept under its own name`, ok: result.etc2 === 'etc2' });
    cases.push({ name: `${label}: the ASTC reference is kept under its own name`, ok: result.astc4x4 === 'astc4x4' });
    cases.push({ name: `${label}: the rows are kept per format`, ok: result.etc2Rows === 'etc2' && result.astc4x4Rows === 'astc4x4' });
    cases.push({ name: `${label}: the shared name is back at its pre-pair content`, ok: result.shared === PRE && result.sharedRows === PRE });
}
cases.push({ name: 'the ETC2 and ASTC references have different names, neither the shared name', ok: referenceFor('etc2') !== referenceFor('astc4x4') && referenceFor('etc2') !== SHARED_REFERENCE && referenceFor('astc4x4') !== SHARED_REFERENCE });

let fail = 0;
for (const testCase of cases) {
    console.log(`${testCase.ok ? 'ok  ' : 'FAIL'} ${testCase.name}`);
    if (!testCase.ok) fail = 1;
}
process.exit(fail);
