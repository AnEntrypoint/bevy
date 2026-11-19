#!/usr/bin/env node
const fs = require('node:fs');
const path = require('node:path');
const { execFileSync } = require('node:child_process');
const { decideLoad, formatLoadLine } = require('./lib/astc-load-ratio.cjs');
const { runArm } = require('./lib/load-arm.cjs');

const repo = path.resolve(__dirname, '..');
const witnessDir = path.join(repo, 'target', 'witness');
const footprintPath = path.join(witnessDir, 'texture-footprint.json');
const timeoutMs = Number(process.env.ASTC_RUN_TIMEOUT_S || 600) * 1000;
const driver = process.env.ASTC_LOAD_DRIVER || 'scripts/check-neon-wasm-models.cjs';
const archive = path.join(witnessDir, 'astc-load-archive', new Date().toISOString().replace(/[:.]/g, '-'));

async function main() {
    fs.mkdirSync(archive, { recursive: true });
    const run = await runArm({ repo, driver, texture: 'astc4x4', label: 'astc', archive, timeoutMs });
    const base = { run_pid: run.runPid, timed_out: run.timedOut, stragglers: run.stragglers, archive: path.relative(repo, archive), reference_file: 'neon-wasm-witness-rgba8-software.json' };
    if (!run.fresh || run.timedOut || run.code !== 0) {
        console.log(formatLoadLine({ run_ok: false, fresh: run.fresh, exit_code: run.code, ...base }));
        process.exit(1);
    }
    execFileSync('node', ['scripts/texture-footprint.cjs'], { cwd: repo, stdio: 'ignore' });
    const astc = JSON.parse(fs.readFileSync(footprintPath, 'utf8')).formats.astc4x4;
    const decision = decideLoad({ astcMs: astc.witness.loadMsMedian, referenceMs: astc.referenceWitness.loadMsMedian, tolerance: astc.reasons.loadTolerance });
    const runOk = run.witness.summary.clean === run.witness.summary.total;
    console.log(formatLoadLine({
        run_ok: runOk,
        astc_ms: astc.witness.loadMsMedian,
        reference_ms: astc.referenceWitness.loadMsMedian,
        ratio: decision.ratio.toFixed(3),
        tolerance: decision.tolerance,
        load_ok: decision.usable,
        footprint_load_ok: astc.reasons.loadOk,
        agrees: decision.usable === astc.reasons.loadOk,
        astc_clean: `${run.witness.summary.clean}/${run.witness.summary.total}`,
        astc_wall_s: (run.witness.wallMs / 1000).toFixed(1),
        ...base,
    }));
    process.exit(runOk ? 0 : 1);
}

main().catch((error) => {
    console.error(error.stack || error.message);
    process.exit(2);
});
