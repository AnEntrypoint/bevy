const { BISE_RANGES, decodeAstc4x4Block, integerSequenceBits, chooseEndpointRange, unquantEndpoint, astcTexelPartition } = require('./lib/astc-decode.cjs');
const { COMPONENT_COUNT_BY_CEM, TWO_SUBSET_POSITION, buildTwoSubsetBlock } = require('./lib/astc-endpoint-block.cjs');

const checks = [];

function check(name, ok, detail) {
    checks.push(ok);
    console.log((ok ? 'ok   ' : 'FAIL ') + name + (ok ? '' : ': ' + detail));
}

const available = 128 - 64 - TWO_SUBSET_POSITION;
const range12 = chooseEndpointRange(12, available);
check('a CEM 8 two-subset block has 12 endpoint values in a trit range, so its tail is two values (wrong output: a range with no trit tail)', BISE_RANGES[range12][1] === 1 && integerSequenceBits(range12, 12) <= available, 'range ' + range12 + ' trits ' + BISE_RANGES[range12][1]);

let seed = 31;
function next(limit) {
    seed = (Math.imul(seed, 1103515245) + 12345) >>> 0;
    return seed % limit;
}

const perCem = {};
for (const cem of [4, 8, 12]) {
    const components = COMPONENT_COUNT_BY_CEM[cem];
    const count = 4 * components;
    const range = chooseEndpointRange(count, available);
    const [bits, trits, quints] = BISE_RANGES[range];
    const digitLimit = trits ? 3 : quints ? 5 : 1;
    let mismatches = 0;
    let texels = 0;
    for (let t = 0; t < 20; t += 1) {
        const values = [];
        for (let i = 0; i < count; i += 1) values.push((next(digitLimit) << bits) | next(1 << bits));
        const subsetSeed = next(1024);
        const weightIndex = next(2) ? 15 : 0;
        const end = weightIndex === 0 ? 0 : 1;
        const { block } = buildTwoSubsetBlock(cem, subsetSeed, values, weightIndex);
        const decoded = decodeAstc4x4Block(block);
        for (let x = 0; x < 16; x += 1) {
            const subset = astcTexelPartition(subsetSeed, x & 3, x >> 2, 2);
            const base = subset * 2 * components;
            const e = [];
            for (let c = 0; c < components; c += 1) e.push(unquantEndpoint(values[base + 2 * c + end], range));
            const want = components === 3 ? [e[0], e[1], e[2], 255] : components === 2 ? [e[0], e[0], e[0], e[1]] : e;
            texels += 1;
            if (Array.from(decoded.subarray(x * 4, x * 4 + 4)).join(',') !== want.join(',')) mismatches += 1;
        }
    }
    perCem[cem] = { range, mismatches, texels };
}
check('each texel of a CEM 8 two-subset block decodes to its own subset endpoint (wrong output: the other subset or a single-subset layout)', perCem[8].mismatches === 0, JSON.stringify(perCem[8]));
check('each texel of a CEM 4 two-subset block decodes to its own subset endpoint (wrong output: the other subset or a shifted endpoint)', perCem[4].mismatches === 0, JSON.stringify(perCem[4]));
check('each texel of a CEM 12 two-subset block decodes to its own subset endpoint (wrong output: the other subset or a shifted endpoint)', perCem[12].mismatches === 0, JSON.stringify(perCem[12]));

const passed = checks.filter(Boolean).length;
console.log(passed === checks.length ? 'ALL PASS ' + passed + ' checks' : 'FAIL ' + (checks.length - passed) + ' of ' + checks.length);
process.exit(passed === checks.length ? 0 : 1);
