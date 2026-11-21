import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const git = (args) => execFileSync('git', ['-C', spoint, ...args]).toString()
const rev = git(['rev-parse', '--short', 'HEAD']).trim()
const statusBefore = git(['status', '--porcelain'])
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-camera-rig.json')
const { defineCameraRig } = await import(pathToFileURL(join(spoint, 'src/behaviours/cameraRig.js')).href)

const PLAYER = 'p'
const P1 = { position: [1, 2, 3], yaw: 0.3 }
const P2 = { position: [-4, 0.5, 2], yaw: -2.1 }
const FRAMES = [
  { t: 0, position: [0, 0, 0], target: [0, 0, -1] },
  { t: 1, position: [10, 0, 0], target: [0, 1, 0] },
  { t: 3, position: [10, 5, 10], target: [0, 0, 0] }
]
const U = (pose, dt) => ({ pose, dt })
const I = (input) => ({ input })
const one = (kind, extra = {}) => ({ layers: [{ id: 'L', kind, ...extra }] })
const cutscene = (extra = {}) => one('cutscene', { frames: FRAMES, ...extra })

const outcome = (fn) => {
  try {
    return { ok: true, value: fn() }
  } catch (e) {
    if (typeof e.reasonCode !== 'string') throw e
    return { ok: false, reasonCode: e.reasonCode }
  }
}
const defineError = (spec) => {
  const r = outcome(() => defineCameraRig(spec))
  return r.ok ? null : r.reasonCode
}
const runCase = (spec, steps) => {
  const rig = defineCameraRig(spec)
  return steps.map((step) => {
    if ('input' in step) return { input: step.input, returned: rig.onInput(PLAYER, step.input) }
    const r = outcome(() => rig.update(PLAYER, step.pose, step.dt))
    if (!r.ok) return { pose: step.pose, dt: step.dt, error: r.reasonCode }
    const { layer, position, target, done } = r.value
    return { pose: step.pose, dt: step.dt, camera: { layer, position, target, done } }
  })
}

const defineCases = [
  ['ok-orbit', one('orbit')],
  ['ok-default-second', { layers: [{ id: 'A', kind: 'orbit' }, { id: 'B', kind: 'chase' }], default: 'B' }],
  ['ok-default-null', { layers: [{ id: 'A', kind: 'firstPerson' }], default: null }],
  ['spec-null', null],
  ['spec-array', []],
  ['spec-string', 'layers'],
  ['no-layers-missing', {}],
  ['no-layers-empty', { layers: [] }],
  ['no-layers-object', { layers: { id: 'L' } }],
  ['layer-number', { layers: [5] }],
  ['layer-null', { layers: [null] }],
  ['layer-string', { layers: ['orbit'] }],
  ['layer-array', { layers: [[]] }],
  ['layer-no-id', { layers: [{ kind: 'orbit' }] }],
  ['layer-empty-id', { layers: [{ id: '', kind: 'orbit' }] }],
  ['layer-numeric-id', { layers: [{ id: 3, kind: 'orbit' }] }],
  ['layer-unknown-kind', one('warp')],
  ['layer-kind-missing', { layers: [{ id: 'L' }] }],
  ['layer-duplicate-id', { layers: [{ id: 'L', kind: 'orbit' }, { id: 'L', kind: 'chase' }] }],
  ['default-unknown', { layers: [{ id: 'L', kind: 'orbit' }], default: 'M' }],
  ['default-not-string', { layers: [{ id: 'L', kind: 'orbit' }], default: 0 }],
  ['cutscene-no-frames', one('cutscene', { frames: [] })],
  ['cutscene-frames-missing', one('cutscene')],
  ['cutscene-frame-null', one('cutscene', { frames: [null] })],
  ['cutscene-frame-no-t', one('cutscene', { frames: [{ position: [0, 0, 0], target: [0, 0, 1] }] })],
  ['cutscene-frame-short-vec', one('cutscene', { frames: [{ t: 0, position: [0, 0], target: [0, 0, 1] }] })],
  ['cutscene-frame-string-t', one('cutscene', { frames: [{ t: '1', position: [0, 0, 0], target: [0, 0, 1] }] })],
  ['cutscene-unsorted', one('cutscene', { frames: [{ t: 2, position: [0, 0, 0], target: [0, 0, 1] }, { t: 1, position: [0, 0, 0], target: [0, 0, 1] }] })]
]

