#!/usr/bin/env node
const { parseTop, parsePs, classify } = require('./lib/heavy-jobs.cjs');

const HEADER = '  PID USER      PR  NI    VIRT    RES    SHR S  %CPU  %MEM     TIME+ COMMAND';
const GIB = 1048576;
const procs = [
    [301, 900000, '/config/.gmnew/versions/x/gm-core worker --handshake {"protocolVersion":"2025-11-25"}', 100, 100],
    [302, 900000, '/config/.gmnew/versions/x/gm-core worker --foo', 100, 100],
    [303, 1000, '/usr/lib/gcc/cc1plus -quiet -O2', 50, 50],
    [304, 1000, '/data/cache/rustup/toolchains/stable/bin/rustc --crate-name x', 50, 50],
    [305, 1000, '/usr/bin/cargo test --workspace', 50, 50],
    [306, 1000, '/tmp/zig-toolchain/zig test .zig-cache/x', 50, 50],
    [307, 1.5 * GIB, '/usr/lib/chromium/chromium --type=renderer --field-trial', 100, 100],
    [308, 0.5 * GIB, '/usr/lib/chromium/chromium --type=renderer --field-trial', 100, 100],
    [309, 1000, 'ninja -C build', 50, 50],
    [310, 1000, 'grep -rln ANTHROPIC .', 100, 100],
    [311, 1000, '/usr/bin/cargo build --jobs 2', 50, 50],
    [312, 1000, '/usr/lib/chromium/chromium.real --type=gpu-process --headless', 100, 100],
    [313, 1000, 'node --max-old-space-size=6144 .spawn/local/export-pages.mjs', 113, 113],
    [314, 1000, '/usr/lib/gcc/cc1plus -quiet -O2', 50, 0],
    [315, 1000, '/usr/lib/gcc/cc1plus -quiet -O2', 3, 3],
];
const frame = (cpuOf) => [HEADER, ...procs.map((p) => '  ' + p[0] + ' abc       20   0  1000  2000   100 R  ' + cpuOf(p).toFixed(1) + '   0.1   0:00.1 x')].join('\n');
const topText = [frame((p) => p[3]), frame((p) => p[4])].join('\n');
const psText = procs.map((p) => '  ' + p[0] + ' ' + p[1] + ' ' + p[2]).join('\n');
const found = classify(parsePs(psText), parseTop(topText));
const kinds = (pid) => found.filter((item) => item.pid === pid).map((item) => item.kind);
const cases = [
    ['parseTop keeps one frame per PID header', () => parseTop(topText).length === 2],
    ['a gm-core worker with --handshake at 100% in every frame is not busy', () => kinds(301).length === 0],
    ['a gm-core worker without --handshake at 100% in every frame is not busy', () => kinds(302).length === 0],
    ['cc1plus above the floor in every frame is compile', () => JSON.stringify(kinds(303)) === '["compile"]'],
    ['rustc above the floor in every frame is compile', () => JSON.stringify(kinds(304)) === '["compile"]'],
    ['cargo test above the floor in every frame is test-check', () => JSON.stringify(kinds(305)) === '["test-check"]'],
    ['zig test above the floor in every frame is compile', () => JSON.stringify(kinds(306)) === '["compile"]'],
    ['a Chromium renderer over 1 GiB at 100% is not busy', () => kinds(307).length === 0],
    ['a Chromium renderer under 1 GiB at 100% is not busy', () => kinds(308).length === 0],
    ['ninja above the floor in every frame is compile', () => JSON.stringify(kinds(309)) === '["compile"]'],
    ['grep at 100% in every frame is not busy', () => kinds(310).length === 0],
    ['cargo build above the floor in every frame is compile', () => JSON.stringify(kinds(311)) === '["compile"]'],
    ['a Chromium GPU process at 100% in every frame is not busy', () => kinds(312).length === 0],
    ['a node process above 100% in every frame is not busy', () => kinds(313).length === 0],
    ['a compiler above the floor in one frame only is not busy', () => kinds(314).length === 0],
    ['a compiler below the floor in every frame is not busy', () => kinds(315).length === 0],
    ['a busy process is listed once', () => found.filter((item) => item.pid === 303).length === 1],
    ['an empty process list is clear', () => classify([], []).length === 0],
];
let fail = 0;
for (const [name, run] of cases) {
    const ok = run();
    console.log((ok ? 'ok   ' : 'FAIL ') + name);
    if (!ok) fail = 1;
}
process.exit(fail);
