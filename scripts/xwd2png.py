import struct, sys
import numpy as np
from PIL import Image

def main(src, dst):
    data = open(src, 'rb').read()
    hdr = struct.unpack('>25I', data[:100])
    header_size, _, fmt, depth, width, height, _, byte_order, _, _, _, bpp, bpl, _, rmask, gmask, bmask, _, cmap_entries, ncolors = hdr[:20]
    off = header_size
    off += ncolors * 12
    if bpp not in (24, 32):
        raise SystemExit(f'unsupported bpp {bpp}')
    step = bpp // 8
    raw = np.frombuffer(data, dtype=np.uint8, count=bpl * height, offset=off).reshape(height, bpl)
    raw = raw[:, : width * step].reshape(height, width, step)
    if byte_order == 0:
        b, g, r = raw[:, :, 0], raw[:, :, 1], raw[:, :, 2]
    else:
        r, g, b = raw[:, :, step - 3], raw[:, :, step - 2], raw[:, :, step - 1]
    img = np.stack([r, g, b], axis=-1)
    Image.fromarray(img, 'RGB').save(dst)
    print(dst, width, height, 'bpp', bpp, 'byte_order', byte_order)

main(sys.argv[1], sys.argv[2])
