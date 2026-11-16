#!/usr/bin/env node
const fs = require('node:fs');
const path = require('node:path');
const zlib = require('node:zlib');

const out = path.resolve(__dirname, '..', 'testdata', 'normal-map', 'normal-map.glb');
const size = 256;
const cell = 64;
const radius = 26;
const strength = 12;

const crcTable = Array.from({ length: 256 }, (_, n) => {
    let c = n;
    for (let k = 0; k < 8; k += 1) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    return c >>> 0;
});

function crc32(bytes) {
    let crc = 0xffffffff;
    for (const byte of bytes) crc = crcTable[(crc ^ byte) & 0xff] ^ (crc >>> 8);
    return (crc ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
    const typed = Buffer.concat([Buffer.from(type, 'latin1'), data]);
    const length = Buffer.alloc(4);
    length.writeUInt32BE(data.length, 0);
    const checksum = Buffer.alloc(4);
    checksum.writeUInt32BE(crc32(typed), 0);
    return Buffer.concat([length, typed, checksum]);
}

function png(width, height, raw) {
    const header = Buffer.alloc(13);
    header.writeUInt32BE(width, 0);
    header.writeUInt32BE(height, 4);
    header[8] = 8;
    header[9] = 2;
    return Buffer.concat([
        Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
        chunk('IHDR', header),
        chunk('IDAT', zlib.deflateSync(raw)),
        chunk('IEND', Buffer.alloc(0)),
    ]);
}

function bump(x, y) {
    const u = ((x % cell) + cell) % cell - cell / 2;
    const v = ((y % cell) + cell) % cell - cell / 2;
    const d2 = u * u + v * v;
    return d2 < radius * radius ? Math.sqrt(1 - d2 / (radius * radius)) : 0;
}

function normal(x, y) {
    const dx = (bump(x + 1, y) - bump(x - 1, y)) / 2;
    const dy = (bump(x, y + 1) - bump(x, y - 1)) / 2;
    const nx = -dx * strength;
    const ny = -dy * strength;
    const length = Math.hypot(nx, ny, 1);
    return [nx / length, ny / length, 1 / length];
}

function bumpMapRows() {
    const rows = [];
    for (let y = 0; y < size; y += 1) {
        const row = Buffer.alloc(1 + size * 3);
        for (let x = 0; x < size; x += 1) {
            const n = normal(x, y);
            for (let c = 0; c < 3; c += 1) row[1 + x * 3 + c] = Math.round((n[c] * 0.5 + 0.5) * 255);
        }
        rows.push(row);
    }
    return Buffer.concat(rows);
}

function floatBytes(values) {
    const buffer = Buffer.alloc(values.length * 4);
    values.forEach((value, index) => buffer.writeFloatLE(value, index * 4));
    return buffer;
}

function int16Bytes(values) {
    const buffer = Buffer.alloc(values.length * 2);
    values.forEach((value, index) => buffer.writeInt16LE(value, index * 2));
    return buffer;
}

function uint16Bytes(values) {
    const buffer = Buffer.alloc(values.length * 2);
    values.forEach((value, index) => buffer.writeUInt16LE(value, index * 2));
    return buffer;
}

const positions = [-0.5, -0.5, 0, 0.5, -0.5, 0, 0.5, 0.5, 0, -0.5, 0.5, 0];
const normals = [0, 0, 1, 0, 0, 1, 0, 0, 1, 0, 0, 1];
const uvs = [0, 1, 1, 1, 1, 0, 0, 0];
const tangents = [32767, 0, 0, 32767, 32767, 0, 0, 32767, 32767, 0, 0, 32767, 32767, 0, 0, 32767];
const indices = [0, 1, 2, 0, 2, 3];

const pad4 = (length) => (4 - (length % 4)) % 4;
let bin = Buffer.alloc(0);
const views = [];

function addView(data, target) {
    bin = Buffer.concat([bin, Buffer.alloc(pad4(bin.length)), data]);
    const view = { buffer: 0, byteOffset: bin.length - data.length, byteLength: data.length };
    if (target) view.target = target;
    views.push(view);
    return views.length - 1;
}

const positionView = addView(floatBytes(positions), 34962);
const normalView = addView(floatBytes(normals), 34962);
const uvView = addView(floatBytes(uvs), 34962);
const tangentView = addView(int16Bytes(tangents), 34962);
const indexView = addView(uint16Bytes(indices), 34963);
const imageView = addView(png(size, size, bumpMapRows()), undefined);
bin = Buffer.concat([bin, Buffer.alloc(pad4(bin.length))]);

const accessors = [
    { bufferView: positionView, componentType: 5126, count: 4, type: 'VEC3', min: [-0.5, -0.5, 0], max: [0.5, 0.5, 0] },
    { bufferView: normalView, componentType: 5126, count: 4, type: 'VEC3' },
    { bufferView: uvView, componentType: 5126, count: 4, type: 'VEC2' },
    { bufferView: tangentView, componentType: 5122, normalized: true, count: 4, type: 'VEC4' },
    { bufferView: indexView, componentType: 5123, count: 6, type: 'SCALAR' },
];

const json = {
    asset: { version: '2.0', generator: 'bevy-jayzee make-normal-map-fixture' },
    scene: 0,
    scenes: [{ nodes: [0] }],
    nodes: [{ mesh: 0 }],
    meshes: [
        {
            primitives: [
                {
                    attributes: { POSITION: 0, NORMAL: 1, TEXCOORD_0: 2, TANGENT: 3 },
                    indices: 4,
                    material: 0,
                },
            ],
        },
    ],
    materials: [
        {
            pbrMetallicRoughness: { baseColorFactor: [0.6, 0.6, 0.6, 1], metallicFactor: 0, roughnessFactor: 0.7 },
            normalTexture: { index: 0 },
        },
    ],
    textures: [{ source: 0 }],
    images: [{ mimeType: 'image/png', bufferView: imageView }],
    accessors,
    bufferViews: views,
    buffers: [{ byteLength: bin.length }],
};

const jsonBytes = Buffer.from(JSON.stringify(json), 'utf8');
const jsonPadded = Buffer.concat([jsonBytes, Buffer.alloc(pad4(jsonBytes.length), 0x20)]);

const header = Buffer.alloc(12);
header.writeUInt32LE(0x46546c67, 0);
header.writeUInt32LE(2, 4);
header.writeUInt32LE(12 + 8 + jsonPadded.length + 8 + bin.length, 8);

const jsonHeader = Buffer.alloc(8);
jsonHeader.writeUInt32LE(jsonPadded.length, 0);
jsonHeader.writeUInt32LE(0x4e4f534a, 4);

const binHeader = Buffer.alloc(8);
binHeader.writeUInt32LE(bin.length, 0);
binHeader.writeUInt32LE(0x004e4942, 4);

const glb = Buffer.concat([header, jsonHeader, jsonPadded, binHeader, bin]);

fs.mkdirSync(path.dirname(out), { recursive: true });
fs.writeFileSync(out, glb);
console.log(JSON.stringify({ kind: 'normal-map-fixture', path: out, bytes: glb.length }));
