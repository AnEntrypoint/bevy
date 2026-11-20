const endpoint = require(process.env.E4_PATH || './check-etc2-endpoint-search.cjs');
const { glbParts, ktx2Images } = require('./lib/texture-cache.cjs');
const { decodeEtc2Rgba8Block } = require('./lib/texel-decode.cjs');
const fs = require('node:fs');
const path = require('node:path');

const root = path.join(__dirname, '..');
const TEXTURE_PREFIX = 'value.17c7bd864f7f18268f';
const W = 1024;
const BLOCKS_PER_ROW = 256;
const TEXELS = 1048576;
const BLUE = 2;
const TOP_BLOCK = 250;
const SCAN_BLOCKS = 4000;
const TOP_CHECKS = 300;
const checks = [];

function check(name, ok, detail) {
    checks.push(ok);
    console.log((ok ? 'ok   ' : 'FAIL ') + name + (ok ? '' : ': ' + detail));
}

function level0(dir, name) {
    const { json, bin } = glbParts(fs.readFileSync(path.join(root, 'target', dir, name)));
    const image = ktx2Images(json, bin)[0];
    const offset = Number(image.readBigUInt64LE(80));
    return image.subarray(offset, offset + Number(image.readBigUInt64LE(88)));
}

const name = fs.readdirSync(path.join(root, 'target', 'gltf-cache-etc2')).find((file) => file.startsWith(TEXTURE_PREFIX));
const etc2 = level0('gltf-cache-etc2', name);
const rgba8 = level0('gltf-cache-rgba8', name);

function targetOf(block) {
    const bx = block % BLOCKS_PER_ROW;
    const by = Math.floor(block / BLOCKS_PER_ROW);
    const target = [];
    for (let r = 0; r < 16; r += 1) {
        const pixel = ((by * 4 + (r >> 2)) * W + bx * 4 + (r & 3)) * 4;
        target.push([rgba8[pixel], rgba8[pixel + 1], rgba8[pixel + 2]]);
    }
    return target;
}

let diffBlocks = 0;
let reencodeSame = 0;
let evalMismatch = 0;
let topChecked = 0;
let searchMismatch = 0;
for (let block = 0; block < SCAN_BLOCKS; block += 1) {
    const bytes = etc2.subarray(block * 16, block * 16 + 16);
    const colour = bytes.subarray(8, 16);
    const hi0 = endpoint.word(colour, 0);
    const lo0 = endpoint.word(colour, 4);
    if (((hi0 >>> 1) & 1) === 0) continue;
    diffBlocks += 1;
    const state0 = endpoint.parseDiff(colour);
    const kBase = endpoint.baselineSelectors(lo0);
    if (endpoint.encode(state0, hi0, lo0, kBase).equals(colour)) reencodeSame += 1;
    const libRgb = endpoint.toRgb(decodeEtc2Rgba8Block(bytes));
    const predBase = endpoint.predict(state0, kBase);
    for (let r = 0; r < 16; r += 1) {
        for (let c = 0; c < 3; c += 1) if (predBase[r][c] !== libRgb[r][c]) evalMismatch += 1;
    }
    const target = targetOf(block);
    const mean = target.reduce((acc, v) => acc + v[BLUE], 0) / 16;
    if (mean < TOP_BLOCK || topChecked >= TOP_CHECKS) continue;
    topChecked += 1;
    const result = endpoint.runScenario('e4', state0, target);
    const newColour = endpoint.encode(result.state, hi0, lo0, result.kByT);
    const newRgb = endpoint.toRgb(decodeEtc2Rgba8Block(Buffer.concat([bytes.subarray(0, 8), newColour])));
    const pred = endpoint.predict(result.state, result.kByT);
    for (let r = 0; r < 16; r += 1) {
        for (let c = 0; c < 3; c += 1) if (pred[r][c] !== newRgb[r][c]) searchMismatch += 1;
    }
}

check('differential blocks found in the scan', diffBlocks > 0, diffBlocks);
check('re-encoding the transcoder bits is byte-identical (' + reencodeSame + ' of ' + diffBlocks + ')', reencodeSame === diffBlocks, reencodeSame + ' of ' + diffBlocks);
check('evaluator equals the library decode on every differential texel', evalMismatch === 0, evalMismatch + ' mismatches');
check('E4 top-end blocks checked: ' + topChecked, topChecked >= 100, topChecked);
check('E4 output decodes to its own prediction on every checked texel', searchMismatch === 0, searchMismatch + ' mismatches');

const sums = [0, 0, 0];
for (let block = 0; block < 65536; block += 1) {
    const lib = decodeEtc2Rgba8Block(etc2.subarray(block * 16, block * 16 + 16));
    const target = targetOf(block);
    for (let r = 0; r < 16; r += 1) {
        for (let c = 0; c < 3; c += 1) sums[c] += (target[r][c] - lib[r * 4 + c]) ** 2;
    }
}
const values = sums.map((s) => +(10 * Math.log10(65025 / (s / TEXELS))).toFixed(2));
check('baseline library decode blue ' + values[2] + ' dB, min ' + Math.min(...values) + ' dB', Math.abs(values[2] - 28.49) < 1e-9 && Math.abs(Math.min(...values) - 28.49) < 1e-9, values.join(' / '));

const passed = checks.filter(Boolean).length;
console.log(passed === checks.length ? 'ALL PASS ' + passed + ' checks' : 'FAIL ' + (checks.length - passed) + ' of ' + checks.length);
process.exit(passed === checks.length ? 0 : 1);
