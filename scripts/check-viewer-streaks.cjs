const fs = require('node:fs');
const { readXwd } = require('./lib/xwd-png.cjs');
const { detect } = require('./lib/streak-detect.cjs');

const [, , target, minRun = '40', step = '24'] = process.argv;

function synthetic(width, height, streak) {
  const rgb = new Uint8Array(width * height * 3).fill(120);
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const hit = streak && x >= 200 && x < 203 && y >= 100 && y < 180;
      const i = (y * width + x) * 3;
      rgb[i] = rgb[i + 1] = rgb[i + 2] = hit ? 230 : 120;
    }
  }
  return rgb;
}

if (target === '--selftest') {
  const found = detect(400, 300, synthetic(400, 300, true), 40, 24).length;
  const clean = detect(400, 300, synthetic(400, 300, false), 40, 24).length;
  const ok = found > 0 && clean === 0;
  console.log(`${ok ? 'ok   ' : 'FAIL '}streak detector: synthetic streak runs ${found}, flat runs ${clean}`);
  process.exit(ok ? 0 : 1);
}

if (!target) {
  console.error('usage: check-viewer-streaks.cjs --selftest | <capture.xwd> [minRun] [step]');
  process.exit(2);
}

const { width, height, rgb } = readXwd(fs.readFileSync(target));
const runs = detect(width, height, rgb, Number(minRun), Number(step));
console.log(JSON.stringify({ width, height, minRun: Number(minRun), step: Number(step), longRuns: runs.length, first: runs.slice(0, 5) }));
