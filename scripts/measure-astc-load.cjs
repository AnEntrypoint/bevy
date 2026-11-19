#!/usr/bin/env node
const fs = require('node:fs');
const path = require('node:path');
const { spawn, execFileSync } = require('node:child_process');
const { decideLoad, formatLoadLine } = require('./lib/astc-load-ratio.cjs');

const repo = path.resolve(__dirname, '..');
const witnessDir = path.join(repo, 'target', 'witness');
const shotDir = path.join(witnessDir, 'neon-shots-astc4x4');
const witnessPath = path.join(witnessDir, 'neon-wasm-witness-astc4x4.json');
const rowsPath = path.join(witnessDir, 'neon-rows-astc4x4.json');
const footprintPath = path.join(witnessDir, 'texture-footprint.json');
const runTimeoutMs = Number(process.env.ASTC_RUN_TIMEOUT_S || 600) * 1000;
const driver = process.env.ASTC_LOAD_DRIVER || 'scripts/check-neon-wasm-models.cjs';
const archive = path.join(witnessDir, 'astc-load-archive', new Date().toISOString().replace(/[:.]/g, '-'));

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

function treeOf(root) {
    const rows = execFileSync('ps', ['-eo', 'pid=,ppid='], { encoding: 'utf8' }).trim().split('\n').map((line) => line.trim().split(/\s+/).map(Number));
    const tree = new Set([root]);
    let grew = true;
    while (grew) {
        grew = false;
        for (const [pid, ppid] of rows) {
            if (!tree.has(pid) && tree.has(ppid)) {
                tree.add(pid);
                grew = true;
            }
        }
    }
    return [...tree];
}

function alive(pid) {
    try {
        process.kill(pid, 0);
        return true;
    } catch {
        return false;
    }
}

function signal(pids, name) {
    for (const pid of pids) {
        if (!alive(pid)) continue;
        try {
            process.kill(pid, name);
        } catch {}
    }
}

async function stopPids(pids) {
    signal(pids, 'SIGTERM');
    await sleep(5000);
    signal(pids, 'SIGKILL');
}

async function runDriver() {
    const log = fs.openSync(path.join(archive, 'driver.log'), 'w');
    const child = spawn('node', [driver], { cwd: repo, env: { ...process.env, NEON_TEXTURE: 'astc4x4' }, stdio: ['ignore', log, log] });
    const runPid = child.pid;
    const seen = new Set([runPid]);
    const sampler = setInterval(() => {
        for (const pid of treeOf(runPid)) seen.add(pid);
    }, 2000);
    let timedOut = false;
    const exited = new Promise((resolve) => child.on('exit', (code) => resolve(code)));
    const limit = setTimeout(async () => {
        timedOut = true;
        for (const pid of treeOf(runPid)) seen.add(pid);
        await stopPids([...seen]);
    }, runTimeoutMs);
    const code = await exited;
    clearTimeout(limit);
    clearInterval(sampler);
    fs.closeSync(log);
    const stragglers = [...seen].filter(alive);
    await stopPids(stragglers);
    return { code, runPid, timedOut, stragglers: stragglers.length };
}

async function main() {
    fs.mkdirSync(archive, { recursive: true });
    const beforeShots = path.join(archive, 'before-neon-shots-astc4x4');
    const afterShots = path.join(archive, 'after-neon-shots-astc4x4');
    fs.copyFileSync(witnessPath, path.join(archive, 'before-neon-wasm-witness-astc4x4.json'));
    fs.copyFileSync(rowsPath, path.join(archive, 'before-neon-rows-astc4x4.json'));
    fs.renameSync(shotDir, beforeShots);
    const startedMs = Date.now();
    let run;
    try {
        run = await runDriver();
    } finally {
        if (fs.existsSync(shotDir)) fs.renameSync(shotDir, afterShots);
        fs.renameSync(beforeShots, shotDir);
    }
    const fresh = fs.existsSync(witnessPath) && fs.statSync(witnessPath).mtimeMs >= startedMs;
    const base = { run_pid: run.runPid, timed_out: run.timedOut, stragglers: run.stragglers, archive: path.relative(repo, archive), reference_file: 'neon-wasm-witness-rgba8-software.json' };
    if (!fresh || run.timedOut || run.code !== 0) {
        console.log(formatLoadLine({ run_ok: false, fresh, exit_code: run.code, ...base }));
        process.exit(1);
    }
    const witness = JSON.parse(fs.readFileSync(witnessPath, 'utf8'));
    execFileSync('node', ['scripts/texture-footprint.cjs'], { cwd: repo, stdio: 'ignore' });
    const astc = JSON.parse(fs.readFileSync(footprintPath, 'utf8')).formats.astc4x4;
    const decision = decideLoad({ astcMs: astc.witness.loadMsMedian, referenceMs: astc.referenceWitness.loadMsMedian, tolerance: astc.reasons.loadTolerance });
    const runOk = witness.summary.clean === witness.summary.total;
    console.log(formatLoadLine({
        run_ok: runOk,
        astc_ms: astc.witness.loadMsMedian,
        reference_ms: astc.referenceWitness.loadMsMedian,
        ratio: decision.ratio.toFixed(3),
        tolerance: decision.tolerance,
        load_ok: decision.usable,
        footprint_load_ok: astc.reasons.loadOk,
        agrees: decision.usable === astc.reasons.loadOk,
        astc_clean: `${witness.summary.clean}/${witness.summary.total}`,
        astc_wall_s: (witness.wallMs / 1000).toFixed(1),
        ...base,
    }));
    process.exit(runOk ? 0 : 1);
}

main().catch((error) => {
    console.error(error.stack || error.message);
    process.exit(2);
});
