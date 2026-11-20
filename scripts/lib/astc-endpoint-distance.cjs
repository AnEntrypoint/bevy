function nearestCodeDistance(value, codeValues) {
    let best = Infinity;
    for (const v of codeValues) {
        const d = Math.abs(value - v);
        if (d < best) best = d;
    }
    return best;
}

function tabulateNoCodeDistances(records, codeValues) {
    const table = new Map();
    for (const r of records) {
        if (r.class !== 'gpu value not a code value') continue;
        const key = 'distance ' + nearestCodeDistance(r.gpuValue, codeValues);
        table.set(key, (table.get(key) || 0) + 1);
    }
    return table;
}

module.exports = { nearestCodeDistance, tabulateNoCodeDistances };
