const { WEIGHT_LEVELS, WEIGHT_RANGE_BY_FIELD } = require('./lib/astc-decode.cjs');
const { WIDTH, BLOCKS_PER_ROW, uniformTexel, endpointRows, tabulateDeltas, cacheBlocks } = require('./lib/astc-endpoint-readback.cjs');
const { WEIGHT_MODE } = require('./lib/astc-probe.cjs');

const checks = [];

function check(name, ok, detail) {
    checks.push(ok);
    console.log((ok ? 'ok   ' : 'FAIL ') + name + (ok ? '' : ': ' + detail));
}

function pixelsWith(texels) {
    const pixels = new Uint8Array(WIDTH * 4 * 4);
    texels.forEach((texel, r) => {
        const p = ((r >> 2) * WIDTH + (r & 3)) * 4;
        pixels.set([texel[0], texel[1], texel[2], 255], p);
    });
    return pixels;
}

const uniform = pixelsWith(new Array(16).fill([7, 8, 9]));
const got = uniformTexel(uniform, 0);
check('a block whose 16 texels are equal reads as that texel', got !== null && got.join(',') === '7,8,9', 'got ' + JSON.stringify(got));

const mixed = new Array(16).fill([7, 8, 9]);
mixed[5] = [7, 8, 10];
const rejected = uniformTexel(pixelsWith(mixed), 0);
check('a block with one texel different is not read as uniform (wrong output: a texel value)', rejected === null, 'got ' + JSON.stringify(rejected));

const rows = endpointRows(3, 0, [10, 20, 30], [10, 21, 30]);
check('a GPU channel one above the CPU channel is reported as delta 1 (wrong output: delta 0)', rows[1].delta === 1 && rows[0].delta === 0 && rows[2].delta === 0, 'deltas ' + rows.map((r) => r.delta).join(','));

const counts = tabulateDeltas([...endpointRows(0, 0, [1, 2, 3], [1, 2, 3]), ...endpointRows(1, 0, [1, 2, 3], [1, 3, 3])]);
check('the table counts each end, channel and delta separately', counts.get('end 0 channel 0 delta 0') === 2 && counts.get('end 0 channel 1 delta 1') === 1 && counts.get('end 0 channel 1 delta 0') === 1, JSON.stringify([...counts]));

const levels = WEIGHT_LEVELS[WEIGHT_RANGE_BY_FIELD[WEIGHT_MODE]];
check('the weight table for mode 0x242 runs from 0 to 64 in 16 levels, so index 0 reads e0 and index 15 reads e1', levels.length === 16 && levels[0] === 0 && levels[15] === 64, JSON.stringify(levels));

const cached = cacheBlocks(16);
check('the block loader returns 16 blocks in mode 0x242 (wrong output: a block in another mode)', cached.length === 16 && cached.every((blk) => ((blk[0] | (blk[1] << 8)) & 0x7ff) === WEIGHT_MODE), 'modes ' + cached.map((blk) => ((blk[0] | (blk[1] << 8)) & 0x7ff).toString(16)).join(','));
const passed = checks.filter(Boolean).length;
console.log(passed === checks.length ? 'ALL PASS ' + passed + ' checks' : 'FAIL ' + (checks.length - passed) + ' of ' + checks.length);
process.exit(passed === checks.length ? 0 : 1);
