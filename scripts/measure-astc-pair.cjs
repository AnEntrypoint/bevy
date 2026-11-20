#!/usr/bin/env node
const fs = require('node:fs');
const path = require('node:path');
const { execFileSync } = require('node:child_process');
const { formatLine, checkPairs } = require('./lib/astc-load-ratio.cjs');
const { runArm } = require('./lib/load-arm.cjs');
const { armBudgets } = require('./lib/pair-budget.cjs');

const shortNames = { astc4x4: 'astc', etc2: 'etc2' };
const format = process.argv[2] || process.env.PAIR_FORMAT || 'astc4x4';
if (!shortNames[format]) throw new Error(`unknown format ${format}; expected one of ${Object.keys(shortNames).join(', ')}`);
const short = shortNames[format];
const repo = path.resolve(__dirname, '..');
const witnessDir = path.join(repo, 'target', 'witness');
const footprintPath = path.join(witnessDir, 'texture-footprint.json');
const budgets = armBudgets(process.env);
const tolerance = Number(process.env.LOAD_TOLERANCE || 1.1);
const formatDriver = process.env.PAIR_FORMAT_DRIVER || process.env.ASTC_LOAD_DRIVER || 'scripts/check-neon-wasm-models.cjs';
const referenceDriver = process.env.PAIR_REFERENCE_DRIVER || formatDriver;
const windowId = new Date().toISOString().replace(/[:.]/g, '-');
const archiveRoot = short === 'astc' ? 'astc-load-pair-archive' : `${short}-load-pair-archive`;
const archive = path.join(witnessDir, archiveRoot, windowId);
const prefix = `${short.toUpperCase()}_PAIR`;

function armFailed(run) {
    return !run.fresh || run.timedOut || run.code !== 0;
}

function armFailure(stage, run, common) {
    return formatLine(prefix, { run_ok: false, stage, fresh: run.fresh, exit_code: run.code, timed_out: run.timedOut, stragglers: run.stragglers, run_pid: run.runPid, ...common });
}

async function main() {
    fs.mkdirSync(archive, { recursive: true });
    const formatRun = await runArm({ repo, driver: formatDriver, texture: format, label: short, archive, timeoutMs: budgets.formatS * 1000 });
    const common = { window: windowId, archive: path.relative(repo, archive) };
    if (armFailed(formatRun)) {
        console.log(armFailure(short, formatRun, common));
        process.exit(1);
    }
    const referenceRun = await runArm({ repo, driver: referenceDriver, texture: 'rgba8-software', label: 'reference', archive, timeoutMs: budgets.referenceS * 1000 });
    if (armFailed(referenceRun)) {
        console.log(armFailure('reference', referenceRun, common));
        process.exit(1);
    }
    const runs = [
        { window: windowId, arm: short, startS: formatRun.startedMs / 1000, loadStart: formatRun.loadStart, loadMs: formatRun.witness.summary.loadMsMedian, clean: formatRun.witness.summary.clean, total: formatRun.witness.summary.total, wallS: formatRun.witness.wallMs / 1000 },
        { window: windowId, arm: 'reference', startS: referenceRun.startedMs / 1000, loadStart: referenceRun.loadStart, loadMs: referenceRun.witness.summary.loadMsMedian, clean: referenceRun.witness.summary.clean, total: referenceRun.witness.summary.total, wallS: referenceRun.witness.wallMs / 1000 },
    ];
    const pair = checkPairs(runs, tolerance);
    execFileSync('node', ['scripts/texture-footprint.cjs'], { cwd: repo, stdio: 'ignore' });
    const footprintOk = JSON.parse(fs.readFileSync(footprintPath, 'utf8')).formats[format].reasons.loadOk;
    const [formatResult, reference] = runs;
    const pairOne = pair.pairs[0] || null;
    const cleanOk = runs.every((run) => run.clean === run.total);
    const runOk = cleanOk && pair.ok;
    console.log(formatLine(prefix, {
        run_ok: runOk,
        ...common,
        order: pairOne ? pairOne.order : 'na',
        interleaved: pair.interleaved,
        [`load_start_${short}`]: formatResult.loadStart,
        load_start_reference: reference.loadStart,
        [`${short}_ms`]: formatResult.loadMs,
        reference_ms: reference.loadMs,
        ratio: pairOne ? pairOne.ratio.toFixed(3) : 'na',
        tolerance,
        pair_ok: pair.ok,
        footprint_load_ok: footprintOk,
        [`${short}_clean`]: `${formatResult.clean}/${formatResult.total}`,
        reference_clean: `${reference.clean}/${reference.total}`,
        [`${short}_wall_s`]: formatResult.wallS.toFixed(1),
        reference_wall_s: reference.wallS.toFixed(1),
        run_pids: `${formatRun.runPid},${referenceRun.runPid}`,
        stragglers: formatRun.stragglers + referenceRun.stragglers,
        reason: pair.reason.replace(/ /g, '_'),
    }));
    process.exit(runOk ? 0 : 1);
}

main().catch((error) => {
    console.error(error.stack || error.message);
    process.exit(2);
});
