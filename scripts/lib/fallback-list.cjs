const SAME_WINDOW_GAP_S = 780;
const FALLBACK_FORMATS = ['astc4x4', 'etc2'];

function windowEvidence(witnessMtime, referenceMtime) {
    const gapS = Math.abs(Date.parse(witnessMtime) - Date.parse(referenceMtime)) / 1000;
    return { witnessMtime, referenceMtime, gapS, sameWindow: gapS <= SAME_WINDOW_GAP_S };
}

function checkFallbackList(footprint) {
    const failures = [];
    const list = footprint.fallbackFormats || {};
    const keys = Object.keys(list).sort();
    if (keys.join(',') !== [...FALLBACK_FORMATS].sort().join(',')) failures.push(`list keys are ${keys.join(',') || 'none'}`);
    for (const name of FALLBACK_FORMATS) {
        const verdict = footprint.formats && footprint.formats[name];
        if (!verdict) {
            failures.push(`${name} has no verdict`);
            continue;
        }
        if (list[name] !== verdict.usable) failures.push(`${name} listed ${list[name]} against usable ${verdict.usable}`);
        const evidence = verdict.loadEvidence;
        const consistent = Boolean(evidence) && typeof evidence.gapS === 'number' && evidence.sameWindow === (evidence.gapS <= SAME_WINDOW_GAP_S);
        if (!consistent) failures.push(`${name} window evidence missing or inconsistent`);
    }
    const bcUsable = Boolean(footprint.formats && footprint.formats.bc7 && footprint.formats.bc7.usable);
    if (footprint.decision !== (bcUsable ? 'bc7' : 'rgba8')) failures.push(`decision ${footprint.decision} does not follow bc7 usable ${bcUsable}`);
    return failures;
}

module.exports = { SAME_WINDOW_GAP_S, FALLBACK_FORMATS, windowEvidence, checkFallbackList };
