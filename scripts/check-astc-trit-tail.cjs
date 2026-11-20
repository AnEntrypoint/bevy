const path = require('path');
const decoder = require(process.env.ASTC_DECODE_PATH || path.join(__dirname, 'lib', 'astc-decode.cjs'));

const PINS = [
    { block: 0, astc: '420279f4fc3b812086006b08bf86ffeb', gpu: '7783ffff7589feff758bfeff758bfeff787effff7782ffff7589feff758bfeff787dffff787effff7782ffff7589feff787dffff787dffff787effff7782ffff' },
    { block: 3, astc: '4202fdf5fe3b810000f5005000000000', gpu: '7d81ffff7d81ffff7d81ffff7d81ffff7d81ffff7d81ffff7d81ffff7d81ffff7888ffff7d81ffff7d81ffff7d81ffff758bffff7888ffff7d81ffff7d81ffff' },
    { block: 28, astc: '42027fee7fbe8340bc7f5cb71c3b10dd', gpu: '8d7ffcff8d7ffcff8b7ffcff8381fbff8e7ffdff8f7efdff8b7ffcff8680fbff8f7efdff907efdff8c7ffcff8680fbff907efdff917efdff8f7efdff8680fbff' },
    { block: 29, astc: '4202e97df23f830120f928f9a4f9a4f5', gpu: '7d83fbff7686fbff6f8afbff6b8cfbff7d83fbff7587fbff6f8afbff6b8cfbff7d83fbff7587fbff6e8afbff698cfbff7d83fbff7587fbff6e8afbff688dfbff' },
    { block: 37, astc: '42027f7f556c854128f528f528f528f5', gpu: '8159f9ff8157f8ff8155f8ff8154f7ff8159f9ff8157f8ff8155f8ff8154f7ff8159f9ff8157f8ff8155f8ff8154f7ff8159f9ff8157f8ff8155f8ff8154f7ff' },
    { block: 38, astc: '42027dff512a8502f5ffffffffffffff', gpu: '8154f7ff8154f7ff8154f7ff8154f7ff8154f7ff8154f7ff8154f7ff8154f7ff8154f7ff8154f7ff8154f7ff8154f7ff8154f7ff8154f7ff8154f7ff8053f7ff' },
    { block: 51, astc: '42022b0ee2aba518ff0fffffffffffff', gpu: 'f3a99ffff3a99ffff3a99ffff3a99ffff3a99ffff3a99ffff3a99ffff3a99ffff3a99ffff3a99ffff3a99ffff3a99fffd79eb7fff3a99ffff3a99ffff3a99fff' },
    { block: 54, astc: '4202e97c174bec1a9f029f8a9f8a574a', gpu: '6ceb61ff70eb6dff77ea83ff7dea93ff6aeb5dff70eb6dff76ea7fff7eea97ff6aeb5dff70eb6dff76ea7fff7eea97ff69eb59ff6feb69ff76ea7fff7eea97ff' },
];

let failed = 0;
for (const pin of PINS) {
    const block = Buffer.from(pin.astc, "hex");
    const want = Buffer.from(pin.gpu, "hex");
    const got = decoder.decodeAstc4x4Block(block);
    const bad = [];
    for (let i = 0; i < 64; i += 1) {
        if (got[i] !== want[i]) bad.push(i);
    }
    const ok = bad.length === 0;
    if (!ok) failed += 1;
    console.log((ok ? "ok   " : "FAIL ") + "cache block " + pin.block + " one-value trit tail code " + decoder.readBits(block, 55, 8) + ": 16 texels RGBA vs GPU decode, mismatched bytes " + bad.length + (ok ? "" : " first at " + bad[0]));
}
console.log(failed === 0 ? "ALL PASS " + PINS.length + " pinned blocks" : "FAILED " + failed + " of " + PINS.length);
process.exitCode = failed === 0 ? 0 : 1;
