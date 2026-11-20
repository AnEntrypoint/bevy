const { unquantEndpoint } = require('./lib/astc-decode.cjs');
const { unquantWithConstant, constantsExplaining } = require('./lib/astc-endpoint-constant.cjs');

const checks = [];

function check(name, ok, detail) {
    checks.push(ok);
    console.log((ok ? 'ok   ' : 'FAIL ') + name + (ok ? '' : ': ' + detail));
}

let mismatches = 0;
for (let code = 0; code < 192; code += 1) {
    if (unquantWithConstant(code, 5) !== unquantEndpoint(code, 19)) mismatches += 1;
}
check('the constant formula with c = 5 matches the decoder for all 192 range-19 codes', mismatches === 0, mismatches + ' codes differ');

const PLANTED = 9;
const entries = [];
for (let code = 0; code < 192; code += 7) entries.push({ code, gpuValue: unquantWithConstant(code, PLANTED) });
const found = constantsExplaining(entries, 255);
check('a planted constant is recovered from entries generated with it', found.includes(PLANTED), 'found ' + JSON.stringify(found));

const passed = checks.filter(Boolean).length;
console.log(passed === checks.length ? 'ALL PASS ' + passed + ' checks' : 'FAIL ' + (checks.length - passed) + ' of ' + checks.length);
process.exit(passed === checks.length ? 0 : 1);
