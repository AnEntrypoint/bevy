const { TRIT_CODES, QUINT_CODES } = require('./lib/astc-decode.cjs');

const checks = [];

function check(name, ok, detail) {
    checks.push(ok);
    console.log((ok ? 'ok   ' : 'FAIL ') + name + (ok ? '' : ': ' + detail));
}

function distinctWithin(table, max) {
    return new Set(table).size === table.length && table.every((v) => Number.isInteger(v) && v >= 0 && v < max);
}

check('trit table holds 243 distinct 8-bit codes', TRIT_CODES.length === 243 && distinctWithin(TRIT_CODES, 256), 'entries ' + TRIT_CODES.length + ', distinct ' + new Set(TRIT_CODES).size);
check('quint table holds 125 distinct 7-bit codes', QUINT_CODES.length === 125 && distinctWithin(QUINT_CODES, 128), 'entries ' + QUINT_CODES.length + ', distinct ' + new Set(QUINT_CODES).size);

const passed = checks.filter(Boolean).length;
console.log(passed === checks.length ? 'ALL PASS ' + passed + ' checks' : 'FAIL ' + (checks.length - passed) + ' of ' + checks.length);
process.exit(passed === checks.length ? 0 : 1);
