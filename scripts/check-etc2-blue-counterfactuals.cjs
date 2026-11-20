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
const TOP_BLOCK = 250;
const BAR_DB = 35;
const EXPECT_BLUE_PSNR = 28.49;
const EXPECT_MIN_PSNR = 28.49;
const MODIFIERS = [[-8, -2, 2, 8], [-17, -5, 5, 17], [-29, -9, 9, 29], [-42, -13, 13, 42], [-60, -18, 18, 60], [-80, -24, 24, 80], [-106, -33, 33, 106], [-183, -47, 47, 183]];
const INDEX_TO_MODIFIER = [2, 3, 1, 0];
const MODIFIER_TO_INDEX = [3, 2, 0, 1];
const FIELD_MASK = (31 << 27) | (31 << 19) | (31 << 11) | (7 << 24) | (7 << 16) | (7 << 8);

function level0(dir, name) {
    const { json, bin } = glbParts(fs.readFileSync(path.join(root, 'target', dir, name)));
    const image = ktx2Images(json, bin)[0];
    const offset = Number(image.readBigUInt64LE(80));
    const length = Number(image.readBigUInt64LE(88));
    return image.subarray(offset, offset + length);
}

function clamp8(value) {
    return value < 0 ? 0 : value > 255 ? 255 : value;
}

function signed3(bits) {
    return bits >= 4 ? bits - 8 : bits;
}

function expand5(value) {
    return (value << 3) | (value >> 2);
}

function word(bytes, offset) {
    return ((bytes[offset] << 24) | (bytes[offset + 1] << 16) | (bytes[offset + 2] << 8) | bytes[offset + 3]) >>> 0;
}

function putWord(bytes, offset, value) {
    bytes[offset] = (value >>> 24) & 255;
    bytes[offset + 1] = (value >>> 16) & 255;
    bytes[offset + 2] = (value >>> 8) & 255;
    bytes[offset + 3] = value & 255;
}

function parse(colour) {
    const hi = word(colour, 0);
    const diff = (hi >>> 1) & 1;
    const flip = hi & 1;
    const tables = [(hi >>> 5) & 7, (hi >>> 2) & 7];
    let base;
    if (diff) {
        const first = [(hi >>> 27) & 31, (hi >>> 19) & 31, (hi >>> 11) & 31];
        const deltas = [signed3((hi >>> 24) & 7), signed3((hi >>> 16) & 7), signed3((hi >>> 8) & 7)];
        const second = first.map((value, channel) => value + deltas[channel]);
        if (second.some((value) => value < 0 || value > 31)) throw new Error('special mode');
        base = [first.map(expand5), second.map(expand5)];
    } else {
        const first = [(hi >>> 28) & 15, (hi >>> 20) & 15, (hi >>> 12) & 15];
        const second = [(hi >>> 24) & 15, (hi >>> 16) & 15, (hi >>> 8) & 15];
        base = [first.map((value) => value * 17), second.map((value) => value * 17)];
    }
    return { hi, diff, flip, tables, base, lo: word(colour, 4) };
}

function rasterToTexel(r) {
    return (r & 3) * 4 + (r >> 2);
}

function halfOf(p, r) {
    const row = r >> 2;
    const col = r & 3;
    return p.flip ? row >> 1 : col >> 1;
}

function baselineK(p, t) {
    return INDEX_TO_MODIFIER[(((p.lo >>> (t + 16)) & 1) << 1) | ((p.lo >>> t) & 1)];
}

function baselineSelectors(p) {
    const kByT = new Array(16);
    for (let t = 0; t < 16; t += 1) kByT[t] = baselineK(p, t);
    return kByT;
}

function reconstruct(p, kByT) {
    const out = [];
    for (let r = 0; r < 16; r += 1) {
        const h = halfOf(p, r);
        const k = kByT[rasterToTexel(r)];
        out.push([0, 1, 2].map((c) => clamp8(p.base[h][c] + MODIFIERS[p.tables[h]][k])));
    }
    return out;
}

function bestSelectors(p, cache, channels) {
    const kByT = new Array(16).fill(0);
    for (let r = 0; r < 16; r += 1) {
        const h = halfOf(p, r);
        let best = 0;
        let bestError = Infinity;
        for (let k = 0; k < 4; k += 1) {
            let error = 0;
            for (const c of channels) {
                const d = cache[r][c] - clamp8(p.base[h][c] + MODIFIERS[p.tables[h]][k]);
                error += d * d;
            }
            if (error < bestError) {
                bestError = error;
                best = k;
            }
        }
        kByT[rasterToTexel(r)] = best;
    }
    return kByT;
}

