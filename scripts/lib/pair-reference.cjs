const fs = require('node:fs');
const path = require('node:path');

const SHARED_REFERENCE = 'rgba8-software';
const KINDS = ['neon-wasm-witness', 'neon-rows'];

function referenceFor(format) {
    return `${SHARED_REFERENCE}-${format}`;
}

function keepReference({ witnessDir, format, beforeDir }) {
    const stored = [];
    for (const kind of KINDS) {
        const shared = path.join(witnessDir, `${kind}-${SHARED_REFERENCE}.json`);
        const own = path.join(witnessDir, `${kind}-${referenceFor(format)}.json`);
        const before = path.join(beforeDir, `before-${kind}-${SHARED_REFERENCE}.json`);
        if (fs.existsSync(shared)) {
            fs.copyFileSync(shared, own);
            const { atime, mtime } = fs.statSync(shared);
            fs.utimesSync(own, atime, mtime);
            stored.push(path.basename(own));
        }
        if (fs.existsSync(before)) {
            fs.copyFileSync(before, shared);
        } else if (fs.existsSync(shared)) {
            fs.rmSync(shared);
        }
    }
    return stored;
}

module.exports = { SHARED_REFERENCE, referenceFor, keepReference };
