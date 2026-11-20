const fs = require('node:fs');
const path = require('node:path');
const { glbParts, ktx2Images } = require('./lib/texture-cache.cjs');
const { decodeEtc2Rgba8Block } = require('./lib/texel-decode.cjs');
const { halfMask, fitMask, freeCandidates } = require('./lib/block-geometry.cjs');

const root = path.join(__dirname, '..');
const TEXTURE_PREFIX = 'value.17c7bd864f7f18268f';
const W = 1024;
const BLOCKS_PER_ROW = 256;
const BLOCKS = 65536;
const TEXELS = 1048576;
const BLUE = 2;
const TOP_BLOCK = 250;
const BAR_DB = 35;
const KEYS = ['cache', 'half', 'free', 'bestOfBoth'];
const LABELS = {
    cache: 'cache as it is (library decode)',
    half: 'ETC half split, same palette search (encodable shape)',
    free: 'free split, top 3 candidates, same palette search (not ETC)',
    bestOfBoth: 'best of half or free per top-end block (not ETC)',
};

function level0(dir, name) {
    const { json, bin } = glbParts(fs.readFileSync(path.join(root, 'target', dir, name)));
    const image = ktx2Images(json, bin)[0];
    const offset = Number(image.readBigUInt64LE(80));
    return image.subarray(offset, offset + Number(image.readBigUInt64LE(88)));
}

function psnr(s) {
    return +(10 * Math.log10(65025 / (s / TEXELS))).toFixed(2);
}

function main() {
    const started = Date.now();
    const name = fs.readdirSync(path.join(root, 'target', 'gltf-cache-etc2')).find((file) => file.startsWith(TEXTURE_PREFIX));
    const etc2 = level0('gltf-cache-etc2', name);
    const rgba8 = level0('gltf-cache-rgba8', name);
    const sums = {};
    for (const key of KEYS) sums[key] = [0, 0, 0];
    let top = 0;
    let freeWins = 0;
    let halfWins = 0;
    for (let block = 0; block < BLOCKS; block += 1) {
        const bytes = etc2.subarray(block * 16, block * 16 + 16);
        const bx = block % BLOCKS_PER_ROW;
        const by = Math.floor(block / BLOCKS_PER_ROW);
        const target = [];
        for (let r = 0; r < 16; r += 1) {
            const pixel = ((by * 4 + (r >> 2)) * W + bx * 4 + (r & 3)) * 4;
            target.push([rgba8[pixel], rgba8[pixel + 1], rgba8[pixel + 2]]);
        }
        const lib = decodeEtc2Rgba8Block(bytes);
        const eCache = [0, 1, 2].map((c) => target.reduce((acc, t, r) => acc + (t[c] - lib[r * 4 + c]) ** 2, 0));
        for (let c = 0; c < 3; c += 1) sums.cache[c] += eCache[c];
        const meanBlue = target.reduce((acc, v) => acc + v[BLUE], 0) / 16;
        if (meanBlue < TOP_BLOCK) {
            for (const key of KEYS.slice(1)) for (let c = 0; c < 3; c += 1) sums[key][c] += eCache[c];
            continue;
        }
        top += 1;
        const halves = [0, 1].map((flip) => fitMask(target, halfMask(flip)));
        const half = halves[0].sse <= halves[1].sse ? halves[0] : halves[1];
        const frees = freeCandidates(target, 3).map((mask) => fitMask(target, mask));
        const free = frees.reduce((p, q) => (q.sse < p.sse ? q : p));
        if (free.sse < half.sse) freeWins += 1;
        if (half.sse < free.sse) halfWins += 1;
        const best = free.sse < half.sse ? free : half;
        for (let c = 0; c < 3; c += 1) {
            sums.half[c] += half.channels[c];
            sums.free[c] += free.channels[c];
            sums.bestOfBoth[c] += best.channels[c];
        }
    }
    for (const key of KEYS) {
        const values = [0, 1, 2].map((c) => psnr(sums[key][c]));
        const min = Math.min(...values);
        console.log(LABELS[key] + ': R/G/B ' + values.join(' / ') + ' dB, min ' + min + ' dB' + (min >= BAR_DB ? ' (clears the 35 dB bar)' : '') + '; blue ' + values[BLUE] + ' dB');
    }
    console.log('top-end blocks (mean cache blue >= ' + TOP_BLOCK + '): ' + top + ' of ' + BLOCKS + '; free beats half on ' + freeWins + ', half beats free on ' + halfWins);
    console.log('wall ' + ((Date.now() - started) / 1000).toFixed(1) + ' s');
    const cacheBlue = psnr(sums.cache[BLUE]);
    const ok = Math.abs(cacheBlue - 28.49) < 1e-9;
    console.log((ok ? 'ok   ' : 'FAIL ') + 'cache arm is the library baseline (blue 28.49 dB)');
    process.exit(ok ? 0 : 1);
}

main();