function idealEndpoints(colour, p, cache) {
    if (!p.diff) return null;
    const ideal = [0, 1, 2].map((c) => {
        const sums = [0, 0];
        for (let r = 0; r < 16; r += 1) sums[halfOf(p, r)] += cache[r][c];
        return sums.map((sum) => Math.floor((sum * 31 + 1020) / (8 * 255)));
    });
    const shifts = [[27, 24], [19, 16], [11, 8]];
    let hi = p.hi & ~FIELD_MASK;
    for (let c = 0; c < 3; c += 1) {
        const delta = ideal[c][1] - ideal[c][0];
        if (delta < -4 || delta > 3) return null;
        hi |= (ideal[c][0] << shifts[c][0]) | ((delta & 7) << shifts[c][1]);
    }
    const out = Buffer.from(colour);
    putWord(out, 0, hi >>> 0);
    return out;
}

function psnr(sumSquares, samples) {
    return +(10 * Math.log10(65025 / (sumSquares / samples))).toFixed(2);
}

function shiftBlue(colour) {
    const base = colour[2] >> 3;
    const delta = signed3(colour[2] & 7);
    if (base >= 31 || delta <= -4) return null;
    const out = Buffer.from(colour);
    out[2] = ((base + 1) << 3) | ((delta - 1) & 7);
    return out;
}

function toRgb(decoded) {
    const out = [];
    for (let r = 0; r < 16; r += 1) out.push([decoded[r * 4], decoded[r * 4 + 1], decoded[r * 4 + 2]]);
    return out;
}

function accumulate(sums, reconstructed, cache) {
    for (let r = 0; r < 16; r += 1) {
        for (let c = 0; c < 3; c += 1) {
            const d = cache[r][c] - reconstructed[r][c];
            sums[c] += d * d;
        }
    }
}

