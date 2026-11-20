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
const FIELDS = [[27, 24], [19, 16], [11, 8]];
const TABLE_MASK = (7 << 5) | (7 << 2);
const FIELD_MASK = (31 << 27) | (31 << 19) | (31 << 11) | (7 << 24) | (7 << 16) | (7 << 8);
const ALL_TABLES = [[0, 1, 2, 3, 4, 5, 6, 7], [0, 1, 2, 3, 4, 5, 6, 7]];
const SCENARIOS = ['sel', 'e1', 'e2', 'e3', 'e4'];

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

function parseDiff(colour) {
    const hi = word(colour, 0);
    const first5 = [(hi >>> 27) & 31, (hi >>> 19) & 31, (hi >>> 11) & 31];
    const second5 = first5.map((value, c) => value + signed3((hi >>> FIELDS[c][1]) & 7));
    if (second5.some((value) => value < 0 || value > 31)) throw new Error('special mode');
    return { flip: hi & 1, tables: [(hi >>> 5) & 7, (hi >>> 2) & 7], first5, second5 };
}

function halfOf(flip, r) {
    const row = r >> 2;
    const col = r & 3;
    return flip ? row >> 1 : col >> 1;
}

function texelOf(r) {
    return (r & 3) * 4 + (r >> 2);
}

function rasterOf(t) {
    return (t & 3) * 4 + (t >> 2);
}

function base8(state, h, c) {
    return expand5(h ? state.second5[c] : state.first5[c]);
}

function cloneState(state) {
    return { flip: state.flip, tables: state.tables.slice(), first5: state.first5.slice(), second5: state.second5.slice() };
}

function predict(state, kByT) {
    const out = [];
    for (let r = 0; r < 16; r += 1) {
        const h = halfOf(state.flip, r);
        const k = kByT[texelOf(r)];
        out.push([0, 1, 2].map((c) => clamp8(base8(state, h, c) + MODIFIERS[state.tables[h]][k])));
    }
    return out;
}

function chooseK(state, cache, r, selCh) {
    const h = halfOf(state.flip, r);
    let best = 0;
    let bestError = Infinity;
    for (let k = 0; k < 4; k += 1) {
        let error = 0;
        for (const x of selCh) {
            const d = cache[r][x] - clamp8(base8(state, h, x) + MODIFIERS[state.tables[h]][k]);
            error += d * d;
        }
        if (error < bestError) {
            bestError = error;
            best = k;
        }
    }
    return best;
}

function selectors(state, cache, selCh) {
    const kByT = new Array(16);
    for (let t = 0; t < 16; t += 1) kByT[t] = chooseK(state, cache, rasterOf(t), selCh);
    return kByT;
}

function baselineSelectors(lo) {
    const kByT = new Array(16);
    for (let t = 0; t < 16; t += 1) kByT[t] = INDEX_TO_MODIFIER[(((lo >>> (t + 16)) & 1) << 1) | ((lo >>> t) & 1)];
    return kByT;
}

function encode(state, hi0, lo0, kByT) {
    let hi = (hi0 & ~(TABLE_MASK | FIELD_MASK | 1)) >>> 0;
    hi = (hi | state.flip | (state.tables[0] << 5) | (state.tables[1] << 2)) >>> 0;
    for (let c = 0; c < 3; c += 1) {
        hi = (hi | (state.first5[c] << FIELDS[c][0]) | (((state.second5[c] - state.first5[c]) & 7) << FIELDS[c][1])) >>> 0;
    }
    let lo = lo0 >>> 0;
    for (let t = 0; t < 16; t += 1) {
        const index = MODIFIER_TO_INDEX[kByT[t]];
        const mask = (1 << t) | (1 << (t + 16));
        lo = ((lo & ~mask) | ((index & 1) << t) | ((index >> 1) << (t + 16))) >>> 0;
    }
    const out = Buffer.alloc(8);
    putWord(out, 0, hi);
    putWord(out, 4, lo);
    return out;
}

function subsetCost(state, cache, h, c, t, b, selCh, costCh) {
    const vals = [0, 1, 2].map((x) => (x === c ? expand5(b) : base8(state, h, x)));
    let total = 0;
    for (let r = 0; r < 16; r += 1) {
        if (halfOf(state.flip, r) !== h) continue;
        let bestK = 0;
        let bestError = Infinity;
        for (let k = 0; k < 4; k += 1) {
            let error = 0;
            for (const x of selCh) {
                const d = cache[r][x] - clamp8(vals[x] + MODIFIERS[t][k]);
                error += d * d;
            }
            if (error < bestError) {
                bestError = error;
                bestK = k;
            }
        }
        for (const x of costCh) {
            const d = cache[r][x] - clamp8(vals[x] + MODIFIERS[t][bestK]);
            total += d * d;
        }
    }
    return total;
}

