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

module.exports = { channelFits, classify };
