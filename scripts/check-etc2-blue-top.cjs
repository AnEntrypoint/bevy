const fs = require('node:fs');
const path = require('node:path');
const { glbParts, ktx2Images } = require('./lib/texture-cache.cjs');
const { decodeEtc2Rgba8Block } = require('./lib/texel-decode.cjs');

const root = path.join(__dirname, '..');
const TEXTURE_PREFIX = 'value.17c7bd864f7f18268f';
const W = 1024;
const BLOCKS_PER_ROW = 256;
const BLOCKS = 65536;
const TEXELS = 1048576;
const BLUE = 2;
const TOP_END = 240;
const TOP_BLOCK = 250;
const EXPECT_BLUE_PSNR = 28.49;

function level0(dir, name) {
    const { json, bin } = glbParts(fs.readFileSync(path.join(root, 'target', dir, name)));
    const image = ktx2Images(json, bin)[0];
    const offset = Number(image.readBigUInt64LE(80));
    const length = Number(image.readBigUInt64LE(88));
    return image.subarray(offset, offset + length);
}

function signed3(value) {
    return ((value & 7) ^ 4) - 4;
}

function etcMode(colour) {
    if (!(colour[3] & 2)) return 'individual';
    for (let channel = 0; channel < 3; channel += 1) {
        const value = (colour[channel] >> 3) + signed3(colour[channel]);
        if (value < 0 || value > 31) return 'special';
    }
    return 'differential';
}

function shiftBlue(bytes) {
    const out = Buffer.from(bytes);
    const colour = out.subarray(8, 16);
    const base = colour[2] >> 3;
    const delta = signed3(colour[2]);
    if (base >= 31 || delta <= -4) return null;
    colour[2] = ((base + 1) << 3) | ((delta - 1) & 7);
    return out;
}

function psnr(sumSquares, samples) {
    return +(10 * Math.log10(65025 / (sumSquares / samples))).toFixed(2);
}

function top(map, count) {
    return [...map.entries()].sort((a, b) => b[1] - a[1]).slice(0, count).map(([k, v]) => k + ' x' + v).join(', ');
}

function main() {
    const name = fs.readdirSync(path.join(root, 'target', 'gltf-cache-etc2')).find((file) => file.startsWith(TEXTURE_PREFIX));
    const etc2 = level0('gltf-cache-etc2', name);
    const rgba8 = level0('gltf-cache-rgba8', name);
    const modes = {};
    const pairs = new Map();
    const baseHist = new Map();
    const sq = [0, 0, 0, 0];
    let sqShift = 0;
    let shifted = 0;
    const signed = [0, 0, 0];
    let topTexels = 0;
    let topBlocks = 0;
    let blockTotal = 0;
    let texelTotal = 0;
    let topDiffer = 0;
    for (let block = 0; block < BLOCKS; block += 1) {
        const bytes = etc2.subarray(block * 16, block * 16 + 16);
        const colour = bytes.subarray(8, 16);
        const mode = etcMode(colour);
        const decoded = decodeEtc2Rgba8Block(bytes);
        const stat = modes[mode] || (modes[mode] = { blocks: 0, texels: 0, differ: 0, maxAbs: 0, top: 0, topDiffer: 0 });
        stat.blocks += 1;
        blockTotal += 1;
        const bx = block % BLOCKS_PER_ROW;
        const by = Math.floor(block / BLOCKS_PER_ROW);
        let blueSum = 0;
        for (let y = 0; y < 4; y += 1) {
            for (let x = 0; x < 4; x += 1) {
                blueSum += rgba8[((by * 4 + y) * W + bx * 4 + x) * 4 + BLUE];
            }
        }
        const isTopBlock = blueSum / 16 >= TOP_BLOCK;
        if (isTopBlock && mode === 'differential') {
            const key = 'blue5=' + (colour[2] >> 3);
            baseHist.set(key, (baseHist.get(key) || 0) + 1);
            topBlocks += 1;
        }
        const moved = isTopBlock && mode === 'differential' ? shiftBlue(bytes) : null;
        const shiftedDecode = moved ? decodeEtc2Rgba8Block(moved) : decoded;
        if (moved) shifted += 1;
        for (let y = 0; y < 4; y += 1) {
            for (let x = 0; x < 4; x += 1) {
                const pixel = ((by * 4 + y) * W + bx * 4 + x) * 4;
                const texel = (y * 4 + x) * 4;
                for (let c = 0; c < 4; c += 1) {
                    const d = rgba8[pixel + c] - decoded[texel + c];
                    sq[c] += d * d;
                }
                const dShift = rgba8[pixel + BLUE] - shiftedDecode[texel + BLUE];
                sqShift += dShift * dShift;
                const error = rgba8[pixel + BLUE] - decoded[texel + BLUE];
                stat.texels += 1;
                texelTotal += 1;
                if (rgba8[pixel + BLUE] >= TOP_END) {
                    stat.top += 1;
                    topTexels += 1;
                    for (let c = 0; c < 3; c += 1) {
                        signed[c] += decoded[texel + c] - rgba8[pixel + c];
                    }
                }
                if (error !== 0) {
                    stat.differ += 1;
                    stat.maxAbs = Math.max(stat.maxAbs, Math.abs(error));
                    if (rgba8[pixel + BLUE] >= TOP_END) {
                        stat.topDiffer += 1;
                        topDiffer += 1;
                    }
                    const key = rgba8[pixel + BLUE] + '->' + decoded[texel + BLUE];
                    pairs.set(key, (pairs.get(key) || 0) + 1);
                }
            }
        }
    }
    for (const mode of Object.keys(modes)) {
        const s = modes[mode];
        console.log('MODE ' + mode + ' blocks=' + s.blocks + ' texels=' + s.texels + ' blueDiffer=' + s.differ + ' maxAbs=' + s.maxAbs + ' topEnd=' + s.top + ' topEndDiffer=' + s.topDiffer);
    }
    console.log('BLUE cache->etc2 pairs: ' + top(pairs, 12));
    console.log('TOP-BLOCK differential blue5 base (cache block mean >= ' + TOP_BLOCK + '): ' + top(baseHist, 8) + ' (' + topBlocks + ' blocks)');
    console.log('TOP-END bias, mean (etc2 - cache) over ' + topTexels + ' texels with cache blue >= ' + TOP_END + ': R ' + (signed[0] / topTexels).toFixed(2) + ' G ' + (signed[1] / topTexels).toFixed(2) + ' B ' + (signed[2] / topTexels).toFixed(2));
    const psnrs = [0, 1, 2].map((c) => psnr(sq[c], TEXELS));
    console.log('COUNTERFACTUAL blue base +1 and delta -1 on ' + shifted + ' top-end differential blocks: blue PSNR ' + psnr(sqShift, TEXELS) + ' dB (was ' + psnrs[BLUE] + ')');
    console.log('PSNR R/G/B ' + psnrs.join(' / ') + ' dB (blue expected ' + EXPECT_BLUE_PSNR + ')');
    const sums = Object.values(modes).reduce((acc, s) => acc + s.texels, 0);
    const ok = blockTotal === BLOCKS && texelTotal === TEXELS && sums === TEXELS && psnrs[BLUE] === EXPECT_BLUE_PSNR;
    console.log('TOP-END blue >= ' + TOP_END + ': ' + topTexels + ' texels, ' + topDiffer + ' differ from the ETC2 decode');
    console.log((ok ? 'ok   ' : 'FAIL ') + 'blocks ' + blockTotal + ', texels ' + texelTotal + ', mode totals ' + sums + ', blue PSNR ' + psnrs[BLUE]);
    process.exit(ok ? 0 : 1);
}

main();
