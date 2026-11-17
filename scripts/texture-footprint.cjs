#!/usr/bin/env node
const fs = require('node:fs');
const path = require('node:path');

const repo = path.resolve(__dirname, '..');
const cdn = process.env.NEON_CDN || '/workspace/neon-syndicates/assets/cdn';
const witnessDir = path.join(repo, 'target', 'witness');
const caches = { rgba8: path.join(repo, 'target', 'gltf-cache-rgba8'), bc7: path.join(repo, 'target', 'gltf-cache') };
const outPath = path.join(witnessDir, 'texture-footprint.json');
const loadTolerance = Number(process.env.LOAD_TOLERANCE || 1.1);
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

function witnessSummary(texture) {
    const file = path.join(witnessDir, `neon-wasm-witness-${texture}.json`);
    if (!fs.existsSync(file)) return null;
    const { summary } = JSON.parse(fs.readFileSync(file, 'utf8'));
    return { clean: summary.clean, total: summary.total, loadMsMedian: summary.loadMsMedian, loadMsMax: summary.loadMsMax, glbEncodedBytesTotal: summary.glbEncodedBytesTotal };
}

function texelSummary() {
    const file = path.join(witnessDir, 'texture-texel.json');
    if (!fs.existsSync(file)) return null;
    const texel = JSON.parse(fs.readFileSync(file, 'utf8'));
    return { ok: texel.ok, images: texel.images, models: texel.models.length, psnrLevel0Min: texel.psnrLevel0Min, psnrLevel0Median: texel.psnrLevel0Median, maxError: texel.maxError, criteria: texel.criteria, errors: texel.errors };
}

function screenshotSummary() {
    const file = path.join(witnessDir, 'texture-visual.json');
    if (!fs.existsSync(file)) return null;
    const visual = JSON.parse(fs.readFileSync(file, 'utf8'));
    return { ok: visual.ok, pairs: visual.pairs, psnrMin: visual.psnrMin, psnrMedian: visual.psnrMedian, minPsnr: visual.minPsnr };
}

function main() {
    const names = fs.readdirSync(cdn).filter((file) => file.endsWith('.glb')).sort();
    const models = names.filter((name) => sourceIsZstd(path.join(cdn, name)));
    const rows = models.map((name) => {
        const rgba8 = measure(path.join(caches.rgba8, name));
        const bc7 = measure(path.join(caches.bc7, name));
        return { name, rgba8, bc7, ratio: rgba8.present && bc7.present && rgba8.textureBytes ? bc7.textureBytes / rgba8.textureBytes : null };
    });
    const sum = (texture, key) => rows.reduce((total, row) => total + (row[texture].present ? row[texture][key] : 0), 0);
    const complete = rows.length > 0 && rows.every((row) => row.rgba8.present && row.bc7.present);
    const totals = {
        models: rows.length,
        rgba8TextureBytes: sum('rgba8', 'textureBytes'),
        bc7TextureBytes: sum('bc7', 'textureBytes'),
        rgba8GlbBytes: sum('rgba8', 'glbBytes'),
        bc7GlbBytes: sum('bc7', 'glbBytes'),
    };
    const footprintWins = complete && totals.bc7TextureBytes < totals.rgba8TextureBytes && totals.bc7GlbBytes < totals.rgba8GlbBytes;
    const witness = { rgba8: witnessSummary('rgba8'), bc7: witnessSummary('bc7') };
    const visual = texelSummary();
    const screenshots = screenshotSummary();
    const loadOk = Boolean(witness.rgba8 && witness.bc7 && witness.bc7.loadMsMedian !== null && witness.rgba8.loadMsMedian !== null && witness.bc7.loadMsMedian <= witness.rgba8.loadMsMedian * loadTolerance);
    const cleanOk = Boolean(witness.bc7 && witness.bc7.total > 0 && witness.bc7.clean === witness.bc7.total);
    const visualOk = Boolean(visual && visual.ok);
    const decision = footprintWins && loadOk && cleanOk && visualOk ? 'bc7' : 'rgba8';
    const reasons = {
        footprintWins,
        loadOk,
        cleanOk,
        visualOk,
        loadTolerance,
    };
    const result = { ok: complete, decision, reasons, totals, ratio: totals.rgba8TextureBytes ? totals.bc7TextureBytes / totals.rgba8TextureBytes : null, witness, visual, screenshots, rows };
    fs.mkdirSync(witnessDir, { recursive: true });
    fs.writeFileSync(outPath, JSON.stringify(result, null, 2));
    console.log(JSON.stringify({ ok: complete, decision, reasons, totals, ratio: result.ratio, witness, visual, screenshots, witnessPath: outPath }, null, 2));
    process.exit(complete ? 0 : 1);
}

main();
