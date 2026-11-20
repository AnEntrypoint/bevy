
const path = require('path');
const decoder = require(process.env.ASTC_DECODE_PATH || path.join(__dirname, 'lib', 'astc-decode.cjs'));

const VENDORED_MODES = [
  { mode: 0, field: 0x242, weightRange: 8, endpointRange: 19, subsets: 1, planes: 1, cem: 8 },
  { mode: 1, field: 0x42, weightRange: 2, endpointRange: 20, subsets: 1, planes: 1, cem: 8 },
  { mode: 2, field: 0x53, weightRange: 5, endpointRange: 8, subsets: 2, planes: 1, cem: 8 },
  { mode: 3, field: 0x42, weightRange: 2, endpointRange: 7, subsets: 3, planes: 1, cem: 8 },
  { mode: 4, field: 0x42, weightRange: 2, endpointRange: 12, subsets: 2, planes: 1, cem: 8 },
  { mode: 5, field: 0x53, weightRange: 5, endpointRange: 20, subsets: 1, planes: 1, cem: 8 },
  { mode: 6, field: 0x442, weightRange: 2, endpointRange: 18, subsets: 1, planes: 2, cem: 8 },
  { mode: 7, field: 0x42, weightRange: 2, endpointRange: 12, subsets: 2, planes: 1, cem: 8 },
  { mode: 9, field: 0x42, weightRange: 2, endpointRange: 8, subsets: 2, planes: 1, cem: 12 },
  { mode: 10, field: 0x242, weightRange: 8, endpointRange: 13, subsets: 1, planes: 1, cem: 12 },
  { mode: 11, field: 0x442, weightRange: 2, endpointRange: 13, subsets: 1, planes: 2, cem: 12 },
  { mode: 12, field: 0x53, weightRange: 5, endpointRange: 19, subsets: 1, planes: 1, cem: 12 },
  { mode: 13, field: 0x441, weightRange: 0, endpointRange: 20, subsets: 1, planes: 2, cem: 12 },
  { mode: 14, field: 0x42, weightRange: 2, endpointRange: 20, subsets: 1, planes: 1, cem: 12 },
  { mode: 15, field: 0x242, weightRange: 8, endpointRange: 20, subsets: 1, planes: 1, cem: 4 },
  { mode: 16, field: 0x42, weightRange: 2, endpointRange: 20, subsets: 2, planes: 1, cem: 4 },
  { mode: 17, field: 0x442, weightRange: 2, endpointRange: 20, subsets: 1, planes: 2, cem: 4 },
  { mode: 18, field: 0x253, weightRange: 11, endpointRange: 11, subsets: 1, planes: 1, cem: 8 },
];
const WEIGHT_BITS = { 0: 1, 2: 2, 5: 3, 8: 4, 11: 5 };
const COMPONENTS = { 4: 2, 8: 3, 12: 4 };
const LEVELS_2 = [0, 21, 43, 64];
const LEVELS_4 = [0, 4, 8, 12, 17, 21, 25, 29, 35, 39, 43, 47, 52, 56, 60, 64];

const failures = [];
function check(name, ok, detail) {
  console.log((ok ? 'PASS ' : 'FAIL ') + name + (detail ? ' ' + detail : ''));
  if (!ok) failures.push(name);
}

function setBits(bytes, position, value, count) {
  for (let i = 0; i < count; i += 1) {
    const bit = position + i;
    if ((value >> i) & 1) bytes[bit >> 3] |= 1 << (bit & 7);
  }
}

function reverse(value, count) {
  let out = 0;
  for (let i = 0; i < count; i += 1) out |= ((value >> i) & 1) << (count - 1 - i);
  return out;
}

function packSolid(rgba) {
  const bytes = Buffer.alloc(16);
  bytes.set([0xfc, 0xfd, 0xff, 0xff]);
  bytes.fill(0xff, 4, 8);
  rgba.forEach((v, c) => setBits(bytes, 64 + 16 * c, v | (v << 8), 16));
  return bytes;
}

function packSingle(field, cem, endpointValues, weightIndices, weightBits) {
  const bytes = Buffer.alloc(16);
  setBits(bytes, 0, field, 11);
  setBits(bytes, 11, 0, 2);
  setBits(bytes, 13, cem, 4);
  let position = 17;
  endpointValues.forEach((v) => {
    setBits(bytes, position, v, 8);
    position += 8;
  });
  weightIndices.forEach((index, i) => setBits(bytes, 128 - weightBits * (i + 1), reverse(index, weightBits), weightBits));
  return bytes;
}

