#!/usr/bin/env node
const { parseTop, parsePs, classify, CPU_LIMIT } = require('./lib/heavy-jobs.cjs');

const topFixture = `top - 00:00:01 up 1 day
Tasks: 3 total
  PID USER      PR  NI    VIRT    RES    SHR S  %CPU  %MEM     TIME+ COMMAND
  101 abc       20   0  1000  2000   100 S   5.0   0.1   0:00.1 bash

top - 00:00:03 up 1 day
  PID USER      PR  NI    VIRT    RES    SHR S  %CPU  %MEM     TIME+ COMMAND
  201 abc       20   0  1000  2000   100 R  100.0   0.1   0:00.1 gm-core
  202 abc       20   0  1000  2000   100 R   25.0   0.1   0:00.1 grep
  203 abc       20   0  1000  2000   100 R   20.0   0.1   0:00.1 quiet
  `;
const psFixture = `  301 900000 /config/.gmnew/versions/x/gm-core worker --handshake {"protocolVersion":"2025-11-25"}
  302 900000 /config/.gmnew/versions/x/gm-core worker --foo
  303 1000 /usr/lib/gcc/cc1plus -quiet -O2
  304 1000 /data/cache/rustup/toolchains/stable/bin/rustc --crate-name x
  305 1000 /usr/bin/cargo test --workspace
  306 1000 /tmp/zig-toolchain/zig test .zig-cache/x
  307 1500000 /usr/lib/chromium/chromium --type=renderer --field-trial
  308 500000 /usr/lib/chromium/chromium --type=renderer --field-trial
  309 1000 sleep 5
  310 1000 grep -rln ANTHROPIC .
  311 1000 /usr/bin/cargo build --jobs 2
  201 900000 /config/.gmnew/versions/x/gm-core worker --foo
  202 1000 grep -rln ANTHROPIC .
  203 1000 sleep 5
  `;

const topRows = parseTop(topFixture);
const psRows = parsePs(psFixture);
const found = classify(psRows, topRows);
const kinds = (pid) => found.filter((item) => item.pid === pid).map((item) => item.kind);
const cases = [
    { name: 'the parser reads only the last top frame', run: () => JSON.stringify(topRows.map((row) => row.pid)) === JSON.stringify([201, 202, 203]) },
    { name: 'a gm-core worker with --handshake is exempt even at 100% CPU', run: () => kinds(301).length === 0 },
    { name: 'a gm-core worker without --handshake counts above the limit', run: () => JSON.stringify(kinds(201)) === '["hot"]' },
    { name: 'a C compiler counts as compile', run: () => JSON.stringify(kinds(303)) === '["compile"]' },
    { name: 'rustc counts as compile', run: () => JSON.stringify(kinds(304)) === '["compile"]' },
    { name: 'cargo test counts as test-check', run: () => JSON.stringify(kinds(305)) === '["test-check"]' },
    { name: 'cargo build is a compile', run: () => JSON.stringify(kinds(311)) === '["compile"]' },
    { name: 'a zig test process counts as compile', run: () => JSON.stringify(kinds(306)) === '["compile"]' },
    { name: 'a Chromium renderer over 1 GB counts', run: () => JSON.stringify(kinds(307)) === '["chromium-renderer"]' },
    { name: 'a Chromium renderer under 1 GB does not count', run: () => kinds(308).length === 0 },
    { name: 'a 25% process not in the compiler list counts as hot', run: () => JSON.stringify(kinds(202)) === '["hot"]' },
    { name: 'exactly at the 20% limit does not count', run: () => kinds(203).length === 0 && CPU_LIMIT === 20 },
    { name: 'a process above the limit that is also a compiler is listed once', run: () => found.filter((item) => item.pid === 306).length === 1 },
    { name: 'an empty process list is clear', run: () => classify([], []).length === 0 },
];
let fail = 0;
for (const testCase of cases) {
    const ok = testCase.run();
    console.log(`${ok ? 'ok  ' : 'FAIL'} ${testCase.name}`);
    if (!ok) fail = 1;
}
process.exit(fail);
