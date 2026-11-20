const fs = require('node:fs');
const path = require('node:path');
const { glbParts, ktx2Images } = require('./lib/texture-cache.cjs');
const { decodeAstc4x4Block, WEIGHT_LEVELS } = require('./lib/astc-decode.cjs');
const probe = require('./lib/astc-probe.cjs');
const { runProbe } = require('./lib/astc-probe-gpu.cjs');
const { FORMULAS, fitWeights } = require('./lib/astc-weight-fit.cjs');

const root = path.join(__dirname, '..');
const TEXTURE_PREFIX = 'value.17c7bd864f7f18268f';
const WIDTH = 1024;
const BLOCKS_PER_ROW = 256;
const WANT = 15360;
const table = WEIGHT_LEVELS[8];

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

function texelsOf(pixels, b) {
    const x0 = (b % BLOCKS_PER_ROW) * 4;
    const y0 = Math.floor(b / BLOCKS_PER_ROW) * 4;
    const texels = [];
    for (let r = 0; r < 16; r += 1) {
        const p = ((y0 + (r >> 2)) * WIDTH + x0 + (r & 3)) * 4;
        texels.push([pixels[p], pixels[p + 1], pixels[p + 2]]);
    }
    return texels;
}

async function main() {
    const started = Date.now();
    const found = cacheBlocks(WANT);
    const used = BLOCKS_PER_ROW * Math.floor(found.length / BLOCKS_PER_ROW);
    const blocks = found.slice(0, used);
    const { blob, fixtures, levels } = probe.buildFixtures(blocks, probe.WEIGHT_MODE);
    const run = await runProbe(blob, 'fit');
    console.log('gpu run pid ' + run.pid + ' outcome ' + run.outcome + ' wall ' + run.wall.toFixed(1) + ' s leftover ' + run.leftover);
    const height = used / BLOCKS_PER_ROW * 4;
    if (!run.pixels || run.pixels.length !== WIDTH * height * 4) {
        console.log('FAIL no GPU pixels: ' + run.outcome);
        process.exit(1);
    }
    const names = Object.keys(FORMULAS);
    const stats = {};
    for (const name of names) {
        stats[name] = [];
        for (let k = 0; k < levels; k += 1) stats[name].push({ n: 0, none: 0, unique: 0, multi: 0, uniqueAtTable: 0, offsets: new Map() });
    }
    let cpuMisfit = 0;
    for (let b = 0; b < fixtures.length; b += 1) {
        const e0 = decodeAstc4x4Block(probe.withWeightIndex(blocks[b], 0, probe.WEIGHT_MODE)).subarray(0, 3);
        const e1 = decodeAstc4x4Block(probe.withWeightIndex(blocks[b], levels - 1, probe.WEIGHT_MODE)).subarray(0, 3);
        const cpu = decodeAstc4x4Block(fixtures[b]);
        const k = b % levels;
        const cpuTexels = [];
        for (let r = 0; r < 16; r += 1) cpuTexels.push([cpu[r * 4], cpu[r * 4 + 1], cpu[r * 4 + 2]]);
        if (!fitWeights(e0, e1, cpuTexels, FORMULAS.spec).includes(table[k])) cpuMisfit += 1;
        const gpu = texelsOf(run.pixels, b);
        for (const name of names) {
            const s = stats[name][k];
            const fits = fitWeights(e0, e1, gpu, FORMULAS[name]);
            s.n += 1;
            if (fits.length === 0) {
                s.none += 1;
            } else if (fits.length === 1) {
                s.unique += 1;
                if (fits[0] === table[k]) s.uniqueAtTable += 1;
                const offset = fits[0] - table[k];
                s.offsets.set(offset, (s.offsets.get(offset) || 0) + 1);
            } else {
                s.multi += 1;
            }
        }
    }
    for (const name of names) {
        console.log('formula ' + name + ': level | blocks | no fit | unique fit | several fits | unique at table | offsets (fit minus table)');
        for (let k = 0; k < levels; k += 1) {
            const s = stats[name][k];
            const offsets = [...s.offsets.entries()].sort((p, q) => q[1] - p[1]).slice(0, 4).map(([o, c]) => o + ':' + c).join(' ');
            console.log('   ' + String(k).padStart(2) + ' (' + String(table[k]).padStart(2) + ') | ' + s.n + ' | ' + s.none + ' | ' + s.unique + ' | ' + s.multi + ' | ' + s.uniqueAtTable + ' | ' + offsets);
        }
    }
    console.log('CPU decode fits its table weight under the spec formula on all blocks: ' + (cpuMisfit === 0 ? 'yes' : 'no, ' + cpuMisfit + ' blocks'));
    console.log('wall ' + ((Date.now() - started) / 1000).toFixed(1) + ' s');
    process.exit(cpuMisfit === 0 ? 0 : 1);
}

main();
