#!/usr/bin/env node
const { quietFigure } = require('./lib/quiet-figure.cjs');

const rec = (label, wallS, swapNoisy) => ({ label, swapNoisy, build: { wallS } });
const failures = [];

const mixed = quietFigure([rec('a', 3.1, false), rec('b', 16.3, true), rec('c', 3.6, false), rec('d', 14.1, true), rec('e', 3.4, false)]);
if (mixed.figure !== 3.4) failures.push(`mixed: figure expected 3.4, got ${mixed.figure}`);
if (mixed.quietRuns !== 3) failures.push(`mixed: quietRuns expected 3, got ${mixed.quietRuns}`);
if (JSON.stringify(mixed.rejected) !== JSON.stringify(['b', 'd'])) failures.push(`mixed: rejected expected [b,d], got ${JSON.stringify(mixed.rejected)}`);

const allNoisy = quietFigure([rec('f', 16.3, true), rec('g', 14.1, true)]);
if (allNoisy.figure !== null || allNoisy.reason !== 'no quiet record') failures.push('all noisy: figure must be null with a reason');

const multiCrate = quietFigure([rec('h', 3.0, null), rec('i', 3.2, false)]);
if (multiCrate.figure !== 3.2 || JSON.stringify(multiCrate.rejected) !== JSON.stringify(['h'])) failures.push('null swapNoisy must be rejected, not counted as quiet');

const even = quietFigure([rec('j', 3.0, false), rec('k', 4.0, false)]);
if (even.figure !== 3.5) failures.push(`even count: figure expected 3.5, got ${even.figure}`);

const ok = failures.length === 0;
console.log(JSON.stringify({ ok, checked: 4, failures }, null, 2));
process.exit(ok ? 0 : 1);
