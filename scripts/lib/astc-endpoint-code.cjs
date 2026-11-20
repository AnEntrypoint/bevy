const { unquantEndpoint } = require('./astc-decode.cjs');

function codeMap(range, codes) {
    const byValue = new Map();
    for (let code = 0; code < codes; code += 1) byValue.set(unquantEndpoint(code, range), code);
    return byValue;
}

function classifyGpuValue(cpuValue, gpuValue, byValue) {
    const cpuCode = byValue.get(cpuValue);
    if (cpuCode === undefined) return 'cpu value not a code value';
    const gpuCode = byValue.get(gpuValue);
    if (gpuCode === undefined) return 'gpu value not a code value';
    if (Math.abs(gpuCode - cpuCode) === 1) return 'adjacent code';
    return 'other code';
}

function candidateRecord(cpuValue, gpuValue, byValue) {
    return {
        cpuValue,
        gpuValue,
        cpuCode: byValue.get(cpuValue),
        gpuCode: byValue.get(gpuValue),
        class: classifyGpuValue(cpuValue, gpuValue, byValue),
    };
}

function tabulate(records) {
    const table = new Map();
    for (const r of records) {
        const key = r.class + ' | cpu trit digit ' + Math.floor(r.cpuCode / 64);
        table.set(key, (table.get(key) || 0) + 1);
    }
    return table;
}

module.exports = { codeMap, classifyGpuValue, candidateRecord, tabulate };
