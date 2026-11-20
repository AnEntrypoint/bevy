const SMALL_OFFSET = [2, 5, 9, 13, 18, 24, 33, 47];
const LARGE_OFFSET = [8, 17, 29, 42, 60, 80, 106, 183];
const ALPHA_STEPS = [
    [-3, -6, -9, -15, 2, 5, 8, 14],
    [-3, -7, -10, -13, 2, 6, 9, 12],
    [-2, -5, -8, -13, 1, 4, 7, 12],
    [-2, -4, -6, -13, 1, 3, 5, 12],
    [-3, -6, -8, -12, 2, 5, 7, 11],
    [-3, -7, -9, -11, 2, 6, 8, 10],
    [-4, -7, -8, -11, 3, 6, 7, 10],
    [-3, -5, -8, -11, 2, 4, 7, 10],
    [-2, -6, -8, -10, 1, 5, 7, 9],
    [-2, -5, -8, -10, 1, 4, 7, 9],
    [-2, -4, -8, -10, 1, 3, 7, 9],
    [-2, -5, -7, -10, 1, 4, 6, 9],
    [-3, -4, -7, -10, 2, 3, 6, 9],
    [-1, -2, -3, -10, 0, 1, 2, 9],
    [-4, -6, -8, -9, 3, 5, 7, 8],
    [-3, -5, -7, -9, 2, 4, 6, 8],
];

function clampByte(value) {
    return Math.min(255, Math.max(0, value));
}

function replicate(value, bits) {
    if (bits === 4) {
        return value * 17;
    }
    return (value << 3) | (value >> 2);
}

function palette(base, table) {
    const small = SMALL_OFFSET[table];
    const large = LARGE_OFFSET[table];
    const offsets = [small, large, -small, -large];
    return offsets.map((offset) => base.map((channel) => clampByte(channel + offset)));
}

function individualBases(hi) {
    const first = [(hi >>> 28) & 15, (hi >>> 20) & 15, (hi >>> 12) & 15];
    const second = [(hi >>> 24) & 15, (hi >>> 16) & 15, (hi >>> 8) & 15];
    return [first.map((v) => replicate(v, 4)), second.map((v) => replicate(v, 4))];
}

function differentialBases(hi) {
    const signed = (bits) => (bits & 4 ? bits - 8 : bits);
    const first = [(hi >>> 27) & 31, (hi >>> 19) & 31, (hi >>> 11) & 31];
    const deltas = [signed((hi >>> 24) & 7), signed((hi >>> 16) & 7), signed((hi >>> 8) & 7)];
    const second = first.map((v, i) => v + deltas[i]);
    if (second.some((v) => v < 0 || v > 31)) {
        throw new Error('ETC2 T, H or planar block is not decoded');
    }
    return [first.map((v) => replicate(v, 5)), second.map((v) => replicate(v, 5))];
}

function decodeColour(block) {
    const hi = block.readUInt32BE(0);
    const lo = block.readUInt32BE(4);
    const flip = hi & 1;
    const differential = (hi >>> 1) & 1;
    const tables = [(hi >>> 5) & 7, (hi >>> 2) & 7];
    const bases = differential ? differentialBases(hi) : individualBases(hi);
    const palettes = bases.map((base, i) => palette(base, tables[i]));
    const out = new Array(48);
    for (let texel = 0; texel < 16; texel += 1) {
        const column = texel >> 2;
        const row = texel & 3;
        const subBlock = flip ? row >> 1 : column >> 1;
        const pair = ((lo >>> (texel + 16)) & 1) * 2 + ((lo >>> texel) & 1);
        const colour = palettes[subBlock][pair];
        for (let channel = 0; channel < 3; channel += 1) {
            out[(row * 4 + column) * 3 + channel] = colour[channel];
        }
    }
    return out;
}

function decodeAlpha(block) {
    const base = block[0];
    const multiplier = block[1] >> 4;
    const steps = ALPHA_STEPS[block[1] & 15];
    const bits = block.readUIntBE(2, 6);
    const out = new Array(16);
    for (let texel = 0; texel < 16; texel += 1) {
        const column = texel >> 2;
        const row = texel & 3;
        const index = Math.floor(bits / 2 ** (45 - 3 * texel)) % 8;
        out[row * 4 + column] = clampByte(base + steps[index] * multiplier);
    }
    return out;
}

function decodeEtc2Rgba8BlockSecond(block) {
    if (block.length !== 16) {
        throw new Error('an ETC2 RGBA8 block is 16 bytes');
    }
    const alpha = decodeAlpha(block.subarray(0, 8));
    const colour = decodeColour(block.subarray(8, 16));
    const out = new Uint8Array(64);
    for (let texel = 0; texel < 16; texel += 1) {
        out[texel * 4] = colour[texel * 3];
        out[texel * 4 + 1] = colour[texel * 3 + 1];
        out[texel * 4 + 2] = colour[texel * 3 + 2];
        out[texel * 4 + 3] = alpha[texel];
    }
    return out;
}

module.exports = { decodeEtc2Rgba8BlockSecond };
