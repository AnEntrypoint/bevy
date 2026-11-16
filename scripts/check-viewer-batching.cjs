#!/usr/bin/env node
const fs = require('node:fs');
const path = require('node:path');

const repo = path.resolve(__dirname, '..');
const file = process.argv[2] || path.join(repo, 'web/status-main.json');
const status = JSON.parse(fs.readFileSync(file, 'utf8'));
const paletteSlots = 4;
const proxyMaterials = 1;
const bound = status.shapesCreated * paletteSlots + proxyMaterials;
const checks = {
    requestsRecorded: status.meshRequests > 0,
    meshesShared: status.shapesCreated < status.meshRequests,
    mirrorsDrawn: status.mirrorEntities > 0,
    mirrorBatchedBelowEntities: status.mirrorBatchKeys < status.mirrorEntities,
    mirrorBatchesWithinBound: status.mirrorBatchKeys <= bound,
};
const ok = Object.values(checks).every(Boolean);
console.log(JSON.stringify({
    ok,
    meshRequests: status.meshRequests,
    shapesCreated: status.shapesCreated,
    mirrorEntities: status.mirrorEntities,
    mirrorBatchKeys: status.mirrorBatchKeys,
    batchBound: bound,
    meshes: status.meshes,
    meshEntities: status.meshEntities,
    uniqueMaterials: status.uniqueMaterials,
    batchKeys: status.batchKeys,
    checks,
}, null, 2));
process.exit(ok ? 0 : 1);
