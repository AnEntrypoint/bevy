const crypto = require('crypto');
const fs = require('fs');
const { encodePng, readXwd } = require('./lib/xwd-png.cjs');
const { readPng } = require('./lib/png-read.cjs');

const GREEN_MIN_PERCENT = 0.43;
const [xwdPath, logPath, pngPath] = process.argv.slice(2);
if (!xwdPath || !logPath || !pngPath) {
  console.error('usage: node scripts/check-planet-screenshot.cjs <capture.xwd> <game.log> <out.png>');
  process.exit(2);
}

const image = xwdPath.endsWith('.png') ? readPng(fs.readFileSync(xwdPath)) : readXwd(fs.readFileSync(xwdPath));
const png = encodePng(image.width, image.height, image.rgb);
fs.writeFileSync(pngPath, png);

let green = 0;
for (let i = 0; i < image.width * image.height; i += 1) {
  const r = image.rgb[i * 3];
  const g = image.rgb[i * 3 + 1];
  const b = image.rgb[i * 3 + 2];
  if (g > r + 15 && g > b + 15) green += 1;
}
const greenPercent = (100 * green) / (image.width * image.height);

console.log(JSON.stringify({
  png: pngPath,
  sha256: crypto.createHash('sha256').update(png).digest('hex'),
  width: image.width,
  height: image.height,
  greenPercent: Number(greenPercent.toFixed(2)),
  greenMinPercent: GREEN_MIN_PERCENT,
}));
process.exit(greenPercent >= GREEN_MIN_PERCENT ? 0 : 1);
