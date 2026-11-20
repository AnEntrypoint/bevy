const fs = require('node:fs');
const path = require('node:path');
const { glbParts, ktx2Images } = require('./lib/texture-cache.cjs');
const { decodeAstc4x4Block } = require('./lib/astc-decode.cjs');
const probe = require('./lib/astc-probe.cjs');
const { runProbe } = require('./lib/astc-probe-gpu.cjs');

const root = path.join(__dirname, '..');
const TEXTURE_PREFIX = 'value.17c7bd864f7f18268f';
const WIDTH = 1024;
const BLOCKS_PER_ROW = 256;
const WANT = 16384;

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

async function main() {
    const started = Date.now();
    const count = BLOCKS_PER_ROW * Math.floor(WANT / BLOCKS_PER_ROW);
    const blocks = cacheBlocks(count);
    const used = BLOCKS_PER_ROW * Math.floor(blocks.length / BLOCKS_PER_ROW);
    const { blob, fixtures, levels } = probe.buildFixtures(blocks.slice(0, used), probe.WEIGHT_MODE);
    console.log('fixtures ' + used + ' from cache mode 0x242, levels ' + levels + ', texture ' + WIDTH + ' x ' + (used / BLOCKS_PER_ROW * 4));
    const run = await runProbe(blob, 'measure');
    console.log('gpu run pid ' + run.pid + ' outcome ' + run.outcome + ' wall ' + run.wall.toFixed(1) + ' s tree ' + run.treeSize + ' leftover ' + run.leftover);
    const height = used / BLOCKS_PER_ROW * 4;
    if (!run.pixels || run.pixels.length !== WIDTH * height * 4) {
        console.log('FAIL no GPU pixels: ' + run.outcome + ' ' + JSON.stringify(run.result).slice(0, 300));
        process.exit(1);
    }
    const stats = [];
    for (let k = 0; k < levels; k += 1) stats.push({ blocks: 0, badBlocks: 0, texels: 0, channel: [0, 0, 0], one: 0, more: 0 });
    for (let b = 0; b < fixtures.length; b += 1) {
        const k = b % levels;
        const cpu = decodeAstc4x4Block(fixtures[b]);
        const x0 = (b % BLOCKS_PER_ROW) * 4;
        const y0 = Math.floor(b / BLOCKS_PER_ROW) * 4;
        const s = stats[k];
        s.blocks += 1;
        let bad = false;
        for (let r = 0; r < 16; r += 1) {
            const p = ((y0 + (r >> 2)) * WIDTH + x0 + (r & 3)) * 4;
            for (let c = 0; c < 3; c += 1) {
                const d = run.pixels[p + c] - cpu[r * 4 + c];
                if (d !== 0) {
                    bad = true;
                    s.channel[c] += 1;
                    if (Math.abs(d) === 1) s.one += 1; else s.more += 1;
                }
            }
            if (bad) s.texels += 1;
        }
        if (bad) s.badBlocks += 1;
    }
    console.log('level | blocks | blocks off | R G B texel-channels off | |d|=1 | |d|>1');
    let totalBad = 0;
    let totalBlocks = 0;
    for (let k = 0; k < levels; k += 1) {
        const s = stats[k];
        totalBad += s.badBlocks;
        totalBlocks += s.blocks;
        console.log(String(k).padStart(2) + '    | ' + s.blocks + ' | ' + s.badBlocks + ' | ' + s.channel.join(' ') + ' | ' + s.one + ' | ' + s.more);
    }
    console.log('blocks off overall ' + totalBad + ' of ' + totalBlocks + ' (' + (100 * totalBad / totalBlocks).toFixed(2) + '%)');
    console.log('wall ' + ((Date.now() - started) / 1000).toFixed(1) + ' s');
    process.exit(0);
}

main();
