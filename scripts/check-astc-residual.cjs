const fs = require('node:fs');
const path = require('node:path');
const { glbParts, ktx2Images } = require('./lib/texture-cache.cjs');
const { decodeAstc4x4Block } = require('./lib/astc-decode.cjs');
const probe = require('./lib/astc-probe.cjs');
const { runProbe } = require('./lib/astc-probe-gpu.cjs');
const { FORMULAS } = require('./lib/astc-weight-fit.cjs');
const { classify } = require('./lib/astc-residual.cjs');

const root = path.join(__dirname, '..');
const TEXTURE_PREFIX = 'value.17c7bd864f7f18268f';
const WIDTH = 1024;
const BLOCKS_PER_ROW = 256;
const WANT = 15360;

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

function texelsOf(pixels, b) {
    const x0 = (b % BLOCKS_PER_ROW) * 4;
    const y0 = Math.floor(b / BLOCKS_PER_ROW) * 4;
    const texels = [];
    for (let r = 0; r < 16; r += 1) {
        const p = ((y0 + (r >> 2)) * WIDTH + x0 + (r & 3)) * 4;
        texels.push([pixels[p], pixels[p + 1], pixels[p + 2]]);
    }
    return texels;
}

async function main() {
    const started = Date.now();
    const found = cacheBlocks(WANT);
    const used = BLOCKS_PER_ROW * Math.floor(found.length / BLOCKS_PER_ROW);
    const blocks = found.slice(0, used);
    const { blob, fixtures, levels } = probe.buildFixtures(blocks, probe.WEIGHT_MODE);
    const run = await runProbe(blob, 'residual');
    console.log('gpu run pid ' + run.pid + ' outcome ' + run.outcome + ' wall ' + run.wall.toFixed(1) + ' s leftover ' + run.leftover);
    const height = used / BLOCKS_PER_ROW * 4;
    if (!run.pixels || run.pixels.length !== WIDTH * height * 4) {
        console.log('no GPU pixels: ' + run.outcome);
        process.exit(1);
    }
    const patterns = new Map();
    let interior = 0;
    let jointNone = 0;
    for (let b = 0; b < fixtures.length; b += 1) {
        const k = b % levels;
        if (k === 0 || k === levels - 1) continue;
        interior += 1;
        const e0 = decodeAstc4x4Block(probe.withWeightIndex(blocks[b], 0, probe.WEIGHT_MODE)).subarray(0, 3);
        const e1 = decodeAstc4x4Block(probe.withWeightIndex(blocks[b], levels - 1, probe.WEIGHT_MODE)).subarray(0, 3);
        const result = classify(e0, e1, texelsOf(run.pixels, b), FORMULAS.spec);
        if (result.joint.length > 0) continue;
        jointNone += 1;
        const key = result.missing.length === 0 ? 'every channel fits alone' : 'channel without any fit: ' + result.missing.map((c) => 'RGB'[c]).join('');
        patterns.set(key, (patterns.get(key) || 0) + 1);
    }
    console.log('interior blocks ' + interior + ', no weight fits all channels ' + jointNone + ' (' + (100 * jointNone / interior).toFixed(2) + '%)');
    for (const [key, count] of [...patterns.entries()].sort((p, q) => q[1] - p[1])) console.log('   ' + key + ': ' + count);
    console.log('wall ' + ((Date.now() - started) / 1000).toFixed(1) + ' s');
    process.exit(0);
}

main();
