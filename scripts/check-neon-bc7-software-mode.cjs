#!/usr/bin/env node
const fs = require('node:fs');
const path = require('node:path');

const source = fs.readFileSync(path.join(__dirname, 'check-neon-wasm-models.cjs'), 'utf8');
const wanted = "    'bc7-software': { urlDir: 'gltf-cache', decodedDir: 'gltf-cache', query: '&adapter=software&still=1' },";
const present = source.split('\n').includes(wanted);
const disablesBc = /'bc7-software'[^\n]*disable-bc/.test(source);
const ok = present && !disablesBc;
console.log(JSON.stringify({ present, disablesBc, ok }));
process.exit(ok ? 0 : 1);
