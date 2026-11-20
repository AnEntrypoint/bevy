const fs = require('node:fs');
const path = require('node:path');
const transfer = require(process.env.COLOUR_TRANSFER_PATH || './lib/colour-transfer.cjs');
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

function header(dir, name) {
    const { json, bin } = glbParts(fs.readFileSync(path.join(root, 'target', dir, name)));
    return ktx2Images(json, bin)[0];
}

function level0(dir, name) {
    const image = header(dir, name);
    const offset = Number(image.readBigUInt64LE(80));
    return image.subarray(offset, offset + Number(image.readBigUInt64LE(88)));
}

check('sRGB decode of 10 is 1', transfer.srgbDecodeByte(10) === 1, transfer.srgbDecodeByte(10));
check('sRGB decode of 128 is 55', transfer.srgbDecodeByte(128) === 55, transfer.srgbDecodeByte(128));
check('sRGB decode keeps 0 and 255', transfer.srgbDecodeByte(0) === 0 && transfer.srgbDecodeByte(255) === 255, [transfer.srgbDecodeByte(0), transfer.srgbDecodeByte(255)].join(' '));
check('sRGB encode of 55 is 128', transfer.srgbEncodeByte(55) === 128, transfer.srgbEncodeByte(55));
check('sRGB encode inverts decode at 128', transfer.srgbEncodeByte(transfer.srgbDecodeByte(128)) === 128, transfer.srgbEncodeByte(transfer.srgbDecodeByte(128)));

const name = fs.readdirSync(path.join(root, 'target', 'gltf-cache-etc2')).find((file) => file.startsWith(TEXTURE_PREFIX));
const etc2Format = header('gltf-cache-etc2', name).readUInt32LE(12);
const rgbaFormat = header('gltf-cache-rgba8', name).readUInt32LE(12);
check('ETC2 cache is tagged vkFormat 151 (ETC2 R8G8B8A8 sRGB)', etc2Format === 151, etc2Format);
check('RGBA8 reference is vkFormat 37 (R8G8B8A8 UNORM)', rgbaFormat === 37, rgbaFormat);

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
const values = sums.map((s) => +(10 * Math.log10(65025 / (s / TEXELS))).toFixed(2));
check('baseline library decode blue ' + values[2] + ' dB, min ' + Math.min(...values) + ' dB', Math.abs(values[2] - 28.49) < 1e-9 && Math.abs(Math.min(...values) - 28.49) < 1e-9, values.join(' / '));

const passed = checks.filter(Boolean).length;
console.log(passed === checks.length ? 'ALL PASS ' + passed + ' checks' : 'FAIL ' + (checks.length - passed) + ' of ' + checks.length);
process.exit(passed === checks.length ? 0 : 1);
