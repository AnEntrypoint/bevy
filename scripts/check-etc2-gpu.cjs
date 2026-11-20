const fs = require('node:fs');
const http = require('node:http');
const path = require('node:path');
const { spawn, spawnSync } = require('node:child_process');
const { glbParts, ktx2Images } = require('./lib/texture-cache.cjs');
const { decodeEtc2Rgba8Block } = require('./lib/texel-decode.cjs');

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
const BAR_DB = 35;
const MISMATCH_AGREE_FRACTION = 0.01;
const TEXTURE_PREFIX = 'value.17c7bd864f7f18268f';
const W = 1024;
const BLOCKS_PER_ROW = 256;
const FIXTURE_EXPECT = [144, 76, 8, 255];
const root = path.join(__dirname, '..');
const pageFile = path.join(__dirname, 'browser', 'readback.html');
const outDir = path.join(root, 'target', 'readback');
const gpuPath = path.join(outDir, 'gpu-rgba8.bin');

function sleep(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
}

function descendants(pid) {
    const out = spawnSync('ps', ['-o', 'pid=', '--ppid', String(pid)], { encoding: 'utf8' }).stdout || '';
    const kids = out.split('\n').map((s) => Number(s.trim())).filter(Boolean);
    return kids.concat(...kids.map(descendants));
}

function alive(pid) {
    try {
        process.kill(pid, 0);
        return true;
    } catch (error) {
        return false;
    }
}

function sendFile(res, file, type) {
    res.writeHead(200, { 'Content-Type': type });
    fs.createReadStream(file).pipe(res);
}

function startServer(state) {
    return http.createServer((req, res) => {
        const url = new URL(req.url, 'http://127.0.0.1');
        if (req.method === 'GET' && url.pathname === '/readback.html') {
            sendFile(res, pageFile, 'text/html');
            return;
        }
        if (req.method === 'GET' && url.pathname === '/etc2-level0.bin') {
            sendFile(res, path.join(outDir, 'etc2-level0.bin'), 'application/octet-stream');
            return;
        }
        if (req.method === 'POST') {
            const chunks = [];
            req.on('data', (chunk) => chunks.push(chunk));
            req.on('end', () => {
                const body = Buffer.concat(chunks);
                if (url.pathname === '/gpu-rgba8') {
                    fs.writeFileSync(gpuPath, body);
                }
                if (url.pathname === '/result') {
                    state.result = JSON.parse(body.toString('utf8'));
                    state.resolve();
                }
                res.writeHead(204);
                res.end();
            });
            return;
        }
        res.writeHead(404);
        res.end();
    });
}

async function runCase(caseName) {
    fs.mkdirSync(outDir, { recursive: true });
    const profile = path.join(outDir, 'profile-' + caseName);
    fs.rmSync(profile, { recursive: true, force: true });
    const state = { result: null, resolve: null };
    const done = new Promise((resolve) => { state.resolve = resolve; });
    const server = startServer(state);
    await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
    const port = server.address().port;
    const started = Date.now();
    const url = 'http://127.0.0.1:' + port + '/readback.html?case=' + caseName;
    const child = spawn(CHROMIUM, FLAGS.concat(['--user-data-dir=' + profile, url]), { stdio: 'ignore' });
    const timeout = new Promise((resolve) => setTimeout(() => resolve('timeout'), CASE_TIMEOUT_MS));
    const outcome = await Promise.race([done.then(() => 'done'), timeout]);
    const tree = descendants(child.pid);
    const wall = Date.now() - started;
    child.kill('SIGTERM');
    for (const pid of tree) {
        try { process.kill(pid, 'SIGTERM'); } catch (error) { }
    }
    await sleep(3000);
    const leftover = [child.pid].concat(tree).filter(alive);
    for (const pid of leftover) {
        try { process.kill(pid, 'SIGKILL'); } catch (error) { }
    }
    server.close();
    return { outcome, wall, pid: child.pid, treeSize: tree.length, leftover: leftover.length, result: state.result };
}

function texture() {
    const etc2Dir = path.join(root, 'target', 'gltf-cache-etc2');
    const rgba8Dir = path.join(root, 'target', 'gltf-cache-rgba8');
    const name = fs.readdirSync(etc2Dir).find((file) => file.startsWith(TEXTURE_PREFIX));
    const load = (dir) => {
        const { json, bin } = glbParts(fs.readFileSync(path.join(dir, name)));
        const image = ktx2Images(json, bin)[0];
        const offset = Number(image.readBigUInt64LE(80));
        const length = Number(image.readBigUInt64LE(88));
        return image.subarray(offset, offset + length);
    };
    return { name, etc2: load(etc2Dir), rgba8: load(rgba8Dir) };
}

function psnr(sumSquares, samples) {
    return +(10 * Math.log10(65025 / (sumSquares / samples))).toFixed(2);
}