function searchChannel(state, cache, c, selCh, costCh, tableSets) {
    const cost = [[], []];
    const arg = [[], []];
    for (let h = 0; h < 2; h += 1) {
        for (let b = 0; b < 32; b += 1) {
            let best = Infinity;
            let bestT = tableSets[h][0];
            for (const t of tableSets[h]) {
                const value = subsetCost(state, cache, h, c, t, b, selCh, costCh);
                if (value < best) {
                    best = value;
                    bestT = t;
                }
            }
            cost[h][b] = best;
            arg[h][b] = bestT;
        }
    }
    let total = Infinity;
    let pick = null;
    for (let b0 = 0; b0 < 32; b0 += 1) {
        for (let b1 = Math.max(0, b0 - 4); b1 <= Math.min(31, b0 + 3); b1 += 1) {
            const value = cost[0][b0] + cost[1][b1];
            if (value < total) {
                total = value;
                pick = { b0, b1, t0: arg[0][b0], t1: arg[1][b1] };
            }
        }
    }
    return { total, pick };
}

function applyPick(state, c, pick) {
    state.first5[c] = pick.b0;
    state.second5[c] = pick.b1;
    state.tables = [pick.t0, pick.t1];
}

function runScenario(scenario, state0, cache) {
    if (scenario === 'sel') {
        return { state: cloneState(state0), kByT: selectors(state0, cache, [BLUE]) };
    }
    if (scenario === 'e1') {
        let best = null;
        for (const flip of [0, 1]) {
            const state = cloneState(state0);
            state.flip = flip;
            const result = searchChannel(state, cache, BLUE, [BLUE], [BLUE], ALL_TABLES);
            if (!best || result.total < best.total) {
                applyPick(state, BLUE, result.pick);
                best = { total: result.total, state };
            }
        }
        return { state: best.state, kByT: selectors(best.state, cache, [BLUE]) };
    }
    if (scenario === 'e4') {
        const joint = cloneState(state0);
        for (let round = 0; round < 2; round += 1) {
            for (const c of [BLUE, 0, 1]) {
                applyPick(joint, c, searchChannel(joint, cache, c, [0, 1, 2], [0, 1, 2], ALL_TABLES).pick);
            }
        }
        return { state: joint, kByT: selectors(joint, cache, [0, 1, 2]) };
    }
    const state = cloneState(state0);
    applyPick(state, BLUE, searchChannel(state, cache, BLUE, [0, 1, 2], [BLUE], ALL_TABLES).pick);
    if (scenario === 'e2') {
        return { state, kByT: selectors(state, cache, [0, 1, 2]) };
    }
    for (const c of [0, 1]) {
        const fixed = [[state.tables[0]], [state.tables[1]]];
        applyPick(state, c, searchChannel(state, cache, c, [0, 1, 2], [c], fixed).pick);
    }
    return { state, kByT: selectors(state, cache, [0, 1, 2]) };
}

function toRgb(decoded) {
    const out = [];
    for (let r = 0; r < 16; r += 1) out.push([decoded[r * 4], decoded[r * 4 + 1], decoded[r * 4 + 2]]);
    return out;
}

function accumulate(sums, rgb, cache) {
    for (let r = 0; r < 16; r += 1) {
        for (let c = 0; c < 3; c += 1) {
            const d = cache[r][c] - rgb[r][c];
            sums[c] += d * d;
        }
    }
}

function psnr(sumSquares) {
    return +(10 * Math.log10(65025 / (sumSquares / TEXELS))).toFixed(2);
}

