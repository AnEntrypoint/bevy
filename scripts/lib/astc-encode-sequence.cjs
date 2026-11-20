const { BISE_RANGES, TRIT_CODES, QUINT_CODES } = require('./astc-decode.cjs');

const TRIT_TAIL_BITS = [0, 2, 4, 5, 7];
const QUINT_TAIL_BITS = [0, 3, 5];

class BitWriter {
    constructor() {
        this.bits = [];
    }

    put(value, count) {
        for (let i = 0; i < count; i += 1) this.bits.push((value >> i) & 1);
    }

    bytes() {
        const out = new Uint8Array(Math.ceil(this.bits.length / 8) + 4);
        this.bits.forEach((bit, i) => {
            out[i >> 3] |= bit << (i & 7);
        });
        return out;
    }
}

function packedValue(digits, base) {
    return digits.reduceRight((acc, digit) => acc * base + digit, 0);
}

function putTrits(out, digits, lows, bits) {
    if (digits.length === 5) {
        const code = TRIT_CODES[packedValue(digits, 3)];
        const first = lows[0] | ((code & 3) << bits) | (lows[1] << (bits + 2));
        const second = ((code >> 2) & 3) | (lows[2] << 2) | (((code >> 4) & 1) << (2 + bits)) | (lows[3] << (3 + bits)) | (((code >> 5) & 3) << (3 + 2 * bits)) | (lows[4] << (5 + 2 * bits)) | (((code >> 7) & 1) << (5 + 3 * bits));
        out.put(first, 2 * bits + 2);
        out.put(second, 3 * bits + 6);
    } else if (digits.length === 1) {
        out.put(lows[0], bits);
        out.put(digits[0], TRIT_TAIL_BITS[1]);
    } else {
        out.put(packedValue(digits, 3), TRIT_TAIL_BITS[digits.length]);
        lows.forEach((low) => out.put(low, bits));
    }
}

function putQuints(out, digits, lows, bits) {
    if (digits.length === 3) {
        const code = QUINT_CODES[packedValue(digits, 5)];
        const chunk = lows[0] | ((code & 7) << bits) | (lows[1] << (3 + bits)) | (((code >> 3) & 3) << (3 + 2 * bits)) | (lows[2] << (5 + 2 * bits)) | (((code >> 5) & 3) << (5 + 3 * bits));
        out.put(chunk, 7 + 3 * bits);
    } else {
        out.put(packedValue(digits, 5), QUINT_TAIL_BITS[digits.length]);
        lows.forEach((low) => out.put(low, bits));
    }
}

function encodeIntegerSequence(values, range) {
    const [bits, trits, quints] = BISE_RANGES[range];
    const out = new BitWriter();
    if (!trits && !quints) {
        values.forEach((value) => out.put(value, bits));
    } else {
        const groupSize = trits ? 5 : 3;
        for (let start = 0; start < values.length; start += groupSize) {
            const group = values.slice(start, start + groupSize);
            const digits = group.map((value) => value >> bits);
            const lows = group.map((value) => value & ((1 << bits) - 1));
            if (trits) putTrits(out, digits, lows, bits);
            else putQuints(out, digits, lows, bits);
        }
    }
    return { bytes: out.bytes(), bitLength: out.bits.length };
}

module.exports = { encodeIntegerSequence };
