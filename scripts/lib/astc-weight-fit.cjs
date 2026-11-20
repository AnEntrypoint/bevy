const FORMULAS = {
    spec: (a, b, w) => (a * (64 - w) + b * w + 32) >> 6,
    truncated: (a, b, w) => (a * (64 - w) + b * w) >> 6,
    nearest: (a, b, w) => Math.round((a * (64 - w) + b * w) / 64),
};

const MAX_WEIGHT = 64;

function fitWeights(e0, e1, texels, formula) {
    const fits = [];
    for (let w = 0; w <= MAX_WEIGHT; w += 1) {
        let ok = true;
        for (let t = 0; t < texels.length && ok; t += 1) {
            for (let c = 0; c < 3; c += 1) {
                if (formula(e0[c], e1[c], w) !== texels[t][c]) {
                    ok = false;
                    break;
                }
            }
        }
        if (ok) fits.push(w);
    }
    return fits;
}

module.exports = { FORMULAS, MAX_WEIGHT, fitWeights };
