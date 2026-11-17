const fs = require('node:fs');

const KTX2_IDENTIFIER = Buffer.from([0xab, 0x4b, 0x54, 0x58, 0x20, 0x32, 0x30, 0xbb, 0x0d, 0x0a, 0x1a, 0x0a]);
const SUPERCOMPRESSION_ZSTD = 2;

function glbParts(bytes) {
    if (bytes.readUInt32LE(0) !== 0x46546c67) throw new Error('not a GLB');
    const jsonLength = bytes.readUInt32LE(12);
    const json = JSON.parse(bytes.subarray(20, 20 + jsonLength).toString('utf8'));
    const binStart = 20 + jsonLength + 8;
    return { json, bin: bytes.subarray(Math.min(binStart, bytes.length)) };
}

function ktx2Images(json, bin) {
    const views = json.bufferViews ?? [];
    const seen = new Set();
    const out = [];
    for (const image of json.images ?? []) {
        if (image.mimeType !== 'image/ktx2' || image.bufferView === undefined || seen.has(image.bufferView)) continue;
        seen.add(image.bufferView);
        const view = views[image.bufferView];
        const start = view.byteOffset ?? 0;
        out.push(bin.subarray(start, start + view.byteLength));
    }
    return out;
}

function ktx2Info(bytes) {
    if (!bytes.subarray(0, 12).equals(KTX2_IDENTIFIER)) throw new Error('not KTX2');
    const levelCount = Math.max(1, bytes.readUInt32LE(40));
    let payload = 0;
    for (let level = 0; level < levelCount; level += 1) {
        payload += Number(bytes.readBigUInt64LE(80 + level * 24 + 8));
    }
    return {
        vkFormat: bytes.readUInt32LE(12),
        width: bytes.readUInt32LE(20),
        height: bytes.readUInt32LE(24),
        levels: levelCount,
        supercompression: bytes.readUInt32LE(44),
        payload,
    };
}

function sourceIsZstd(file) {
    const { json, bin } = glbParts(fs.readFileSync(file));
    return ktx2Images(json, bin).some((bytes) => ktx2Info(bytes).supercompression === SUPERCOMPRESSION_ZSTD);
}

function measure(file) {
    if (!fs.existsSync(file)) return { present: false };
    const bytes = fs.readFileSync(file);
    const { json, bin } = glbParts(bytes);
    const infos = ktx2Images(json, bin).map(ktx2Info);
    return {
        present: true,
        glbBytes: bytes.length,
        textures: infos.length,
        textureBytes: infos.reduce((sum, info) => sum + info.payload, 0),
        vkFormats: [...new Set(infos.map((info) => info.vkFormat))],
        supercompression: [...new Set(infos.map((info) => info.supercompression))],
    };
}

module.exports = { glbParts, ktx2Images, ktx2Info, sourceIsZstd, measure };
