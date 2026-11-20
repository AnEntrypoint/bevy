const fs = require('node:fs');
const path = require('node:path');
const special = require(process.env.ETC2_SPECIAL_PATH || './lib/etc2-special.cjs');
const { glbParts, ktx2Images } = require('./lib/texture-cache.cjs');
const { decodeEtc2Rgba8Block } = require('./lib/texel-decode.cjs');

const root = path.join(__dirname, '..');
const TEXTURE_PREFIX = 'value.17c7bd864f7f18268f';
const TEXELS = 1048576;
const EXPECT_BLUE_PSNR = 28.49;
const EXPECT_MIN_PSNR = 28.49;
const checks = [];

function check(name, ok, detail) {
    checks.push(ok);
    console.log((ok ? 'ok   ' : 'FAIL ') + name + (ok ? '' : ': ' + detail));
}

function same(a, b) {
    return JSON.stringify(a) === JSON.stringify(b);
}

function level0(dir, name) {
    const { json, bin } = glbParts(fs.readFileSync(path.join(root, 'target', dir, name)));
    const image = ktx2Images(json, bin)[0];
    const offset = Number(image.readBigUInt64LE(80));
    const length = Number(image.readBigUInt64LE(88));
    return image.subarray(offset, offset + length);
}

const tPal = special.tPalette([0, 0, 0], [8, 8, 8], 17);
check('T palette fixture (c0 0, c1 8, d 17)', same(tPal, [[0, 0, 0], [153, 153, 153], [136, 136, 136], [119, 119, 119]]), JSON.stringify(tPal));
const hPal = special.hPalette([4, 4, 4], [12, 12, 12], 9);
check('H palette fixture (c0 4, c1 12, d 9)', same(hPal, [[77, 77, 77], [59, 59, 59], [213, 213, 213], [195, 195, 195]]), JSON.stringify(hPal));

const ramp = [0, 64, 128, 191];
const pH = special.planarPredict([0, 0, 0], [255, 255, 255], [0, 0, 0]);
check('planar fixture, horizontal ramp (H 255)', pH.every((p, r) => p.every((v) => v === ramp[r & 3])), JSON.stringify(pH.map((p) => p[0])));
const pV = special.planarPredict([0, 0, 0], [0, 0, 0], [255, 255, 255]);
check('planar fixture, vertical ramp (V 255)', pV.every((p, r) => p.every((v) => v === ramp[r >> 2])), JSON.stringify(pV.map((p) => p[0])));
const pRound = special.planarPredict([0, 0, 0], [2, 2, 2], [0, 0, 0]);
check('planar rounding fixture (x 1, H 2 gives 1)', pRound[1][0] === 1 && pRound[0][0] === 0, JSON.stringify(pRound[1]));

function roundTrip(name, target, fit) {
    const total = fit(target).total;
    check(name + ' exact on a constructed block', total === 0, 'sse ' + total);
}
const tTarget = Array.from({ length: 16 }, (_, r) => special.tPalette([3, 5, 7], [9, 10, 11], 23)[(r * 3) % 4].slice());
roundTrip('T search', tTarget, special.fitT);
const hTarget = Array.from({ length: 16 }, (_, r) => special.hPalette([2, 6, 9], [13, 4, 12], 41)[(r * 5) % 4].slice());
roundTrip('H search', hTarget, special.fitH);
const g = special.planarGridColours([10, 20, 30, 40, 50, 12, 9, 60, 33]);
roundTrip('planar search', special.planarPredict(g.o, g.h, g.v), special.fitPlanar);

const name = fs.readdirSync(path.join(root, 'target', 'gltf-cache-etc2')).find((file) => file.startsWith(TEXTURE_PREFIX));
const etc2 = level0('gltf-cache-etc2', name);
const rgba8 = level0('gltf-cache-rgba8', name);
const sums = [0, 0, 0];
for (let block = 0; block < 65536; block += 1) {
    const lib = decodeEtc2Rgba8Block(etc2.subarray(block * 16, block * 16 + 16));
    const bx = block % 256;
    const by = Math.floor(block / 256);
    for (let r = 0; r < 16; r += 1) {
        const pixel = ((by * 4 + (r >> 2)) * 1024 + bx * 4 + (r & 3)) * 4;
        for (let c = 0; c < 3; c += 1) {
            const d = rgba8[pixel + c] - lib[r * 4 + c];
            sums[c] += d * d;
        }
    }
}
const psnr = (s) => +(10 * Math.log10(65025 / (s / TEXELS))).toFixed(2);
const values = sums.map(psnr);
check('baseline library decode blue ' + values[2] + ' dB, min ' + Math.min(...values) + ' dB', Math.abs(values[2] - EXPECT_BLUE_PSNR) < 1e-9 && Math.abs(Math.min(...values) - EXPECT_MIN_PSNR) < 1e-9, values.join(' / '));

const special1 = Buffer.alloc(16);
special1.writeUInt32BE(((31 << 27) | (1 << 24) | (1 << 1)) >>> 0, 8);
let refused = false;
try { decodeEtc2Rgba8Block(special1); } catch (error) { refused = /not decoded/.test(error.message); }
check('library still refuses a T, H or planar block', refused, 'no refusal');

const passed = checks.filter(Boolean).length;
console.log(passed === checks.length ? 'ALL PASS ' + passed + ' checks' : 'FAIL ' + (checks.length - passed) + ' of ' + checks.length);
process.exit(passed === checks.length ? 0 : 1);
