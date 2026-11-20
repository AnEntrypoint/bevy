const { unquantEndpoint } = require('./lib/astc-decode.cjs');
const { codeMap, classifyGpuValue } = require('./lib/astc-endpoint-code.cjs');

const checks = [];

function check(name, ok, detail) {
    checks.push(ok);
    console.log((ok ? 'ok   ' : 'FAIL ') + name + (ok ? '' : ': ' + detail));
}

function countBy(values) {
    const counts = {};
    for (const v of values) counts[v] = (counts[v] || 0) + 1;
    return JSON.stringify(counts);
}

const RANGE = 19;
const byValue = codeMap(RANGE, 192);
const cpuCode = 50;
const cpuValue = unquantEndpoint(cpuCode, RANGE);

const unproduced = [];
for (let v = 0; v < 256; v += 1) if (!byValue.has(v)) unproduced.push(v);
const unprodClasses = unproduced.map((v) => classifyGpuValue(cpuValue, v, byValue));
const wrongOnUnproduced = unprodClasses.filter((c) => c !== 'gpu value not a code value');
check('a GPU value no code produces is never reported as a code shift (wrong output: other code or adjacent code)', wrongOnUnproduced.length === 0 && unproduced.length === 64, 'wrong classes on ' + unproduced.length + ' unproduced values: ' + countBy(unprodClasses));

const codeValues = [...byValue.keys()].filter((v) => v !== cpuValue);
const codeClasses = codeValues.map((v) => classifyGpuValue(cpuValue, v, byValue));
const wrongOnCode = codeClasses.filter((c) => c === 'gpu value not a code value');
check('a GPU value that some code produces is never reported as not a code value (wrong output: gpu value not a code value)', wrongOnCode.length === 0 && codeValues.length === 191, 'wrong classes on ' + codeValues.length + ' code values: ' + countBy(codeClasses));

const passed = checks.filter(Boolean).length;
console.log(passed === checks.length ? 'ALL PASS ' + passed + ' checks' : 'FAIL ' + (checks.length - passed) + ' of ' + checks.length);
process.exit(passed === checks.length ? 0 : 1);