function fixtureCheck(result) {
    const pixels = result.pixels || [];
    let ok = pixels.length === 64;
    for (let texel = 0; ok && texel < 16; texel += 1) {
        for (let channel = 0; channel < 4; channel += 1) {
            if (pixels[texel * 4 + channel] !== FIXTURE_EXPECT[channel]) ok = false;
        }
    }
    return ok;
}

function textureCheck(gpu, reference, etc2) {
    const sq = [0, 0, 0, 0];
    let mismatch = 0;
    const samples = gpu.length / 4;
    for (let block = 0; block < etc2.length / 16; block += 1) {
        let first = null;
        try {
            first = decodeEtc2Rgba8Block(etc2.subarray(block * 16, block * 16 + 16));
        } catch (error) {
            first = null;
        }
        const bx = block % BLOCKS_PER_ROW;
        const by = Math.floor(block / BLOCKS_PER_ROW);
        for (let y = 0; y < 4; y += 1) {
            for (let x = 0; x < 4; x += 1) {
                const pixel = ((by * 4 + y) * W + bx * 4 + x) * 4;
                const texel = (y * 4 + x) * 4;
                for (let c = 0; c < 4; c += 1) {
                    const d = gpu[pixel + c] - reference[pixel + c];
                    sq[c] += d * d;
                }
                if (first && (first[texel] !== gpu[pixel] || first[texel + 1] !== gpu[pixel + 1] || first[texel + 2] !== gpu[pixel + 2] || first[texel + 3] !== gpu[pixel + 3])) {
                    mismatch += 1;
                }
            }
        }
    }
    const rgb = [0, 1, 2].map((c) => psnr(sq[c], samples));
    return { rgbPsnr: rgb, alphaPsnr: psnr(sq[3], samples), firstDecoderMismatch: mismatch, texels: samples };
}

function reportTexture(data) {
    if (!fs.existsSync(gpuPath)) {
        console.log('FAIL readback produced no texture output');
        return 1;
    }
    const gpu = fs.readFileSync(gpuPath);
    const check = textureCheck(gpu, data.rgba8, data.etc2);
    const minPsnr = Math.min(...check.rgbPsnr);
    const agreeFirst = check.firstDecoderMismatch / check.texels < MISMATCH_AGREE_FRACTION;
    console.log('TEXTURE ' + data.name.slice(0, 24) + ' readback vs RGBA8 cache RGB PSNR ' + check.rgbPsnr.join(' / ') + ' dB, alpha ' + check.alphaPsnr + ' dB');
    console.log('READBACK vs first decoder: ' + check.firstDecoderMismatch + ' of ' + check.texels + ' texels differ (' + (100 * check.firstDecoderMismatch / check.texels).toFixed(2) + '%)');
    console.log('CACHE ' + (minPsnr >= BAR_DB ? 'CLEAR' : 'OPEN') + ' bar ' + BAR_DB + ' dB, readback min ' + minPsnr + ' dB');
    console.log('SIDE ' + (minPsnr >= BAR_DB ? 'readback agrees with the cache' : (agreeFirst ? 'readback agrees with the first decoder and the cache disagrees' : 'readback disagrees with both the cache and the first decoder')));
    return 0;
}

async function main() {
    const mode = process.argv[2];
    const caseName = mode === 'fixture' || mode === 'analyse' ? mode : 'texture';
    fs.mkdirSync(outDir, { recursive: true });
    if (caseName === 'analyse') {
        console.log('ANALYSE source=' + path.relative(root, gpuPath) + ' (no browser started)');
        process.exit(reportTexture(texture()));
    }
    let data = null;
    if (caseName === 'texture') {
        data = texture();
        fs.writeFileSync(path.join(outDir, 'etc2-level0.bin'), data.etc2);
        fs.rmSync(gpuPath, { force: true });
    }
    const run = await runCase(caseName);
    const meta = run.result || {};
    console.log('CHROMIUM case=' + caseName + ' pid=' + run.pid + ' tree=' + run.treeSize + ' outcome=' + run.outcome + ' wall=' + (run.wall / 1000).toFixed(1) + 's leftover=' + run.leftover);
    console.log('GL renderer=' + meta.renderer + ' version=' + meta.version + ' etc2Listed=' + meta.etc2Listed + ' nativeFormats=' + meta.nativeFormats + (meta.error ? ' error=' + meta.error : ''));
    if (caseName === 'fixture') {
        const ok = !meta.error && fixtureCheck(meta);
        console.log((ok ? 'ok   ' : 'FAIL ') + 'ETC2 flat block (colour 144,76,8, alpha 255) decodes on the software GL, 16 of 16 texels');
        process.exit(ok ? 0 : 1);
    }
    if (meta.error) {
        console.log('FAIL readback: ' + meta.error);
        process.exit(1);
    }
    process.exit(reportTexture(data));
}

main().catch((error) => {
    console.log('FAIL ' + (error && error.stack ? error.stack : error));
    process.exit(1);
});
