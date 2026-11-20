const fs = require('fs');
const { encodePng, readXwd } = require('./lib/xwd-png.cjs');

const [input, output] = process.argv.slice(2);
if (!input || !output) {
  console.error('usage: node scripts/xwd-to-png.cjs <in.xwd> <out.png>');
  process.exit(2);
}
const image = readXwd(fs.readFileSync(input));
fs.writeFileSync(output, encodePng(image.width, image.height, image.rgb));
console.log(JSON.stringify({ input, output, width: image.width, height: image.height }));
