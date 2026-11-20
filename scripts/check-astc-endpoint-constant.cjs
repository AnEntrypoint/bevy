const fs = require('node:fs');
const path = require('node:path');
const { blocksExplained } = require('./lib/astc-endpoint-constant.cjs');

const root = path.join(__dirname, '..');
const records = JSON.parse(fs.readFileSync(path.join(root, 'target', 'witness', 'astc-endpoint-records.json'), 'utf8'));
const scores = [];
let blocks = 0;
for (let c = 0; c <= 255; c += 1) {
    const { explained, blocks: total } = blocksExplained(records, c);
    blocks = total;
    scores.push({ c, explained });
}
const sorted = [...scores].sort((p, q) => q.explained - p.explained);
const median = [...scores].sort((p, q) => p.explained - q.explained)[128].explained;
console.log('blocks with a repair candidate ' + blocks + ', candidate records ' + records.length);
console.log('current table (c = 5) explains ' + scores[5].explained + ' blocks');
console.log('median over c = 0..255 explains ' + median + ' blocks; best ' + sorted[0].explained + ' at c = ' + sorted[0].c);
console.log('top five: ' + sorted.slice(0, 5).map((s) => 'c ' + s.c + ' ' + s.explained).join(', '));
process.exit(0);
