const MIN_ARM_BUDGET_S = 360;

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

module.exports = { MIN_ARM_BUDGET_S, armBudgets, canStartPair };
