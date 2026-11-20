const fs = require('node:fs');
const path = require('node:path');
const individual = require(process.env.ETC1_INDIVIDUAL_PATH || './lib/etc1-individual.cjs');
const { glbParts, ktx2Images } = require('./lib/texture-cache.cjs');
const { decodeEtc2Rgba8Block } = require('./lib/texel-decode.cjs');

const root = path.join(__dirname, '..');
const TEXTURE_PREFIX = 'value.17c7bd864f7f18268f';
const TEXELS = 1048576;
const checks = [];

function check(name, ok, detail) {
    checks.push(ok);
    console.log((ok ? 'ok   ' : 'FAIL ') + name + (ok ? '' : ': ' + detail));
}

function level0(dir, name) {
    const { json, bin } = glbParts(fs.readFileSync(path.join(root, 'target', dir, name)));
    const image = ktx2Images(json, bin)[0];
    const offset = Number(image.readBigUInt64LE(80));
    const length = Number(image.readBigUInt64LE(88));
    return image.subarray(offset, offset + length);
}

const fixtureHi = individual.encodeIndividual({ flip: 1, bases: [[1, 2, 3], [4, 5, 6]], tables: [5, 2], selectors: new Array(16).fill(0) }).readUInt32BE(0);
check('individual hi word fixture (flip 1, tables 5 and 2, bases 1-2-3 and 4-5-6)', fixtureHi === 0x142536a9, '0x' + fixtureHi.toString(16));

let seed = 12345;
const rand = (n) => { seed = (seed * 1103515245 + 12345) & 0x7fffffff; return (seed >>> 16) % n; };
for (const flip of [0, 1]) {
    const bases = [[rand(16), rand(16), rand(16)], [rand(16), rand(16), rand(16)]];
    const tables = [rand(8), rand(8)];
    const selectors = Array.from({ length: 16 }, () => rand(4));
    const pred = individual.predictIndividual(flip, bases, tables, selectors);
    const colour = individual.encodeIndividual({ flip, bases, tables, selectors });
    const lib = decodeEtc2Rgba8Block(Buffer.concat([Buffer.alloc(8), colour]));
    let mismatch = 0;
    for (let r = 0; r < 16; r += 1) for (let c = 0; c < 3; c += 1) if (lib[r * 4 + c] !== pred[r][c]) mismatch += 1;
    check('library decode of an encoded individual block, flip ' + flip, mismatch === 0, mismatch + ' channel mismatches');
    const target = pred.map((p) => p.slice());
    const fitTotal = individual.fitIndividual(target).total;
    check('individual search exact on a constructed block, flip ' + flip, fitTotal === 0, 'sse ' + fitTotal);
}

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
check('baseline library decode blue ' + values[2] + ' dB, min ' + Math.min(...values) + ' dB', Math.abs(values[2] - 28.49) < 1e-9 && Math.abs(Math.min(...values) - 28.49) < 1e-9, values.join(' / '));

const passed = checks.filter(Boolean).length;
console.log(passed === checks.length ? 'ALL PASS ' + passed + ' checks' : 'FAIL ' + (checks.length - passed) + ' of ' + checks.length);
process.exit(passed === checks.length ? 0 : 1);
