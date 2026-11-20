const fs = require('node:fs');
const path = require('node:path');
const { decodeEtc2Rgba8Block } = require('./lib/texel-decode.cjs');
const { glbParts, ktx2Images } = require('./lib/texture-cache.cjs');

const OPAQUE = 'ff00000000000000';
const MIN_PSNR = 35;
const W = 1024;
const BLOCKS_PER_ROW = 256;
const cacheRoot = path.join(__dirname, '..', 'target');

function uniform(rgba) {
    return (texel) => rgba;
}

const fixtures = [
    {
        name: 'individual black with every index 01 is +8 everywhere',
        colour: '000000000000ffff',
        expect: uniform([8, 8, 8, 255]),
    },
    {
        name: 'individual sub-block split: left red 255, right black, both +2',
        colour: 'f000000000000000',
        expect: (texel) => ((texel & 3) < 2 ? [255, 2, 2, 255] : [2, 2, 2, 255]),
    },
    {
        name: 'differential base 16 with zero delta and index 01',
        colour: '800000020000ffff',
        expect: uniform([140, 8, 8, 255]),
    },
    {
        name: 'differential delta -4 splits left 134 and right 101',
        colour: '8400000200000000',
        expect: (texel) => ((texel & 3) < 2 ? [134, 2, 2, 255] : [101, 2, 2, 255]),
    },
    {
        name: 'flip bit switches the split to top and bottom',
        colour: '8400000300000000',
        expect: (texel) => ((texel >> 2) < 2 ? [134, 2, 2, 255] : [101, 2, 2, 255]),
    },
    {
        name: 'index bit for texel 1 lands on column 0 row 1 (texel 4)',
        colour: '0000000000000002',
        expect: (texel) => (texel === 4 ? [8, 8, 8, 255] : [2, 2, 2, 255]),
    },
    {
        name: 'table 7 on the left half gives +47 and right half keeps table 0',
        colour: '000000e000000000',
        expect: (texel) => ((texel & 3) < 2 ? [47, 47, 47, 255] : [2, 2, 2, 255]),
    },
    {
        name: 'EAC table 0 index 7 at texel 0 gives alpha 14, the rest 0',
        alpha: '0010e00000000000',
        colour: '0000000000000000',
        expect: (texel) => [2, 2, 2, texel === 0 ? 14 : 0],
    },
];

let passed = 0;
for (const fixture of fixtures) {
    const hex = (fixture.alpha || OPAQUE) + fixture.colour;
    const out = decodeEtc2Rgba8Block(Buffer.from(hex, 'hex'));
    let ok = true;
    for (let texel = 0; texel < 16; texel += 1) {
        const want = fixture.expect(texel);
        for (let channel = 0; channel < 4; channel += 1) {
            if (out[texel * 4 + channel] !== want[channel]) ok = false;
        }
    }
    console.log((ok ? 'ok   ' : 'FAIL ') + fixture.name);
    if (ok) passed += 1;
}

function level0(image) {
    const offset = Number(image.readBigUInt64LE(80));
    const length = Number(image.readBigUInt64LE(88));
    return image.subarray(offset, offset + length);
}

function firstImage(file) {
    const { json, bin } = glbParts(fs.readFileSync(file));
    const images = ktx2Images(json, bin);
    return images.length ? images[0] : null;
}

function cachePsnr(etc2, rgba8) {
    const sq = [0, 0, 0];
    let samples = 0;
    let unsupported = 0;
    for (let index = 0; index < etc2.length / 16; index += 1) {
        let decoded;
        try {
            decoded = decodeEtc2Rgba8Block(etc2.subarray(index * 16, index * 16 + 16));
        } catch (error) {
            unsupported += 1;
            continue;
        }
        const bx = index % BLOCKS_PER_ROW;
        const by = Math.floor(index / BLOCKS_PER_ROW);
        for (let y = 0; y < 4; y += 1) {
            for (let x = 0; x < 4; x += 1) {
                const reference = ((by * 4 + y) * W + bx * 4 + x) * 4;
                const texel = (y * 4 + x) * 4;
                for (let channel = 0; channel < 3; channel += 1) {
                    const diff = decoded[texel + channel] - rgba8[reference + channel];
                    sq[channel] += diff * diff;
                }
                samples += 1;
            }
        }
    }
    const psnr = sq.map((value) => +(10 * Math.log10(65025 / (value / samples))).toFixed(2));
    return { psnr, unsupported };
}

const etc2Dir = path.join(cacheRoot, 'gltf-cache-etc2');
const rgba8Dir = path.join(cacheRoot, 'gltf-cache-rgba8');
const textures = [];
for (const name of fs.readdirSync(etc2Dir).filter((file) => file.startsWith('value.'))) {
    const etc2Image = firstImage(path.join(etc2Dir, name));
    if (!etc2Image) continue;
    const rgbaImage = firstImage(path.join(rgba8Dir, name));
    if (!rgbaImage) continue;
    textures.push({ name, ...cachePsnr(level0(etc2Image), level0(rgbaImage)) });
}
const minPsnr = Math.min(...textures.flatMap((texture) => texture.psnr));
const unsupportedTotal = textures.reduce((sum, texture) => sum + texture.unsupported, 0);
const cacheOk = textures.length > 0 && minPsnr >= MIN_PSNR && unsupportedTotal === 0;
console.log('CACHE level 0 RGB PSNR min ' + minPsnr + ' dB over ' + textures.length + ' textures (bar ' + MIN_PSNR + ' dB, unsupported ' + unsupportedTotal + '): ' + (cacheOk ? 'CLEAR' : 'OPEN (below bar, bar unchanged)'));

const fixturesOk = passed === fixtures.length;
console.log('texel-decode fixtures ' + passed + ' of ' + fixtures.length + ' ok; cache ' + (cacheOk ? 'CLEAR' : 'OPEN'));
process.exit(fixturesOk ? 0 : 1);
