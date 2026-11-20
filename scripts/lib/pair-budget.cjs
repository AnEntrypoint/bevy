const MIN_ARM_BUDGET_S = 360;
const PAIR_WINDOW_S = 1200;

function budget(value) {
    const seconds = Number(value);
    return Number.isFinite(seconds) ? Math.max(MIN_ARM_BUDGET_S, seconds) : MIN_ARM_BUDGET_S;
}

function armBudgets(env) {
    const formatS = budget(env.PAIR_FORMAT_BUDGET_S);
    const referenceS = budget(env.PAIR_REFERENCE_BUDGET_S);
    return { formatS, referenceS, pairS: formatS + referenceS };
}

function canStartPair(remainingS, pairS) {
    return remainingS >= pairS;
}

function latestPairStartS(pairS) {
    return PAIR_WINDOW_S - pairS;
}

function pairStartAllowedAt(elapsedS, pairS) {
    return canStartPair(PAIR_WINDOW_S - elapsedS, pairS);
}

module.exports = { MIN_ARM_BUDGET_S, PAIR_WINDOW_S, armBudgets, canStartPair, latestPairStartS, pairStartAllowedAt };
