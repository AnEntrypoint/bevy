#!/usr/bin/env node
const mib = (pages) => (Number(pages) * 4) / 1024;

function classify({ crates, swapInMiB, swapOutMiB, thresholdMiB = Number(process.env.NOISY_MIB || 10) }) {
    if (Number(crates) > 1) {
        return { swapNoisy: null, rule: 'not classified: more than one crate compiled, and swap does not predict the wall time of those builds' };
    }
    const swapNoisy = swapInMiB + swapOutMiB > thresholdMiB;
    return { swapNoisy, rule: `classified: swap in plus out above ${thresholdMiB} MiB` };
}

if (require.main === module) {
    const [crates, swapInPages, swapOutPages] = process.argv.slice(2);
    const { swapNoisy, rule } = classify({ crates, swapInMiB: mib(swapInPages), swapOutMiB: mib(swapOutPages) });
    process.stdout.write(`${swapNoisy === null ? 'null' : swapNoisy}|${rule}\n`);
}

module.exports = { classify };
