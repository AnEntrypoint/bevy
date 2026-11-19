#!/usr/bin/env node
const fs = require('node:fs');
const path = require('node:path');
const { execFileSync } = require('node:child_process');
const { formatLine, checkPairs } = require('./lib/astc-load-ratio.cjs');
const { runArm } = require('./lib/load-arm.cjs');

const repo = path.resolve(__dirname, '..');
const witnessDir = path.join(repo, 'target', 'witness');
const footprintPath = path.join(witnessDir, 'texture-footprint.json');
const windowMs = Number(process.env.PAIR_WINDOW_S || 780) * 1000;
const referenceBudgetMs = Number(process.env.PAIR_REFERENCE_BUDGET_S || 360) * 1000;
const tolerance = Number(process.env.LOAD_TOLERANCE || 1.1);
const astcDriver = process.env.ASTC_LOAD_DRIVER || 'scripts/check-neon-wasm-models.cjs';
const referenceDriver = process.env.PAIR_REFERENCE_DRIVER || astcDriver;
const windowId = new Date().toISOString().replace(/[:.]/g, '-');
const archive = path.join(witnessDir, 'astc-load-pair-archive', windowId);

function armFailed(run) {
    return !run.fresh || run.timedOut || run.code !== 0;
}

async function main() {
    const windowStartMs = Date.now();
    fs.mkdirSync(archive, { recursive: true });
    const astcRun = await runArm({ repo, driver: astcDriver, texture: 'astc4x4', label: 'astc', archive, timeoutMs: windowMs - referenceBudgetMs });
    const common = { window: windowId, archive: path.relative(repo, archive) };
    if (armFailed(astcRun)) {
        console.log(formatLine('ASTC_PAIR', { run_ok: false, stage: 'astc', fresh: astcRun.fresh, exit_code: astcRun.code, timed_out: astcRun.timedOut, stragglers: astcRun.stragglers, run_pid: astcRun.runPid, ...common }));
        process.exit(1);
    }
    const referenceRun = await runArm({ repo, driver: referenceDriver, texture: 'rgba8-software', label: 'reference', archive, timeoutMs: windowMs - (Date.now() - windowStartMs) });
    if (armFailed(referenceRun)) {
        console.log(formatLine('ASTC_PAIR', { run_ok: false, stage: 'reference', fresh: referenceRun.fresh, exit_code: referenceRun.code, timed_out: referenceRun.timedOut, stragglers: referenceRun.stragglers, run_pid: referenceRun.runPid, ...common }));
        process.exit(1);
    }
    const runs = [
        { window: windowId, arm: 'astc', startS: astcRun.startedMs / 1000, loadStart: astcRun.loadStart, loadMs: astcRun.witness.summary.loadMsMedian, clean: astcRun.witness.summary.clean, total: astcRun.witness.summary.total, wallS: astcRun.witness.wallMs / 1000 },
        { window: windowId, arm: 'reference', startS: referenceRun.startedMs / 1000, loadStart: referenceRun.loadStart, loadMs: referenceRun.witness.summary.loadMsMedian, clean: referenceRun.witness.summary.clean, total: referenceRun.witness.summary.total, wallS: referenceRun.witness.wallMs / 1000 },
    ];
    const pair = checkPairs(runs, tolerance);
    execFileSync('node', ['scripts/texture-footprint.cjs'], { cwd: repo, stdio: 'ignore' });
    const footprintOk = JSON.parse(fs.readFileSync(footprintPath, 'utf8')).formats.astc4x4.reasons.loadOk;
    const [astc, reference] = runs;
    const pairOne = pair.pairs[0] || null;
    const cleanOk = runs.every((run) => run.clean === run.total);
    const runOk = cleanOk && pair.ok;
    console.log(formatLine('ASTC_PAIR', {
        run_ok: runOk,
        ...common,
        order: pairOne ? pairOne.order : 'na',
        interleaved: pair.interleaved,
        load_start_astc: astc.loadStart,
        load_start_reference: reference.loadStart,
        astc_ms: astc.loadMs,
        reference_ms: reference.loadMs,
        ratio: pairOne ? pairOne.ratio.toFixed(3) : 'na',
        tolerance,
        pair_ok: pair.ok,
        footprint_load_ok: footprintOk,
        astc_clean: `${astc.clean}/${astc.total}`,
        reference_clean: `${reference.clean}/${reference.total}`,
        astc_wall_s: astc.wallS.toFixed(1),
        reference_wall_s: reference.wallS.toFixed(1),
        run_pids: `${astcRun.runPid},${referenceRun.runPid}`,
        stragglers: astcRun.stragglers + referenceRun.stragglers,
        reason: pair.reason.replace(/ /g, '_'),
    }));
    process.exit(runOk ? 0 : 1);
}

main().catch((error) => {
    console.error(error.stack || error.message);
    process.exit(2);
});
