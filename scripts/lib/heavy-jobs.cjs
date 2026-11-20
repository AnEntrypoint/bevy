const COMPILERS = new Set(['cc1plus', 'cc1', 'rustc', 'cargo', 'zig', 'cmake', 'ninja', 'collect2', 'clang', 'lld', 'mold', 'sccache']);
const COMPILE_CPU_FLOOR = 5;

function parseTop(text) {
    const frames = [];
    for (const line of text.split('\n')) {
        if (/^\s*PID\b/.test(line)) {
            frames.push([]);
            continue;
        }
        const fields = line.trim().split(/\s+/);
        if (frames.length === 0 || fields.length < 9 || !/^\d+$/.test(fields[0])) continue;
        frames[frames.length - 1].push({ pid: Number(fields[0]), cpu: Number(fields[8]) });
    }
    return frames;
}

function parsePs(text) {
    return text.split('\n').map((line) => line.trim()).filter(Boolean).map((line) => {
        const [pid, rss, ...rest] = line.split(/\s+/);
        return { pid: Number(pid), rss: Number(rss), args: rest.join(' ') };
    });
}

function classify(psRows, topFrames) {
    const maps = topFrames.map((frame) => new Map(frame.map((row) => [row.pid, row.cpu])));
    const sustained = (pid) => maps.length > 0 && maps.every((map) => (map.get(pid) ?? -Infinity) > COMPILE_CPU_FLOOR);
    const found = [];
    for (const row of psRows) {
        const words = row.args.split(' ');
        const base = (words[0] || '').split('/').pop();
        let kind = null;
        if (base === 'cargo' && ['test', 'check', 'nextest'].includes(words[1])) kind = 'test-check';
        else if (COMPILERS.has(base)) kind = 'compile';
        if (kind !== null && sustained(row.pid)) found.push({ kind, pid: row.pid, cpu: maps[maps.length - 1].get(row.pid), args: row.args });
    }
    return found;
}

module.exports = { parseTop, parsePs, classify, COMPILE_CPU_FLOOR };
