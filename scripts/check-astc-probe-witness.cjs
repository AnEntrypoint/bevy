const fs = require('node:fs');
const path = require('node:path');
const { glbParts, ktx2Images } = require('./lib/texture-cache.cjs');
const { decodeAstc4x4Block, readBits, reverseBits } = require('./lib/astc-decode.cjs');
const probe = require(process.env.ASTC_PROBE_PATH || './lib/astc-probe.cjs');
const { runProbe } = require('./lib/astc-probe-gpu.cjs');

const root = path.join(__dirname, '..');
const TEXTURE_PREFIX = 'value.17c7bd864f7f18268f';
const WIDTH = 1024;
const FIXTURES = 256;
const checks = [];

function check(name, ok, detail) {
    checks.push(ok);
    console.log((ok ? 'ok   ' : 'FAIL ') + name + (ok ? '' : ': ' + detail));
}

function cacheBlocks(count) {
    const dir = path.join(root, 'target', 'gltf-cache-astc');
    const name = fs.readdirSync(dir).find((file) => file.startsWith(TEXTURE_PREFIX));
    const { json, bin } = glbParts(fs.readFileSync(path.join(dir, name)));
    const image = ktx2Images(json, bin)[0];
    const offset = Number(image.readBigUInt64LE(80));
    const level0 = image.subarray(offset, offset + Number(image.readBigUInt64LE(88)));
    const blocks = [];
    for (let b = 0; b < 65536 && blocks.length < count; b += 4) {
        const block = Uint8Array.from(level0.subarray(b * 16, b * 16 + 16));
        if (((block[0] | (block[1] << 8)) & 0x7ff) === probe.WEIGHT_MODE) blocks.push(block);
    }
    return blocks;
}

const blocks = cacheBlocks(FIXTURES);
const levels = probe.weightIndexLevels(probe.WEIGHT_MODE);
const bits = probe.weightLevelBits(probe.WEIGHT_MODE);
check('fixture blocks come from the cache in mode 0x242 (' + blocks.length + ' of ' + FIXTURES + ')', blocks.length === FIXTURES, blocks.length);

let roundTrip = true;
for (let trial = 0; trial < 500; trial += 1) {
    const count = 1 + (trial % 8);
    const value = (trial * 37) & ((1 << count) - 1);
    const bytes = new Uint8Array(16);
    probe.setBits(bytes, trial % 40, count, value);
    if (readBits(bytes, trial % 40, count) !== value) roundTrip = false;
}
check('setBits round-trips through readBits for 500 values', roundTrip);

let weightsOk = true;
let untouched = true;
let constant = true;
for (let k = 0; k < levels; k += 1) {
    const fixture = probe.withWeightIndex(blocks[0], k, probe.WEIGHT_MODE);
    for (let i = 0; i < 16; i += 1) {
        const stored = readBits(fixture, 128 - bits * (i + 1), bits);
        if (reverseBits(stored, bits) !== k) weightsOk = false;
    }
    for (let p = 0; p < 128 - 16 * bits; p += 1) {
        if (readBits(fixture, p, 1) !== readBits(blocks[0], p, 1)) untouched = false;
    }
    const cpu = decodeAstc4x4Block(fixture);
    for (let t = 1; t < 16; t += 1) {
        for (let c = 0; c < 3; c += 1) if (cpu[t * 4 + c] !== cpu[c]) constant = false;
    }
}
check('withWeightIndex stores the index in every weight, for all ' + levels + ' levels', weightsOk);
check('withWeightIndex leaves the endpoint bits untouched', untouched);
check('CPU decode of a fixture is one colour across its 16 texels, for every level', constant);

(async () => {
    const { blob, fixtures } = probe.buildFixtures(blocks, probe.WEIGHT_MODE);
    const run = await runProbe(blob, 'witness');
    console.log('gpu run pid ' + run.pid + ' outcome ' + run.outcome + ' wall ' + run.wall.toFixed(1) + ' s tree ' + run.treeSize + ' leftover ' + run.leftover);
    const pixelsOk = run.outcome === 'done' && run.pixels && run.pixels.length === WIDTH * 4 * 4;
    check('GPU readback finished with ' + WIDTH + ' x 4 texels', pixelsOk, run.outcome + ' ' + (run.pixels ? run.pixels.length : 0));
    if (pixelsOk) {
        const at = (x, y) => (y * WIDTH + x) * 4;
        let flatBad = 0;
        let nonzero = 0;
        for (let b = 0; b < fixtures.length; b += 1) {
            const x0 = (b % 256) * 4;
            const y0 = Math.floor(b / 256) * 4;
            const ref = at(x0, y0);
            let flat = true;
            for (let r = 0; r < 16; r += 1) {
                const p = at(x0 + (r & 3), y0 + (r >> 2));
                if (run.pixels[p] || run.pixels[p + 1] || run.pixels[p + 2]) nonzero += 1;
                for (let c = 0; c < 3; c += 1) if (run.pixels[p + c] !== run.pixels[ref + c]) flat = false;
            }
            if (!flat) flatBad += 1;
        }
        check('GPU shows one colour across the 16 texels of every fixture block', flatBad === 0, flatBad + ' blocks not flat');
        check('GPU output carries colour (non-zero texels: ' + nonzero + ')', nonzero > 0, nonzero);
    }
    const passed = checks.filter(Boolean).length;
    console.log(passed === checks.length ? 'ALL PASS ' + passed + ' checks' : 'FAIL ' + (checks.length - passed) + ' of ' + checks.length);
    process.exit(passed === checks.length ? 0 : 1);
})();
