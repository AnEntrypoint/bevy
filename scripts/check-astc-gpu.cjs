
const fs = require('node:fs');
const http = require('node:http');
const path = require('node:path');
const { spawn, spawnSync } = require('node:child_process');
const { glbParts, ktx2Images, ktx2Info } = require('./lib/texture-cache.cjs');
const { decodeAstc4x4Block } = require('./lib/astc-decode.cjs');

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
const SIZE = 1024;
const BLOCKS_PER_ROW = SIZE / 4;
const FIXTURE_EXPECT = [144, 76, 8, 255];
const ASTC_4X4_FORMAT = 0x93b0;
const VK_FORMAT_ASTC_4X4 = 157;
const root = path.join(__dirname, '..');
const pageFile = path.join(__dirname, 'browser', 'readback-astc.html');
const outDir = path.join(root, 'target', 'readback');
const gpuPath = path.join(outDir, 'gpu-astc-rgba8.bin');

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

function startServer(state) {
    return http.createServer((req, res) => {
        const url = new URL(req.url, 'http://127.0.0.1');
        if (req.method === 'GET' && url.pathname === '/readback-astc.html') {
            res.writeHead(200, { 'Content-Type': 'text/html' });
            fs.createReadStream(pageFile).pipe(res);
            return;
        }
        if (req.method === 'GET' && url.pathname === '/astc-level0.bin') {
            res.writeHead(200, { 'Content-Type': 'application/octet-stream' });
            fs.createReadStream(path.join(outDir, 'astc-level0.bin')).pipe(res);
            return;
        }
        if (req.method === 'POST') {
            const chunks = [];
            req.on('data', (chunk) => chunks.push(chunk));
            req.on('end', () => {
                const body = Buffer.concat(chunks);
                if (url.pathname === '/gpu-astc-rgba8') {
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

async function runCase() {
    fs.mkdirSync(outDir, { recursive: true });
    const profile = path.join(outDir, 'profile-astc');
    fs.rmSync(profile, { recursive: true, force: true });
    const state = { result: null, resolve: null };
    const done = new Promise((resolve) => { state.resolve = resolve; });
    const server = startServer(state);
    await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
    const port = server.address().port;
    const started = Date.now();
    const url = 'http://127.0.0.1:' + port + '/readback-astc.html?case=astc';
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

function load(dir) {
    const name = fs.readdirSync(dir).find((file) => file.startsWith(TEXTURE_PREFIX));
    const { json, bin } = glbParts(fs.readFileSync(path.join(dir, name)));
    const image = ktx2Images(json, bin)[0];
    const info = ktx2Info(image);
    const offset = Number(image.readBigUInt64LE(80));
    const length = Number(image.readBigUInt64LE(88));
    return { name, info, level0: image.subarray(offset, offset + length) };
}

function texture() {
    const astc = load(path.join(root, 'target', 'gltf-cache-astc'));
    const rgba8 = load(path.join(root, 'target', 'gltf-cache-rgba8'));
    if (astc.info.vkFormat !== VK_FORMAT_ASTC_4X4 || astc.info.width !== SIZE || astc.info.height !== SIZE) throw new Error('ASTC cache texture is not 1024 VK_FORMAT_ASTC_4x4');
    if (rgba8.info.width !== SIZE || rgba8.info.height !== SIZE) throw new Error('RGBA8 cache texture is not 1024');
    return { name: astc.name, astc: astc.level0, rgba8: rgba8.level0 };
}

function decodeAll(astc) {
    const image = Buffer.alloc(SIZE * SIZE * 4);
    let errors = 0;
    let firstError = '';
    for (let block = 0; block < astc.length / 16; block += 1) {
        const bx = block % BLOCKS_PER_ROW;
        const by = Math.floor(block / BLOCKS_PER_ROW);
        let pixels = null;
        try {
            pixels = decodeAstc4x4Block(astc.subarray(block * 16, block * 16 + 16));
        } catch (error) {
            errors += 1;
            if (!firstError) firstError = String(error.message || error);
            continue;
        }
        for (let y = 0; y < 4; y += 1) {
            for (let x = 0; x < 4; x += 1) {
                const at = ((by * 4 + y) * SIZE + bx * 4 + x) * 4;
                const from = (y * 4 + x) * 4;
                image.set(pixels.subarray(from, from + 4), at);
            }
        }
    }
    return { image, errors, firstError };
}

function psnrPerChannel(a, b) {
    const sq = [0, 0, 0, 0];
    const texels = a.length / 4;
    for (let i = 0; i < a.length; i += 1) {
        const d = a[i] - b[i];
        sq[i & 3] += d * d;
    }
    return sq.map((s) => +(10 * Math.log10(65025 / (s / texels))).toFixed(2));
}

function mismatchCount(a, b) {
    let count = 0;
    for (let texel = 0; texel < a.length / 4; texel += 1) {
        const at = texel * 4;
        if (a[at] !== b[at] || a[at + 1] !== b[at + 1] || a[at + 2] !== b[at + 2] || a[at + 3] !== b[at + 3]) count += 1;
    }
    return count;
}

function fixtureOk(pixels) {
    if (!pixels || pixels.length !== 64) return false;
    for (let texel = 0; texel < 16; texel += 1) {
        for (let channel = 0; channel < 4; channel += 1) {
            if (pixels[texel * 4 + channel] !== FIXTURE_EXPECT[channel]) return false;
        }
    }
    return true;
}

function pageConstants() {
    const source = fs.readFileSync(pageFile, 'utf8');
    const block = source.match(/const FIXTURE_BLOCK = \[([^\]]*)\]/);
    const format = source.match(/const ASTC_4X4 = (0x[0-9a-fA-F]+)/);
    if (!block || !format) throw new Error('readback-astc.html does not declare FIXTURE_BLOCK and ASTC_4X4');
    return { fixtureBlock: Uint8Array.from(block[1].split(',').map((v) => Number(v.trim()))), format: Number(format[1]) };
}

function reportTexture(data, gpuImage) {
    const rgba8 = data.rgba8;
    const cpu = decodeAll(data.astc);
    const cpuPsnr = psnrPerChannel(cpu.image, rgba8);
    console.log('DECODER cpu astc-decode vs RGBA8 cache PSNR R/G/B/A ' + cpuPsnr.join(' / ') + ' dB, blocks failing to decode ' + cpu.errors + (cpu.firstError ? ' (first: ' + cpu.firstError + ')' : ''));
    if (!gpuImage) {
        console.log('FAIL readback produced no texture output');
        return false;
    }
    const gpuPsnr = psnrPerChannel(gpuImage, rgba8);
    const mismatch = mismatchCount(gpuImage, cpu.image);
    console.log('TEXTURE ' + data.name.slice(0, 24) + ' readback vs RGBA8 cache PSNR R/G/B/A ' + gpuPsnr.join(' / ') + ' dB');
    console.log('READBACK vs cpu decoder: ' + mismatch + ' of ' + (gpuImage.length / 4) + ' texels differ (' + (100 * mismatch / (gpuImage.length / 4)).toFixed(2) + '%)');
    const minRgb = Math.min(...gpuPsnr.slice(0, 3));
    const minCpu = Math.min(...cpuPsnr.slice(0, 3));
    console.log('CACHE ' + (minRgb >= BAR_DB ? 'CLEAR' : 'OPEN') + ' bar ' + BAR_DB + ' dB, readback min RGB ' + minRgb + ' dB, cpu decoder min RGB ' + minCpu + ' dB');
    console.log('SIDE ' + (mismatch / (gpuImage.length / 4) < MISMATCH_AGREE_FRACTION ? 'readback agrees with the cpu decoder' : 'readback disagrees with the cpu decoder'));
    return true;
}

function selftest() {
    const failures = [];
    const check = (name, ok, detail) => {
        console.log((ok ? 'ok   ' : 'FAIL ') + name + (detail ? ' ' + detail : ''));
        if (!ok) failures.push(name);
    };
    const page = pageConstants();
    check('page declares COMPRESSED_RGBA_ASTC_4x4 0x93b0', page.format === ASTC_4X4_FORMAT, '0x' + page.format.toString(16));
    check('page fixture block is 16 bytes', page.fixtureBlock.length === 16, String(page.fixtureBlock.length));
    const decoded = decodeAstc4x4Block(page.fixtureBlock);
    check('page fixture block decodes to 144,76,8,255 on all 16 texels', fixtureOk(decoded));
    const wrong = Uint8Array.from(decoded);
    wrong[5] ^= 1;
    check('fixtureOk rejects one wrong channel', !fixtureOk(wrong));
    const same = Uint8Array.from(decoded);
    check('psnr of identical images is infinite', psnrPerChannel(decoded, same).every((v) => v === Infinity));
    const offset = Uint8Array.from(decoded, (v) => (v === 0 ? 1 : v - 1));
    const expectedOffset = +(10 * Math.log10(65025)).toFixed(2);
    check('psnr of a one-level offset is 10 log10(255^2)', psnrPerChannel(offset, decoded).every((v) => v === expectedOffset), 'expected ' + expectedOffset);
    const one = Uint8Array.from(decoded);
    one[4] ^= 0x80;
    check('mismatchCount counts one changed texel', mismatchCount(one, decoded) === 1);
    const solid = new Uint8Array(SIZE * SIZE / 16 * 16);
    for (let block = 0; block < solid.length / 16; block += 1) solid.set(page.fixtureBlock, block * 16);
    const full = decodeAll(solid);
    let uniform = full.errors === 0;
    for (let i = 0; uniform && i < full.image.length; i += 4) {
        for (let c = 0; c < 4; c += 1) if (full.image[i + c] !== FIXTURE_EXPECT[c]) uniform = false;
    }
    check('decodeAll fills 1024x1024 with the fixture colour, 0 errors', uniform, 'errors ' + full.errors);
    if (failures.length) {
        console.log('SELFTEST FAILED: ' + failures.join('; '));
        return 1;
    }
    console.log('SELFTEST ALL PASS');
    return 0;
}

async function main() {
    const mode = process.argv[2];
    fs.mkdirSync(outDir, { recursive: true });
    if (mode === 'selftest') {
        process.exit(selftest());
    }
    if (mode === 'analyse') {
        const data = texture();
        const gpuImage = fs.existsSync(gpuPath) ? fs.readFileSync(gpuPath) : null;
        console.log('ANALYSE source=' + path.relative(root, gpuPath) + ' (no browser started)');
        process.exit(reportTexture(data, gpuImage) ? 0 : 1);
    }
    const data = texture();
    fs.writeFileSync(path.join(outDir, 'astc-level0.bin'), data.astc);
    fs.rmSync(gpuPath, { force: true });
    const run = await runCase();
    const meta = run.result || {};
    console.log('CHROMIUM case=astc pid=' + run.pid + ' tree=' + run.treeSize + ' outcome=' + run.outcome + ' wall=' + (run.wall / 1000).toFixed(1) + 's leftover=' + run.leftover);
    console.log('GL renderer=' + meta.renderer + ' version=' + meta.version + ' astcListed=' + meta.astcListed + ' nativeFormats=' + meta.nativeFormats + (meta.error ? ' error=' + meta.error : ''));
    const fixture = fixtureOk(meta.fixturePixels);
    console.log((fixture ? 'ok   ' : 'FAIL ') + 'ASTC void-extent block (colour 144,76,8, alpha 255) decodes on the software GL, 16 of 16 texels');
    if (meta.error) {
        console.log('FAIL readback: ' + meta.error);
        process.exit(1);
    }
    const gpuImage = fs.existsSync(gpuPath) ? fs.readFileSync(gpuPath) : null;
    const reported = reportTexture(data, gpuImage);
    process.exit(fixture && reported ? 0 : 1);
}

if (require.main === module) {
    main().catch((error) => {
        console.log('FAIL ' + (error && error.stack ? error.stack : error));
        process.exit(1);
    });
}

module.exports = { psnrPerChannel, mismatchCount, fixtureOk, decodeAll, pageConstants };
