const COMPILERS = new Set(['cc1plus', 'cc1', 'rustc', 'cargo', 'zig', 'cmake', 'ninja', 'collect2', 'clang', 'lld', 'mold', 'sccache']);
const CPU_LIMIT = 20;
const COMPILE_CPU_FLOOR = 5;
const RENDERER_RSS_KIB = 1048576;

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
    const sustained = (pid, limit) => maps.length > 0 && maps.every((map) => (map.get(pid) ?? -Infinity) > limit);
    const found = [];
    for (const row of psRows) {
        const words = row.args.split(' ');
        const base = (words[0] || '').split('/').pop();
        if (/gm-core/.test(row.args) && /--handshake/.test(row.args)) continue;
        let kind = null;
        if (base === 'cargo' && ['test', 'check', 'nextest'].includes(words[1])) {
            if (sustained(row.pid, COMPILE_CPU_FLOOR)) kind = 'test-check';
        } else if (COMPILERS.has(base)) {
            if (sustained(row.pid, COMPILE_CPU_FLOOR)) kind = 'compile';
        } else if (/--type=renderer/.test(row.args) && /chrom/.test(row.args) && row.rss > RENDERER_RSS_KIB) {
            if (sustained(row.pid, CPU_LIMIT)) kind = 'chromium-renderer';
        }
        if (kind === null && sustained(row.pid, CPU_LIMIT)) kind = 'hot';
        if (kind !== null) found.push({ kind, pid: row.pid, cpu: maps[maps.length - 1].get(row.pid), args: row.args });
    }
    return found;
}

module.exports = { parseTop, parsePs, classify, CPU_LIMIT, COMPILE_CPU_FLOOR };
