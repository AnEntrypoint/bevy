#!/usr/bin/env node
const fs = require('node:fs');
const path = require('node:path');

const repo = path.resolve(__dirname, '..');
const file = process.argv[2] || path.join(repo, 'web/status-main.json');
const status = JSON.parse(fs.readFileSync(file, 'utf8'));
const checks = {
    objectsDrawn: status.objects > 0,
    countMatchesExpected: status.objects === status.expectedObjects,
    noDuplicates: status.duplicateObjects === 0,
    noMissing: status.missingObjects.length === 0,
    noPlaceholderOverlap: status.visibleIds.length === new Set(status.visibleIds).size,
};
const ok = Object.values(checks).every(Boolean);
console.log(JSON.stringify({
    ok,
    objects: status.objects,
    expectedObjects: status.expectedObjects,
    duplicateObjects: status.duplicateObjects,
    missingObjects: status.missingObjects,
    placeholders: status.placeholders,
    placeholdersRetired: status.placeholdersRetired,
    checks,
}, null, 2));
process.exit(ok ? 0 : 1);
