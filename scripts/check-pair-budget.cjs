#!/usr/bin/env node
const { armBudgets, canStartPair, latestPairStartS, pairStartAllowedAt, PAIR_WINDOW_S } = require('./lib/pair-budget.cjs');

const same = (budgets, formatS, referenceS) => budgets.formatS === formatS && budgets.referenceS === referenceS && budgets.pairS === formatS + referenceS;

const cases = [
    { name: 'the defaults are 360 s for each arm, 720 s for the pair', ok: () => same(armBudgets({}), 360, 360) },
    { name: 'the window variable does not set the format arm (667 s window gives 360 s)', ok: () => same(armBudgets({ PAIR_WINDOW_S: '667' }), 360, 360) },
    { name: 'the window variable does not set the reference arm (780 s window gives 360 s)', ok: () => same(armBudgets({ PAIR_WINDOW_S: '780' }), 360, 360) },
    { name: 'the ETC2 arm keeps the fixed budget at every short remaining time', ok: () => [665, 400, 307, 60].every((remainingS) => armBudgets({ PAIR_WINDOW_S: String(remainingS) }).formatS === 360) },
    { name: 'the ETC2 arm keeps 360 s in the 1200 s window whatever the window variable says', ok: () => [1200, 600, 3600].every((windowS) => armBudgets({ PAIR_WINDOW_S: String(windowS) }).formatS === 360) },
    { name: 'a larger explicit format budget is kept (420 s gives 780 s pair)', ok: () => same(armBudgets({ PAIR_FORMAT_BUDGET_S: '420' }), 420, 360) },
    { name: 'an explicit budget below the minimum is raised to 360 s', ok: () => same(armBudgets({ PAIR_FORMAT_BUDGET_S: '100', PAIR_REFERENCE_BUDGET_S: '200' }), 360, 360) },
    { name: 'a non-numeric budget falls back to 360 s', ok: () => same(armBudgets({ PAIR_FORMAT_BUDGET_S: 'abc' }), 360, 360) },
    { name: 'a start with 665 s left is refused for a 720 s pair', ok: () => canStartPair(665, 720) === false },
    { name: 'a start with 700 s left is refused for a 720 s pair', ok: () => canStartPair(700, 720) === false },
    { name: 'a start with exactly 720 s left is allowed', ok: () => canStartPair(720, 720) === true },
    { name: 'the pair window is 1200 s', ok: () => PAIR_WINDOW_S === 1200 },
    { name: 'the latest pair start is 480 s into the window', ok: () => latestPairStartS(720) === 480 },
    { name: 'a start at 800 s into the 1200 s window is refused (400 s left, 720 s needed)', ok: () => pairStartAllowedAt(800, 720) === false },
    { name: 'a start at 480 s into the window is allowed (720 s left)', ok: () => pairStartAllowedAt(480, 720) === true },
    { name: 'a start at 481 s into the window is refused', ok: () => pairStartAllowedAt(481, 720) === false },
    { name: 'a start at 0 s into the window is allowed', ok: () => pairStartAllowedAt(0, 720) === true },
];

let fail = 0;
for (const testCase of cases) {
    const ok = testCase.ok();
    console.log((ok ? 'ok   ' : 'FAIL ') + testCase.name);
    if (!ok) fail = 1;
}
process.exit(fail);
