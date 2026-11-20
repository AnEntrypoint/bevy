const { FORMULAS, fitWeights } = require(process.env.ASTC_WEIGHT_FIT_PATH || './lib/astc-weight-fit.cjs');
const { WEIGHT_LEVELS, decodeAstc4x4Block } = require('./lib/astc-decode.cjs');
const probe = require('./lib/astc-probe.cjs');
const { runProbe } = require('./lib/astc-probe-gpu.cjs');
const fs = require('node:fs');
const path = require('node:path');
const { glbParts, ktx2Images } = require('./lib/texture-cache.cjs');

const root = path.join(__dirname, '..');
const TEXTURE_PREFIX = 'value.17c7bd864f7f18268f';
const WIDTH = 1024;
const checks = [];

function check(name, ok, detail) {
    checks.push(ok);
    console.log((ok ? 'ok   ' : 'FAIL ') + name + (ok ? '' : ': ' + detail));
}

const table = WEIGHT_LEVELS[8];
check('weight table for range 8 has 16 levels from 0 to 64', table.length === 16 && table[0] === 0 && table[15] === 64, table.join(','));

let seed = 12345;
function draw() {
    seed = (seed * 1103515245 + 12345) >>> 0;
    return (seed >>> 16) % 256;
}
let recovered = true;
let misses = 0;
for (let trial = 0; trial < 200; trial += 1) {
    const e0 = [draw(), draw(), draw()];
    const e1 = [draw(), draw(), draw()];
    for (const formulaName of Object.keys(FORMULAS)) {
        const formula = FORMULAS[formulaName];
        for (const w of table) {
            const texels = [];
            for (let t = 0; t < 16; t += 1) texels.push([0, 1, 2].map((c) => formula(e0[c], e1[c], w)));
            const fits = fitWeights(e0, e1, texels, formula);
            if (!fits.includes(w)) {
                recovered = false;
                misses += 1;
            }
        }
    }
}
check('synthetic blocks: the fit includes every table weight, for each formula, in 200 trials', recovered, misses + ' misses');

function cacheBlocks(count) {
    const dir = path.join(root, 'target', 'gltf-cache-astc');
    const name = fs.readdirSync(dir).find((file) => file.startsWith(TEXTURE_PREFIX));
    const { json, bin } = glbParts(fs.readFileSync(path.join(dir, name)));
    const image = ktx2Images(json, bin)[0];
    const offset = Number(image.readBigUInt64LE(80));
    const level0 = image.subarray(offset, offset + Number(image.readBigUInt64LE(88)));
    const blocks = [];
    for (let b = 0; b < 65536 && blocks.length < count; b += 4) {
        const block = Uint8Array.from(level0.subarray(b * 16, b * 16 + 16));
        if (((block[0] | (block[1] << 8)) & 0x7ff) === probe.WEIGHT_MODE) blocks.push(block);
    }
    return blocks;
}

(async () => {
    const blocks = cacheBlocks(256);
    const { blob, fixtures, levels } = probe.buildFixtures(blocks, probe.WEIGHT_MODE);
    const run = await runProbe(blob, 'fit-witness');
    console.log('gpu run pid ' + run.pid + ' outcome ' + run.outcome + ' wall ' + run.wall.toFixed(1) + ' s leftover ' + run.leftover);
    const pixelsOk = run.outcome === 'done' && run.pixels && run.pixels.length === WIDTH * 4 * 4;
    check('GPU readback finished with ' + WIDTH + ' x 4 texels', pixelsOk, run.outcome);
    if (pixelsOk) {
        let ends = 0;
        let endsOk = 0;
        for (let b = 0; b < fixtures.length; b += 1) {
            const k = b % levels;
            if (k !== 0 && k !== levels - 1) continue;
            ends += 1;
            const e0 = decodeAstc4x4Block(probe.withWeightIndex(blocks[b], 0, probe.WEIGHT_MODE)).subarray(0, 3);
            const e1 = decodeAstc4x4Block(probe.withWeightIndex(blocks[b], levels - 1, probe.WEIGHT_MODE)).subarray(0, 3);
            const texels = [];
            const x0 = (b % 256) * 4;
            for (let r = 0; r < 16; r += 1) {
                const p = ((r >> 2) * WIDTH + x0 + (r & 3)) * 4;
                texels.push([run.pixels[p], run.pixels[p + 1], run.pixels[p + 2]]);
            }
            if (fitWeights(e0, e1, texels, FORMULAS.spec).includes(table[k])) endsOk += 1;
        }
        check('GPU end levels fit their table weight (' + endsOk + ' of ' + ends + ' blocks)', endsOk === ends, endsOk + ' of ' + ends);
    }
    const passed = checks.filter(Boolean).length;
    console.log(passed === checks.length ? 'ALL PASS ' + passed + ' checks' : 'FAIL ' + (checks.length - passed) + ' of ' + checks.length);
    process.exit(passed === checks.length ? 0 : 1);
})();
