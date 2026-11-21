import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const git = (args) => execFileSync('git', ['-C', spoint, ...args]).toString()
const rev = git(['rev-parse', '--short', 'HEAD']).trim()
const statusBefore = git(['status', '--porcelain'])
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-fx-program.json')
const { parseFxProgram, createFxSimulation, FxProgramError } = await import(pathToFileURL(join(spoint, 'src/effects/fxProgram.js')).href)

const outcome = (fn) => {
  try {
    return { ok: true, value: fn() }
  } catch (e) {
    if (!(e instanceof FxProgramError)) throw e
    const prefix = '[fxProgram] ' + e.reasonCode
    const detail = e.reasonCode === 'json-unreadable' || e.message === prefix ? null : e.message.slice(prefix.length + 2)
    return { ok: false, reasonCode: e.reasonCode, detail }
  }
}

const doc = (seed, pops) => (seed === null ? '{' : '{"seed":' + seed + ',') + '"populations":[' + pops.join(',') + ']}'
const P_SPARKS = '{"name":"sparks","spawn":{"rate":12,"burst":3},"lifetime":[0.5,1.5],"velocity":{"speed":[1,4],"coneDeg":30},"accelerate":{"gravity":[0,-9.8,0],"drag":0.5},"render":{"kind":"sprite","blend":"add"},"maxParticles":40}'
const P_DUST = '{"name":"dust","spawn":{"rate":5},"render":{"kind":"mesh"}}'
const P_SMOKE = '{"name":"smoke","spawn":{"rate":2.5,"burst":0},"lifetime":[1,2],"velocity":{"speed":[0.5,0.5],"coneDeg":180},"accelerate":{"gravity":[0,1,0],"drag":0},"render":{"kind":"sprite","blend":"screen"}}'

const texts = [
  doc(7, [P_SPARKS]),
  doc(null, [P_DUST]),
  doc(4294967295, [P_SPARKS, P_SMOKE]),
  '{"seed":0,"populations":[{"name":"a","spawn":{"rate":0},"lifetime":[1,1],"render":{"kind":"sprite"},"velocity":null,"accelerate":null,"maxParticles":null}]}',
  '{"populations":[{"name":"b","spawn":{"rate":3,"burst":2.0},"render":{"kind":"mesh","blend":"multiply"},"maxParticles":2.5}]}',
  '{"populations":[{"name":"c","spawn":{"rate":1},"render":{"kind":"sprite","blend":"subtract"},"velocity":{"speed":[2,2]},"accelerate":{"gravity":[1,2,3]},"lifetime":[0,600]}]}',
  '{"populations":[{"name":"d","spawn":{"rate":10000,"burst":100000},"render":{"kind":"sprite"},"maxParticles":100000,"lifetime":[0,0.5]}]}',
  'not json{',
  '',
  '[]',
  'null',
  '{"populations":{}}',
  '{"populations":[]}',
  doc(null, Array(17).fill(P_DUST)),
  doc(-1, [P_DUST]),
  doc(1.5, [P_DUST]),
  doc(4294967296, [P_DUST]),
  doc('"7"', [P_DUST]),
  '{"populations":[5]}',
  '{"populations":[null]}',
  '{"populations":[{"spawn":{"rate":1},"render":{"kind":"mesh"}}]}',
  '{"populations":[{"name":"","spawn":{"rate":1},"render":{"kind":"mesh"}}]}',
  '{"populations":[{"name":"x","spawn":{"rate":-1},"render":{"kind":"mesh"}}]}',
  '{"populations":[{"name":"x","spawn":{"rate":10001},"render":{"kind":"mesh"}}]}',
  '{"populations":[{"name":"x","render":{"kind":"mesh"}}]}',
  '{"populations":[{"name":"x","spawn":{"rate":1,"burst":1.5},"render":{"kind":"mesh"}}]}',
  '{"populations":[{"name":"x","spawn":{"rate":1,"burst":100001},"render":{"kind":"mesh"}}]}',
  '{"populations":[{"name":"x","spawn":{"rate":1},"lifetime":[2,1],"render":{"kind":"mesh"}}]}',
  '{"populations":[{"name":"x","spawn":{"rate":1},"lifetime":[0,601],"render":{"kind":"mesh"}}]}',
  '{"populations":[{"name":"x","spawn":{"rate":1},"lifetime":[1],"render":{"kind":"mesh"}}]}',
  '{"populations":[{"name":"x","spawn":{"rate":1},"render":{"kind":"mesh2"}}]}',
  '{"populations":[{"name":"x","spawn":{"rate":1}}]}',
  '{"populations":[{"name":"x","spawn":{"rate":1},"render":{"kind":"sprite","blend":"xor"}}]}',
  '{"populations":[{"name":"x","spawn":{"rate":1},"render":{"kind":"sprite","blend":null}}]}',
  '{"populations":[{"name":"x","spawn":{"rate":1},"render":{"kind":"sprite"},"maxParticles":0}]}',
  '{"populations":[{"name":"x","spawn":{"rate":1},"render":{"kind":"sprite"},"maxParticles":100001}]}',
  '{"populations":[{"name":"x","spawn":{"rate":1},"render":{"kind":"sprite"},"velocity":{"speed":[3,1]}}]}',
  '{"populations":[{"name":"x","spawn":{"rate":1},"render":{"kind":"sprite"},"velocity":{"speed":[0,1001]}}]}',
  '{"populations":[{"name":"x","spawn":{"rate":1},"render":{"kind":"sprite"},"velocity":{"coneDeg":181}}]}',
  '{"populations":[{"name":"x","spawn":{"rate":1},"render":{"kind":"sprite"},"velocity":"fast"}]}',
  '{"populations":[{"name":"x","spawn":{"rate":1},"render":{"kind":"sprite"},"accelerate":{"gravity":[0,0]}}]}',
  '{"populations":[{"name":"x","spawn":{"rate":1},"render":{"kind":"sprite"},"accelerate":{"drag":101}}]}',
  '{"populations":[{"name":"x","spawn":{"rate":1},"render":{"kind":"sprite"},"accelerate":[]}]}',
  doc(null, [P_DUST, P_DUST]),
]

