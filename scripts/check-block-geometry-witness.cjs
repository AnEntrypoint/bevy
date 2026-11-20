const geometry = require(process.env.BLOCK_GEOMETRY_PATH || './lib/block-geometry.cjs');
const { MODIFIERS } = require('./lib/etc1-individual.cjs');
const { decodeEtc2Rgba8Block } = require('./lib/texel-decode.cjs');
const fs = require('node:fs');
const path = require('node:path');
const { glbParts, ktx2Images } = require('./lib/texture-cache.cjs');

const root = path.join(__dirname, '..');
const TEXTURE_PREFIX = 'value.17c7bd864f7f18268f';
const TEXELS = 1048576;
const checks = [];

function check(name, ok, detail) {
    checks.push(ok);
    console.log((ok ? 'ok   ' : 'FAIL ') + name + (ok ? '' : ': ' + detail));
}

function paletteBlock(mask, bases, tables, selectors) {
    return selectors.map((m, r) => [0, 1, 2].map((c) => Math.min(255, Math.max(0, bases[mask[r]][c] * 17 + MODIFIERS[tables[mask[r]]][m]))));
}

const diagonal = Array.from({ length: 16 }, (_, r) => ((r & 3) + (r >> 2) < 3 ? 0 : 1));
const freeTarget = paletteBlock(diagonal, [[3, 5, 7], [9, 10, 11]], [2, 6], Array.from({ length: 16 }, (_, r) => (r * 7) % 4));
const freeFit = geometry.fitMask(freeTarget, diagonal);
check('free geometry block is reproduced exactly', freeFit.sse === 0, freeFit.sse);
const halfFit = geometry.fitMask(freeTarget, geometry.halfMask(0));
check('the same block cannot be reproduced by the half split', halfFit.sse > 0, halfFit.sse);

const halfTarget = paletteBlock(geometry.halfMask(1), [[2, 6, 9], [13, 4, 12]], [5, 1], Array.from({ length: 16 }, (_, r) => (r * 5) % 4));
const halfExact = [0, 1].map((flip) => geometry.fitMask(halfTarget, geometry.halfMask(flip)).sse);
check('a half-split block is reproduced exactly by the half split', Math.min(...halfExact) === 0, halfExact.join(' '));
check('subset of raster texel 2 is 1 for flip 0 and 0 for flip 1', geometry.subsetOf(0, 2) === 1 && geometry.subsetOf(1, 2) === 0, geometry.subsetOf(0, 2) + ' ' + geometry.subsetOf(1, 2));
const candidates = geometry.freeCandidates(freeTarget, 3);
check('free candidates are 3 distinct 16-texel masks', candidates.length === 3 && candidates.every((m) => m.length === 16) && new Set(candidates.map((m) => m.join(''))).size === 3, candidates.length);

const name = fs.readdirSync(path.join(root, 'target', 'gltf-cache-etc2')).find((file) => file.startsWith(TEXTURE_PREFIX));
const level0 = (dir) => { const { json, bin } = glbParts(fs.readFileSync(path.join(root, 'target', dir, name))); const image = ktx2Images(json, bin)[0]; const o = Number(image.readBigUInt64LE(80)); return image.subarray(o, o + Number(image.readBigUInt64LE(88))); };
const etc2 = level0('gltf-cache-etc2');
const rgba8 = level0('gltf-cache-rgba8');
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
const values = sums.map((s) => +(10 * Math.log10(65025 / (s / TEXELS))).toFixed(2));
check('baseline library decode blue ' + values[2] + ' dB, min ' + Math.min(...values) + ' dB', Math.abs(values[2] - 28.49) < 1e-9 && Math.abs(Math.min(...values) - 28.49) < 1e-9, values.join(' / '));

const passed = checks.filter(Boolean).length;
console.log(passed === checks.length ? 'ALL PASS ' + passed + ' checks' : 'FAIL ' + (checks.length - passed) + ' of ' + checks.length);
process.exit(passed === checks.length ? 0 : 1);
