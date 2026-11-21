const crypto = require('crypto');
const fs = require('fs');
const { encodePng, readXwd } = require('./lib/xwd-png.cjs');
const { readPng } = require('./lib/png-read.cjs');

const BLACK_MAX_PERCENT = 0.5;
const SKY_MIN_PERCENT = 25;
const WATER_MIN_PERCENT = 25;
const BLACK_BELOW = 24;
const SKY_MIN_RED_GREEN = 150;
const WATER_BLUE_OVER_RED = 40;
const [xwdPath, logPath, pngPath] = process.argv.slice(2);
if (!xwdPath || !logPath || !pngPath) {
  console.error('usage: node scripts/check-planet-screenshot.cjs <capture.xwd> <game.log> <out.png>');
  process.exit(2);
}

const image = xwdPath.endsWith('.png') ? readPng(fs.readFileSync(xwdPath)) : readXwd(fs.readFileSync(xwdPath));
const png = encodePng(image.width, image.height, image.rgb);
fs.writeFileSync(pngPath, png);

const share = { black: 0, sky: 0, water: 0, other: 0 };
for (let i = 0; i < image.width * image.height; i += 1) {
  const r = image.rgb[i * 3];
  const g = image.rgb[i * 3 + 1];
  const b = image.rgb[i * 3 + 2];
  if (Math.max(r, g, b) < BLACK_BELOW) share.black += 1;
  else if (r >= SKY_MIN_RED_GREEN && g >= SKY_MIN_RED_GREEN) share.sky += 1;
  else if (b >= r + WATER_BLUE_OVER_RED && b >= g) share.water += 1;
  else share.other += 1;
}
const percent = (count) => Number(((100 * count) / (image.width * image.height)).toFixed(2));
const blackPercent = percent(share.black);
const skyPercent = percent(share.sky);
const waterPercent = percent(share.water);
console.log(JSON.stringify({
  png: pngPath,
  sha256: crypto.createHash('sha256').update(png).digest('hex'),
  width: image.width,
  height: image.height,
  blackPercent,
  skyPercent,
  waterPercent,
  otherPercent: percent(share.other),
  blackMaxPercent: BLACK_MAX_PERCENT,
  skyMinPercent: SKY_MIN_PERCENT,
  waterMinPercent: WATER_MIN_PERCENT,
}));
process.exit(blackPercent <= BLACK_MAX_PERCENT && skyPercent >= SKY_MIN_PERCENT && waterPercent >= WATER_MIN_PERCENT ? 0 : 1);
