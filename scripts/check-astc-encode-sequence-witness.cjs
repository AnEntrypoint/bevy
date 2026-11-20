const { BISE_RANGES, integerSequenceBits, readIntegerSequence } = require('./lib/astc-decode.cjs');
const { encodeIntegerSequence } = require('./lib/astc-encode-sequence.cjs');

const checks = [];

function check(name, ok, detail) {
    checks.push(ok);
    console.log((ok ? 'ok   ' : 'FAIL ') + name + (ok ? '' : ': ' + detail));
}

const hand = encodeIntegerSequence([21, 41], 13);
check('two values in a range-13 trit tail are written in the hand-derived bits (wrong output: the tail digit shifted or the lows swapped)', hand.bitLength === 12 && hand.bytes[0] === 0x57 && hand.bytes[1] === 0x09, 'bits ' + hand.bitLength + ' bytes ' + hand.bytes[0].toString(16) + ' ' + hand.bytes[1].toString(16));
const decoded = readIntegerSequence(hand.bytes, 0, 2, 13);
check('the hand-derived bits decode to the two values (wrong output: a different pair)', decoded.join(',') === '21,41', 'decoded ' + decoded.join(','));

let seed = 7;
function next(limit) {
    seed = (Math.imul(seed, 1103515245) + 12345) >>> 0;
    return seed % limit;
}

let trials = 0;
let failures = 0;
let firstFailure = '';
for (let range = 0; range < BISE_RANGES.length; range += 1) {
    const [bits, trits, quints] = BISE_RANGES[range];
    const digitLimit = trits ? 3 : quints ? 5 : 1;
    for (let count = 1; count <= 8; count += 1) {
        for (let t = 0; t < 20; t += 1) {
            const values = [];
            for (let i = 0; i < count; i += 1) values.push((next(digitLimit) << bits) | next(1 << bits));
            const { bytes, bitLength } = encodeIntegerSequence(values, range);
            const back = readIntegerSequence(bytes, 0, count, range);
            trials += 1;
            if (bitLength !== integerSequenceBits(range, count) || back.join(',') !== values.join(',')) {
                failures += 1;
                if (!firstFailure) firstFailure = 'range ' + range + ' count ' + count + ' wrote ' + values.join(',') + ' read ' + back.join(',') + ' bits ' + bitLength + ' expected ' + integerSequenceBits(range, count);
            }
        }
    }
}
check('every range and every count from 1 to 8 reads back as written, with the length the decoder expects (wrong output: a value or a bit count that differs)', failures === 0, failures + ' of ' + trials + ' trials; first ' + firstFailure);

const passed = checks.filter(Boolean).length;
console.log(passed === checks.length ? 'ALL PASS ' + passed + ' checks' : 'FAIL ' + (checks.length - passed) + ' of ' + checks.length);
process.exit(passed === checks.length ? 0 : 1);
