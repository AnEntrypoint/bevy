const BISE_RANGES = [
    [1, 0, 0],
    [0, 1, 0],
    [2, 0, 0],
    [0, 0, 1],
    [1, 1, 0],
    [3, 0, 0],
    [1, 0, 1],
    [2, 1, 0],
    [4, 0, 0],
    [2, 0, 1],
    [3, 1, 0],
    [5, 0, 0],
    [3, 0, 1],
    [4, 1, 0],
    [6, 0, 0],
    [4, 0, 1],
    [5, 1, 0],
    [7, 0, 0],
    [5, 0, 1],
    [6, 1, 0],
    [8, 0, 0],
];
const TRIT_CODES = [
    0, 1, 2, 4, 5, 6, 8, 9, 10, 16, 17, 18, 20, 21, 22, 24,
    25, 26, 3, 7, 11, 19, 23, 27, 12, 13, 14, 32, 33, 34, 36, 37,
    38, 40, 41, 42, 48, 49, 50, 52, 53, 54, 56, 57, 58, 35, 39, 43,
    51, 55, 59, 44, 45, 46, 64, 65, 66, 68, 69, 70, 72, 73, 74, 80,
    81, 82, 84, 85, 86, 88, 89, 90, 67, 71, 75, 83, 87, 91, 76, 77,
    78, 128, 129, 130, 132, 133, 134, 136, 137, 138, 144, 145, 146, 148, 149, 150,
    152, 153, 154, 131, 135, 139, 147, 151, 155, 140, 141, 142, 160, 161, 162, 164,
    165, 166, 168, 169, 170, 176, 177, 178, 180, 181, 182, 184, 185, 186, 163, 167,
    171, 179, 183, 187, 172, 173, 174, 192, 193, 194, 196, 197, 198, 200, 201, 202,
    208, 209, 210, 212, 213, 214, 216, 217, 218, 195, 199, 203, 211, 215, 219, 204,
    205, 206, 96, 97, 98, 100, 101, 102, 104, 105, 106, 112, 113, 114, 116, 117,
    118, 120, 121, 122, 99, 103, 107, 115, 119, 123, 108, 109, 110, 224, 225, 226,
    228, 229, 230, 232, 233, 234, 240, 241, 242, 244, 245, 246, 248, 249, 250, 227,
    231, 235, 243, 247, 251, 236, 237, 238, 28, 29, 30, 60, 61, 62, 92, 93,
    94, 156, 157, 158, 188, 189, 190, 220, 221, 222, 31, 63, 95, 159, 191, 223,
    124, 125, 126,
];
const QUINT_CODES = [
    0, 1, 2, 3, 4, 8, 9, 10, 11, 12, 16, 17, 18, 19, 20, 24,
    25, 26, 27, 28, 5, 13, 21, 29, 6, 32, 33, 34, 35, 36, 40, 41,
    42, 43, 44, 48, 49, 50, 51, 52, 56, 57, 58, 59, 60, 37, 45, 53,
    61, 14, 64, 65, 66, 67, 68, 72, 73, 74, 75, 76, 80, 81, 82, 83,
    84, 88, 89, 90, 91, 92, 69, 77, 85, 93, 22, 96, 97, 98, 99, 100,
    104, 105, 106, 107, 108, 112, 113, 114, 115, 116, 120, 121, 122, 123, 124, 101,
    109, 117, 125, 30, 102, 103, 70, 71, 38, 110, 111, 78, 79, 46, 118, 119,
    86, 87, 54, 126, 127, 94, 95, 62, 39, 47, 55, 63, 31,
];
const ENDPOINT_UNQUANT = [
    { c: 0, pb: '' },
    { c: 0, pb: '' },
    { c: 0, pb: '' },
    { c: 0, pb: '' },
    { c: 204, pb: '000000000' },
    { c: 0, pb: '' },
    { c: 113, pb: '000000000' },
    { c: 93, pb: 'b000b0bb0' },
    { c: 0, pb: '' },
    { c: 54, pb: 'b0000bb00' },
    { c: 44, pb: 'cb000cbcb' },
    { c: 0, pb: '' },
    { c: 26, pb: 'cb0000cbc' },
    { c: 22, pb: 'dcb000dcb' },
    { c: 0, pb: '' },
    { c: 13, pb: 'dcb0000dc' },
    { c: 11, pb: 'edcb000ed' },
    { c: 0, pb: '' },
    { c: 6, pb: 'edcb0000e' },
    { c: 5, pb: 'fedcb000f' },
    { c: 0, pb: '' },
];
const WEIGHT_LEVELS = {
    0: [0, 64],
    2: [0, 21, 43, 64],
    5: [0, 9, 18, 27, 37, 46, 55, 64],
    8: [0, 4, 8, 12, 17, 21, 25, 29, 35, 39, 43, 47, 52, 56, 60, 64],
    11: [0, 2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 22, 24, 26, 28, 30, 34, 36, 38, 40, 42, 44, 46, 48, 50, 52, 54, 56, 58, 60, 62, 64],
};
const WEIGHT_RANGE_BY_FIELD = {
    0x242: 8,
    0x42: 2,
    0x53: 5,
    0x442: 2,
    0x441: 0,
    0x253: 11,
};
const COMPONENT_COUNT_BY_CEM = { 4: 2, 8: 3, 12: 4 };
const TRIT_TAIL_BITS = [0, 2, 4, 5, 7];
const QUINT_TAIL_BITS = [0, 3, 5];
const DUAL_PLANE_BIT = 0x400;
const VOID_EXTENT_LOW_NINE = 0x1fc;
const TRIT_DECODE = inverseOf(TRIT_CODES, 256);
const QUINT_DECODE = inverseOf(QUINT_CODES, 128);

