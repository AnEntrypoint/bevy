const MODIFIERS = [
    [-8, -2, 2, 8],
    [-17, -5, 5, 17],
    [-29, -9, 9, 29],
    [-42, -13, 13, 42],
    [-60, -18, 18, 60],
    [-80, -24, 24, 80],
    [-106, -33, 33, 106],
    [-183, -47, 47, 183],
];
const INDEX_TO_MODIFIER = [2, 3, 1, 0];
const MODIFIER_TO_INDEX = [3, 2, 0, 1];

function clamp8(value) {
    return value < 0 ? 0 : value > 255 ? 255 : value;
}

function texelBit(r) {
    return (r & 3) * 4 + (r >> 2);
}

function halfOf(flip, r) {
    return flip ? (r >> 2) >> 1 : (r & 3) >> 1;
}

function predictIndividual(flip, bases, tables, selectors) {
    const pred = [];
    for (let r = 0; r < 16; r += 1) {
        const s = halfOf(flip, r);
        pred.push([0, 1, 2].map((c) => clamp8(bases[s][c] * 17 + MODIFIERS[tables[s]][selectors[r]])));
    }
    return pred;
}

function totalError(target, pred) {
    let total = 0;
    for (let r = 0; r < 16; r += 1) {
        for (let c = 0; c < 3; c += 1) {
            const d = target[r][c] - pred[r][c];
            total += d * d;
        }
    }
    return total;
}

function chooseSelectors(target, flip, bases, tables) {
    const selectors = new Array(16);
    for (let r = 0; r < 16; r += 1) {
        const s = halfOf(flip, r);
        let best = Infinity;
        for (let m = 0; m < 4; m += 1) {
            let e = 0;
            for (let c = 0; c < 3; c += 1) {
                const d = target[r][c] - clamp8(bases[s][c] * 17 + MODIFIERS[tables[s]][m]);
                e += d * d;
            }
            if (e < best) {
                best = e;
                selectors[r] = m;
            }
        }
    }
    return selectors;
}

function refine(target, flip, bases, tables, selectors) {
    for (let round = 0; round < 3; round += 1) {
        for (let s = 0; s < 2; s += 1) {
            let bestCost = Infinity;
            let bestTable = tables[s];
            let bestBase = bases[s].slice();
            for (let t = 0; t < 8; t += 1) {
                const base = [0, 0, 0];
                let cost = 0;
                for (let c = 0; c < 3; c += 1) {
                    let bestV = 0;
                    let bestE = Infinity;
                    for (let v = 0; v <= 15; v += 1) {
                        let e = 0;
                        for (let r = 0; r < 16; r += 1) {
                            if (halfOf(flip, r) !== s) continue;
                            const d = target[r][c] - clamp8(v * 17 + MODIFIERS[t][selectors[r]]);
                            e += d * d;
                        }
                        if (e < bestE) {
                            bestE = e;
                            bestV = v;
                        }
                    }
                    base[c] = bestV;
                    cost += bestE;
                }
                if (cost < bestCost) {
                    bestCost = cost;
                    bestTable = t;
                    bestBase = base;
                }
            }
            tables[s] = bestTable;
            bases[s] = bestBase;
        }
        selectors = chooseSelectors(target, flip, bases, tables);
    }
    return { bases, tables, selectors };
}

function fitFlip(target, flip) {
    let best = null;
    for (let t0 = 0; t0 < 8; t0 += 1) {
        const t1 = t0;
        const bases = [[0, 0, 0], [0, 0, 0]];
        for (let s = 0; s < 2; s += 1) {
            const sum = [0, 0, 0];
            let count = 0;
            for (let r = 0; r < 16; r += 1) {
                if (halfOf(flip, r) !== s) continue;
                count += 1;
                for (let c = 0; c < 3; c += 1) sum[c] += target[r][c];
            }
            bases[s] = sum.map((v) => Math.min(15, Math.max(0, Math.round(v / count / 17))));
        }
        const tables = [t0, t1];
        const selectors = chooseSelectors(target, flip, bases, tables);
        const state = refine(target, flip, bases, tables, selectors);
        const pred = predictIndividual(flip, state.bases, state.tables, state.selectors);
        const total = totalError(target, pred);
        if (!best || total < best.total) best = { total, flip, bases: state.bases, tables: state.tables, selectors: state.selectors, pred };
    }
    return best;
}

function fitIndividual(target) {
    const a = fitFlip(target, 0);
    const b = fitFlip(target, 1);
    return a.total <= b.total ? a : b;
}

function encodeIndividual(fit) {
    const { flip, bases, tables, selectors } = fit;
    let hi = (flip | (tables[0] << 5) | (tables[1] << 2)) >>> 0;
    hi = (hi | (bases[0][0] << 28) | (bases[1][0] << 24) | (bases[0][1] << 20) | (bases[1][1] << 16) | (bases[0][2] << 12) | (bases[1][2] << 8)) >>> 0;
    let lo = 0;
    for (let r = 0; r < 16; r += 1) {
        const q = texelBit(r);
        const code = MODIFIER_TO_INDEX[selectors[r]];
        lo = (lo | ((code & 1) << q) | ((code >> 1) << (q + 16))) >>> 0;
    }
    const out = Buffer.alloc(8);
    out.writeUInt32BE(hi, 0);
    out.writeUInt32BE(lo, 4);
    return out;
}

module.exports = { MODIFIERS, INDEX_TO_MODIFIER, MODIFIER_TO_INDEX, predictIndividual, fitIndividual, encodeIndividual };
