const fs = require('node:fs');
const path = require('node:path');
const { glbParts, ktx2Images } = require('./lib/texture-cache.cjs');
const { decodeEtc2Rgba8Block } = require('./lib/texel-decode.cjs');
const { fitIndividual } = require('./lib/etc1-individual.cjs');

const root = path.join(__dirname, '..');
const TEXTURE_PREFIX = 'value.17c7bd864f7f18268f';
const W = 1024;
const BLOCKS_PER_ROW = 256;
const BLOCKS = 65536;
const TEXELS = 1048576;
const BLUE = 2;
const TOP_BLOCK = 250;
const BAR_DB = 35;
const KEYS = ['base', 'individual', 'unionIndividual'];
const LABELS = {
    base: 'cache as it is (library decode)',
    individual: 'individual mode on top-end blocks (searched)',
    unionIndividual: 'cache or individual, best per top-end block',
};

function level0(dir, name) {
    const { json, bin } = glbParts(fs.readFileSync(path.join(root, 'target', dir, name)));
    const image = ktx2Images(json, bin)[0];
    const offset = Number(image.readBigUInt64LE(80));
    const length = Number(image.readBigUInt64LE(88));
    return image.subarray(offset, offset + length);
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
        for (let c = 0; c < 3; c += 1) {
            const d = target[r][c] - pred[r][c];
            e[c] += d * d;
        }
    }
    return e;
}

function addInto(sums, errors) {
    for (let c = 0; c < 3; c += 1) sums[c] += errors[c];
}

function psnr(sumSquares) {
    return +(10 * Math.log10(65025 / (sumSquares / TEXELS))).toFixed(2);
}

function main() {
    const started = Date.now();
    const name = fs.readdirSync(path.join(root, 'target', 'gltf-cache-etc2')).find((file) => file.startsWith(TEXTURE_PREFIX));
    const etc2 = level0('gltf-cache-etc2', name);
    const rgba8 = level0('gltf-cache-rgba8', name);
    const sums = {};
    for (const key of KEYS) sums[key] = [0, 0, 0];
    let topBlocks = 0;
    let topIndividualInCache = 0;
    let winsIndividual = 0;
    let winsByCacheKind = { individual: 0, differential: 0 };
    let chosenIndividual = 0;
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
        topBlocks += 1;
        const isIndividual = ((bytes[11] >> 1) & 1) === 0;
        if (isIndividual) topIndividualInCache += 1;
        const fit = fitIndividual(target);
        const eInd = channelSse(fit.pred, target);
        addInto(sums.individual, eInd);
        const total = (e) => e[0] + e[1] + e[2];
        if (total(eInd) < total(eLib)) {
            winsIndividual += 1;
            winsByCacheKind[isIndividual ? 'individual' : 'differential'] += 1;
            chosenIndividual += 1;
            addInto(sums.unionIndividual, eInd);
        } else {
            addInto(sums.unionIndividual, eLib);
        }
    }
    for (const key of KEYS) {
        const values = [0, 1, 2].map((c) => psnr(sums[key][c]));
        const min = Math.min(...values);
        console.log(LABELS[key] + ': R/G/B ' + values.join(' / ') + ' dB, min ' + min + ' dB' + (min >= BAR_DB ? ' (clears the 35 dB bar)' : '') + '; blue ' + values[BLUE] + ' dB');
    }
    console.log('top-end blocks (mean cache blue >= ' + TOP_BLOCK + '): ' + topBlocks + ' of ' + BLOCKS + '; cache block already individual: ' + topIndividualInCache);
    console.log('top-end blocks where individual beats the cache block: ' + winsIndividual + ' (cache individual ' + winsByCacheKind.individual + ', cache differential ' + winsByCacheKind.differential + ')');
    console.log('wall ' + ((Date.now() - started) / 1000).toFixed(1) + ' s');
    const baseValues = [0, 1, 2].map((c) => psnr(sums.base[c]));
    const baseOk = Math.abs(psnr(sums.base[BLUE]) - 28.49) < 1e-9 && Math.abs(Math.min(...baseValues) - 28.49) < 1e-9;
    console.log((baseOk ? 'ok   ' : 'FAIL ') + 'baseline library decode blue ' + psnr(sums.base[BLUE]) + ' dB (expect 28.49)');
    process.exit(baseOk ? 0 : 1);
}

main();
