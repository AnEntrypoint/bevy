const fs = require('node:fs');
const path = require('node:path');
const { spawn, execFileSync } = require('node:child_process');

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

function loadAverage() {
    return Number(fs.readFileSync('/proc/loadavg', 'utf8').split(' ')[0]);
}

async function runDriver({ repo, driver, texture, logPath, timeoutMs }) {
    const log = fs.openSync(logPath, 'w');
    const child = spawn('node', [driver], { cwd: repo, env: { ...process.env, NEON_TEXTURE: texture }, stdio: ['ignore', log, log] });
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
    }, timeoutMs);
    const code = await exited;
    clearTimeout(limit);
    clearInterval(sampler);
    fs.closeSync(log);
    const stragglers = [...seen].filter(alive);
    await stopPids(stragglers);
    return { code, runPid, timedOut, stragglers: stragglers.length };
}

async function runArm({ repo, driver, texture, label, archive, timeoutMs }) {
    const witnessDir = path.join(repo, 'target', 'witness');
    const shotDir = path.join(witnessDir, 'neon-shots-' + texture);
    const witnessPath = path.join(witnessDir, 'neon-wasm-witness-' + texture + '.json');
    const rowsPath = path.join(witnessDir, 'neon-rows-' + texture + '.json');
    const armDir = path.join(archive, label);
    fs.mkdirSync(armDir, { recursive: true });
    const beforeShots = path.join(armDir, 'before-neon-shots-' + texture);
    const afterShots = path.join(armDir, 'after-neon-shots-' + texture);
    for (const file of [witnessPath, rowsPath]) {
        if (fs.existsSync(file)) fs.copyFileSync(file, path.join(armDir, 'before-' + path.basename(file)));
    }
    const hadShots = fs.existsSync(shotDir);
    if (hadShots) fs.renameSync(shotDir, beforeShots);
    const loadStart = loadAverage();
    const startedMs = Date.now();
    let run;
    try {
        run = await runDriver({ repo, driver, texture, logPath: path.join(armDir, 'driver.log'), timeoutMs });
    } finally {
        if (fs.existsSync(shotDir)) fs.renameSync(shotDir, afterShots);
        if (hadShots) fs.renameSync(beforeShots, shotDir);
    }
    const fresh = fs.existsSync(witnessPath) && fs.statSync(witnessPath).mtimeMs >= startedMs;
    const witness = fresh ? JSON.parse(fs.readFileSync(witnessPath, 'utf8')) : null;
    return { ...run, fresh, startedMs, loadStart, witness, archive: path.relative(repo, armDir) };
}

module.exports = { runArm, treeOf, stopPids, loadAverage };
