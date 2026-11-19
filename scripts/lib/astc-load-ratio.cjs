function decideLoad({ astcMs, referenceMs, tolerance }) {
    const inputs = { astcMs, referenceMs, tolerance };
    for (const [name, value] of Object.entries(inputs)) {
        if (!Number.isFinite(value) || value <= 0) throw new Error(`${name} must be a positive number, got ${value}`);
    }
    return { ratio: astcMs / referenceMs, tolerance, usable: astcMs <= referenceMs * tolerance };
}

function formatLine(prefix, fields) {
    return [prefix, ...Object.entries(fields).map(([key, value]) => `${key}=${value}`)].join(' ');
}

function formatLoadLine(fields) {
    return formatLine('ASTC_LOAD', fields);
}

function checkPairs(runs, tolerance) {
    const windows = new Map();
    for (const run of runs) {
        if (!windows.has(run.window)) windows.set(run.window, []);
        windows.get(run.window).push(run);
    }
    const pairs = [];
    let interleaved = runs.length > 0;
    let reason = runs.length > 0 ? 'interleaved' : 'no runs';
    for (const [window, group] of windows) {
        group.sort((a, b) => a.startS - b.startS);
        for (let i = 1; i < group.length; i++) {
            if (group[i].arm === group[i - 1].arm) {
                interleaved = false;
                reason = `window ${window}: ${group[i].arm} runs back to back`;
            }
        }
        if (group.length % 2 !== 0) {
            interleaved = false;
            reason = `window ${window}: odd number of runs`;
        }
        for (let i = 0; i + 1 < group.length; i += 2) {
            const chunk = [group[i], group[i + 1]];
            const astc = chunk.find((run) => run.arm === 'astc');
            const reference = chunk.find((run) => run.arm === 'reference');
            if (!astc || !reference) {
                interleaved = false;
                reason = `window ${window}: a pair lacks one arm`;
                continue;
            }
            pairs.push({ window, order: chunk.map((run) => run.arm).join(','), astcLoadStart: astc.loadStart, referenceLoadStart: reference.loadStart, ...decideLoad({ astcMs: astc.loadMs, referenceMs: reference.loadMs, tolerance }) });
        }
    }
    const allUsable = pairs.length > 0 && pairs.every((pair) => pair.usable);
    if (interleaved && !allUsable) reason = 'a pair ratio is above the tolerance';
    return { interleaved, reason, pairs, ok: interleaved && allUsable };
}

module.exports = { decideLoad, formatLine, formatLoadLine, checkPairs };