function expectedPixels(lows, highs, levels, indices, alphaFromChannel) {
  const out = [];
  for (let t = 0; t < 16; t += 1) {
    const w = levels[indices[t]];
    const v = lows.map((lo, c) => (lo * (64 - w) + highs[c] * w + 32) >> 6);
    out.push(...(alphaFromChannel ? v : [...v, 255]));
  }
  return Uint8Array.from(out);
}

function sameBytes(a, b) {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i += 1) if (a[i] !== b[i]) return false;
  return true;
}

function firstDiff(a, b) {
  for (let i = 0; i < a.length; i += 1) if (a[i] !== b[i]) return i;
  return -1;
}

const weightsRange2 = Array.from({ length: 16 }, (_, t) => t & 3);
const weightsRange8 = Array.from({ length: 16 }, (_, t) => t);

{
  const colours = [[144, 76, 8, 255], [0, 128, 255, 64], [255, 255, 255, 255]];
  colours.forEach((rgba) => {
    const block = packSolid(rgba);
    const got = decoder.decodeAstc4x4Block(block);
    const want = Uint8Array.from(Array.from({ length: 16 }, () => rgba).flat());
    check('void-extent ' + rgba.join(','), sameBytes(got, want), sameBytes(got, want) ? '' : 'first diff at ' + firstDiff(got, want));
  });
}

{
  const block = packSingle(0x42, 8, [90, 90, 160, 160, 30, 30], weightsRange2, 2);
  const got = decoder.decodeAstc4x4Block(block);
  const want = expectedPixels([90, 160, 30], [90, 160, 30], LEVELS_2, weightsRange2, false);
  check('flat block mode 1 (two equal endpoints)', sameBytes(got, want), sameBytes(got, want) ? '' : 'first diff at ' + firstDiff(got, want));
}

{
  const block = packSingle(0x42, 8, [20, 230, 40, 100, 200, 10], weightsRange2, 2);
  const got = decoder.decodeAstc4x4Block(block);
  const want = expectedPixels([20, 40, 200], [230, 100, 10], LEVELS_2, weightsRange2, false);
  check('two-endpoint block mode 1 (RGB)', sameBytes(got, want), sameBytes(got, want) ? '' : 'first diff at ' + firstDiff(got, want));
}

{
  const block = packSingle(0x42, 12, [10, 200, 20, 180, 30, 160, 40, 120], weightsRange2, 2);
  const got = decoder.decodeAstc4x4Block(block);
  const want = expectedPixels([10, 20, 30, 40], [200, 180, 160, 120], LEVELS_2, weightsRange2, true);
  check('two-endpoint block mode 14 (RGBA)', sameBytes(got, want), sameBytes(got, want) ? '' : 'first diff at ' + firstDiff(got, want));
}

{
  const block = packSingle(0x242, 4, [30, 220, 0, 255], weightsRange8, 4);
  const got = decoder.decodeAstc4x4Block(block);
  const want = expectedPixels([30, 0], [220, 255], LEVELS_4, weightsRange8, true);
  const wantRgb = Uint8Array.from(Array.from({ length: 16 }, (_, t) => [want[2 * t], want[2 * t], want[2 * t], want[2 * t + 1]]).flat());
  check('two-endpoint block mode 15 (LA, 4-bit weights)', sameBytes(got, wantRgb), sameBytes(got, wantRgb) ? '' : 'first diff at ' + firstDiff(got, wantRgb));
}

VENDORED_MODES.forEach((m) => {
  const fieldOk = decoder.WEIGHT_RANGE_BY_FIELD[m.field] === m.weightRange;
  check('field table mode ' + m.mode, fieldOk, 'field 0x' + m.field.toString(16) + ' range ' + m.weightRange);
  const dual = m.planes === 2;
  const weightCount = dual ? 32 : 16;
  const weightTotal = weightCount * WEIGHT_BITS[m.weightRange];
  const position = m.subsets === 1 ? 17 : 29;
  const available = 128 - weightTotal - (dual ? 2 : 0) - position;
  const count = m.subsets * 2 * COMPONENTS[m.cem];
  const got = decoder.chooseEndpointRange(count, available);
  check('endpoint range mode ' + m.mode, got === m.endpointRange, 'vendored ' + m.endpointRange + ' decoder ' + got + ' avail ' + available + ' values ' + count);
});

const failed = failures.length;
console.log(failed === 0 ? 'ALL PASS' : 'FAILED ' + failed + ': ' + failures.join('; '));
process.exitCode = failed === 0 ? 0 : 1;
