const zlib = require('zlib');

const CRC_TABLE = Array.from({ length: 256 }, (_, n) => {
  let c = n;
  for (let k = 0; k < 8; k += 1) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c >>> 0;
});

function crc32(buffer) {
  let c = 0xffffffff;
  for (const byte of buffer) c = CRC_TABLE[(c ^ byte) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const length = Buffer.alloc(4);
  length.writeUInt32BE(data.length);
  const body = Buffer.concat([Buffer.from(type, 'ascii'), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(body));
  return Buffer.concat([length, body, crc]);
}

function encodePng(width, height, rgb) {
  const header = Buffer.alloc(13);
  header.writeUInt32BE(width, 0);
  header.writeUInt32BE(height, 4);
  header[8] = 8;
  header[9] = 2;
  const stride = width * 3;
  const raw = Buffer.alloc((stride + 1) * height);
  for (let y = 0; y < height; y += 1) {
    raw[y * (stride + 1)] = 0;
    rgb.copy(raw, y * (stride + 1) + 1, y * stride, (y + 1) * stride);
  }
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk('IHDR', header),
    chunk('IDAT', zlib.deflateSync(raw)),
    chunk('IEND', Buffer.alloc(0)),
  ]);
}

function readXwd(buffer) {
  const field = (index) => buffer.readUInt32BE(index * 4);
  const headerSize = field(0);
  const fileVersion = field(1);
  if (fileVersion !== 7) throw new Error('not an XWD version 7 file: ' + fileVersion);
  const width = field(4);
  const height = field(5);
  const byteOrder = field(7);
  const bitsPerPixel = field(11);
  const bytesPerLine = field(12);
  const redMask = field(14);
  const greenMask = field(15);
  const blueMask = field(16);
  const colormapEntries = field(19);
  const ncolors = field(20);
  if (bitsPerPixel !== 32) throw new Error('unsupported bits per pixel: ' + bitsPerPixel);
  const pixels = headerSize + ncolors * 12;
  const shiftOf = (mask) => { let s = 0; while (mask && !((mask >>> s) & 1)) s += 1; return s; };
  const shifts = [shiftOf(redMask), shiftOf(greenMask), shiftOf(blueMask)];
  const masks = [redMask, greenMask, blueMask];
  const rgb = Buffer.alloc(width * height * 3);
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const at = pixels + y * bytesPerLine + x * 4;
      const value = byteOrder === 0 ? buffer.readUInt32LE(at) : buffer.readUInt32BE(at);
      for (let c = 0; c < 3; c += 1) {
        rgb[(y * width + x) * 3 + c] = ((value & masks[c]) >>> shifts[c]) & 0xff;
      }
    }
  }
  return { width, height, rgb, colormapEntries, ncolors };
}

module.exports = { encodePng, readXwd, crc32 };
