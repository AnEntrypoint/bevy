const T_DISTANCES = [3, 6, 11, 16, 23, 32, 41, 64];
const PLANAR_BITS = [6, 7, 6];
const T_SLOTS = [
    { group: 0, sign: 0 },
    { group: 1, sign: 1 },
    { group: 1, sign: 0 },
    { group: 1, sign: -1 },
];
const H_SLOTS = [
    { group: 0, sign: 1 },
    { group: 0, sign: -1 },
    { group: 1, sign: 1 },
    { group: 1, sign: -1 },
];

function clamp8(value) {
    return value < 0 ? 0 : value > 255 ? 255 : value;
}

function expand444(value) {
    return value * 17;
}

function codeOf(value) {
    return Math.min(15, Math.max(0, Math.round(value / 17)));
}

function expandPlanar(channel, value) {
    if (PLANAR_BITS[channel] === 6) return (value << 2) | (value >> 4);
    return (value << 1) | (value >> 6);
}

function tPalette(c0, c1, d) {
    const a = c0.map(expand444);
    const b = c1.map(expand444);
    return [a, b.map((v) => clamp8(v + d)), b, b.map((v) => clamp8(v - d))];
}

function hPalette(c0, c1, d) {
    const a = c0.map(expand444);
    const b = c1.map(expand444);
    return [
        a.map((v) => clamp8(v + d)),
        a.map((v) => clamp8(v - d)),
        b.map((v) => clamp8(v + d)),
        b.map((v) => clamp8(v - d)),
    ];
}

function planarPredict(o, h, v) {
    const out = [];
    for (let r = 0; r < 16; r += 1) {
        const x = r & 3;
        const y = r >> 2;
        out.push([0, 1, 2].map((c) => clamp8((x * (h[c] - o[c]) + y * (v[c] - o[c]) + 4 * o[c] + 2) >> 2)));
    }
    return out;
}

function paletteDetail(target, make, s, d) {
    const palette = make(s.slice(0, 3), s.slice(3, 6), d);
    const pred = [];
    for (let r = 0; r < 16; r += 1) {
        let best = Infinity;
        let pick = null;
        for (const colour of palette) {
            let e = 0;
            for (let c = 0; c < 3; c += 1) {
                const diff = target[r][c] - colour[c];
                e += diff * diff;
            }
            if (e < best) {
                best = e;
                pick = colour;
            }
        }
        pred.push(pick);
    }
    return pred;
}

function paletteTotal(target, make, s, d) {
    const pred = paletteDetail(target, make, s, d);
    let total = 0;
    for (let r = 0; r < 16; r += 1) {
        for (let c = 0; c < 3; c += 1) {
            const diff = target[r][c] - pred[r][c];
            total += diff * diff;
        }
    }
    return total;
}

function paletteStarts(target) {
    let dark = 0;
    let bright = 0;
    const sum = [0, 0, 0];
    for (let r = 0; r < 16; r += 1) {
        const s = target[r][0] + target[r][1] + target[r][2];
        if (s < target[dark][0] + target[dark][1] + target[dark][2]) dark = r;
        if (s > target[bright][0] + target[bright][1] + target[bright][2]) bright = r;
        for (let c = 0; c < 3; c += 1) sum[c] += target[r][c];
    }
    const mean = sum.map((v) => v / 16);
    const starts = [[...mean, ...mean]];
    for (let k = 0; k < 16; k += 1) {
        starts.push([...target[dark], ...target[k]]);
        starts.push([...target[k], ...target[bright]]);
    }
    return starts;
}

function lloydPalette(target, make, slots, d, start) {
    let c0 = start.slice(0, 3);
    let c1 = start.slice(3, 6);
    for (let iter = 0; iter < 6; iter += 1) {
        const palette = make(c0.map(codeOf), c1.map(codeOf), d);
        const sum = [[0, 0, 0], [0, 0, 0]];
        const count = [0, 0];
        for (let r = 0; r < 16; r += 1) {
            let best = Infinity;
            let pick = 0;
            for (let k = 0; k < 4; k += 1) {
                let e = 0;
                for (let c = 0; c < 3; c += 1) {
                    const diff = target[r][c] - palette[k][c];
                    e += diff * diff;
                }
                if (e < best) {
                    best = e;
                    pick = k;
                }
            }
            const slot = slots[pick];
            count[slot.group] += 1;
            for (let c = 0; c < 3; c += 1) sum[slot.group][c] += target[r][c] - slot.sign * d;
        }
        if (count[0]) c0 = sum[0].map((v) => v / count[0]);
        if (count[1]) c1 = sum[1].map((v) => v / count[1]);
    }
    return [...c0.map(codeOf), ...c1.map(codeOf)];
}

function polishPalette(target, make, s, d) {
    let cur = paletteTotal(target, make, s, d);
    for (let pass = 0; pass < 4; pass += 1) {
        let moved = false;
        for (let i = 0; i < 6; i += 1) {
            for (const step of [-1, 1]) {
                const before = s[i];
                const value = before + step;
                if (value < 0 || value > 15) continue;
                s[i] = value;
                const total = paletteTotal(target, make, s, d);
                if (total < cur) {
                    cur = total;
                    moved = true;
                } else {
                    s[i] = before;
                }
            }
        }
        if (!moved) break;
    }
    return cur;
}

