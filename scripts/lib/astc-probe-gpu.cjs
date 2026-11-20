const fs = require('node:fs');
const http = require('node:http');
const path = require('node:path');
const { spawn, spawnSync } = require('node:child_process');

const root = path.join(__dirname, '..', '..');
const CHROMIUM = '/usr/lib/chromium/chromium.real';
const FLAGS = [
    '--headless=new',
    '--no-sandbox',
    '--use-gl=angle',
    '--use-angle=swiftshader',
    '--enable-unsafe-swiftshader',
    '--ignore-gpu-blocklist',
    '--no-first-run',
    '--disable-background-networking',
    '--disable-component-update',
    '--disable-default-apps',
    '--disable-sync',
];
const CASE_TIMEOUT_MS = 180000;
const pageFile = path.join(root, 'scripts', 'browser', 'readback-astc-blocks.html');
const profileRoot = path.join(root, 'target', 'readback');

function alive(pid) {
    try {
        process.kill(pid, 0);
        return true;
    } catch (error) {
        return false;
    }
}

function descendants(pid) {
    const found = [];
    const listing = spawnSync('ps', ['-o', 'pid=', '--ppid', String(pid)], { encoding: 'utf8' });
    for (const line of listing.stdout.split('\n')) {
        const child = Number(line.trim());
        if (child) {
            found.push(child);
            found.push(...descendants(child));
        }
    }
    return found;
}

function collect(req, done) {
    const chunks = [];
    req.on('data', (chunk) => chunks.push(chunk));
    req.on('end', () => done(Buffer.concat(chunks)));
}

function startServer(state, blob) {
    return http.createServer((req, res) => {
        const url = new URL(req.url, 'http://127.0.0.1');
        if (req.method === 'GET' && url.pathname === '/readback-astc-blocks.html') {
            res.writeHead(200, { 'Content-Type': 'text/html' });
            fs.createReadStream(pageFile).pipe(res);
        } else if (req.method === 'GET' && url.pathname === '/astc-blocks.bin') {
            res.writeHead(200, { 'Content-Type': 'application/octet-stream' });
            res.end(blob);
        } else if (req.method === 'POST' && url.pathname === '/gpu-astc-rgba8') {
            collect(req, (body) => {
                state.pixels = body;
                res.end('ok');
            });
        } else if (req.method === 'POST' && url.pathname === '/result') {
            collect(req, (body) => {
                state.result = JSON.parse(body.toString());
                res.end('ok');
                state.resolve();
            });
        } else {
            res.writeHead(404);
            res.end();
        }
    });
}

async function runProbe(blob, label) {
    fs.mkdirSync(profileRoot, { recursive: true });
    const profile = path.join(profileRoot, 'profile-' + label);
    const state = { pixels: null, result: null, resolve: null };
    const done = new Promise((resolve) => {
        state.resolve = resolve;
    });
    const server = startServer(state, blob);
    await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
    const url = 'http://127.0.0.1:' + server.address().port + '/readback-astc-blocks.html';
    const started = Date.now();
    const child = spawn(CHROMIUM, FLAGS.concat(['--user-data-dir=' + profile, url]), { stdio: 'ignore' });
    const timeout = new Promise((resolve) => setTimeout(() => resolve('timeout'), CASE_TIMEOUT_MS));
    const outcome = await Promise.race([done.then(() => 'done'), timeout]);
    const wall = (Date.now() - started) / 1000;
    const tree = descendants(child.pid);
    child.kill('SIGTERM');
    for (const pid of tree) {
        try {
            process.kill(pid, 'SIGTERM');
        } catch (error) {
        }
    }
    await new Promise((resolve) => setTimeout(resolve, 3000));
    const leftover = [child.pid].concat(tree).filter(alive);
    for (const pid of leftover) {
        try {
            process.kill(pid, 'SIGKILL');
        } catch (error) {
        }
    }
    server.close();
    return { outcome, wall, pid: child.pid, treeSize: tree.length, leftover: leftover.length, pixels: state.pixels, result: state.result };
}

module.exports = { runProbe };