const cases = [
  ['orbit-defaults', one('orbit'), [U(P1, 0), U(P2, 0.25)]],
  ['orbit-tuned', one('orbit', { distance: 6, minPitch: -0.5, maxPitch: 0.7, sensitivity: 0.01, targetHeight: 2.5 }), [U(P1, 0), I({ lookX: 40, lookY: -30 }), U(P1, 0.1), I({ lookX: -200, lookY: 500 }), U(P2, 0.1), I({ lookX: 10 }), U(P2, 0.1), I({ lookX: 'far', lookY: null }), U(P2, 0.1), I({ lookX: 0, lookY: -1000 }), U(P1, 0)]],
  ['orbit-bad-fields', one('orbit', { distance: 'far', minPitch: null, maxPitch: [], sensitivity: true, targetHeight: {} }), [U(P1, 0), I({ lookX: 50, lookY: 50 }), U(P2, 0)]],
  ['chase-defaults', one('chase'), [U({ position: [0, 0, 0], yaw: 0 }, 0), U({ position: [1, 2, 3], yaw: 1.2 }, 0.5), U({ position: [-3, 1, 7], yaw: -2.5 }, 0), I({ lookX: 5, lookY: 5 }), U(P1, 0)]],
  ['chase-tuned', one('chase', { distance: 3, height: 4, targetHeight: 0.5 }), [U(P1, 0), U(P2, 0)]],
  ['firstperson-defaults', one('firstPerson'), [U(P1, 0), I({ lookX: 100, lookY: -200 }), U(P1, 0), I({ lookX: -5000, lookY: 0 }), U(P2, 0), I({ lookX: 0, lookY: 900 }), U(P2, 0)]],
  ['firstperson-tuned', one('firstPerson', { eyeHeight: 2.2, sensitivity: 0.05, maxPitch: 0.4 }), [I({ lookX: 3, lookY: -4 }), U(P1, 0), I({ lookX: 0, lookY: 100 }), U(P2, 0), I({ lookX: -7, lookY: 2 }), U(P2, 0)]],
  ['cutscene-linear', cutscene(), [U(P1, 0), U(P1, 0.5), U(P1, 0.5), U(P1, 1.5), U(P1, 0.75), U(P1, 0.5)]],
  ['cutscene-loop', cutscene({ loop: true }), [U(P1, 0.7), U(P1, 1.1), U(P1, 2.6), U(P1, 0.9), U(P1, 0.7)]],
  ['cutscene-single-frame', one('cutscene', { frames: [{ t: 0, position: [1, 1, 1], target: [2, 2, 2] }] }), [U(P1, 0), U(P1, 1)]],
  ['cutscene-equal-times', one('cutscene', { frames: [{ t: 0, position: [0, 0, 0], target: [0, 0, 1] }, { t: 0, position: [5, 0, 0], target: [1, 1, 1] }, { t: 2, position: [0, 0, 0], target: [0, 0, 0] }] }), [U(P1, 0.1), U(P1, 1), U(P1, 1.5)]],
  ['pose-and-dt-errors', one('orbit'), [U({ position: [0, 0, 0] }, 0), U({ position: [0, 0, 0], yaw: '0' }, 0), U({ position: [0, 0], yaw: 0 }, 0), U({ position: [0, 0, 0, 0], yaw: 0 }, 0), U(null, 0), U(P1, -1), U(P1, 0.5)]]
]

const fixture = {
  spoint: rev,
  source: 'spoint ' + rev + ': src/behaviours/cameraRig.js (defineCameraRig layer validation; orbit, chase, firstPerson and cutscene pose math; rig update and onInput on the default layer)',
  define: defineCases.map(([name, spec]) => ({ name, spec, error: defineError(spec) })),
  cases: cases.map(([name, spec, steps]) => ({ name, spec, steps: runCase(spec, steps) }))
}

const statusAfter = git(['status', '--porcelain'])
if (statusAfter !== statusBefore) throw new Error('spoint working tree changed')
fixture.spointStatusBefore = statusBefore
fixture.spointStatusAfter = statusAfter
writeFileSync(outPath, JSON.stringify(fixture, null, 1) + '\n')
console.log(JSON.stringify({ rev, out: outPath, status: 'unchanged', defines: fixture.define.length, cases: fixture.cases.length }))
