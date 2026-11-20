const { chooseBest, totalOf } = require(process.env.BLOCK_MODES_PATH || './lib/block-modes.cjs');
const { predictIndividual, fitIndividual } = require('./lib/etc1-individual.cjs');
const { glbParts, ktx2Images } = require('./lib/texture-cache.cjs');
const { decodeEtc2Rgba8Block } = require('./lib/texel-decode.cjs');
const fs = require('node:fs');
const path = require('node:path');

const root = path.join(__dirname, '..');
const TEXTURE_PREFIX = 'value.17c7bd864f7f18268f';
const TEXELS = 1048576;
const checks = [];

function check(name, ok, detail) {
    checks.push(ok);
    console.log((ok ? 'ok   ' : 'FAIL ') + name + (ok ? '' : ': ' + detail));
}

function channelSse(pred, target) {
    const e = [0, 0, 0];
    for (let r = 0; r < 16; r += 1) {
        for (let c = 0; c < 3; c += 1) e[c] += (target[r][c] - pred[r][c]) ** 2;
    }
    return e;
}

check('totalOf sums the three channels', totalOf([1, 2, 3]) === 6, totalOf([1, 2, 3]));
const pick = chooseBest([{ name: 'a', errors: [5, 5, 5] }, { name: 'b', errors: [1, 1, 1] }, { name: 'c', errors: [9, 0, 0] }]);
check('chooseBest picks the lowest total (b, total 3)', pick.name === 'b', pick.name);
const tie = chooseBest([{ name: 'x', errors: [2, 2, 2] }, { name: 'y', errors: [1, 2, 3] }]);
check('chooseBest keeps the first option on a tie', tie.name === 'x', tie.name);

const bases = [[3, 9, 12], [14, 2, 7]];
const tables = [5, 2];
const selectors = Array.from({ length: 16 }, (_, r) => (r * 7) % 4);
const target = predictIndividual(1, bases, tables, selectors);
const fit = fitIndividual(target);
const exact = channelSse(fit.pred, target).reduce((a, b) => a + b, 0);
check('individual search is exact on a constructed block (the arm that should win)', exact === 0, exact);
const grey = Array.from({ length: 16 }, () => [128, 128, 128]);
const won = chooseBest([{ name: 'grey', errors: channelSse(grey, target) }, { name: 'individual', errors: channelSse(fit.pred, target) }]);
check('chooseBest picks the exact individual block over a grey cache block', won.name === 'individual', won.name);

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
