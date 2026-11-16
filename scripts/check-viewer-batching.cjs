#!/usr/bin/env node
const fs = require('node:fs');
const path = require('node:path');

const repo = path.resolve(__dirname, '..');
const file = process.argv[2] || path.join(repo, 'web/status-main.json');
const status = JSON.parse(fs.readFileSync(file, 'utf8'));
const paletteSlots = 4;
const proxyMaterials = 1;
const mirrored = status.mirrorEntities > 0;
const checks = {
    objectsDrawn: status.objectDraws > 0,
    objectsBatched: status.objectDraws < 2 || status.objectBatchKeys < status.objectDraws,
    meshesShared: status.meshRequests === 0 || status.shapesCreated < status.meshRequests,
    mirrorsWithinBound: !mirrored || status.mirrorBatchKeys <= status.shapesCreated * paletteSlots + proxyMaterials,
};
const ok = Object.values(checks).every(Boolean);
console.log(JSON.stringify({
    ok,
    objectDraws: status.objectDraws,
    objectBatchKeys: status.objectBatchKeys,
    meshRequests: status.meshRequests,
    shapesCreated: status.shapesCreated,
    mirrorEntities: status.mirrorEntities,
    mirrorBatchKeys: status.mirrorBatchKeys,
    checks,
}, null, 2));
process.exit(ok ? 0 : 1);
