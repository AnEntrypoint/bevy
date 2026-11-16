#!/usr/bin/env node
const fs = require('node:fs');

const ERROR_LIMIT = 0.05;
const MIN_MOVE = 0.5;
const SPAWN = [0, 1.5, 0];

function records(file, kind) {
    return fs
        .readFileSync(file, 'utf8')
        .split('\n')
        .filter((line) => line.includes(`"kind":"${kind}"`))
        .map((line) => {
            try {
                return JSON.parse(line);
            } catch {
                return { kind, unparsed: line };
            }
        });
}

const [clientFile, serverFile] = process.argv.slice(2);
const reports = records(clientFile, 'client-report');
const report = reports.length === 1 ? reports[0] : {};
const moved = report.current
    ? Math.hypot(report.current[0] - SPAWN[0], report.current[1] - SPAWN[1], report.current[2] - SPAWN[2])
    : 0;
const states = records(serverFile, 'server-state');
const checks = {
    singleClientReport: reports.length === 1,
    onePlayer: report.players === 1,
    onePredicted: report.predicted === 1,
    oneControlled: report.controlled === 1,
    noRemote: report.remote === 0,
    predictionHistory: report.predictionHistory > 0,
    confirmedHistory: report.confirmedHistory > 0,
    confirmedTick: report.confirmedTick > 0,
    predictionMatchesConfirmed: typeof report.errorM === "number" && report.errorM <= ERROR_LIMIT,
    playerMoved: moved >= MIN_MOVE,
    serverSawClient: states.some((state) => state.clients >= 1 && state.players?.length >= 1),
};
const ok = Object.values(checks).every(Boolean);
console.log(JSON.stringify({
    ok,
    players: report.players,
    predicted: report.predicted,
    controlled: report.controlled,
    remote: report.remote,
    confirmedTick: report.confirmedTick,
    errorM: report.errorM,
    errorLimit: ERROR_LIMIT,
    moved,
    minMove: MIN_MOVE,
    serverStates: states.length,
    checks,
}, null, 2));
process.exit(ok ? 0 : 1);
