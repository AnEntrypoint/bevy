function detect(width, height, rgb, minRun = 40, step = 24) {
  const lum = (x, y) => {
    const i = (y * width + x) * 3;
    return 0.2126 * rgb[i] + 0.7152 * rgb[i + 1] + 0.0722 * rgb[i + 2];
  };
  const runs = [];
  for (let x = 0; x < width - 1; x++) {
    let start = -1;
    for (let y = 0; y <= height; y++) {
      const sharp = y < height && Math.abs(lum(x + 1, y) - lum(x, y)) > step;
      if (sharp && start < 0) start = y;
      if (!sharp && start >= 0) {
        if (y - start >= minRun) runs.push({ x, y0: start, len: y - start });
        start = -1;
      }
    }
  }
  return runs;
}

module.exports = { detect };
