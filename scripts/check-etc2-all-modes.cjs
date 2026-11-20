const fs = require('node:fs');
const path = require('node:path');
const { glbParts, ktx2Images } = require('./lib/texture-cache.cjs');
const { decodeEtc2Rgba8Block } = require('./lib/texel-decode.cjs');
const { fitT, fitH, fitPlanar } = require('./lib/etc2-special.cjs');
const { fitIndividual } = require('./lib/etc1-individual.cjs');
const { chooseBest, totalOf } = require('./lib/block-modes.cjs');

const root = path.join(__dirname, '..');
const TEXTURE_PREFIX = 'value.17c7bd864f7f18268f';
const W = 1024;
const BLOCKS_PER_ROW = 256;
const BLOCKS = 65536;
const TEXELS = 1048576;
const BLUE = 2;
const TOP_BLOCK = 250;
const BAR_DB = 35;
const EXPECT_BLUE = 28.49;
const KEYS = ['base', 'specialUnion', 'cacheOrIndividual', 'allModes'];
const LABELS = {
    base: 'cache as it is (library decode)',
    specialUnion: 'cache, T, H or planar, best per top-end block (Round 94 arm, recomputed here)',
    cacheOrIndividual: 'cache or individual, best per top-end block (Round 95 arm)',
    allModes: 'cache, T, H, planar or individual, best per top-end block (untested union)',
};

function level0(dir, name) {
    const { json, bin } = glbParts(fs.readFileSync(path.join(root, 'target', dir, name)));
    const image = ktx2Images(json, bin)[0];
    const offset = Number(image.readBigUInt64LE(80));
    return image.subarray(offset, offset + Number(image.readBigUInt64LE(88)));
}

function libraryRaster(bytes) {
    const out = decodeEtc2Rgba8Block(bytes);
    const raster = [];
    for (let r = 0; r < 16; r += 1) raster.push([out[r * 4], out[r * 4 + 1], out[r * 4 + 2]]);
    return raster;
}

function channelSse(pred, target) {
    const e = [0, 0, 0];
    for (let r = 0; r < 16; r += 1) {
        for (let c = 0; c < 3; c += 1) e[c] += (target[r][c] - pred[r][c]) ** 2;
    }
    return e;
}

function addInto(sums, errors) {
    for (let c = 0; c < 3; c += 1) sums[c] += errors[c];
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
    const chosen = {};
    let top = 0;
    let violations = 0;
    for (let block = 0; block < BLOCKS; block += 1) {
        const bytes = etc2.subarray(block * 16, block * 16 + 16);
        const bx = block % BLOCKS_PER_ROW;
        const by = Math.floor(block / BLOCKS_PER_ROW);
        const target = [];
        for (let r = 0; r < 16; r += 1) {
            const pixel = ((by * 4 + (r >> 2)) * W + bx * 4 + (r & 3)) * 4;
            target.push([rgba8[pixel], rgba8[pixel + 1], rgba8[pixel + 2]]);
        }
        const lib = libraryRaster(bytes);
        const eLib = channelSse(lib, target);
        addInto(sums.base, eLib);
        const meanBlue = target.reduce((acc, v) => acc + v[BLUE], 0) / 16;
        if (meanBlue < TOP_BLOCK) {
            for (const key of KEYS.slice(1)) addInto(sums[key], eLib);
            continue;
        }
        top += 1;
        const arms = [
            { name: 'cache', errors: eLib },
            { name: 'T', errors: channelSse(fitT(target).pred, target) },
            { name: 'H', errors: channelSse(fitH(target).pred, target) },
            { name: 'P', errors: channelSse(fitPlanar(target).pred, target) },
            { name: 'individual', errors: channelSse(fitIndividual(target).pred, target) },
        ];
        const special = chooseBest(arms.filter((a) => a.name !== 'individual'));
        const cacheOrInd = chooseBest(arms.filter((a) => a.name === 'cache' || a.name === 'individual'));
        const all = chooseBest(arms);
        if (totalOf(all.errors) > totalOf(special.errors) + 1e-9) violations += 1;
        chosen[all.name] = (chosen[all.name] || 0) + 1;
        addInto(sums.specialUnion, special.errors);
        addInto(sums.cacheOrIndividual, cacheOrInd.errors);
        addInto(sums.allModes, all.errors);
    }
    for (const key of KEYS) {
        const values = [0, 1, 2].map((c) => psnr(sums[key][c]));
        const min = Math.min(...values);
        console.log(LABELS[key] + ': R/G/B ' + values.join(' / ') + ' dB, min ' + min + ' dB' + (min >= BAR_DB ? ' (clears the 35 dB bar)' : '') + '; blue ' + values[BLUE] + ' dB');
    }
    console.log('top-end blocks (mean cache blue >= ' + TOP_BLOCK + '): ' + top + ' of ' + BLOCKS);
    console.log('top-end blocks chosen in allModes: ' + Object.entries(chosen).map(([k, v]) => k + ' ' + v).join(', '));
    console.log('allModes above specialUnion on ' + violations + ' top-end blocks (expect 0)');
    console.log('wall ' + ((Date.now() - started) / 1000).toFixed(1) + ' s');
    const ok = Math.abs(psnr(sums.base[BLUE]) - EXPECT_BLUE) < 1e-9 && violations === 0;
    console.log((ok ? 'ok   ' : 'FAIL ') + 'cache arm is the library baseline (blue 28.49 dB) and the union never loses to its own arms');
    process.exit(ok ? 0 : 1);
}

main();
