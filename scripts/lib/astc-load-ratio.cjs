function decideLoad({ astcMs, referenceMs, tolerance }) {
    const inputs = { astcMs, referenceMs, tolerance };
    for (const [name, value] of Object.entries(inputs)) {
        if (!Number.isFinite(value) || value <= 0) throw new Error(`${name} must be a positive number, got ${value}`);
    }
    return { ratio: astcMs / referenceMs, tolerance, usable: astcMs <= referenceMs * tolerance };
}

function formatLoadLine(fields) {
    return ['ASTC_LOAD', ...Object.entries(fields).map(([key, value]) => `${key}=${value}`)].join(' ');
}

module.exports = { decideLoad, formatLoadLine };