function main() {
    const name = fs.readdirSync(path.join(root, 'target', 'gltf-cache-etc2')).find((file) => file.startsWith(TEXTURE_PREFIX));
    const etc2 = level0('gltf-cache-etc2', name);
    const rgba8 = level0('gltf-cache-rgba8', name);
    const rows = {
        baseline: [0, 0, 0],
        blueSelectors: [0, 0, 0],
        rgbSelectors: [0, 0, 0],
        idealEndpoints: [0, 0, 0],
        idealAndRgbSelectors: [0, 0, 0],
        flatExact: [0, 0, 0],
    };
    const bias = [0, 0, 0];
    const counts = { mismatch: 0, diffBlocks: 0, representable: 0, notRepresentable: 0, flat: 0, flatTop: 0, flatTopIndividual: 0, flatExact: 0, flatOtherValue: 0, flatOtherValueDiff: 0, flatNonFlat: 0, topDiff: 0, shifted: 0 };
    let signedFlat = 0;
    for (let block = 0; block < BLOCKS; block += 1) {
        const bytes = etc2.subarray(block * 16, block * 16 + 16);
        const colour = bytes.subarray(8, 16);
        const bx = block % BLOCKS_PER_ROW;
        const by = Math.floor(block / BLOCKS_PER_ROW);
        const cache = [];
        for (let r = 0; r < 16; r += 1) {
            const pixel = ((by * 4 + (r >> 2)) * W + bx * 4 + (r & 3)) * 4;
            cache.push([rgba8[pixel], rgba8[pixel + 1], rgba8[pixel + 2]]);
        }
        const p = parse(colour);
        const library = decodeEtc2Rgba8Block(bytes);
        const libraryRgb = [];
        for (let r = 0; r < 16; r += 1) libraryRgb.push([library[r * 4], library[r * 4 + 1], library[r * 4 + 2]]);
        const baseRecon = reconstruct(p, baselineSelectors(p));
        for (let r = 0; r < 16; r += 1) {
            for (let c = 0; c < 3; c += 1) {
                if (baseRecon[r][c] !== libraryRgb[r][c]) counts.mismatch += 1;
            }
        }
        accumulate(rows.baseline, libraryRgb, cache);
        const flat = cache.every((v) => v[BLUE] === cache[0][BLUE]);
        const topBlock = cache.reduce((acc, v) => acc + v[BLUE], 0) / 16 >= TOP_BLOCK;
        if (p.diff) counts.diffBlocks += 1;
        accumulate(rows.blueSelectors, reconstruct(p, bestSelectors(p, cache, [BLUE])), cache);
        accumulate(rows.rgbSelectors, reconstruct(p, bestSelectors(p, cache, [0, 1, 2])), cache);
        const endColour = idealEndpoints(colour, p, cache);
        const endP = endColour ? parse(endColour) : p;
        if (endColour) counts.representable += 1;
        else if (p.diff) counts.notRepresentable += 1;
        accumulate(rows.idealEndpoints, reconstruct(endP, baselineSelectors(endP)), cache);
        accumulate(rows.idealAndRgbSelectors, reconstruct(endP, bestSelectors(endP, cache, [0, 1, 2])), cache);
        if (flat) {
            counts.flat += 1;
            if (topBlock) {
                counts.flatTop += 1;
                if (!p.diff) counts.flatTopIndividual += 1;
            }
            const decodedFlat = libraryRgb.every((v) => v[BLUE] === libraryRgb[0][BLUE]);
            if (libraryRgb.every((v) => v[BLUE] === cache[0][BLUE])) counts.flatExact += 1;
            else if (decodedFlat) {
                counts.flatOtherValue += 1;
                signedFlat += libraryRgb[0][BLUE] - cache[0][BLUE];
                if (p.diff) counts.flatOtherValueDiff += 1;
            } else counts.flatNonFlat += 1;
        } else {
            accumulate(rows.flatExact, libraryRgb, cache);
        }
        const topDiffer = topBlock && p.diff;
        if (topDiffer) counts.topDiff += 1;
        const shiftedColour = topDiffer ? shiftBlue(colour) : null;
        if (shiftedColour) counts.shifted += 1;
        accumulate(bias, shiftedColour ? toRgb(decodeEtc2Rgba8Block(Buffer.concat([bytes.subarray(0, 8), shiftedColour]))) : libraryRgb, cache);
    }
    const minOf = (sums) => Math.min(...[0, 1, 2].map((c) => psnr(sums[c], TEXELS)));
    const report = (label, sums) => {
        const values = [0, 1, 2].map((c) => psnr(sums[c], TEXELS));
        const best = Math.min(...values);
        console.log('CF ' + label + ': R/G/B ' + values.join(' / ') + ' dB, min ' + best + ' dB' + (best >= BAR_DB ? ' (clears the 35 dB bar)' : ''));
        return best;
    };
    report('baseline (transcoder output)', rows.baseline);
    report('bias step undone on top-end differential blocks (blue)', bias);
    report('blue-only selectors (luma-free, blue error only)', rows.blueSelectors);
    report('RGB-unweighted selectors, transcoder endpoints', rows.rgbSelectors);
    report('ideal 5-bit endpoints, all channels, transcoder selectors', rows.idealEndpoints);
    report('ideal 5-bit endpoints and RGB-unweighted selectors', rows.idealAndRgbSelectors);
    report('flat cache blocks exact (bound)', rows.flatExact);
    console.log('ENDPOINTS representable (all channels) ' + counts.representable + ', not representable (clamped delta) ' + counts.notRepresentable + ' of ' + counts.diffBlocks + ' differential blocks');
    console.log('FLAT cache blocks ' + counts.flat + ' of ' + BLOCKS + ', top-end flat ' + counts.flatTop + ' (individual mode ' + counts.flatTopIndividual + '); decoded exactly at the cache value ' + counts.flatExact + ', decoded flat at another value ' + counts.flatOtherValue + ' (differential ' + counts.flatOtherValueDiff + ', mean offset ' + (counts.flatOtherValue ? (signedFlat / counts.flatOtherValue).toFixed(2) : 'n/a') + '), decoded non-flat ' + counts.flatNonFlat);
    const baseMin = minOf(rows.baseline);
    const biasKeepsRG = bias[0] === rows.baseline[0] && bias[1] === rows.baseline[1];
    const ok = counts.mismatch === 0 && biasKeepsRG && Math.abs(psnr(rows.baseline[BLUE], TEXELS) - EXPECT_BLUE_PSNR) < 1e-9 && Math.abs(baseMin - EXPECT_MIN_PSNR) < 1e-9;
    console.log((ok ? 'ok   ' : 'FAIL ') + 'baseline reconstruction mismatches ' + counts.mismatch + ', blue PSNR ' + psnr(rows.baseline[BLUE], TEXELS) + ', min ' + baseMin);
    process.exit(ok ? 0 : 1);
}

main();
