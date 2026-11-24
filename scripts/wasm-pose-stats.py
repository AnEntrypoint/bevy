import math, os, struct, sys, zlib
def read_png(path):
    data = open(path, 'rb').read()
    assert data[:8] == b'\x89PNG\r\n\x1a\n'
    pos = 8; idat = b''; w = h = depth = ctype = None
    while pos < len(data):
        n, = struct.unpack('>I', data[pos:pos+4]); kind = data[pos+4:pos+8]; body = data[pos+8:pos+8+n]; pos += 12 + n
        if kind == b'IHDR': w, h, depth, ctype, _, _, interlace = struct.unpack('>IIBBBBB', body)
        elif kind == b'IDAT': idat += body
        elif kind == b'IEND': break
    assert depth == 8 and interlace == 0 and ctype in (2, 6), (depth, ctype, interlace)
    bpp = 3 if ctype == 2 else 4
    raw = zlib.decompress(idat); stride = w * bpp
    rows = []; prev = bytearray(stride); i = 0
    for _ in range(h):
        f = raw[i]; line = bytearray(raw[i+1:i+1+stride]); i += 1 + stride
        for x in range(stride):
            a = line[x-bpp] if x >= bpp else 0; b = prev[x]; c = prev[x-bpp] if x >= bpp else 0
            if f == 1: line[x] = (line[x] + a) & 255
            elif f == 2: line[x] = (line[x] + b) & 255
            elif f == 3: line[x] = (line[x] + ((a + b) >> 1)) & 255
            elif f == 4:
                p = a + b - c; pa, pb, pc = abs(p-a), abs(p-b), abs(p-c)
                pred = a if pa <= pb and pa <= pc else (b if pb <= pc else c)
                line[x] = (line[x] + pred) & 255
        rows.append(line); prev = line
    return w, h, bpp, rows
LUMA_STD_MIN = 0.01
DISTINCT_COLOURS_MIN = 32


def pose_stats(path):
    w, h, bpp, rows = read_png(path)
    lumas = []
    colours = set()
    for row in rows:
        for x in range(0, len(row), bpp):
            r, g, b = row[x], row[x + 1], row[x + 2]
            colours.add((r, g, b))
            lumas.append((0.299 * r + 0.587 * g + 0.114 * b) / 255.0)
    mean = sum(lumas) / len(lumas)
    std = math.sqrt(sum((v - mean) ** 2 for v in lumas) / len(lumas))
    return std, len(colours)


if __name__ == '__main__':
    failed = False
    for path in sys.argv[1:]:
        std, colours = pose_stats(path)
        ok = std >= LUMA_STD_MIN and colours >= DISTINCT_COLOURS_MIN
        failed = failed or not ok
        print(os.path.basename(path), 'luma_std', round(std, 4), 'distinct', colours, 'pass', ok)
    sys.exit(1 if failed else 0)
