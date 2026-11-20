const ETC1_MODIFIERS = [
    [-8, -2, 2, 8],
    [-17, -5, 5, 17],
    [-29, -9, 9, 29],
    [-42, -13, 13, 42],
    [-60, -18, 18, 60],
    [-80, -24, 24, 80],
    [-106, -33, 33, 106],
    [-183, -47, 47, 183],
];
const ETC1_INDEX_TO_MODIFIER = [2, 3, 1, 0];
const EAC_MODIFIERS = [
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

function clamp8(value) {
    return value < 0 ? 0 : value > 255 ? 255 : value;
}

function signed3(bits) {
    return bits >= 4 ? bits - 8 : bits;
}

function expand5(value) {
    return (value << 3) | (value >> 2);
}

function bigEndianWord(bytes, offset) {
    return ((bytes[offset] << 24) | (bytes[offset + 1] << 16) | (bytes[offset + 2] << 8) | bytes[offset + 3]) >>> 0;
}

function decodeEtc1Color(bytes) {
    const hi = bigEndianWord(bytes, 0);
    const lo = bigEndianWord(bytes, 4);
    const diff = (hi >>> 1) & 1;
    const flip = hi & 1;
    const tables = [(hi >>> 5) & 7, (hi >>> 2) & 7];
    let base1;
    let base2;
    if (diff) {
        const first = [(hi >>> 27) & 31, (hi >>> 19) & 31, (hi >>> 11) & 31];
        const deltas = [signed3((hi >>> 24) & 7), signed3((hi >>> 16) & 7), signed3((hi >>> 8) & 7)];
        const second = first.map((value, channel) => value + deltas[channel]);
        if (second.some((value) => value < 0 || value > 31)) {
            throw new Error('ETC2 T, H or planar block is not decoded');
        }
        base1 = first.map(expand5);
        base2 = second.map(expand5);
    } else {
        const first = [(hi >>> 28) & 15, (hi >>> 20) & 15, (hi >>> 12) & 15];
        const second = [(hi >>> 24) & 15, (hi >>> 16) & 15, (hi >>> 8) & 15];
        base1 = first.map((value) => value * 17);
        base2 = second.map((value) => value * 17);
    }
    const rgb = new Array(48);
    for (let texel = 0; texel < 16; texel += 1) {
        const x = texel >> 2;
        const y = texel & 3;
        const half = flip ? y >> 1 : x >> 1;
        const index = (((lo >>> (texel + 16)) & 1) << 1) | ((lo >>> texel) & 1);
        const modifier = ETC1_MODIFIERS[tables[half]][ETC1_INDEX_TO_MODIFIER[index]];
        const base = half ? base2 : base1;
        for (let channel = 0; channel < 3; channel += 1) {
            rgb[(y * 4 + x) * 3 + channel] = clamp8(base[channel] + modifier);
        }
    }
    return rgb;
}

function decodeEac(bytes) {
    const base = bytes[0];
    const multiplier = bytes[1] >> 4;
    const modifiers = EAC_MODIFIERS[bytes[1] & 15];
    let bits = 0;
    for (let offset = 2; offset < 8; offset += 1) {
        bits = bits * 256 + bytes[offset];
    }
    const alpha = new Array(16);
    for (let texel = 0; texel < 16; texel += 1) {
        const x = texel >> 2;
        const y = texel & 3;
        const index = Math.floor(bits / 2 ** (45 - 3 * texel)) % 8;
        alpha[y * 4 + x] = clamp8(base + modifiers[index] * multiplier);
    }
    return alpha;
}

function decodeEtc2Rgba8Block(block) {
    if (block.length !== 16) {
        throw new Error('an ETC2 RGBA8 block is 16 bytes');
    }
    const alpha = decodeEac(block.subarray(0, 8));
    const rgb = decodeEtc1Color(block.subarray(8, 16));
    const out = new Uint8Array(64);
    for (let texel = 0; texel < 16; texel += 1) {
        out[texel * 4] = rgb[texel * 3];
        out[texel * 4 + 1] = rgb[texel * 3 + 1];
        out[texel * 4 + 2] = rgb[texel * 3 + 2];
        out[texel * 4 + 3] = alpha[texel];
    }
    return out;
}

module.exports = { decodeEtc2Rgba8Block, decodeEtc1Color, decodeEac };
