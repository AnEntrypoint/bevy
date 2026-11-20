#!/usr/bin/env node
const fs = require('node:fs');
const path = require('node:path');
const { sourceIsZstd, measure } = require('./lib/texture-cache.cjs');
const { windowEvidence } = require('./lib/fallback-list.cjs');
const { referenceFor } = require('./lib/pair-reference.cjs');

const repo = path.resolve(__dirname, '..');
const cdn = process.env.NEON_CDN || '/workspace/neon-syndicates/assets/cdn';
const witnessDir = path.join(repo, 'target', 'witness');
const outPath = path.join(witnessDir, 'texture-footprint.json');
const loadTolerance = Number(process.env.LOAD_TOLERANCE || 1.1);
const caches = { rgba8: 'target/gltf-cache-rgba8', bc7: 'target/gltf-cache', astc4x4: 'target/gltf-cache-astc', etc2: 'target/gltf-cache-etc2' };
const judged = {
    bc7: { reference: 'rgba8', witness: 'bc7', visual: 'bc7', texel: true },
    astc4x4: { reference: referenceFor('astc4x4'), witness: 'astc4x4', visual: 'astc4x4', texel: false },
    etc2: { reference: referenceFor('etc2'), witness: 'etc2', visual: 'etc2', texel: false },
};

function readWitness(name) {
    const file = path.join(witnessDir, name);
    if (!fs.existsSync(file)) return null;
    return { file: name, mtime: fs.statSync(file).mtime.toISOString(), value: JSON.parse(fs.readFileSync(file, 'utf8')) };
}

function witnessSummary(texture) {
    const found = readWitness(`neon-wasm-witness-${texture}.json`);
    if (!found || !found.value.summary) return null;
    const { summary } = found.value;
    return { file: found.file, mtime: found.mtime, clean: summary.clean, total: summary.total, loadMsMedian: summary.loadMsMedian, loadMsMax: summary.loadMsMax, glbEncodedBytesTotal: summary.glbEncodedBytesTotal };
}

function texelSummary() {
    const found = readWitness('texture-texel.json');
    if (!found) return null;
    const texel = found.value;
    return { file: found.file, mtime: found.mtime, ok: texel.ok, images: texel.images, models: texel.models.length, psnrLevel0Min: texel.psnrLevel0Min, psnrLevel0Median: texel.psnrLevel0Median, maxError: texel.maxError, criteria: texel.criteria, errors: texel.errors };
}

function visualSummary(format) {
    const found = readWitness(`texture-visual-${format}.json`);
    if (!found) return null;
    const visual = found.value;
    return { file: found.file, mtime: found.mtime, ok: visual.ok, pairs: visual.pairs, psnrMin: visual.psnrMin, psnrMedian: visual.psnrMedian, minPsnr: visual.minPsnr, failing: visual.failing };
}

function footprint(models, target) {
    const rows = models.map((name) => ({ name, rgba8: measure(path.join(repo, caches.rgba8, name)), target: measure(path.join(repo, caches[target], name)) }));
    const complete = rows.length > 0 && rows.every((row) => row.rgba8.present && row.target.present);
    const sum = (side, key) => rows.reduce((total, row) => total + (row[side].present ? row[side][key] : 0), 0);
    const totals = {
        models: rows.length,
        rgba8TextureBytes: sum('rgba8', 'textureBytes'),
        targetTextureBytes: sum('target', 'textureBytes'),
        rgba8GlbBytes: sum('rgba8', 'glbBytes'),
        targetGlbBytes: sum('target', 'glbBytes'),
    };
    return {
        complete,
        totals,
        ratio: totals.rgba8TextureBytes ? totals.targetTextureBytes / totals.rgba8TextureBytes : null,
        wins: complete && totals.targetTextureBytes < totals.rgba8TextureBytes && totals.targetGlbBytes < totals.rgba8GlbBytes,
    };
}

function judge(format, spec, models, names) {
    const fp = footprint(models, format);
    const witness = witnessSummary(spec.witness);
    const reference = witnessSummary(spec.reference);
    const visual = visualSummary(spec.visual);
    const texel = spec.texel ? texelSummary() : null;
    const loadEvidence = witness && reference ? windowEvidence(witness.mtime, reference.mtime) : null;
    const loadOk = Boolean(witness && reference && witness.loadMsMedian !== null && reference.loadMsMedian !== null && witness.loadMsMedian <= reference.loadMsMedian * loadTolerance);
    const cleanOk = Boolean(witness && witness.total > 0 && witness.clean === witness.total);
    const visualOk = Boolean(visual && visual.ok && visual.pairs === names.length);
    const texelOk = spec.texel ? Boolean(texel && texel.ok) : null;
    const usable = fp.wins && loadOk && cleanOk && visualOk && texelOk !== false;
    return { footprint: fp, reference: spec.reference, witness, referenceWitness: reference, loadEvidence, visual, texel, reasons: { footprintWins: fp.wins, loadOk, cleanOk, visualOk, texelOk, loadTolerance }, usable };
}

function main() {
    const names = fs.readdirSync(cdn).filter((file) => file.endsWith('.glb')).sort();
    const models = names.filter((name) => sourceIsZstd(path.join(cdn, name)));
    const formats = Object.fromEntries(Object.entries(judged).map(([format, spec]) => [format, judge(format, spec, models, names)]));
    const complete = Object.values(formats).every((format) => format.footprint.complete);
    const decision = formats.bc7.usable ? 'bc7' : 'rgba8';
    const result = {
        ok: complete,
        decision,
        fallbackFormats: { astc4x4: formats.astc4x4.usable, etc2: formats.etc2.usable },
        formats,
    };
    fs.mkdirSync(witnessDir, { recursive: true });
    fs.writeFileSync(outPath, JSON.stringify(result, null, 2));
    console.log(JSON.stringify({ ok: complete, decision, fallbackFormats: result.fallbackFormats, formats: Object.fromEntries(Object.entries(formats).map(([format, verdict]) => [format, { usable: verdict.usable, reasons: verdict.reasons, ratio: verdict.footprint.ratio, totals: verdict.footprint.totals }])), witnessPath: outPath }, null, 2));
    process.exit(complete ? 0 : 1);
}

main();
