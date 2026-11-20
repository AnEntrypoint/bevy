const fs = require('node:fs');
const path = require('node:path');
const { glbParts, ktx2Images } = require('./lib/texture-cache.cjs');
const { decodeEtc2Rgba8Block } = require('./lib/texel-decode.cjs');
const { srgbDecodeByte, srgbEncodeByte } = require('./lib/colour-transfer.cjs');

const root = path.join(__dirname, '..');
const TEXTURE_PREFIX = 'value.17c7bd864f7f18268f';
const W = 1024;
const BLOCKS_PER_ROW = 256;
const BLOCKS = 65536;
const TEXELS = 1048576;
const BLUE = 2;
const BINS = [0, 32, 64, 96, 128, 160, 192, 224, 240, 248, 255];

function header(dir, name) {
    const { json, bin } = glbParts(fs.readFileSync(path.join(root, 'target', dir, name)));
    return ktx2Images(json, bin)[0];
}

function level0(dir, name) {
    const { json, bin } = glbParts(fs.readFileSync(path.join(root, 'target', dir, name)));
    const image = ktx2Images(json, bin)[0];
    const offset = Number(image.readBigUInt64LE(80));
    const length = Number(image.readBigUInt64LE(88));
    return image.subarray(offset, offset + length);
}

function psnr(sumSquares) {
    return +(10 * Math.log10(65025 / (sumSquares / TEXELS))).toFixed(2);
}

function main() {
    const started = Date.now();
    const name = fs.readdirSync(path.join(root, 'target', 'gltf-cache-etc2')).find((file) => file.startsWith(TEXTURE_PREFIX));
    console.log('vkFormat etc2 ' + header('gltf-cache-etc2', name).readUInt32LE(12) + ' rgba8 ' + header('gltf-cache-rgba8', name).readUInt32LE(12));
    const etc2 = level0('gltf-cache-etc2', name);
    const rgba8 = level0('gltf-cache-rgba8', name);
    const arms = { identity: (v) => v, decodeEtc2AsSrgb: srgbDecodeByte, encodeEtc2AsSrgb: srgbEncodeByte };
    const sums = {};
    for (const key of Object.keys(arms)) sums[key] = [0, 0, 0];
    const binSum = new Array(256).fill(0);
    const binCount = new Array(256).fill(0);
    for (let block = 0; block < BLOCKS; block += 1) {
        const lib = decodeEtc2Rgba8Block(etc2.subarray(block * 16, block * 16 + 16));
        const bx = block % BLOCKS_PER_ROW;
        const by = Math.floor(block / BLOCKS_PER_ROW);
        for (let r = 0; r < 16; r += 1) {
            const pixel = ((by * 4 + (r >> 2)) * W + bx * 4 + (r & 3)) * 4;
            for (let c = 0; c < 3; c += 1) {
                const value = lib[r * 4 + c];
                const want = rgba8[pixel + c];
                for (const [key, f] of Object.entries(arms)) {
                    const d = want - f(value);
                    sums[key][c] += d * d;
                }
                if (c === BLUE) {
                    binSum[value] += want - value;
                    binCount[value] += 1;
                }
            }
        }
    }
    for (const key of Object.keys(arms)) {
        const values = sums[key].map(psnr);
        console.log(key + ': R/G/B ' + values.join(' / ') + ' dB, min ' + Math.min(...values) + ' dB; blue ' + values[BLUE] + ' dB');
    }
    console.log('mean rgba8 minus etc2 on blue, by etc2 value:');
    for (const v of BINS) console.log('  etc2 ' + v + ': mean ' + (binCount[v] ? (binSum[v] / binCount[v]).toFixed(2) : 'none') + ' over ' + binCount[v] + ' texels');
    const topTexels = binCount.slice(240).reduce((a, b) => a + b, 0);
    const topMean = binSum.slice(240).reduce((a, b) => a + b, 0) / topTexels;
    console.log('blue texels with etc2 value at least 240: ' + topTexels + ', mean rgba8 minus etc2 ' + topMean.toFixed(2));
    console.log('wall ' + ((Date.now() - started) / 1000).toFixed(1) + ' s');
    const base = [0, 1, 2].map((c) => psnr(sums.identity[c]));
    const ok = Math.abs(base[BLUE] - 28.49) < 1e-9 && Math.abs(Math.min(...base) - 28.49) < 1e-9;
    console.log((ok ? 'ok   ' : 'FAIL ') + 'identity arm is the library baseline (blue 28.49 dB, min 28.49 dB)');
    process.exit(ok ? 0 : 1);
}

main();
