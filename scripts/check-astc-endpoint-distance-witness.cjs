const { unquantEndpoint } = require('./lib/astc-decode.cjs');
const { codeMap, candidateRecord } = require('./lib/astc-endpoint-code.cjs');
const { nearestCodeDistance, tabulateNoCodeDistances } = require('./lib/astc-endpoint-distance.cjs');

const checks = [];

function check(name, ok, detail) {
    checks.push(ok);
    console.log((ok ? 'ok   ' : 'FAIL ') + name + (ok ? '' : ': ' + detail));
}

const RANGE = 19;
const byValue = codeMap(RANGE, 192);
const codeValues = [...byValue.keys()];

check('a code value is at distance 0 from the code table', nearestCodeDistance(unquantEndpoint(50, RANGE), codeValues) === 0, String(nearestCodeDistance(unquantEndpoint(50, RANGE), codeValues)));

let gap = -1;
for (let v = 0; v < 256; v += 1) {
    if (!byValue.has(v)) { gap = v; break; }
}
const expected = Math.min(...codeValues.map((v) => Math.abs(gap - v)));
check('a value no code produces is at the distance of its nearest code value', nearestCodeDistance(gap, codeValues) === expected, nearestCodeDistance(gap, codeValues) + ' vs ' + expected);

const top = codeValues.reduce((a, b) => (a > b ? a : b));
check('a value above the top code value is measured against the top code value', nearestCodeDistance(top + 3, codeValues) === 3, String(nearestCodeDistance(top + 3, codeValues)));

const records = [
    candidateRecord(unquantEndpoint(50, RANGE), gap, byValue),
    candidateRecord(unquantEndpoint(50, RANGE), unquantEndpoint(51, RANGE), byValue),
];
const table = tabulateNoCodeDistances(records, codeValues);
check('only no-code candidates enter the distance table', table.size === 1 && table.get('distance ' + expected) === 1, JSON.stringify([...table.entries()]));

const passed = checks.filter(Boolean).length;
console.log(passed === checks.length ? 'ALL PASS ' + passed + ' checks' : 'FAIL ' + (checks.length - passed) + ' of ' + checks.length);
process.exit(passed === checks.length ? 0 : 1);
