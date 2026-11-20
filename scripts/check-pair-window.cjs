#!/usr/bin/env node
const { spawnSync } = require('node:child_process');
const path = require('node:path');
const { PAIR_WINDOW_S } = require('./lib/pair-budget.cjs');

const repo = path.join(__dirname, '..');
const wrapper = path.join(__dirname, 'pair-window.sh');

function printEnv(format, extraEnv) {
  const env = Object.assign({}, process.env, extraEnv || {});
  delete env.PAIR_FORMAT_BUDGET_S;
  Object.assign(env, extraEnv || {});
  const result = spawnSync('bash', [wrapper, format, '--print-env'], { cwd: repo, env, encoding: 'utf8' });
  const values = {};
  for (const line of (result.stdout || '').split(String.fromCharCode(10))) {
    const match = /^([A-Z_]+)=(\d+)$/.exec(line.trim());
    if (match) values[match[1]] = Number(match[2]);
  }
  return { status: result.status, values, stdout: result.stdout || '' };
}

const etc2 = printEnv('etc2');
const astc = printEnv('astc4x4');
const wide = printEnv('etc2', { PAIR_FORMAT_BUDGET_S: '500' });
const bad = printEnv('bc7');

const cases = [
  { name: 'the window for ETC2 is the library constant (1200 s)', ok: etc2.values.WINDOW_S === PAIR_WINDOW_S && PAIR_WINDOW_S === 1200 },
  { name: 'the start threshold for ETC2 is the full pair budget (720 s)', ok: etc2.values.MIN_REMAINING_S === 720 },
  { name: 'the latest start for ETC2 is 480 s into the window', ok: etc2.values.WINDOW_S - etc2.values.MIN_REMAINING_S === 480 },
  { name: 'ASTC gets the same window and threshold as ETC2', ok: astc.values.WINDOW_S === etc2.values.WINDOW_S && astc.values.MIN_REMAINING_S === etc2.values.MIN_REMAINING_S },
  { name: 'a larger format budget raises the start threshold (500 s arm gives 860 s)', ok: wide.values.MIN_REMAINING_S === 860 && wide.values.WINDOW_S === 1200 },
  { name: 'an unknown format exits 2 and prints no values', ok: bad.status === 2 && Object.keys(bad.values).length === 0 },
];

let fail = 0;
for (const testCase of cases) {
  console.log((testCase.ok ? 'ok   ' : 'FAIL ') + testCase.name);
  if (!testCase.ok) fail = 1;
}
process.exit(fail);