function main() {
    const name = fs.readdirSync(path.join(root, 'target', 'gltf-cache-etc2')).find((file) => file.startsWith(TEXTURE_PREFIX));
    const etc2 = level0('gltf-cache-etc2', name);
    const rgba8 = level0('gltf-cache-rgba8', name);
    const keys = ['base', ...SCENARIOS];
    const sums = {};
    const changed = {};
    const searchMismatch = {};
    for (const key of keys) {
        sums[key] = [0, 0, 0];
        changed[key] = 0;
        searchMismatch[key] = 0;
    }
    const stats = { diffBlocks: 0, topBlocks: 0, reencodeSame: 0, evalMismatch: 0 };
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
        const libraryRgb = toRgb(decodeEtc2Rgba8Block(bytes));
        const hi0 = word(colour, 0);
        const lo0 = word(colour, 4);
        if (!((hi0 >>> 1) & 1)) {
            for (const key of keys) accumulate(sums[key], libraryRgb, cache);
            continue;
        }
        stats.diffBlocks += 1;
        const state0 = parseDiff(colour);
        const kBase = baselineSelectors(lo0);
        if (encode(state0, hi0, lo0, kBase).equals(colour)) stats.reencodeSame += 1;
        const predBase = predict(state0, kBase);
        for (let r = 0; r < 16; r += 1) {
            for (let c = 0; c < 3; c += 1) {
                if (predBase[r][c] !== libraryRgb[r][c]) stats.evalMismatch += 1;
            }
        }
        accumulate(sums.base, libraryRgb, cache);
        const mean = cache.reduce((acc, v) => acc + v[BLUE], 0) / 16;
        if (mean < TOP_BLOCK) {
            for (const key of SCENARIOS) accumulate(sums[key], libraryRgb, cache);
            continue;
        }
        stats.topBlocks += 1;
        for (const key of SCENARIOS) {
            const result = runScenario(key, state0, cache);
            const newColour = encode(result.state, hi0, lo0, result.kByT);
            const newRgb = toRgb(decodeEtc2Rgba8Block(Buffer.concat([bytes.subarray(0, 8), newColour])));
            const pred = predict(result.state, result.kByT);
            for (let r = 0; r < 16; r += 1) {
                for (let c = 0; c < 3; c += 1) {
                    if (pred[r][c] !== newRgb[r][c]) searchMismatch[key] += 1;
                }
            }
            if (!newColour.equals(colour)) changed[key] += 1;
            accumulate(sums[key], newRgb, cache);
        }
    }
    const labels = {
        sel: 'SEL blue-optimal selectors, transcoder endpoints (top-end blocks)',
        e1: 'E1 blue-error endpoint search over tables, base pairs and flip, blue-optimal selectors',
        e2: 'E2 blue-error endpoint search over tables and base pairs, RGB selectors',
        e3: 'E3 E2 then base-only R and G searches with tables fixed',
        e4: 'E4 joint RGB coordinate search over tables and base pairs, RGB selectors (2 rounds)',
    };
    for (const key of SCENARIOS) {
        const values = [0, 1, 2].map((c) => psnr(sums[key][c]));
        const min = Math.min(...values);
        console.log(labels[key] + ': R/G/B ' + values.join(' / ') + ' dB, min ' + min + ' dB' + (min >= BAR_DB ? ' (clears the 35 dB bar)' : '') + '; blue ' + values[BLUE] + ' dB; blocks changed ' + changed[key] + '; search-vs-library mismatches ' + searchMismatch[key]);
    }
    console.log('CHECK re-encode of transcoder blocks is byte-identical: ' + stats.reencodeSame + ' of ' + stats.diffBlocks + ' differential blocks; evaluator vs library mismatches ' + stats.evalMismatch + '; top-end differential blocks searched ' + stats.topBlocks);
    const baseValues = [0, 1, 2].map((c) => psnr(sums.base[c]));
    console.log('BASELINE R/G/B ' + baseValues.join(' / ') + ' dB, min ' + Math.min(...baseValues) + ' dB');
    const baseOk = Math.abs(psnr(sums.base[BLUE]) - EXPECT_BLUE_PSNR) < 1e-9 && Math.abs(Math.min(...baseValues) - EXPECT_MIN_PSNR) < 1e-9;
    const ok = baseOk && stats.reencodeSame === stats.diffBlocks && stats.evalMismatch === 0 && SCENARIOS.every((key) => searchMismatch[key] === 0);
    console.log((ok ? 'ok   ' : 'FAIL ') + 're-encode identical ' + stats.reencodeSame + '/' + stats.diffBlocks + ', evaluator mismatches ' + stats.evalMismatch + ', search mismatches ' + SCENARIOS.map((key) => searchMismatch[key]).join('/') + ', baseline blue PSNR ' + psnr(sums.base[BLUE]));
    process.exit(ok ? 0 : 1);
}

main();
