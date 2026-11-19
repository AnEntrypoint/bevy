#!/usr/bin/env node
const fs = require('node:fs');

const file = process.argv[2];
const count = Number(process.argv[3] || 10);
const html = fs.readFileSync(file, 'utf8');
const match = html.match(/UNIT_DATA = (\[[\s\S]*?\]);\s*\n/);
if (!match) {
    console.error(`no UNIT_DATA in ${file}`);
    process.exit(1);
}
const units = JSON.parse(match[1]);
const wall = Math.max(...units.map((unit) => unit.start + unit.duration));
const top = [...units]
    .sort((a, b) => b.duration - a.duration)
    .slice(0, count)
    .map((unit) => ({
        name: unit.name,
        version: unit.version,
        mode: unit.mode,
        target: unit.target,
        startS: Number(unit.start.toFixed(2)),
        durationS: Number(unit.duration.toFixed(2)),
        sections: unit.sections,
    }));
const game = units.filter((unit) => unit.name === 'bevy-jayzee').map((unit) => ({ durationS: Number(unit.duration.toFixed(2)), sections: unit.sections }));
console.log(JSON.stringify({ file, units: units.length, reportWallS: Number(wall.toFixed(2)), top, gameCrate: game }, null, 2));
