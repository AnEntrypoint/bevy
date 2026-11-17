#!/usr/bin/env node
const fs = require('node:fs');
const path = require('node:path');

const repo = path.resolve(__dirname, '..');
const files = process.argv.length > 2 ? process.argv.slice(2) : [path.join(repo, 'web/status-main.json')];
const paletteSlots = Number(process.env.MIRROR_SLOTS || 1);
const proxyMaterials = Number(process.env.MIRROR_PROXY || 0);
const slack = Number(process.env.MIRROR_SLACK || 1);

function check(file) {
    const status = JSON.parse(fs.readFileSync(file, 'utf8'));
    const mirrored = status.mirrorEntities > 0;
    const bound = status.shapesCreated * paletteSlots + proxyMaterials + slack;
    const checks = {
        objectsDrawn: status.objectDraws > 0,
        objectsBatched: status.objectDraws < 2 || status.shapesCreated >= status.objectDraws || status.objectBatchKeys < status.objectDraws,
        meshesShared: status.meshRequests === 0 || status.shapesCreated < status.meshRequests,
        mirrorsWithinBound: !mirrored || status.mirrorBatchKeys <= bound,
    };
    return {
        file,
        ok: Object.values(checks).every(Boolean),
        objectDraws: status.objectDraws,
        objectBatchKeys: status.objectBatchKeys,
        meshRequests: status.meshRequests,
        shapesCreated: status.shapesCreated,
        mirrorEntities: status.mirrorEntities,
        mirrorBatchKeys: status.mirrorBatchKeys,
        bound,
        slack,
        checks,
    };
}

const results = files.map(check);
const ok = results.every((result) => result.ok);
console.log(JSON.stringify({ ok, paletteSlots, proxyMaterials, slack, results }, null, 2));
process.exit(ok ? 0 : 1);
