const fs = require('node:fs');
const path = require('node:path');
const { glbParts, ktx2Images } = require('./texture-cache.cjs');
const probe = require('./astc-probe.cjs');

const root = path.join(__dirname, '..', '..');
const TEXTURE_PREFIX = 'value.17c7bd864f7f18268f';
const WIDTH = 1024;
const BLOCKS_PER_ROW = 256;
const TEXELS = 16;

function uniformTexel(pixels, b) {
    const x0 = (b % BLOCKS_PER_ROW) * 4;
    const y0 = Math.floor(b / BLOCKS_PER_ROW) * 4;
    let first = null;
    for (let r = 0; r < TEXELS; r += 1) {
        const p = ((y0 + (r >> 2)) * WIDTH + x0 + (r & 3)) * 4;
        const texel = [pixels[p], pixels[p + 1], pixels[p + 2]];
        if (first === null) first = texel;
        else if (texel.some((v, i) => v !== first[i])) return null;
    }
    return first;
}

function endpointRows(b, end, cpu, gpu) {
    return [0, 1, 2].map((channel) => ({ block: b, end, channel, cpu: cpu[channel], gpu: gpu[channel], delta: gpu[channel] - cpu[channel] }));
}

function tabulateDeltas(rows) {
    const counts = new Map();
    for (const r of rows) {
        const key = 'end ' + r.end + ' channel ' + r.channel + ' delta ' + r.delta;
        counts.set(key, (counts.get(key) || 0) + 1);
    }
    return counts;
}


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

module.exports = { WIDTH, BLOCKS_PER_ROW, TEXELS, uniformTexel, endpointRows, tabulateDeltas, cacheBlocks };
