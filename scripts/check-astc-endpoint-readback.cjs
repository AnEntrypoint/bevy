const fs = require('node:fs');
const path = require('node:path');
const { decodeAstc4x4Block } = require('./lib/astc-decode.cjs');
const probe = require('./lib/astc-probe.cjs');
const { runProbe } = require('./lib/astc-probe-gpu.cjs');
const { codeMap, classifyGpuValue } = require('./lib/astc-endpoint-code.cjs');
const { WIDTH, BLOCKS_PER_ROW, uniformTexel, endpointRows, tabulateDeltas, cacheBlocks } = require('./lib/astc-endpoint-readback.cjs');

const root = path.join(__dirname, '..');
const RANGE = 19;
const CODES = 192;
const WANT = 15360;
const OUT = path.join(root, 'target', 'witness', 'astc-endpoint-readback.json');

async function main() {
    const started = Date.now();
    const found = cacheBlocks(WANT);
    const used = BLOCKS_PER_ROW * Math.floor(found.length / BLOCKS_PER_ROW);
    const blocks = found.slice(0, used);
    const { blob, fixtures, levels } = probe.buildFixtures(blocks, probe.WEIGHT_MODE);
    const run = await runProbe(blob, 'endpoint-readback');
    console.log('gpu run pid ' + run.pid + ' outcome ' + run.outcome + ' wall ' + run.wall.toFixed(1) + ' s leftover ' + run.leftover);
    const height = used / BLOCKS_PER_ROW * 4;
    if (!run.pixels || run.pixels.length !== WIDTH * height * 4) {
        console.log('no GPU pixels: ' + run.outcome);
        process.exit(1);
    }
    const byValue = codeMap(RANGE, CODES);
    const rows = [];
    let compared = 0;
    let nonUniform = 0;
    let noCode = 0;
    for (let b = 0; b < fixtures.length; b += 1) {
        const k = b % levels;
        if (k !== 0 && k !== levels - 1) continue;
        compared += 1;
        const gpu = uniformTexel(run.pixels, b);
        if (gpu === null) {
            nonUniform += 1;
            continue;
        }
        const cpu = decodeAstc4x4Block(probe.withWeightIndex(blocks[b], k, probe.WEIGHT_MODE)).subarray(0, 3);
        for (const row of endpointRows(b, k === 0 ? 0 : 1, cpu, gpu)) {
            const code = byValue.get(row.cpu);
            if (code === undefined) {
                noCode += 1;
                continue;
            }
            rows.push({ ...row, code, gpuClass: row.delta === 0 ? 'match' : classifyGpuValue(row.cpu, row.gpu, byValue) });
        }
    }
    fs.mkdirSync(path.dirname(OUT), { recursive: true });
    fs.writeFileSync(OUT, JSON.stringify(rows));
    console.log('endpoints compared ' + compared + ', non-uniform GPU blocks ' + nonUniform + ', channel rows with cpu not a code ' + noCode + ', channel rows saved ' + rows.length);
    for (const [key, count] of [...tabulateDeltas(rows)].sort()) console.log('   ' + key + ': ' + count);
    const classes = new Map();
    for (const r of rows) {
        const key = r.gpuClass + ' | cpu code digit ' + Math.floor(r.code / 64);
        classes.set(key, (classes.get(key) || 0) + 1);
    }
    for (const [key, count] of [...classes].sort()) console.log('   ' + key + ': ' + count);
    console.log('wall ' + ((Date.now() - started) / 1000).toFixed(1) + ' s');
    process.exit(0);
}

main();
