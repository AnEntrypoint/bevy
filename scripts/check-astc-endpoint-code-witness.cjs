const { unquantEndpoint } = require('./lib/astc-decode.cjs');
const { codeMap, classifyGpuValue } = require('./lib/astc-endpoint-code.cjs');

const checks = [];

function check(name, ok, detail) {
    checks.push(ok);
    console.log((ok ? 'ok   ' : 'FAIL ') + name + (ok ? '' : ': ' + detail));
}

const RANGE = 19;
const CODES = 192;
const byValue = codeMap(RANGE, CODES);

check('range 19 has 192 codes with 192 distinct values', byValue.size === CODES, 'distinct ' + byValue.size);

const cpuCode = 50;
const cpuValue = unquantEndpoint(cpuCode, RANGE);
const nextValue = unquantEndpoint(cpuCode + 1, RANGE);
check('a GPU value one code up is classified as an adjacent code', classifyGpuValue(cpuValue, nextValue, byValue) === 'adjacent code', classifyGpuValue(cpuValue, nextValue, byValue));

const farValue = unquantEndpoint(cpuCode + 5, RANGE);
check('a GPU value five codes up is classified as another code', classifyGpuValue(cpuValue, farValue, byValue) === 'other code', classifyGpuValue(cpuValue, farValue, byValue));

let gap = -1;
for (let v = 0; v < 256; v += 1) {
    if (!byValue.has(v)) { gap = v; break; }
}
check('a value that no code produces is classified as not a code value', gap >= 0 && classifyGpuValue(cpuValue, gap, byValue) === 'gpu value not a code value', 'gap ' + gap);

const passed = checks.filter(Boolean).length;
console.log(passed === checks.length ? 'ALL PASS ' + passed + ' checks' : 'FAIL ' + (checks.length - passed) + ' of ' + checks.length);
process.exit(passed === checks.length ? 0 : 1);
