const { unquantEndpoint } = require('./lib/astc-decode.cjs');
const { codeMap, candidateRecord, tabulate } = require('./lib/astc-endpoint-code.cjs');

const checks = [];

function check(name, ok, detail) {
    checks.push(ok);
    console.log((ok ? 'ok   ' : 'FAIL ') + name + (ok ? '' : ': ' + detail));
}

const RANGE = 19;
const byValue = codeMap(RANGE, 192);
const cpuCode = 50;
const adjacent = candidateRecord(unquantEndpoint(cpuCode, RANGE), unquantEndpoint(cpuCode + 1, RANGE), byValue);
check('a candidate record holds the CPU and GPU codes and the class', adjacent.cpuCode === cpuCode && adjacent.gpuCode === cpuCode + 1 && adjacent.class === 'adjacent code', JSON.stringify(adjacent));

let gap = -1;
for (let v = 0; v < 256; v += 1) {
    if (!byValue.has(v)) { gap = v; break; }
}
const noCode = candidateRecord(unquantEndpoint(cpuCode, RANGE), gap, byValue);
const far = candidateRecord(unquantEndpoint(cpuCode, RANGE), unquantEndpoint(cpuCode + 5, RANGE), byValue);
const records = [adjacent, noCode, far, candidateRecord(unquantEndpoint(150, RANGE), gap, byValue)];
const table = tabulate(records);
check('records tabulate by class and CPU trit digit', table.get('adjacent code | cpu trit digit 0') === 1 && table.get('gpu value not a code value | cpu trit digit 0') === 1 && table.get('other code | cpu trit digit 0') === 1 && table.get('gpu value not a code value | cpu trit digit 2') === 1, JSON.stringify([...table.entries()]));

const passed = checks.filter(Boolean).length;
console.log(passed === checks.length ? 'ALL PASS ' + passed + ' checks' : 'FAIL ' + (checks.length - passed) + ' of ' + checks.length);
process.exit(passed === checks.length ? 0 : 1);