function inverseOf(codes, size) {
    const inverse = new Array(size).fill(-1);
    codes.forEach((code, value) => {
        inverse[code] = value;
    });
    return inverse;
}

function readBits(bytes, position, count) {
    let value = 0;
    for (let i = 0; i < count; i += 1) {
        const bit = position + i;
        value |= ((bytes[bit >> 3] >> (bit & 7)) & 1) << i;
    }
    return value;
}

function reverseBits(value, count) {
    let reversed = 0;
    for (let i = 0; i < count; i += 1) {
        reversed |= ((value >> i) & 1) << (count - 1 - i);
    }
    return reversed;
}

function digitsOf(number, base, count) {
    const digits = [];
    let rest = number;
    for (let i = 0; i < count; i += 1) {
        digits.push(rest % base);
        rest = Math.floor(rest / base);
    }
    return digits;
}

function integerSequenceBits(range, count) {
    const [bits, trits, quints] = BISE_RANGES[range];
    if (trits) {
        const rest = count % 5;
        return Math.floor(count / 5) * (5 * bits + 8) + (rest ? TRIT_TAIL_BITS[rest] + rest * bits : 0);
    }
    if (quints) {
        const rest = count % 3;
        return Math.floor(count / 3) * (3 * bits + 7) + (rest ? QUINT_TAIL_BITS[rest] + rest * bits : 0);
    }
    return count * bits;
}

function chooseEndpointRange(count, available) {
    let chosen = -1;
    for (let range = 0; range < BISE_RANGES.length; range += 1) {
        if (integerSequenceBits(range, count) <= available) {
            chosen = range;
        }
    }
    if (chosen < 0) {
        throw new Error('no ASTC endpoint range fits ' + count + ' values in ' + available + ' bits');
    }
    return chosen;
}

function unquantBits(value, bits) {
    let unquantized = 0;
    let left = 8;
    while (left > 0) {
        const count = Math.min(left, bits);
        const piece = count < bits ? value >> (bits - count) : value;
        unquantized |= piece << (left - count);
        left -= count;
    }
    return unquantized;
}

function unquantEndpoint(value, range) {
    const [bits, trits, quints] = BISE_RANGES[range];
    if (!trits && !quints) {
        return unquantBits(value, bits);
    }
    const low = value & ((1 << bits) - 1);
    const digit = value >> bits;
    const { c, pb } = ENDPOINT_UNQUANT[range];
    const all = (low & 1) ? 511 : 0;
    let spread = 0;
    for (let i = 0; i < 9; i += 1) {
        spread <<= 1;
        if (pb[i] !== '0') {
            spread |= (low >> (pb.charCodeAt(i) - 97)) & 1;
        }
    }
    const mixed = ((digit * c + spread) ^ all);
    return (all & 0x80) | (mixed >> 2);
}

