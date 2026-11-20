const { FORMULAS } = require('./lib/astc-weight-fit.cjs');
const { WEIGHT_LEVELS } = require('./lib/astc-decode.cjs');
const { channelFits, classify } = require('./lib/astc-residual.cjs');

const checks = [];

function check(name, ok, detail) {
    checks.push(ok);
    console.log((ok ? 'ok   ' : 'FAIL ') + name + (ok ? '' : ': ' + detail));
}

const spec = FORMULAS.spec;
const table = WEIGHT_LEVELS[8];

function texelsAt(e0, e1, w) {
    const texels = [];
    for (let r = 0; r < 16; r += 1) texels.push([0, 1, 2].map((c) => spec(e0[c], e1[c], w)));
    return texels;
}

const e0 = [30, 200, 90];
const e1 = [220, 40, 160];
const w = table[6];
const built = classify(e0, e1, texelsAt(e0, e1, w), spec);
check('a block built at a table weight has no missing channel and fits that weight', built.missing.length === 0 && built.joint.includes(w), JSON.stringify(built));

const flat = [100, 50, 20];
const flatTexels = texelsAt(flat, flat, 0).map((t) => [t[0], t[1], 21]);
const flatResult = classify(flat, flat, flatTexels, spec);
check('an unreachable value in one channel with equal endpoints is reported as that channel missing', JSON.stringify(flatResult.missing) === '[2]' && flatResult.joint.length === 0, JSON.stringify(flatResult));

const low = [0, 0, 0];
const high = [255, 255, 255];
const at10 = texelsAt(low, high, 10);
const at40 = texelsAt(low, high, 40);
const split = at10.map((t, i) => [t[0], at40[i][1], t[2]]);
const split0 = channelFits(low, high, split, spec, 0);
const split1 = channelFits(low, high, split, spec, 1);
const split2 = channelFits(low, high, split, spec, 2);
const splitResult = classify(low, high, split, spec);
check('each channel fits alone but no weight fits all three (joint only)', splitResult.missing.length === 0 && splitResult.joint.length === 0 && split0.length > 0 && split1.length > 0 && split2.length > 0, JSON.stringify({ splitResult, split0, split1, split2 }));

const passed = checks.filter(Boolean).length;
console.log(passed === checks.length ? 'ALL PASS ' + passed + ' checks' : 'FAIL ' + (checks.length - passed) + ' of ' + checks.length);
process.exit(passed === checks.length ? 0 : 1);
