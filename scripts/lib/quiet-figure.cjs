#!/usr/bin/env node
const fs = require('fs');

function quietFigure(records) {
    const quiet = records.filter((record) => record.swapNoisy === false);
    const rejected = records.filter((record) => record.swapNoisy !== false).map((record) => record.label);
    if (quiet.length === 0) return { figure: null, quietRuns: 0, rejected, reason: 'no quiet record' };
    const walls = quiet.map((record) => record.build.wallS).sort((a, b) => a - b);
    const mid = Math.floor(walls.length / 2);
    const figure = walls.length % 2 ? walls[mid] : (walls[mid - 1] + walls[mid]) / 2;
    return { figure, quietRuns: quiet.length, rejected, reason: null };
}

if (require.main === module) {
    const [file, prefix] = process.argv.slice(2);
    const records = fs.readFileSync(file, 'utf8').trim().split('\n').map((line) => JSON.parse(line)).filter((record) => record.label.startsWith(prefix));
    process.stdout.write(JSON.stringify(quietFigure(records)) + '\n');
}

module.exports = { quietFigure };
