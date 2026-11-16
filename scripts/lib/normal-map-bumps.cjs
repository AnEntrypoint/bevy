const size = 256;
const cell = 64;
const radius = 26;
const strength = 12;

function bump(x, y) {
    const u = ((x % cell) + cell) % cell - cell / 2;
    const v = ((y % cell) + cell) % cell - cell / 2;
    const d2 = u * u + v * v;
    return d2 < radius * radius ? Math.sqrt(1 - d2 / (radius * radius)) : 0;
}

function normal(x, y) {
    const dx = (bump(x + 1, y) - bump(x - 1, y)) / 2;
    const dy = (bump(x, y + 1) - bump(x, y - 1)) / 2;
    const nx = -dx * strength;
    const ny = dy * strength;
    const length = Math.hypot(nx, ny, 1);
    return [nx / length, ny / length, 1 / length];
}

module.exports = { size, cell, radius, strength, bump, normal };
