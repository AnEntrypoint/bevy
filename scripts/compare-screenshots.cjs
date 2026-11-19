#!/usr/bin/env node
const fs = require('node:fs');
const path = require('node:path');
const zlib = require('node:zlib');

const repo = path.resolve(__dirname, '..');
const witnessDir = path.join(repo, 'target', 'witness');
const left = process.env.SHOTS_LEFT || path.join(witnessDir, 'neon-shots-rgba8');
const right = process.env.SHOTS_RIGHT || path.join(witnessDir, 'neon-shots-bc7');
const minPsnr = Number(process.env.VISUAL_PSNR_MIN || 30);
const outPath = path.join(witnessDir, `texture-visual-${path.basename(right).replace(/^neon-shots-/, '')}.json`);

function decodePng(file) {
    const bytes = fs.readFileSync(file);
    if (!bytes.subarray(0, 8).equals(Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]))) throw new Error('not a PNG: ' + file);
    let offset = 8;
    let width = 0;
    let height = 0;
    let colorType = 0;
    const idat = [];
    while (offset < bytes.length) {
        const length = bytes.readUInt32BE(offset);
        const type = bytes.subarray(offset + 4, offset + 8).toString('latin1');
        const data = bytes.subarray(offset + 8, offset + 8 + length);
        if (type === 'IHDR') {
            width = data.readUInt32BE(0);
            height = data.readUInt32BE(4);
            if (data[8] !== 8) throw new Error('only 8-bit PNG is supported: ' + file);
            colorType = data[9];
        } else if (type === 'IDAT') {
            idat.push(data);
        } else if (type === 'IEND') {
            break;
        }
        offset += 12 + length;
    }
    const channels = colorType === 6 ? 4 : colorType === 2 ? 3 : null;
    if (channels === null) throw new Error('unsupported PNG color type ' + colorType + ': ' + file);
    const stride = width * channels;
    const raw = zlib.inflateSync(Buffer.concat(idat));
    const pixels = Buffer.alloc(stride * height);
    for (let y = 0; y < height; y += 1) {
        const filter = raw[y * (stride + 1)];
        const line = raw.subarray(y * (stride + 1) + 1, (y + 1) * (stride + 1));
        const row = pixels.subarray(y * stride, (y + 1) * stride);
        const prior = y > 0 ? pixels.subarray((y - 1) * stride, y * stride) : Buffer.alloc(stride);
        for (let x = 0; x < stride; x += 1) {
            const a = x >= channels ? row[x - channels] : 0;
            const b = prior[x];
            const c = x >= channels ? prior[x - channels] : 0;
            let predictor = 0;
            if (filter === 1) predictor = a;
            else if (filter === 2) predictor = b;
            else if (filter === 3) predictor = (a + b) >> 1;
            else if (filter === 4) {
                const p = a + b - c;
                const pa = Math.abs(p - a);
                const pb = Math.abs(p - b);
                const pc = Math.abs(p - c);
                predictor = pa <= pb && pa <= pc ? a : pb <= pc ? b : c;
            }
            row[x] = (line[x] + predictor) & 0xff;
        }
    }
    const rgb = Buffer.alloc(width * height * 3);
    for (let i = 0; i < width * height; i += 1) {
        pixels.copy(rgb, i * 3, i * channels, i * channels + 3);
    }
    return { width, height, rgb };
}

function compare(a, b) {
    if (a.width !== b.width || a.height !== b.height) throw new Error(`size mismatch ${a.width}x${a.height} vs ${b.width}x${b.height}`);
    let squared = 0;
    let absolute = 0;
    for (let i = 0; i < a.rgb.length; i += 1) {
        const delta = a.rgb[i] - b.rgb[i];
        squared += delta * delta;
        absolute += Math.abs(delta);
    }
    const mse = squared / a.rgb.length;
    return {
        psnr: mse === 0 ? null : 10 * Math.log10((255 * 255) / mse),
        identical: mse === 0,
        meanAbs: absolute / a.rgb.length,
    };
}

function main() {
    const names = fs.existsSync(left) ? fs.readdirSync(left).filter((file) => file.endsWith('.png')).sort() : [];
    const pairs = names.filter((file) => fs.existsSync(path.join(right, file))).map((file) => {
        try {
            const result = compare(decodePng(path.join(left, file)), decodePng(path.join(right, file)));
            return { name: file.replace(/\.png$/, ''), ...result, ok: result.identical || result.psnr >= minPsnr };
        } catch (error) {
            return { name: file.replace(/\.png$/, ''), error: String(error.message), ok: false };
        }
    });
    const finite = pairs.map((pair) => pair.psnr).filter((value) => typeof value === 'number');
    const result = {
        ok: pairs.length > 0 && pairs.every((pair) => pair.ok),
        reference: path.basename(left),
        source: fs.existsSync(right) ? path.basename(fs.realpathSync(right)) : null,
        minPsnr,
        pairs: pairs.length,
        psnrMin: finite.length ? Math.min(...finite) : null,
        psnrMedian: finite.length ? finite.sort((x, y) => x - y)[Math.floor(finite.length / 2)] : null,
        failing: pairs.filter((pair) => !pair.ok).map((pair) => pair.name),
        rows: pairs,
    };
    fs.mkdirSync(witnessDir, { recursive: true });
    fs.writeFileSync(outPath, JSON.stringify(result, null, 2));
    console.log(JSON.stringify({ ok: result.ok, pairs: result.pairs, psnrMin: result.psnrMin, psnrMedian: result.psnrMedian, failing: result.failing, witness: outPath }, null, 2));
    process.exit(result.ok ? 0 : 1);
}

main();
