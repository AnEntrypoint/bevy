const { decodeAstc4x4Block, WEIGHT_LEVELS, WEIGHT_RANGE_BY_FIELD } = require('./lib/astc-decode.cjs');
const probe = require('./lib/astc-probe.cjs');
const { buildEndpointBlock, expectedEndpoints } = require('./lib/astc-endpoint-block.cjs');
const { FORMULAS } = require('./lib/astc-interp.cjs');

const checks = [];

function check(name, ok, detail) {
    checks.push(ok);
    console.log((ok ? 'ok   ' : 'FAIL ') + name + (ok ? '' : ': ' + detail));
}

check('the three formulas give 2, 1 and 2 for low 0, high 3, weight 32 (wrong output: the spec value 1 or the floor value 2)', FORMULAS.spec(0, 3, 32) === 2 && FORMULAS.floor(0, 3, 32) === 1 && FORMULAS.even(0, 3, 32) === 2, [FORMULAS.spec(0, 3, 32), FORMULAS.floor(0, 3, 32), FORMULAS.even(0, 3, 32)].join(','));

const levels = WEIGHT_LEVELS[WEIGHT_RANGE_BY_FIELD[probe.WEIGHT_MODE]];
let differFloor = 0;
let differEven = 0;
for (const w of levels) {
    for (let low = 0; low < 256; low += 1) {
        for (let high = 0; high < 256; high += 1) {
            if (FORMULAS.spec(low, high, w) !== FORMULAS.floor(low, high, w)) differFloor += 1;
            if (FORMULAS.spec(low, high, w) !== FORMULAS.even(low, high, w)) differEven += 1;
        }
    }
}
check('the formulas differ on real inputs, so a measurement can tell them apart (wrong output: no difference from the spec)', differFloor > 0 && differEven > 0, 'spec-floor ' + differFloor + ' spec-even ' + differEven);

let seed = 23;
function next(limit) {
    seed = (Math.imul(seed, 1103515245) + 12345) >>> 0;
    return seed % limit;
}

let mismatches = 0;
let trials = 0;
let firstFailure = '';
for (let t = 0; t < 40; t += 1) {
    const values = [];
    for (let i = 0; i < 6; i += 1) values.push(next(192));
    const k = 1 + next(levels.length - 2);
    const { block, range } = buildEndpointBlock(8, values, k);
    const texel = decodeAstc4x4Block(block);
    const low = expectedEndpoints(values, range, 0);
    const high = expectedEndpoints(values, range, 1);
    for (let c = 0; c < 3; c += 1) {
        trials += 1;
        const want = FORMULAS.spec(low[c], high[c], levels[k]);
        if (texel[c] !== want) {
            mismatches += 1;
            if (!firstFailure) firstFailure = 'k ' + k + ' channel ' + c + ' decoded ' + texel[c] + ' spec ' + want;
        }
    }
}
check('the decoder interpolates with the spec formula at every mid weight (wrong output: a value from the floor or even formula)', mismatches === 0, mismatches + ' of ' + trials + '; first ' + firstFailure);

const passed = checks.filter(Boolean).length;
console.log(passed === checks.length ? 'ALL PASS ' + passed + ' checks' : 'FAIL ' + (checks.length - passed) + ' of ' + checks.length);
process.exit(passed === checks.length ? 0 : 1);
