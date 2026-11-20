const { fitWeights, MAX_WEIGHT } = require('./astc-weight-fit.cjs');

function channelFits(e0, e1, texels, formula, channel) {
    const fits = [];
    for (let w = 0; w <= MAX_WEIGHT; w += 1) {
        if (texels.every((texel) => formula(e0[channel], e1[channel], w) === texel[channel])) fits.push(w);
    }
    return fits;
}

function classify(e0, e1, texels, formula) {
    const missing = [];
    for (let c = 0; c < 3; c += 1) {
        if (channelFits(e0, e1, texels, formula, c).length === 0) missing.push(c);
    }
    return { missing, joint: fitWeights(e0, e1, texels, formula) };
}

function endpointRepairs(e0, e1, texels, formula) {
    const repairs = [];
    for (let c = 0; c < 3; c += 1) {
        for (const end of [0, 1]) {
            for (const delta of [-1, 1]) {
                const a = Array.from(e0);
                const b = Array.from(e1);
                const target = end === 0 ? a : b;
                const value = target[c] + delta;
                if (value < 0 || value > 255) continue;
                target[c] = value;
                if (fitWeights(a, b, texels, formula).length > 0) repairs.push({ channel: c, end, delta });
            }
        }
    }
    return repairs;
}

module.exports = { channelFits, classify, endpointRepairs };