const simulations = [
  { text: doc(7, [P_SPARKS]), dts: Array.from({ length: 24 }, (_, i) => [0.1, 0.25, 0.016, 0.5][i % 4]).concat([0, -0.1, 1.5, 0.1]) },
  { text: doc(4294967295, [P_SPARKS, P_SMOKE]), dts: Array.from({ length: 40 }, () => 0.05) },
  { text: doc(null, [P_DUST]), dts: [0.2, 0.2, 0.1, 0.5] },
  { text: doc(null, ['{"name":"cap","spawn":{"rate":100},"lifetime":[1,1],"render":{"kind":"sprite"},"maxParticles":5}']), dts: Array.from({ length: 15 }, () => 0.1) },
  { text: doc(3, ['{"name":"burst","spawn":{"rate":0,"burst":4},"lifetime":[0.3,0.6],"velocity":{"speed":[1,2],"coneDeg":90},"accelerate":{"gravity":[0,-1,0],"drag":2},"render":{"kind":"mesh","blend":"alpha"}}']), dts: [0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1] },
]

const fixture = {
  spoint: rev,
  source: 'spoint ' + rev + ': src/effects/fxProgram.js (parseFxProgram, createFxSimulation)',
  spointStatusBefore: statusBefore,
  parse: texts.map((text) => ({ text, expect: outcome(() => parseFxProgram(text)) })),
  simulate: simulations.map(({ text, dts }) => {
    const sim = createFxSimulation(parseFxProgram(text))
    const initial = sim.snapshot()
    return { text, initial, steps: dts.map((dt) => ({ dt, expect: outcome(() => sim.step(dt)) })) }
  }),
}

const statusAfter = git(['status', '--porcelain'])
if (statusAfter !== statusBefore) throw new Error('spoint working tree changed')
fixture.spointStatusAfter = statusAfter
writeFileSync(outPath, JSON.stringify(fixture, null, 1) + '\n')
console.log(JSON.stringify({ rev, out: outPath, status: 'unchanged', parse: fixture.parse.length, simulate: fixture.simulate.map((s) => s.steps.length) }))
