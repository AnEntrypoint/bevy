#!/usr/bin/env node
const fs = require('node:fs');
const path = require('node:path');

const repo = path.resolve(__dirname, '..');
const cdn = process.env.NEON_CDN || '/workspace/neon-syndicates/assets/cdn';
const witnessDir = path.join(repo, 'target', 'witness');
const caches = { rgba8: path.join(repo, 'target', 'gltf-cache-rgba8'), bc7: path.join(repo, 'target', 'gltf-cache') };
const outPath = path.join(witnessDir, 'texture-footprint.json');
const loadTolerance = Number(process.env.LOAD_TOLERANCE || 1.1);
const { sourceIsZstd, measure } = require('./lib/texture-cache.cjs');

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
