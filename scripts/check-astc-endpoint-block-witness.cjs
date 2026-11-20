const { BISE_RANGES, decodeAstc4x4Block, readBits } = require('./lib/astc-decode.cjs');
const probe = require('./lib/astc-probe.cjs');
const { COMPONENT_COUNT_BY_CEM, CEM_POSITION, ENDPOINT_POSITION, buildEndpointBlock, expectedEndpoints, endpointRangeFor } = require('./lib/astc-endpoint-block.cjs');

const checks = [];

function check(name, ok, detail) {
    checks.push(ok);
    console.log((ok ? 'ok   ' : 'FAIL ') + name + (ok ? '' : ': ' + detail));
}

let seed = 11;
function next(limit) {
    seed = (Math.imul(seed, 1103515245) + 12345) >>> 0;
    return seed % limit;
}

const hand = endpointRangeFor(8);
check('a CEM 8 block has six endpoint values and the range the decoder picks for 47 bits (wrong output: a range without the trit tail)', hand.count === 6 && hand.range === 19, 'count ' + hand.count + ' range ' + hand.range);

const sample = buildEndpointBlock(8, [10, 20, 30, 40, 50, 60], 0).block;
check('the CEM field sits at bits 13 to 16 and the mode field reads 0x242 (wrong output: a CEM of 0 or a different mode)', readBits(sample, CEM_POSITION, 4) === 8 && (sample[0] | (sample[1] << 8)) === 0x242, 'cem ' + readBits(sample, CEM_POSITION, 4) + ' mode 0x' + (sample[0] | (sample[1] << 8)).toString(16));

let mismatches = 0;
let trials = 0;
let firstFailure = '';
for (const cem of [8, 12]) {
    const components = COMPONENT_COUNT_BY_CEM[cem];
    const count = 2 * components;
    for (let t = 0; t < 20; t += 1) {
        const { range } = endpointRangeFor(cem);
        const [bits, trits, quints] = BISE_RANGES[range];
        const digitLimit = trits ? 3 : quints ? 5 : 1;
        const values = [];
        for (let i = 0; i < count; i += 1) values.push((next(digitLimit) << bits) | next(1 << bits));
        for (const weightIndex of [0, 15]) {
            const { block } = buildEndpointBlock(cem, values, weightIndex);
            const texel = decodeAstc4x4Block(block).subarray(0, components === 2 ? 4 : components === 3 ? 3 : 4);
            const end = weightIndex === 0 ? 0 : 1;
            const expected = expectedEndpoints(values, range, end);
            const got = components === 2 ? [texel[0], texel[3]] : Array.from(texel.subarray(0, components));
            const want = components === 2 ? [expected[0], expected[1]] : expected;
            trials += 1;
            if (got.join(',') !== want.join(',')) {
                mismatches += 1;
                if (!firstFailure) firstFailure = 'cem ' + cem + ' weight ' + weightIndex + ' decoded ' + got.join(',') + ' expected ' + want.join(',');
            }
        }
    }
}
check('the decoded endpoint of each channel matches the written value at weight 0 and at weight 64 (wrong output: an endpoint shifted by a bit or a value from the other end)', mismatches === 0, mismatches + ' of ' + trials + ' ; first ' + firstFailure);

const passed = checks.filter(Boolean).length;
console.log(passed === checks.length ? 'ALL PASS ' + passed + ' checks' : 'FAIL ' + (checks.length - passed) + ' of ' + checks.length);
process.exit(passed === checks.length ? 0 : 1);
