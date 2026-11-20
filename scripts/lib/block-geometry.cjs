const { MODIFIERS } = require('./etc1-individual.cjs');

function clamp8(value) {
    return value < 0 ? 0 : value > 255 ? 255 : value;
}

function subsetOf(flip, r) {
    return flip ? (r >> 2) >> 1 : (r & 3) >> 1;
}

function halfMask(flip) {
    return Array.from({ length: 16 }, (_, r) => subsetOf(flip, r));
}

function predictMask(mask, bases, tables, selectors) {
    const pred = [];
    for (let r = 0; r < 16; r += 1) {
        const s = mask[r];
        pred.push([0, 1, 2].map((c) => clamp8(bases[s][c] * 17 + MODIFIERS[tables[s]][selectors[r]])));
    }
    return pred;
}

function errorOf(target, pred) {
    const channels = [0, 0, 0];
    for (let r = 0; r < 16; r += 1) {
        for (let c = 0; c < 3; c += 1) channels[c] += (target[r][c] - pred[r][c]) ** 2;
    }
    return { sse: channels[0] + channels[1] + channels[2], channels };
}

function chooseSelectors(target, mask, bases, tables) {
    const selectors = new Array(16);
    for (let r = 0; r < 16; r += 1) {
        const s = mask[r];
        let best = Infinity;
        for (let m = 0; m < 4; m += 1) {
            let e = 0;
            for (let c = 0; c < 3; c += 1) e += (target[r][c] - clamp8(bases[s][c] * 17 + MODIFIERS[tables[s]][m])) ** 2;
            if (e < best) {
                best = e;
                selectors[r] = m;
            }
        }
    }
    return selectors;
}

function meanBases(target, mask) {
    const bases = [[0, 0, 0], [0, 0, 0]];
    for (let s = 0; s < 2; s += 1) {
        const sum = [0, 0, 0];
        let count = 0;
        for (let r = 0; r < 16; r += 1) {
            if (mask[r] !== s) continue;
            count += 1;
            for (let c = 0; c < 3; c += 1) sum[c] += target[r][c];
        }
        bases[s] = sum.map((v) => Math.min(15, Math.max(0, Math.round(v / count / 17))));
    }
    return bases;
}

function refine(target, mask, start) {
    let { bases, tables, selectors } = start;
    bases = bases.map((b) => b.slice());
    tables = tables.slice();
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
                            if (mask[r] !== s) continue;
                            e += (target[r][c] - clamp8(v * 17 + MODIFIERS[t][selectors[r]])) ** 2;
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
        selectors = chooseSelectors(target, mask, bases, tables);
    }
    return { bases, tables, selectors };
}

function fitMask(target, mask) {
    const bases = meanBases(target, mask);
    const starts = [];
    for (let a = 0; a < 8; a += 1) {
        for (let b = 0; b < 8; b += 1) {
            const tables = [a, b];
            const selectors = chooseSelectors(target, mask, bases, tables);
            const first = errorOf(target, predictMask(mask, bases, tables, selectors));
            starts.push({ first: first.sse, state: { bases, tables, selectors } });
        }
    }
    starts.sort((p, q) => p.first - q.first);
    let best = null;
    for (const start of starts.slice(0, 3)) {
        const state = refine(target, mask, start.state);
        const error = errorOf(target, predictMask(mask, state.bases, state.tables, state.selectors));
        if (!best || error.sse < best.sse) best = { sse: error.sse, channels: error.channels, mask, pred: predictMask(mask, state.bases, state.tables, state.selectors) };
    }
    return best;
}

function constantSse(target, mask) {
    let total = 0;
    for (let c = 0; c < 3; c += 1) {
        const sum = [0, 0];
        const count = [0, 0];
        for (let r = 0; r < 16; r += 1) {
            sum[mask[r]] += target[r][c];
            count[mask[r]] += 1;
        }
        for (let r = 0; r < 16; r += 1) total += (target[r][c] - sum[mask[r]] / count[mask[r]]) ** 2;
    }
    return total;
}

function freeCandidates(target, keep) {
    const seen = new Map();
    for (let key = 0; key < 4; key += 1) {
        const value = (r) => (key < 3 ? target[r][key] : target[r][0] + target[r][1] + target[r][2]);
        const order = Array.from({ length: 16 }, (_, r) => r).sort((p, q) => value(p) - value(q));
        for (let k = 1; k <= 15; k += 1) {
            const mask = new Array(16).fill(1);
            for (let i = 0; i < k; i += 1) mask[order[i]] = 0;
            const id = mask.join('');
            if (!seen.has(id)) seen.set(id, { mask, sse: constantSse(target, mask) });
        }
    }
    return [...seen.values()].sort((p, q) => p.sse - q.sse).slice(0, keep).map((c) => c.mask);
}

module.exports = { subsetOf, halfMask, fitMask, freeCandidates };
