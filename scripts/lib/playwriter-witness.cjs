const { execFileSync, spawn } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

const gateBin = process.env.PLAYWRITER_BIN || '/config/.gmnew/pw/bin';

function gatePath() {
    const dirs = fs.existsSync(path.join(gateBin, 'playwriter-project')) ? [gateBin, process.env.PATH || ''] : [process.env.PATH || ''];
    return dirs.join(path.delimiter);
}

function children(pid) {
    const rows = execFileSync('ps', ['-eo', 'pid=,ppid='], { encoding: 'utf8' }).trim().split('\n');
    const map = new Map();
    for (const row of rows) {
        const [child, parent] = row.trim().split(/\s+/).map(Number);
        if (!map.has(parent)) map.set(parent, []);
        map.get(parent).push(child);
    }
    const found = [];
    const queue = [pid];
    while (queue.length) {
        for (const child of map.get(queue.shift()) || []) {
            found.push(child);
            queue.push(child);
        }
    }
    return found;
}

function killTree(pid) {
    if (!pid) return;
    for (const target of [...children(pid).reverse(), pid]) {
        try {
            process.kill(target, 'SIGTERM');
        } catch {}
    }
}

function gateCli(repo, args, timeout) {
    return execFileSync('playwriter-project', args, {
        cwd: repo,
        encoding: 'utf8',
        timeout,
        maxBuffer: 64 * 1024 * 1024,
        env: { ...process.env, PATH: gatePath(), PLAYWRITER_PROJECT_DIR: repo, PLAYWRITER_WEBGPU: '1' },
    });
}

function gateStatus(repo) {
    return JSON.parse(gateCli(repo, ['status', '--json'], 30000));
}

async function waitFor(check, limitMs) {
    const deadline = Date.now() + limitMs;
    while (Date.now() < deadline) {
        if (await check()) return true;
        await sleep(500);
    }
    return false;
}

async function listening(url) {
    try {
        return (await fetch(url)).ok;
    } catch {
        return false;
    }
}

async function withGateWitness({ repo, port }, body) {
    const pageBase = `http://127.0.0.1:${port}/web/wasm/index.html`;
    if (await listening(pageBase)) throw new Error(`port ${port} already serves ${pageBase}; refusing to run a second server`);
    const before = gateStatus(repo);
    const ctx = { pageBase, run: null, browser: { before: before.browser?.pid ?? null, started: null }, gate: { before: before.gate?.pid ?? null, started: null } };
    const server = spawn('bash', [path.join(repo, 'scripts/serve-wasm.sh')], { cwd: repo, env: { ...process.env, PORT: String(port) }, stdio: 'ignore' });
    let session = null;
    let value;
    try {
        if (!(await waitFor(() => listening(pageBase), 20000))) throw new Error(`server did not answer ${pageBase} within 20 s`);
        gateCli(repo, ['start'], 120000);
        const status = gateStatus(repo);
        if (!status.browser?.pid || !status.debugPort) throw new Error(`gate browser not running: ${JSON.stringify(status.browser)}`);
        const wsUrl = (await (await fetch(`http://127.0.0.1:${status.debugPort}/json/version`)).json()).webSocketDebuggerUrl;
        const created = gateCli(repo, ['session', 'new', '--direct', wsUrl], 60000);
        session = /Session (\d+)/.exec(created)?.[1];
        if (!session) throw new Error(`no session id in: ${created.trim()}`);
        ctx.run = (code, timeout) => gateCli(repo, ['-s', session, '--timeout', String(timeout), '-e', code], timeout + 60000);
        value = await body(ctx);
    } finally {
        if (session) {
            try {
                gateCli(repo, ['session', 'delete', session], 30000);
            } catch {}
        }
        let after = null;
        try {
            after = gateStatus(repo);
        } catch {}
        ctx.browser.started = after?.browser?.pid && after.browser.pid !== ctx.browser.before ? after.browser.pid : null;
        ctx.gate.started = after?.gate?.pid && after.gate.pid !== ctx.gate.before ? after.gate.pid : null;
        killTree(ctx.browser.started);
        killTree(ctx.gate.started);
        killTree(server.pid);
    }
    return { value, browser: ctx.browser, gate: ctx.gate };
}

module.exports = { withGateWitness, sleep, waitFor };
