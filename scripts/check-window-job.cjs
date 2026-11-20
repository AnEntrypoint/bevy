#!/usr/bin/env node
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const job = path.join(__dirname, 'window-job.sh');
const vmstatHeader = 'procs -----------memory---------- ---swap-- -----io---- -system-- ------cpu-----';
const vmstatPrint = "const parts = entry.split(' ');console.log(" + JSON.stringify(vmstatHeader) + ");console.log('1 0 0 100 0 0 ' + parts[0] + ' ' + parts[1] + ' 0 0 0 0 0 0 0 0');";
const heavyPrint = "if (entry === 'clear') { console.log('HEAVY_JOBS clear;'); process.exit(0); } console.log('HEAVY_JOBS busy n=1 hot pid=1 cpu=30 node peer;'); process.exit(1);";

function sequenceStub(dir, name, entries, printEntry) {
  const seqFile = path.join(dir, name + '.seq');
  const counter = path.join(dir, name + '.count');
  fs.writeFileSync(seqFile, entries.join('|'));
  const body = [
    "const fs = require('node:fs');",
    'const seq = fs.readFileSync(' + JSON.stringify(seqFile) + ", 'utf8').split('|');",
    'const counter = ' + JSON.stringify(counter) + ';',
    "const n = fs.existsSync(counter) ? Number(fs.readFileSync(counter, 'utf8')) : 0;",
    'fs.writeFileSync(counter, String(n + 1));',
    'const entry = seq[Math.min(n, seq.length - 1)];',
    printEntry,
  ].join('');
  const file = path.join(dir, name + '.cjs');
  fs.writeFileSync(file, body);
  return 'node ' + file;
}

function runJob(index, options) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'window-job-' + index + '-'));
  try {
    const log = path.join(dir, 'window-job.log');
    const loadFile = path.join(dir, 'loadavg');
    const runs = path.join(dir, 'runs.txt');
    fs.writeFileSync(loadFile, (options.load || '2.00') + ' 1.00 1.00 1/100 1');
    const witness = options.witness || ['sh', '-c', 'echo ran >> ' + runs];
    const env = Object.assign({}, process.env, {
      WINDOW_S: String(options.windowS || 3),
      INTERVAL_S: String(options.intervalS || 1),
      GATE_KB: '1000',
      LOAD_MAX: '4',
      MIN_REMAINING_S: String(options.minRemainingS || 0),
      WINDOW_LOG: log,
      VMSTAT_CMD: sequenceStub(dir, 'vmstat', options.vmstat, vmstatPrint),
      HEAVY_CMD: sequenceStub(dir, 'heavy', options.heavy, heavyPrint),
      LOADAVG_FILE: loadFile,
    });
    const result = spawnSync('bash', [job, '--'].concat(witness), { env, encoding: 'utf8', timeout: 60000 });
    const text = fs.existsSync(log) ? fs.readFileSync(log, 'utf8') : '';
    const readings = text.match(/^reading .*$/gm) || [];
    const runCount = fs.existsSync(runs) ? (fs.readFileSync(runs, 'utf8').match(/ran/g) || []).length : 0;
    return { status: result.status, text, readings, runCount };
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
}

const startsOnSecondSample = (r) => r.status === 0 && r.runCount === 1 && r.readings.length === 2 && /gate=closed/.test(r.readings[0]) && /gate=open/.test(r.readings[1]);

const cases = [
  { name: 'a swap-in spike closes the gate, and the next clear sample starts the witness', options: { vmstat: ['1500 10', '10 10'], heavy: ['clear'] }, ok: startsOnSecondSample },
  { name: 'a swap-out spike alone closes the gate', options: { vmstat: ['10 1500', '10 10'], heavy: ['clear'] }, ok: startsOnSecondSample },
  { name: 'a busy heavy reading holds the start until the next clear reading', options: { vmstat: ['10 10'], heavy: ['busy', 'clear'] }, ok: (r) => r.status === 0 && r.runCount === 1 && r.readings.length === 2 && /heavy=busy/.test(r.readings[0]) && /heavy=clear/.test(r.readings[1]) },
  { name: 'a load above 4 closes the gate, and the window ends without a start', options: { vmstat: ['10 10'], heavy: ['clear'], load: '5.00', windowS: 3 }, ok: (r) => r.status === 3 && r.runCount === 0 && r.readings.length >= 2 && r.readings.every((line) => /gate=closed/.test(line)) },
  { name: 'a heavy rule that never clears ends the window without a start', options: { vmstat: ['10 10'], heavy: ['busy'], windowS: 3 }, ok: (r) => r.status === 3 && r.runCount === 0 && r.text.includes('WINDOW-ENDED-WITHOUT-CLEAR') },
  { name: 'a failing witness sets the job exit status', options: { vmstat: ['10 10'], heavy: ['clear'], witness: ['false'] }, ok: (r) => r.status === 1 && r.text.includes('overall=1') },
  { name: 'readings come at the interval (two or more in a 3 s window at 1 s)', options: { vmstat: ['10 10'], heavy: ['busy'], windowS: 3, intervalS: 1 }, ok: (r) => r.readings.length >= 2 },
  { name: 'a clear sample with no witness ends the job without a run', options: { vmstat: ['10 10'], heavy: ['clear'], witness: [] }, ok: (r) => r.status === 0 && r.runCount === 0 && r.text.includes('no-witness') },
  { name: 'a clear sample with enough window left (10 s, 3 s needed) starts the witness', options: { vmstat: ['2000 0', '10 10'], heavy: ['clear'], windowS: 10, intervalS: 1, minRemainingS: 3 }, ok: startsOnSecondSample },
  { name: 'a clear sample reached with too little window left (4 s, 3 s needed) is refused', options: { vmstat: ['2000 0', '2000 0', '10 10'], heavy: ['clear'], windowS: 4, intervalS: 1, minRemainingS: 3 }, ok: (r) => r.status === 4 && r.runCount === 0 && r.text.includes('START-REFUSED') && r.readings.length === 2 && r.readings.every((line) => /gate=closed/.test(line)) },
];

let failed = 0;
cases.forEach((testCase, index) => {
  const result = runJob(index, testCase.options);
  const ok = testCase.ok(result);
  console.log((ok ? 'ok   ' : 'FAIL ') + testCase.name);
  if (!ok) failed = 1;
});
process.exit(failed);
