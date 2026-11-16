#!/usr/bin/env node
const fs = require('node:fs');
const path = require('node:path');
const zlib = require('node:zlib');
const { size, cell, radius, normal } = require('./lib/normal-map-bumps.cjs');

const repo = path.resolve(__dirname, '..');
const glbPath = path.resolve(process.argv[2] || path.join(repo, 'testdata', 'normal-map', 'normal-map.glb'));
const ENCODING_TOLERANCE = 2;
const PNG_SIGNATURE = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);

function readGlb(bytes) {
    if (bytes.readUInt32LE(0) !== 0x46546c67) throw new Error('not a GLB file');
    let json = null;
    let bin = null;
    let offset = 12;
    while (offset < bytes.length) {
        const length = bytes.readUInt32LE(offset);
        const type = bytes.readUInt32LE(offset + 4);
        const data = bytes.subarray(offset + 8, offset + 8 + length);
        if (type === 0x4e4f534a) json = JSON.parse(data.toString('utf8'));
        if (type === 0x004e4942) bin = data;
        offset += 8 + length;
    }
    if (!json || !bin) throw new Error('GLB needs both JSON and BIN chunks');
    return { json, bin };
}

function bufferViewBytes(json, bin, index) {
    const view = json.bufferViews[index];
    const start = view.byteOffset || 0;
    return bin.subarray(start, start + view.byteLength);
}

function decodePng(bytes) {
    if (!bytes.subarray(0, 8).equals(PNG_SIGNATURE)) throw new Error('embedded image is not a PNG');
    let header = null;
    const idat = [];
    let offset = 8;
    while (offset < bytes.length) {
        const length = bytes.readUInt32BE(offset);
        const type = bytes.subarray(offset + 4, offset + 8).toString('latin1');
        const data = bytes.subarray(offset + 8, offset + 8 + length);
        if (type === 'IHDR') {
            header = { width: data.readUInt32BE(0), height: data.readUInt32BE(4), bitDepth: data[8], colorType: data[9], interlace: data[12] };
        }
        if (type === 'IDAT') idat.push(data);
        offset += 12 + length;
        if (type === 'IEND') break;
    }
    if (!header) throw new Error('PNG has no IHDR');
    if (header.bitDepth !== 8 || header.colorType !== 2 || header.interlace !== 0) {
        throw new Error(`PNG must be 8-bit RGB non-interlaced, got bit depth ${header.bitDepth}, colour type ${header.colorType}, interlace ${header.interlace}`);
    }
    const stride = header.width * 3;
    const raw = zlib.inflateSync(Buffer.concat(idat));
    if (raw.length !== (stride + 1) * header.height) throw new Error('PNG data length does not match its dimensions');
    const pixels = Buffer.alloc(stride * header.height);
    for (let y = 0; y < header.height; y += 1) {
        const filter = raw[y * (stride + 1)];
        if (filter !== 0) throw new Error(`PNG row ${y} uses filter ${filter}; this checker reads filter 0 only`);
        raw.copy(pixels, y * stride, y * (stride + 1) + 1, (y + 1) * (stride + 1));
    }
    return { width: header.width, height: header.height, pixels };
}

function floatAccessor(json, bin, index, components) {
    const accessor = json.accessors[index];
    if (accessor.componentType !== 5126) throw new Error(`accessor ${index} is not FLOAT`);
    const view = json.bufferViews[accessor.bufferView];
    const base = (view.byteOffset || 0) + (accessor.byteOffset || 0);
    const stride = view.byteStride || components * 4;
    const values = [];
    for (let i = 0; i < accessor.count; i += 1) {
        const element = [];
        for (let c = 0; c < components; c += 1) element.push(bin.readFloatLE(base + i * stride + c * 4));
        values.push(element);
    }
    return values;
}

function domeCentre(coordinate) {
    return cell / 2 + Math.round((coordinate - cell / 2) / cell) * cell;
}

function main() {
    const { json, bin } = readGlb(fs.readFileSync(glbPath));
    const primitive = json.meshes[0].primitives[0];
    const material = json.materials[primitive.material];
    const image = json.images[json.textures[material.normalTexture.index].source];
    const { width, height, pixels } = decodePng(bufferViewBytes(json, bin, image.bufferView));
    if (width !== size || height !== size) throw new Error(`normal map is ${width}x${height}, expected ${size}x${size}`);

    let encodingMismatches = 0;
    let slopePixels = 0;
    let conventionViolations = 0;
    for (let y = 0; y < height; y += 1) {
        for (let x = 0; x < width; x += 1) {
            const offset = (y * width + x) * 3;
            const decoded = [pixels[offset], pixels[offset + 1], pixels[offset + 2]];
            const reference = normal(x, y).map((component) => Math.round((component * 0.5 + 0.5) * 255));
            if (decoded.some((value, c) => Math.abs(value - reference[c]) > ENCODING_TOLERANCE)) encodingMismatches += 1;

            const dx = x - domeCentre(x);
            const dy = y - domeCentre(y);
            const distance = Math.hypot(dx, dy);
            if (distance < radius * 0.25 || distance > radius * 0.9) continue;
            slopePixels += 1;
            if (dx !== 0 && (decoded[0] > 127.5) !== (dx > 0)) conventionViolations += 1;
            if (dy !== 0 && (decoded[1] > 127.5) !== (dy < 0)) conventionViolations += 1;
        }
    }

    const positions = floatAccessor(json, bin, primitive.attributes.POSITION, 3);
    const uvs = floatAccessor(json, bin, primitive.attributes.TEXCOORD_0, 2);
    let geometryViolations = 0;
    positions.forEach(([x, y], i) => {
        const [u, v] = uvs[i];
        if (x > 0 && !(u > 0.5)) geometryViolations += 1;
        if (y > 0 && !(v < 0.5)) geometryViolations += 1;
    });

    const ok = encodingMismatches === 0 && conventionViolations === 0 && geometryViolations === 0 && slopePixels > 0;
    console.log(JSON.stringify({
        kind: 'normal-map-sign',
        glb: path.relative(repo, glbPath),
        encodingMismatches,
        slopePixels,
        conventionViolations,
        geometryViolations,
        ok,
    }));
    process.exit(ok ? 0 : 1);
}

try {
    main();
} catch (error) {
    console.log(JSON.stringify({ kind: 'normal-map-sign', glb: path.relative(repo, glbPath), ok: false, error: error.message }));
    process.exit(1);
}
