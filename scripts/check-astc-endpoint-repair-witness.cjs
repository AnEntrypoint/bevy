const { FORMULAS } = require('./lib/astc-weight-fit.cjs');
const { WEIGHT_LEVELS } = require('./lib/astc-decode.cjs');
const { classify, endpointRepairs } = require('./lib/astc-residual.cjs');

const checks = [];

function check(name, ok, detail) {
    checks.push(ok);
    console.log((ok ? 'ok   ' : 'FAIL ') + name + (ok ? '' : ': ' + detail));
}

const spec = FORMULAS.spec;
const table = WEIGHT_LEVELS[8];

function texelsAt(e0, e1, weight) {
    const texels = [];
    for (let r = 0; r < 16; r += 1) texels.push([0, 1, 2].map((c) => spec(e0[c], e1[c], weight)));
    return texels;
}

let seed = 4242;
function draw(count) {
    seed = (seed * 1103515245 + 12345) >>> 0;
    return (seed >>> 16) % count;
}

let jointOnly = 0;
let found = 0;
const missed = [];
for (let trial = 0; trial < 20000; trial += 1) {
    const cpu0 = [0, 1, 2].map(() => 1 + draw(254));
    const cpu1 = [0, 1, 2].map(() => 1 + draw(254));
    const c = draw(3);
    const end = draw(2);
    const delta = draw(2) === 0 ? -1 : 1;
    const gpu0 = cpu0.slice();
    const gpu1 = cpu1.slice();
    (end === 0 ? gpu0 : gpu1)[c] += delta;
    const weight = table[draw(table.length)];
    const result = classify(cpu0, cpu1, texelsAt(gpu0, gpu1, weight), spec);
    if (result.missing.length !== 0 || result.joint.length !== 0) continue;
    jointOnly += 1;
    const repairs = endpointRepairs(cpu0, cpu1, texelsAt(gpu0, gpu1, weight), spec);
    if (repairs.some((r) => r.channel === c && r.end === end && r.delta === delta)) found += 1;
    else if (missed.length < 3) missed.push({ cpu0, cpu1, c, end, delta, weight });
}
check('joint-only blocks with one planted endpoint error have that repair (' + found + ' of ' + jointOnly + ' joint-only trials)', jointOnly >= 20 && found === jointOnly, JSON.stringify({ jointOnly, found, missed }));

const flat = [100, 50, 20];
const flatTexels = texelsAt(flat, flat, table[0]).map((t) => [t[0], t[1], 21]);
const flatRepairs = endpointRepairs(flat, flat, flatTexels, spec);
check('an unreachable value in one channel is repaired by a one-level change to that channel', flatRepairs.some((r) => r.channel === 2 && r.delta === 1), JSON.stringify(flatRepairs));

const passed = checks.filter(Boolean).length;
console.log(passed === checks.length ? 'ALL PASS ' + passed + ' checks' : 'FAIL ' + (checks.length - passed) + ' of ' + checks.length);
process.exit(passed === checks.length ? 0 : 1);
