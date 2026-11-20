const fs = require('node:fs');
const path = require('node:path');
const { glbParts, ktx2Images } = require('./lib/texture-cache.cjs');
const { decodeEtc2Rgba8Block } = require('./lib/texel-decode.cjs');
const { fitT, fitH, fitPlanar } = require('./lib/etc2-special.cjs');
const { fitIndividual } = require('./lib/etc1-individual.cjs');
const { chooseBest, totalOf } = require('./lib/block-modes.cjs');
const endpoint = require('./check-etc2-endpoint-search.cjs');

const root = path.join(__dirname, '..');
const TEXTURE_PREFIX = 'value.17c7bd864f7f18268f';
const W = 1024;
const BLOCKS_PER_ROW = 256;
const BLOCKS = 65536;
const TEXELS = 1048576;
const BLUE = 2;
const TOP_BLOCK = 250;
const BAR_DB = 35;
const EXPECT_BASE_BLUE = 28.49;
const EXPECT_E4 = [29.43, 34.99, 30.67];
const KEYS = ['base', 'e4', 'realisable', 'model'];
const LABELS = {
    base: 'cache as it is (library decode)',
    e4: 'E4 joint RGB search on differential top-end blocks, cache elsewhere (Round 88 arm, reproduced)',
    realisable: 'cache, E4 or individual, best per top-end block (each choice is an encodable block)',
    model: 'cache, E4, individual, T, H or planar, best per top-end block (T, H and planar are not decodable here)',
};

function level0(dir, name) {
    const { json, bin } = glbParts(fs.readFileSync(path.join(root, 'target', dir, name)));
    const image = ktx2Images(json, bin)[0];
    const offset = Number(image.readBigUInt64LE(80));
    return image.subarray(offset, offset + Number(image.readBigUInt64LE(88)));
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
    const chosen = { realisable: {}, model: {} };
    let top = 0;
    let topDiff = 0;
    let violations = 0;
    for (let block = 0; block < BLOCKS; block += 1) {
        const bytes = etc2.subarray(block * 16, block * 16 + 16);
        const colour = bytes.subarray(8, 16);
        const bx = block % BLOCKS_PER_ROW;
        const by = Math.floor(block / BLOCKS_PER_ROW);
        const target = [];
        for (let r = 0; r < 16; r += 1) {
            const pixel = ((by * 4 + (r >> 2)) * W + bx * 4 + (r & 3)) * 4;
            target.push([rgba8[pixel], rgba8[pixel + 1], rgba8[pixel + 2]]);
        }
        const libRgb = endpoint.toRgb(decodeEtc2Rgba8Block(bytes));
        const eLib = channelSse(libRgb, target);
        addInto(sums.base, eLib);
        const hi0 = endpoint.word(colour, 0);
        const lo0 = endpoint.word(colour, 4);
        const isDiff = ((hi0 >>> 1) & 1) === 1;
        const meanBlue = target.reduce((acc, v) => acc + v[BLUE], 0) / 16;
        if (meanBlue < TOP_BLOCK) {
            for (const key of ['e4', 'realisable', 'model']) addInto(sums[key], eLib);
            continue;
        }
        top += 1;
        let eE4 = eLib;
        if (isDiff) {
            topDiff += 1;
            const state0 = endpoint.parseDiff(colour);
            const result = endpoint.runScenario('e4', state0, target);
            const newColour = endpoint.encode(result.state, hi0, lo0, result.kByT);
            const newRgb = endpoint.toRgb(decodeEtc2Rgba8Block(Buffer.concat([bytes.subarray(0, 8), newColour])));
            eE4 = channelSse(newRgb, target);
        }
        const eInd = channelSse(fitIndividual(target).pred, target);
        const arms = [
            { name: 'cache', errors: eLib },
            { name: 'E4', errors: eE4 },
            { name: 'individual', errors: eInd },
        ];
        const realisable = chooseBest(arms);
        const model = chooseBest(arms.concat([
            { name: 'T', errors: channelSse(fitT(target).pred, target) },
            { name: 'H', errors: channelSse(fitH(target).pred, target) },
            { name: 'P', errors: channelSse(fitPlanar(target).pred, target) },
        ]));
        if (totalOf(realisable.errors) > totalOf(eE4) + 1e-9) violations += 1;
        if (totalOf(model.errors) > totalOf(realisable.errors) + 1e-9) violations += 1;
        chosen.realisable[realisable.name] = (chosen.realisable[realisable.name] || 0) + 1;
        chosen.model[model.name] = (chosen.model[model.name] || 0) + 1;
        addInto(sums.e4, eE4);
        addInto(sums.realisable, realisable.errors);
        addInto(sums.model, model.errors);
    }
    for (const key of KEYS) {
        const values = [0, 1, 2].map((c) => psnr(sums[key][c]));
        const min = Math.min(...values);
        console.log(LABELS[key] + ': R/G/B ' + values.join(' / ') + ' dB, min ' + min + ' dB' + (min >= BAR_DB ? ' (clears the 35 dB bar)' : '') + '; blue ' + values[BLUE] + ' dB');
    }
    console.log('top-end blocks (mean cache blue >= ' + TOP_BLOCK + '): ' + top + ' of ' + BLOCKS + ', of which differential ' + topDiff);
    console.log('chosen in realisable: ' + Object.entries(chosen.realisable).map(([k, v]) => k + ' ' + v).join(', '));
    console.log('chosen in model: ' + Object.entries(chosen.model).map(([k, v]) => k + ' ' + v).join(', '));
    console.log('ordering violations (expect 0): ' + violations);
    console.log('wall ' + ((Date.now() - started) / 1000).toFixed(1) + ' s');
    const e4Values = [0, 1, 2].map((c) => psnr(sums.e4[c]));
    const ok = Math.abs(psnr(sums.base[BLUE]) - EXPECT_BASE_BLUE) < 1e-9 && e4Values.every((v, c) => Math.abs(v - EXPECT_E4[c]) < 1e-9) && violations === 0;
    console.log((ok ? 'ok   ' : 'FAIL ') + 'base blue 28.49, E4 arm reproduces 29.43 / 34.99 / 30.67, and the union never loses to its own arms');
    process.exit(ok ? 0 : 1);
}

main();