function searchPalette(target, make, slots) {
    const candidates = [];
    for (const d of T_DISTANCES) {
        for (const start of paletteStarts(target)) {
            const s = lloydPalette(target, make, slots, d, start);
            candidates.push({ total: paletteTotal(target, make, s, d), s, d });
        }
    }
    candidates.sort((p, q) => p.total - q.total);
    let best = null;
    for (const candidate of candidates.slice(0, 3)) {
        const total = polishPalette(target, make, candidate.s, candidate.d);
        if (!best || total < best.total) best = { total, s: candidate.s.slice(), d: candidate.d };
    }
    return {
        total: best.total,
        pred: paletteDetail(target, make, best.s, best.d),
        colours: { c0: best.s.slice(0, 3), c1: best.s.slice(3, 6), d: best.d },
    };
}

function fitT(target) {
    return searchPalette(target, tPalette, T_SLOTS);
}

function fitH(target) {
    return searchPalette(target, hPalette, H_SLOTS);
}

function solve3(m, v) {
    const a = m.map((row, i) => [...row, v[i]]);
    for (let col = 0; col < 3; col += 1) {
        let pivot = col;
        for (let row = col + 1; row < 3; row += 1) {
            if (Math.abs(a[row][col]) > Math.abs(a[pivot][col])) pivot = row;
        }
        [a[col], a[pivot]] = [a[pivot], a[col]];
        for (let row = 0; row < 3; row += 1) {
            if (row === col) continue;
            const f = a[row][col] / a[col][col];
            for (let k = col; k < 4; k += 1) a[row][k] -= f * a[col][k];
        }
    }
    return [0, 1, 2].map((i) => a[i][3] / a[i][i]);
}

function leastSquaresPlanar(target, c) {
    let sx = 0;
    let sy = 0;
    let sxx = 0;
    let syy = 0;
    let sxy = 0;
    let st = 0;
    let sxt = 0;
    let syt = 0;
    for (let r = 0; r < 16; r += 1) {
        const x = r & 3;
        const y = r >> 2;
        const t = target[r][c];
        sx += x;
        sy += y;
        sxx += x * x;
        syy += y * y;
        sxy += x * y;
        st += t;
        sxt += x * t;
        syt += y * t;
    }
    const [alpha, beta, gamma] = solve3([[16, sx, sy], [sx, sxx, sxy], [sy, sxy, syy]], [st, sxt, syt]);
    const o = alpha - 0.5;
    return [o, o + 4 * beta, o + 4 * gamma];
}

function nearestGrid(channel, value) {
    const max = (1 << PLANAR_BITS[channel]) - 1;
    let best = 0;
    for (let k = 1; k <= max; k += 1) {
        if (Math.abs(expandPlanar(channel, k) - value) < Math.abs(expandPlanar(channel, best) - value)) best = k;
    }
    return best;
}

function planarGridColours(gi) {
    const o = [0, 1, 2].map((c) => expandPlanar(c, gi[c]));
    const h = [0, 1, 2].map((c) => expandPlanar(c, gi[3 + c]));
    const v = [0, 1, 2].map((c) => expandPlanar(c, gi[6 + c]));
    return { o, h, v };
}

function planarTotal(target, gi) {
    const { o, h, v } = planarGridColours(gi);
    const pred = planarPredict(o, h, v);
    let total = 0;
    for (let r = 0; r < 16; r += 1) {
        for (let c = 0; c < 3; c += 1) {
            const diff = target[r][c] - pred[r][c];
            total += diff * diff;
        }
    }
    return total;
}

function fitPlanar(target) {
    const gi = new Array(9);
    for (let c = 0; c < 3; c += 1) {
        const [o, h, v] = leastSquaresPlanar(target, c);
        gi[c] = nearestGrid(c, o);
        gi[3 + c] = nearestGrid(c, h);
        gi[6 + c] = nearestGrid(c, v);
    }
    let cur = planarTotal(target, gi);
    for (let pass = 0; pass < 12; pass += 1) {
        let moved = false;
        for (let k = 0; k < 9; k += 1) {
            const c = k % 3;
            const max = (1 << PLANAR_BITS[c]) - 1;
            for (const step of [-2, -1, 1, 2]) {
                const before = gi[k];
                const value = before + step;
                if (value < 0 || value > max) continue;
                gi[k] = value;
                const total = planarTotal(target, gi);
                if (total < cur) {
                    cur = total;
                    moved = true;
                } else {
                    gi[k] = before;
                }
            }
        }
        if (!moved) break;
    }
    const { o, h, v } = planarGridColours(gi);
    return { total: cur, pred: planarPredict(o, h, v), grid: gi.slice() };
}

module.exports = { T_DISTANCES, tPalette, hPalette, planarPredict, planarGridColours, fitT, fitH, fitPlanar };
