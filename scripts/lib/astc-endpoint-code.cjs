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

module.exports = { codeMap, classifyGpuValue };
