const { BISE_RANGES, WEIGHT_RANGE_BY_FIELD, reverseBits } = require('./astc-decode.cjs');

const WEIGHT_MODE = 0x242;
const TEXELS_PER_BLOCK = 16;
const BLOCK_BYTES = 16;

function setBits(bytes, position, count, value) {
    for (let i = 0; i < count; i += 1) {
        const bit = position + i;
        const mask = 1 << (bit & 7);
        if ((value >> i) & 1) {
            bytes[bit >> 3] |= mask;
        } else {
            bytes[bit >> 3] &= ~mask;
        }
    }
}

function weightLevelBits(blockMode) {
    return BISE_RANGES[WEIGHT_RANGE_BY_FIELD[blockMode]][0];
}

function weightIndexLevels(blockMode) {
    return 1 << weightLevelBits(blockMode);
}

function withWeightIndex(block, index, blockMode) {
    const bits = weightLevelBits(blockMode);
    const out = Uint8Array.from(block);
    for (let i = 0; i < TEXELS_PER_BLOCK; i += 1) {
        setBits(out, BLOCK_BYTES * 8 - bits * (i + 1), bits, reverseBits(index, bits));
    }
    return out;
}

function buildFixtures(blocks, blockMode) {
    const levels = weightIndexLevels(blockMode);
    const fixtures = blocks.map((block, i) => withWeightIndex(block, i % levels, blockMode));
    const blob = Buffer.alloc(fixtures.length * BLOCK_BYTES);
    fixtures.forEach((fixture, i) => blob.set(fixture, i * BLOCK_BYTES));
    return { blob, fixtures, levels };
}

module.exports = { WEIGHT_MODE, setBits, weightLevelBits, weightIndexLevels, withWeightIndex, buildFixtures };