function readIntegerSequence(bytes, position, count, range) {
    const [bits, trits, quints] = BISE_RANGES[range];
    const mask = (1 << bits) - 1;
    const values = [];
    let cursor = position;
    if (!trits && !quints) {
        for (let i = 0; i < count; i += 1) {
            values.push(readBits(bytes, cursor, bits));
            cursor += bits;
        }
        return values;
    }
    const groupSize = trits ? 5 : 3;
    while (values.length < count) {
        const remaining = Math.min(groupSize, count - values.length);
        let lows;
        let digits;
        if (trits) {
            if (remaining === 5) {
                const first = readBits(bytes, cursor, 2 * bits + 2);
                const second = readBits(bytes, cursor + 2 * bits + 2, 3 * bits + 6);
                cursor += 5 * bits + 8;
                lows = [first & mask, (first >> (bits + 2)) & mask, (second >> 2) & mask, (second >> (3 + bits)) & mask, (second >> (5 + 2 * bits)) & mask];
                const code = ((first >> bits) & 3) | ((second & 3) << 2) | (((second >> (2 + bits)) & 1) << 4) | (((second >> (3 + 2 * bits)) & 3) << 5) | (((second >> (5 + 3 * bits)) & 1) << 7);
                digits = digitsOf(TRIT_DECODE[code], 3, 5);
            } else {
                const tailBits = TRIT_TAIL_BITS[remaining];
                const code = readBits(bytes, cursor, tailBits);
                cursor += tailBits;
                digits = digitsOf(code, 3, remaining);
                lows = [];
                for (let i = 0; i < remaining; i += 1) {
                    lows.push(readBits(bytes, cursor, bits));
                    cursor += bits;
                }
            }
        } else if (remaining === 3) {
            const chunk = readBits(bytes, cursor, 7 + 3 * bits);
            cursor += 7 + 3 * bits;
            lows = [chunk & mask, (chunk >> (3 + bits)) & mask, (chunk >> (5 + 2 * bits)) & mask];
            const code = ((chunk >> bits) & 7) | (((chunk >> (3 + 2 * bits)) & 3) << 3) | (((chunk >> (5 + 3 * bits)) & 3) << 5);
            digits = digitsOf(QUINT_DECODE[code], 5, 3);
        } else {
            const tailBits = QUINT_TAIL_BITS[remaining];
            const code = readBits(bytes, cursor, tailBits);
            cursor += tailBits;
            digits = digitsOf(code, 5, remaining);
            lows = [];
            for (let i = 0; i < remaining; i += 1) {
                lows.push(readBits(bytes, cursor, bits));
                cursor += bits;
            }
        }
        for (let i = 0; i < remaining; i += 1) {
            values.push((digits[i] << bits) | lows[i]);
        }
    }
    return values;
}

function astcHash52(input) {
    let p = input >>> 0;
    p ^= p >>> 15;
    p = (p - (p << 17)) >>> 0;
    p = (p + (p << 7)) >>> 0;
    p = (p + (p << 4)) >>> 0;
    p ^= p >>> 5;
    p = (p + (p << 16)) >>> 0;
    p ^= p >>> 7;
    p ^= p >>> 3;
    p = (p ^ (p << 6)) >>> 0;
    p ^= p >>> 17;
    return p >>> 0;
}

function astcTexelPartition(seed, x0, y0, count) {
    const x = x0 << 1;
    const y = y0 << 1;
    const adjusted = seed + (count - 1) * 1024;
    const rnum = astcHash52(adjusted);
    let s1 = rnum & 0xf;
    let s2 = (rnum >>> 4) & 0xf;
    let s3 = (rnum >>> 8) & 0xf;
    let s4 = (rnum >>> 12) & 0xf;
    let s5 = (rnum >>> 16) & 0xf;
    let s6 = (rnum >>> 20) & 0xf;
    let s7 = (rnum >>> 24) & 0xf;
    let s8 = (rnum >>> 28) & 0xf;
    let s9 = (rnum >>> 18) & 0xf;
    let s10 = (rnum >>> 22) & 0xf;
    let s11 = (rnum >>> 26) & 0xf;
    let s12 = ((rnum >>> 30) | (rnum << 2)) & 0xf;
    s1 *= s1; s2 *= s2; s3 *= s3; s4 *= s4; s5 *= s5; s6 *= s6;
    s7 *= s7; s8 *= s8; s9 *= s9; s10 *= s10; s11 *= s11; s12 *= s12;
    let sh1;
    let sh2;
    if (adjusted & 1) {
        sh1 = (adjusted & 2) ? 4 : 5;
        sh2 = count === 3 ? 6 : 5;
    } else {
        sh1 = count === 3 ? 6 : 5;
        sh2 = (adjusted & 2) ? 4 : 5;
    }
    const sh3 = (adjusted & 0x10) ? sh1 : sh2;
    s1 >>= sh1; s2 >>= sh2; s3 >>= sh1; s4 >>= sh2; s5 >>= sh1; s6 >>= sh2;
    s7 >>= sh1; s8 >>= sh2; s9 >>= sh3; s10 >>= sh3; s11 >>= sh3; s12 >>= sh3;
    const a = (s1 * x + s2 * y + s11 * 0 + (rnum >>> 14)) & 0x3f;
    const b = (s3 * x + s4 * y + s12 * 0 + (rnum >>> 10)) & 0x3f;
    const c = count < 3 ? 0 : (s5 * x + s6 * y + s9 * 0 + (rnum >>> 6)) & 0x3f;
    const d = count < 4 ? 0 : (s7 * x + s8 * y + s10 * 0 + (rnum >>> 2)) & 0x3f;
    if (a >= b && a >= c && a >= d) {
        return 0;
    }
    if (b >= c && b >= d) {
        return 1;
    }
    if (c >= d) {
        return 2;
    }
    return 3;
}

