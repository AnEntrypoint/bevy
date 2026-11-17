const { execFileSync, spawn } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

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

function gateBrowserPid(repo) {
    try {
        return Number(fs.readFileSync(path.join(repo, '.playwriter', 'browser.pid'), 'utf8').trim()) || null;
    } catch {
        return null;
    }
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

function gate(repo, args, timeout) {
    return execFileSync('playwriter-project', args, {
        cwd: repo,
        encoding: 'utf8',
        timeout,
        maxBuffer: 64 * 1024 * 1024,
        env: { ...process.env, PLAYWRITER_PROJECT_DIR: repo, PLAYWRITER_WEBGPU: '1' },
    });
}

function lastJsonLine(output) {
    for (const line of output.trim().split('\n').reverse()) {
        try {
            const value = JSON.parse(line.replace(/^\[log\] /, '').trim());
            if (value && typeof value === 'object') return value;
        } catch {}
    }
    throw new Error(`no JSON result in playwriter output: ${output.slice(-400)}`);
}

async function withGateWitness({ repo, port, tabGroup }, body) {
    const pageBase = `http://127.0.0.1:${port}/web/wasm/index.html`;
    if (await listening(pageBase)) throw new Error(`port ${port} already serves ${pageBase}; refusing to run a second server`);
    const ctx = { pageBase, browser: { before: gateBrowserPid(repo), started: null }, run: null };
    const server = spawn('bash', [path.join(repo, 'scripts/serve-wasm.sh')], { cwd: repo, env: { ...process.env, PORT: String(port) }, stdio: 'ignore' });
    let session = null;
    let value;
    try {
        if (!(await waitFor(() => listening(pageBase), 20000))) throw new Error(`server did not answer ${pageBase} within 20 s`);
        const created = gate(repo, ['session', 'new', '--tab-group', tabGroup], 60000);
        session = /Session (\d+)/.exec(created)?.[1];
        if (!session) throw new Error(`no session id in: ${created.trim()}`);
        ctx.run = (code, timeout) => gate(repo, ['-s', session, '--timeout', String(timeout), '-e', code], timeout + 60000);
        value = await body(ctx);
    } finally {
        if (session) {
            try {
                gate(repo, ['session', 'delete', session], 30000);
            } catch {}
        }
        const after = gateBrowserPid(repo);
        ctx.browser.started = after && after !== ctx.browser.before ? after : null;
        killTree(ctx.browser.started);
        killTree(server.pid);
    }
    return { value, browser: ctx.browser };
}

module.exports = { withGateWitness, lastJsonLine, sleep, waitFor };
