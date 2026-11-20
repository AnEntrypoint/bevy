const fs = require('node:fs');
const path = require('node:path');
const { decodeAstc4x4Block, WEIGHT_LEVELS, WEIGHT_RANGE_BY_FIELD } = require('./lib/astc-decode.cjs');
const probe = require('./lib/astc-probe.cjs');
const { runProbe } = require('./lib/astc-probe-gpu.cjs');
const { WIDTH, BLOCKS_PER_ROW, uniformTexel, cacheBlocks } = require('./lib/astc-endpoint-readback.cjs');
const { FORMULAS } = require('./lib/astc-interp.cjs');

const root = path.join(__dirname, '..');
const WANT = 15360;
const LEVELS = WEIGHT_LEVELS[WEIGHT_RANGE_BY_FIELD[probe.WEIGHT_MODE]];
const MID = LEVELS.length - 2;
const OUT = path.join(root, 'target', 'witness', 'astc-interp-readback.json');

async function main() {
    const started = Date.now();
    const blocks = cacheBlocks(WANT);
    const total = blocks.length * MID;
    const blob = Buffer.alloc(total * 16);
    blocks.forEach((block, b) => {
        for (let k = 1; k <= MID; k += 1) {
            blob.set(probe.withWeightIndex(block, k, probe.WEIGHT_MODE), (b * MID + (k - 1)) * 16);
        }
    });
    const run = await runProbe(blob, 'interp-readback');
    console.log('gpu run pid ' + run.pid + ' outcome ' + run.outcome + ' wall ' + run.wall.toFixed(1) + ' s leftover ' + run.leftover);
    const height = total / BLOCKS_PER_ROW * 4;
    if (!run.pixels || run.pixels.length !== WIDTH * height * 4) {
        console.log('no GPU pixels: ' + run.outcome);
        process.exit(1);
    }
    const names = Object.keys(FORMULAS);
    const matches = Object.fromEntries(names.map((name) => [name, 0]));
    const byWeight = {};
    let compared = 0;
    let nonUniform = 0;
    for (let b = 0; b < blocks.length; b += 1) {
        const low = decodeAstc4x4Block(probe.withWeightIndex(blocks[b], 0, probe.WEIGHT_MODE)).subarray(0, 3);
        const high = decodeAstc4x4Block(probe.withWeightIndex(blocks[b], LEVELS.length - 1, probe.WEIGHT_MODE)).subarray(0, 3);
        for (let k = 1; k <= MID; k += 1) {
            const gpu = uniformTexel(run.pixels, b * MID + (k - 1));
            if (gpu === null) {
                nonUniform += 1;
                continue;
            }
            const w = LEVELS[k];
            const row = byWeight[w] || (byWeight[w] = { channels: 0, ...Object.fromEntries(names.map((name) => [name, 0])) });
            for (let c = 0; c < 3; c += 1) {
                compared += 1;
                row.channels += 1;
                for (const name of names) {
                    if (FORMULAS[name](low[c], high[c], w) === gpu[c]) {
                        matches[name] += 1;
                        row[name] += 1;
                    }
                }
            }
        }
    }
    fs.mkdirSync(path.dirname(OUT), { recursive: true });
    fs.writeFileSync(OUT, JSON.stringify({ compared, nonUniform, matches, byWeight }));
    console.log('channel values compared ' + compared + ', non-uniform GPU fixtures ' + nonUniform);
    for (const name of names) console.log('   formula ' + name + ': ' + matches[name] + ' of ' + compared + ' match the GPU');
    for (const w of Object.keys(byWeight).map(Number).sort((p, q) => p - q)) {
        const row = byWeight[w];
        console.log('   weight ' + w + ' (' + row.channels + ' channels): ' + names.map((name) => name + ' ' + row[name]).join(', '));
    }
    console.log('wall ' + ((Date.now() - started) / 1000).toFixed(1) + ' s');
    process.exit(0);
}

main();
