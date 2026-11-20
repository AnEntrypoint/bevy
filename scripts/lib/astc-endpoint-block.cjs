const { chooseEndpointRange, integerSequenceBits, unquantEndpoint } = require('./astc-decode.cjs');
const { encodeIntegerSequence } = require('./astc-encode-sequence.cjs');
const probe = require('./astc-probe.cjs');

const COMPONENT_COUNT_BY_CEM = { 4: 2, 8: 3, 12: 4 };
const CEM_POSITION = 13;
const ENDPOINT_POSITION = 17;
const WEIGHT_BITS_TOTAL = 16 * 4;

function setBlockBits(block, position, bits) {
    bits.forEach((bit, i) => {
        const at = position + i;
        const mask = 1 << (at & 7);
        if (bit) block[at >> 3] |= mask;
        else block[at >> 3] &= ~mask;
    });
}

function bitsOf(bytes, length) {
    const bits = [];
    for (let i = 0; i < length; i += 1) bits.push((bytes[i >> 3] >> (i & 7)) & 1);
    return bits;
}

function endpointRangeFor(cem) {
    const count = 2 * COMPONENT_COUNT_BY_CEM[cem];
    const available = 128 - WEIGHT_BITS_TOTAL - ENDPOINT_POSITION;
    return { count, range: chooseEndpointRange(count, available) };
}

function buildEndpointBlock(cem, values, weightIndex) {
    const { count, range } = endpointRangeFor(cem);
    if (values.length !== count) throw new Error('expected ' + count + ' endpoint values, got ' + values.length);
    const block = new Uint8Array(16);
    block[0] = probe.WEIGHT_MODE & 0xff;
    block[1] = (probe.WEIGHT_MODE >> 8) & 0xff;
    const cemBits = [];
    for (let i = 0; i < 4; i += 1) cemBits.push((cem >> i) & 1);
    setBlockBits(block, CEM_POSITION, cemBits);
    const encoded = encodeIntegerSequence(values, range);
    if (encoded.bitLength !== integerSequenceBits(range, count)) throw new Error('endpoint bit count differs from the decoder');
    setBlockBits(block, ENDPOINT_POSITION, bitsOf(encoded.bytes, encoded.bitLength));
    return { block: probe.withWeightIndex(block, weightIndex, probe.WEIGHT_MODE), range, count };
}

function expectedEndpoints(values, range, end) {
    const out = [];
    for (let i = end; i < values.length; i += 2) out.push(unquantEndpoint(values[i], range));
    return out;
}

module.exports = { COMPONENT_COUNT_BY_CEM, ENDPOINT_POSITION, CEM_POSITION, buildEndpointBlock, expectedEndpoints, endpointRangeFor };
