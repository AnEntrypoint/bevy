function half(x) {
    const q = x >> 6;
    const r = x & 63;
    if (r > 32) return q + 1;
    if (r === 32) return q + (q & 1);
    return q;
}

const FORMULAS = {
    spec: (low, high, w) => (low * (64 - w) + high * w + 32) >> 6,
    floor: (low, high, w) => (low * (64 - w) + high * w) >> 6,
    even: (low, high, w) => half(low * (64 - w) + high * w),
};

module.exports = { FORMULAS };
