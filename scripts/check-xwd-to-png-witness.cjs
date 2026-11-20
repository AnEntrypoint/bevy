const zlib = require('zlib');
const { encodePng, readXwd, crc32 } = require('./lib/xwd-png.cjs');

const checks = [];
function check(name, ok, detail) {
  checks.push(ok);
  console.log((ok ? 'ok   ' : 'FAIL ') + name + (ok ? '' : ': ' + detail));
}

const width = 5;
const height = 3;
const bytesPerLine = width * 4;
const header = Buffer.alloc(100);
const fields = [100, 7, 2, 24, width, height, 0, 0, 32, 0, 32, 32, bytesPerLine, 4, 0xff0000, 0xff00, 0xff, 8, 0, 0, width, height, 0, 0, 0];
fields.forEach((value, index) => header.writeUInt32BE(value, index * 4));
const pixels = Buffer.alloc(bytesPerLine * height);
const expected = Buffer.alloc(width * height * 3);
for (let y = 0; y < height; y += 1) {
  for (let x = 0; x < width; x += 1) {
    const r = (x * 37 + y * 11) & 0xff;
    const g = (x * 5 + y * 90) & 0xff;
    const b = (x * 101 + y * 3 + 7) & 0xff;
    pixels.writeUInt32LE(((r << 16) | (g << 8) | b) >>> 0, y * bytesPerLine + x * 4);
    expected[(y * width + x) * 3] = r;
    expected[(y * width + x) * 3 + 1] = g;
    expected[(y * width + x) * 3 + 2] = b;
  }
}
const xwd = Buffer.concat([header, pixels]);
const image = readXwd(xwd);
check('the XWD header decodes to the synthetic width and height (wrong output: transposed or misread header fields)', image.width === width && image.height === height, JSON.stringify({ w: image.width, h: image.height }));
check('every pixel decodes to its red, green and blue values (wrong output: channels swapped or shifted)', Buffer.compare(image.rgb, expected) === 0, 'first differing byte ' + [...image.rgb].findIndex((v, i) => v !== expected[i]));
const png = encodePng(image.width, image.height, image.rgb);
check('the PNG signature is the eight standard bytes (wrong output: a non-PNG file)', png.subarray(0, 8).equals(Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a])), 'signature');
const idatAt = png.indexOf(Buffer.from('IDAT'));
const idatLength = png.readUInt32BE(idatAt - 4);
const raw = zlib.inflateSync(png.subarray(idatAt + 4, idatAt + 4 + idatLength));
const stride = width * 3;
let rowsMatch = raw.length === (stride + 1) * height;
for (let y = 0; y < height && rowsMatch; y += 1) {
  rowsMatch = raw[y * (stride + 1)] === 0 && raw.subarray(y * (stride + 1) + 1, (y + 1) * (stride + 1)).equals(expected.subarray(y * stride, (y + 1) * stride));
}
check('the PNG IDAT inflates to the same pixels, one filter byte per row (wrong output: a misordered or missing scanline)', rowsMatch, 'raw length ' + raw.length);
const ihdrAt = png.indexOf(Buffer.from('IHDR'));
const stored = png.readUInt32BE(ihdrAt + 17);
check('the IHDR CRC matches the chunk type and data (wrong output: a corrupted chunk the viewer rejects)', stored === crc32(png.subarray(ihdrAt, ihdrAt + 17)), 'stored ' + stored.toString(16) + ' computed ' + crc32(png.subarray(ihdrAt, ihdrAt + 17)).toString(16));
const passed = checks.filter(Boolean).length;
console.log(passed === checks.length ? 'ALL PASS ' + passed + ' checks' : 'FAIL ' + (checks.length - passed) + ' of ' + checks.length);
process.exit(passed === checks.length ? 0 : 1);
