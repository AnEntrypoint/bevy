import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const git = (args) => execFileSync('git', ['-C', spoint, ...args]).toString()
const rev = git(['rev-parse', '--short', 'HEAD']).trim()
const statusBefore = git(['status', '--porcelain'])
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-press-router.json')
const { routePress } = await import(pathToFileURL(join(spoint, 'src/interaction/pressRouter.js')).href)
const outcome = (fn) => {
  try {
    return { ok: true, value: fn() }
  } catch (e) {
    return { ok: false, name: e.name, reasonCode: e.reasonCode, message: e.message }
  }
}
const scene = [
  { id: 'near', position: [0, 0, 2] },
  { id: 'behind', position: [0, 0, -2] },
  { id: 'far', position: [0, 0, 20] },
]
const origin = [0, 0, 0]
const forward = [0, 0, 1]
const inputs = [
  ['witness-forward', { objects: scene, origin, forward, label: 'Press E' }],
  ['witness-backward', { objects: scene, origin, forward: [0, 0, -1] }],
  ['no-target-sideways', { objects: scene, origin, forward: [1, 0, 0] }],
  ['custom-label', { objects: scene, origin, forward, label: 'Open' }],
  ['tie-keeps-first', { objects: [{ id: 'first', position: [0, 0, 2] }, { id: 'second', position: [0, 0, 2] }], origin, forward }],
  ['tie-keeps-first-reversed', { objects: [{ id: 'second', position: [0, 0, 2] }, { id: 'first', position: [0, 0, 2] }], origin, forward }],
  ['zero-distance-skipped', { objects: [{ id: 'self', position: [0, 0, 0] }, { id: 'near', position: [0, 0, 2] }], origin, forward }],
  ['only-zero-distance', { objects: [{ id: 'self', position: [0, 0, 0] }], origin, forward }],
  ['beyond-max-distance', { objects: scene, origin, forward, maxDistance: 1.5 }],
  ['max-distance-inclusive', { objects: [{ id: 'edge', position: [0, 0, 3] }], origin, forward }],
  ['angle-excluded', { objects: [{ id: 'side', position: [1, 0, 2] }], origin, forward, maxAngleRad: 0.1 }],
  ['angle-included', { objects: [{ id: 'side', position: [1, 0, 2] }], origin, forward, maxAngleRad: 0.5 }],
  ['default-angle-excluded', { objects: [{ id: 'wide', position: [1, 0, 1.5] }], origin, forward }],
  ['angle-zero-on-axis', { objects: [{ id: 'axis', position: [0, 0, 2] }], origin, forward, maxAngleRad: 0 }],
  ['angle-pi-includes-behind', { objects: scene, origin, forward, maxAngleRad: Math.PI }],
  ['unnormalized-forward', { objects: scene, origin, forward: [0, 0, 5] }],
  ['origin-offset', { objects: [{ id: 'a', position: [5, 1, 5] }, { id: 'b', position: [5, 0, 8] }], origin: [5, 0, 5], forward }],
  ['nearest-in-cone', { objects: [{ id: 'p', position: [0, 0, 2.5] }, { id: 'q', position: [0.2, 0, 1.5] }], origin, forward }],
  ['empty-objects', { objects: [], origin, forward }],
  ['objects-null', { objects: null, origin, forward }],
  ['objects-not-array', { objects: 'near', origin, forward }],
  ['object-empty-id', { objects: [{ id: '', position: [0, 0, 1] }], origin, forward }],
  ['object-numeric-id', { objects: [{ id: 5, position: [0, 0, 1] }], origin, forward }],
  ['object-missing-position', { objects: [{ id: 'x' }], origin, forward }],
  ['object-short-position', { objects: [{ id: 'x', position: [0, 1] }], origin, forward }],
  ['second-object-invalid', { objects: [{ id: 'far', position: [0, 0, 20] }, { id: '', position: [0, 0, 1] }], origin, forward }],
  ['origin-invalid', { objects: scene, origin: [0, 0], forward }],
  ['forward-zero', { objects: scene, origin, forward: [0, 0, 0] }],
  ['max-distance-zero', { objects: scene, origin, forward, maxDistance: 0 }],
  ['max-distance-negative', { objects: scene, origin, forward, maxDistance: -1 }],
  ['max-distance-string', { objects: scene, origin, forward, maxDistance: '3' }],
  ['max-angle-over-pi', { objects: scene, origin, forward, maxAngleRad: 4 }],
  ['max-angle-negative', { objects: scene, origin, forward, maxAngleRad: -0.1 }],
  ['max-angle-null', { objects: scene, origin, forward, maxAngleRad: null }],
]
const fixture = {
  spoint: rev,
  source: 'spoint ' + rev + ': src/interaction/pressRouter.js (routePress)',
  cases: inputs.map(([name, input]) => ({ name, input, expect: outcome(() => ({ ok: true, value: routePress(input) }).value) })),
}

const statusAfter = git(['status', '--porcelain'])
if (statusAfter !== statusBefore) throw new Error('spoint working tree changed')
fixture.spointStatusBefore = statusBefore
fixture.spointStatusAfter = statusAfter
writeFileSync(outPath, JSON.stringify(fixture, null, 1) + '\n')
console.log(JSON.stringify({ rev, out: outPath, cases: fixture.cases.length, status: 'unchanged' }))
