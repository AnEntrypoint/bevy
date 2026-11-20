const { parseTop, parsePs, classify } = require('./lib/heavy-jobs.cjs');

const checks = [];

function check(name, ok, detail) {
    checks.push(ok);
    console.log((ok ? 'ok   ' : 'FAIL ') + name + (ok ? '' : ': ' + detail));
}

const GIB = 1048576;
const ps = parsePs([
    '10 0 /usr/bin/zig build-exe x',
    '11 0 node scripts/x.cjs',
    '12 ' + (2 * GIB) + ' /usr/lib/chromium/chromium.real --type=renderer --x',
    '13 0 /usr/bin/cmake --build .',
    '14 0 some-app --run',
    '15 0 hot-app --run',
    '16 0 cargo test --workspace',
    '17 0 cargo test --workspace',
].join('\n'));

const header = '  PID USER      PR  NI    VIRT    RES    SHR S  %CPU  %MEM     TIME+ COMMAND';
const rowsA = [
    '   10 abc       20   0  1000  100  10 S   0.2   0.1   0:01 zig',
    '   11 abc       20   0  1000  100  10 S  50.0   0.1   0:01 node',
    '   12 abc       20   0  1000  100  10 S   0.0   0.1   0:01 chromium',
    '   13 abc       20   0  1000  100  10 S  50.0   0.1   0:01 cmake',
    '   14 abc       20   0  1000  100  10 S  60.0   0.1   0:01 some-app',
    '   15 abc       20   0  1000  100  10 S  60.0   0.1   0:01 hot-app',
    '   16 abc       20   0  1000  100  10 S  30.0   0.1   0:01 cargo',
    '   17 abc       20   0  1000  100  10 S   0.0   0.1   0:01 cargo',
];
const rowsB = [
    '   10 abc       20   0  1000  100  10 S   0.1   0.1   0:01 zig',
    '   11 abc       20   0  1000  100  10 S   0.0   0.1   0:01 node',
    '   12 abc       20   0  1000  100  10 S   0.0   0.1   0:01 chromium',
    '   13 abc       20   0  1000  100  10 S  50.0   0.1   0:01 cmake',
    '   15 abc       20   0  1000  100  10 S  70.0   0.1   0:01 hot-app',
    '   16 abc       20   0  1000  100  10 S  30.0   0.1   0:01 cargo',
    '   17 abc       20   0  1000  100  10 S   0.0   0.1   0:01 cargo',
];
const frameA = parseTop([header, ...rowsA].join('\n'));
const topText = [header, ...rowsA, header, ...rowsB].join('\n');
const frames = parseTop(topText);
check('parseTop keeps every frame in order (wrong output: only the last frame)', frameA.length === 1 && frameA[0].length === 8 && parseTop('PID x\n 1 a 2 3 4 5 6 7 8.5 b\nPID x\n 2 a 2 3 4 5 6 7 9.5 b').length === 2, 'frames ' + parseTop('PID x\n 1 a 2 3 4 5 6 7 8.5 b\nPID x\n 2 a 2 3 4 5 6 7 9.5 b').length);

const found = classify(ps, frames);
const kinds = new Map(found.map((item) => [item.pid, item.kind]));
check('a node process above 20% in one frame only is not busy (wrong output: hot for pid 11)', !kinds.has(11), 'pid 11 ' + kinds.get(11));
check('an app above 20% in one frame only is not busy (wrong output: hot for pid 14)', !kinds.has(14), 'pid 14 ' + kinds.get(14));
check('an idle zig build is not reported as compiling (wrong output: compile for pid 10)', !kinds.has(10), 'pid 10 ' + kinds.get(10));
check('a cmake above the compile floor in both frames is reported as compile (wrong output: no match)', kinds.get(13) === 'compile', 'pid 13 ' + kinds.get(13));
check('an idle renderer above 1 GiB is not busy (wrong output: chromium-renderer for pid 12)', !kinds.has(12), 'pid 12 ' + kinds.get(12));
check('a process above 20% in both frames is hot (wrong output: no match for pid 15)', kinds.get(15) === 'hot', 'pid 15 ' + kinds.get(15));
check('an idle cargo test is not reported (wrong output: test-check for pid 17)', !kinds.has(17), 'pid 17 ' + kinds.get(17));
check('a cargo test above the compile floor in both frames is reported (wrong output: no match for pid 16)', kinds.get(16) === 'test-check', 'pid 16 ' + kinds.get(16));

const passed = checks.filter(Boolean).length;
console.log(passed === checks.length ? 'ALL PASS ' + passed + ' checks' : 'FAIL ' + (checks.length - passed) + ' of ' + checks.length);
process.exit(passed === checks.length ? 0 : 1);