function decodeVoidExtent(block) {
    const channels = [0, 1, 2, 3].map((channel) => readBits(block, 64 + 16 * channel, 16) >> 8);
    const out = new Uint8Array(64);
    for (let texel = 0; texel < 16; texel += 1) {
        for (let channel = 0; channel < 4; channel += 1) {
            out[texel * 4 + channel] = channels[channel];
        }
    }
    return out;
}

function decodeAstc4x4Block(block) {
    const field = block[0] | (block[1] << 8);
    if ((field & 0x1ff) === VOID_EXTENT_LOW_NINE) {
        return decodeVoidExtent(block);
    }
    const blockMode = field & 0x7ff;
    const weightRange = WEIGHT_RANGE_BY_FIELD[blockMode];
    if (weightRange === undefined) {
        throw new Error('unsupported ASTC block mode 0x' + blockMode.toString(16));
    }
    const dual = (blockMode & DUAL_PLANE_BIT) !== 0;
    const weightBits = BISE_RANGES[weightRange][0];
    const weightCount = dual ? 32 : 16;
    const weightBitsTotal = weightCount * weightBits;
    let position = 11;
    const subsets = readBits(block, position, 2) + 1;
    position += 2;
    let partitionSeed = 0;
    let cem;
    if (subsets === 1) {
        cem = readBits(block, position, 4);
        position += 4;
    } else {
        partitionSeed = readBits(block, position, 10);
        position += 10;
        cem = (readBits(block, position, 6) >> 2) & 15;
        position += 6;
    }
    const componentCount = COMPONENT_COUNT_BY_CEM[cem];
    if (componentCount === undefined) {
        throw new Error('unsupported ASTC colour endpoint mode ' + cem);
    }
    const ccs = dual ? readBits(block, 128 - weightBitsTotal - 2, 2) : 0;
    const perSubset = 2 * componentCount;
    const available = 128 - weightBitsTotal - (dual ? 2 : 0) - position;
    const endpointRange = chooseEndpointRange(perSubset * subsets, available);
    const endpoints = readIntegerSequence(block, position, perSubset * subsets, endpointRange)
        .map((value) => unquantEndpoint(value, endpointRange));
    const weights = [];
    for (let index = 0; index < weightCount; index += 1) {
        const stored = readBits(block, 128 - weightBits * (index + 1), weightBits);
        weights.push(WEIGHT_LEVELS[weightRange][reverseBits(stored, weightBits)]);
    }
    const out = new Uint8Array(64);
    for (let texel = 0; texel < 16; texel += 1) {
        const subset = subsets === 1 ? 0 : astcTexelPartition(partitionSeed, texel & 3, texel >> 2, subsets);
        const base = subset * perSubset;
        const values = [];
        for (let channel = 0; channel < componentCount; channel += 1) {
            const plane = dual && channel === ccs ? 1 : 0;
            const weight = weights[dual ? 2 * texel + plane : texel];
            const low = endpoints[base + 2 * channel];
            const high = endpoints[base + 2 * channel + 1];
            values.push((low * (64 - weight) + high * weight + 32) >> 6);
        }
        const pixel = componentCount === 2 ? [values[0], values[0], values[0], values[1]]
            : componentCount === 3 ? [values[0], values[1], values[2], 255]
            : values;
        out.set(pixel, texel * 4);
    }
    return out;
}

module.exports = {
    decodeAstc4x4Block,
    decodeVoidExtent,
    chooseEndpointRange,
    integerSequenceBits,
    readIntegerSequence,
    unquantEndpoint,
    astcTexelPartition,
    readBits,
    reverseBits,
    WEIGHT_RANGE_BY_FIELD,
    BISE_RANGES,
    DUAL_PLANE_BIT,
};
