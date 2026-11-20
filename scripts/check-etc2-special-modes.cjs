const fs = require('node:fs');
const path = require('node:path');
const { glbParts, ktx2Images } = require('./lib/texture-cache.cjs');
const { decodeEtc2Rgba8Block } = require('./lib/texel-decode.cjs');
const { fitT, fitH, fitPlanar } = require('./lib/etc2-special.cjs');

const root = path.join(__dirname, '..');
const TEXTURE_PREFIX = 'value.17c7bd864f7f18268f';
const W = 1024;
const BLOCKS_PER_ROW = 256;
const BLOCKS = 65536;
const TEXELS = 1048576;
const BLUE = 2;
const TOP_BLOCK = 250;
const BAR_DB = 35;
const EXPECT_BLUE_PSNR = 28.49;
const EXPECT_MIN_PSNR = 28.49;
const KEYS = ['base', 'T', 'H', 'P', 'unionT', 'unionH', 'unionP', 'unionAll'];
const LABELS = {
    base: 'cache as it is (library decode)',
    T: 'T mode on top-end blocks',
    H: 'H mode on top-end blocks',
    P: 'planar on top-end blocks',
    unionT: 'cache or T, best per top-end block',
    unionH: 'cache or H, best per top-end block',
    unionP: 'cache or planar, best per top-end block',
    unionAll: 'cache, T, H or planar, best per top-end block',
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
    for (let r = 0; r < 16; r += 1) {
        raster.push([out[r * 4], out[r * 4 + 1], out[r * 4 + 2]]);
    }
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

function report(sums) {
    const rows = [];
    for (const key of KEYS) {
        const values = [0, 1, 2].map((c) => psnr(sums[key][c]));
        const min = Math.min(...values);
        rows.push(LABELS[key] + ': R/G/B ' + values.join(' / ') + ' dB, min ' + min + ' dB' + (min >= BAR_DB ? ' (clears the 35 dB bar)' : '') + '; blue ' + values[BLUE] + ' dB');
    }
    return rows;
}

function main() {
    const started = Date.now();
    const name = fs.readdirSync(path.join(root, 'target', 'gltf-cache-etc2')).find((file) => file.startsWith(TEXTURE_PREFIX));
    const etc2 = level0('gltf-cache-etc2', name);
    const rgba8 = level0('gltf-cache-rgba8', name);
    const sums = {};
    for (const key of KEYS) sums[key] = [0, 0, 0];
    const wins = { T: 0, H: 0, P: 0 };
    const chosen = { lib: 0, T: 0, H: 0, P: 0 };
    let topBlocks = 0;
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
        addInto(sums.base, channelSse(lib, target));
        const meanBlue = target.reduce((acc, v) => acc + v[BLUE], 0) / 16;
        if (meanBlue < TOP_BLOCK) {
            const errors = channelSse(lib, target);
            for (const key of KEYS.slice(1)) addInto(sums[key], errors);
            continue;
        }
        topBlocks += 1;
        const fT = fitT(target).pred;
        const fH = fitH(target).pred;
        const fP = fitPlanar(target).pred;
        const eLib = channelSse(lib, target);
        const eT = channelSse(fT, target);
        const eH = channelSse(fH, target);
        const eP = channelSse(fP, target);
        const total = (e) => e[0] + e[1] + e[2];
        addInto(sums.T, eT);
        addInto(sums.H, eH);
        addInto(sums.P, eP);
        if (total(eT) < total(eLib)) wins.T += 1;
        if (total(eH) < total(eLib)) wins.H += 1;
        if (total(eP) < total(eLib)) wins.P += 1;
        const options = [['lib', eLib], ['T', eT], ['H', eH], ['P', eP]];
        const bestOf = (keys) => options.filter(([k]) => keys.includes(k)).reduce((p, q) => (total(q[1]) < total(p[1]) ? q : p));
        addInto(sums.unionT, bestOf(['lib', 'T'])[1]);
        addInto(sums.unionH, bestOf(['lib', 'H'])[1]);
        addInto(sums.unionP, bestOf(['lib', 'P'])[1]);
        const all = bestOf(['lib', 'T', 'H', 'P']);
        addInto(sums.unionAll, all[1]);
        chosen[all[0]] += 1;
    }
    const rows = report(sums);
    for (const row of rows) console.log(row);
    console.log('top-end blocks (mean cache blue >= ' + TOP_BLOCK + '): ' + topBlocks + ' of ' + BLOCKS);
    console.log('top-end blocks where the mode beats the cache block: T ' + wins.T + ', H ' + wins.H + ', planar ' + wins.P);
    console.log('top-end blocks chosen in unionAll: cache ' + chosen.lib + ', T ' + chosen.T + ', H ' + chosen.H + ', planar ' + chosen.P);
    const baseValues = [0, 1, 2].map((c) => psnr(sums.base[c]));
    const baseOk = Math.abs(psnr(sums.base[BLUE]) - EXPECT_BLUE_PSNR) < 1e-9 && Math.abs(Math.min(...baseValues) - EXPECT_MIN_PSNR) < 1e-9;
    console.log((baseOk ? 'ok   ' : 'FAIL ') + 'baseline library decode blue ' + psnr(sums.base[BLUE]) + ' dB, min ' + Math.min(...baseValues) + ' dB (expect 28.49)');
    console.log('wall ' + ((Date.now() - started) / 1000).toFixed(1) + ' s');
    process.exit(baseOk ? 0 : 1);
}

main();
