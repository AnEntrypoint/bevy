const PB_19 = 'fedcb000f';

function unquantWithConstant(value, c) {
    const low = value & 63;
    const digit = value >> 6;
    const all = (low & 1) ? 511 : 0;
    let spread = 0;
    for (let i = 0; i < 9; i += 1) {
        spread <<= 1;
        if (PB_19[i] !== '0') {
            spread |= (low >> (PB_19.charCodeAt(i) - 97)) & 1;
        }
    }
    const mixed = ((digit * c + spread) ^ all);
    return (all & 0x80) | (mixed >> 2);
}

function constantsExplaining(entries, maxC) {
    const found = [];
    for (let c = 0; c <= maxC; c += 1) {
        if (entries.every((e) => unquantWithConstant(e.code, c) === e.gpuValue)) found.push(c);
    }
    return found;
}

function blocksExplained(records, c) {
    const byBlock = new Map();
    for (const r of records) {
        if (!byBlock.has(r.block)) byBlock.set(r.block, []);
        byBlock.get(r.block).push(r);
    }
    let explained = 0;
    for (const list of byBlock.values()) {
        if (list.some((r) => unquantWithConstant(r.cpuCode, c) === r.gpuValue)) explained += 1;
    }
    return { explained, blocks: byBlock.size };
}

module.exports = { PB_19, unquantWithConstant, constantsExplaining, blocksExplained };
