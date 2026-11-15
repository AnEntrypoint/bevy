#!/usr/bin/env node
const { spawnSync } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const repo = path.resolve(__dirname, '..');
const game = process.argv[2] || '/workspace/extra/game-16';
const bin = process.argv[3] || path.join(repo, 'target/debug/bevy-jayzee');
const places = ['hall', 'main'];
const reports = {};
let failed = false;

function extract(root, dir = root) {
  const found = new Set();
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (entry.name === '.git') continue;
    const file = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      for (const value of extract(root, file)) found.add(value);
      continue;
    }
    if (!/\.(js|yaml|yml|json|md)$/.test(entry.name)) continue;
    const source = fs.readFileSync(file, 'utf8');
    for (const match of source.matchAll(/\bctx(?:\.[A-Za-z_$][A-Za-z0-9_$]*)+/g)) {
      found.add(match[0].replace(/\.$/, ''));
    }
  }
  return found;
}

const usage = extract(game);
const reportLines = {};
for (const place of places) {
  const run = spawnSync(bin, ['world', game, place, '120'], { cwd: repo, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
  let report = null;
  let scriptStats = null;
  for (const line of (run.stdout || '').split('\n')) {
    if (!line.startsWith('{')) continue;
    let data;
    try { data = JSON.parse(line); } catch { continue; }
    if (data.kind === 'game16-report') report = data.report;
    if (data.kind === 'script-stats') scriptStats = data.stats;
  }
  reportLines[place] = { exit: run.status, report, scriptStats, stderr: (run.stderr || '').trim() };
  reports[place] = report;
  if (run.status !== 0 || !report || report.errors !== 0 || report.missingApiCount !== 0 || scriptStats?.errors !== 0) failed = true;
}

const dynamic = new Set((reports.hall?.ctxKeys || []).map((key) => `ctx.${key}`));
for (const key of [
  'additive', 'albedo', 'atlas', 'attribute', 'authoring', 'bone', 'canvas', 'cover', 'crossing', 'cube', 'cycle', 'data', 'day', 'depth', 'falloff', 'field', 'flat', 'fx', 'getTerrainHeight', 'groundNormal', 'groundY', 'instances', 'instance', 'metalness', 'noise', 'normal', 'portal', 'purchase', 'quad', 'rejectedRooms', 'requestedRoomId', 'room', 'rooms', 'roughness', 'scatter', 'seedOffset', 'slope', 'smooth', 'sprite', 'strength', 'strokeStartValue', 'sway', 'terrain', 'texture', 'tile', 'tri', 'wang', 'x', 'y', 'z'
]) dynamic.add(`ctx.${key}`);
const missingUsage = [...usage].filter((item) => {
  const top = item.slice(4).split('.')[0];
  return !dynamic.has(`ctx.${top}`) && !dynamic.has(item);
}).sort();
if (missingUsage.length) failed = true;

const summary = {
  kind: 'game16-api-check',
  game,
  bin,
  usageCount: usage.size,
  missingUsage,
  missingUsageCount: missingUsage.length,
  places: Object.fromEntries(places.map((place) => {
    const line = reportLines[place];
    return [place, {
      exit: line.exit,
      errors: line.report?.errors,
      scriptErrors: line.scriptStats?.errors,
      missingApiCount: line.report?.missingApiCount,
      missingApi: line.report?.missingApi,
      moduleCount: line.report?.moduleCount,
      hostVerbs: line.report?.hostVerbs,
      hash: line.report?.hash
    }];
  })),
  ok: !failed
};
console.log(JSON.stringify(summary, null, 2));
process.exit(failed ? 1 : 0);
