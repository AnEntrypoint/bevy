import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const git = (args) => execFileSync('git', ['-C', spoint, ...args]).toString()
const rev = git(['rev-parse', '--short', 'HEAD']).trim()
const statusBefore = git(['status', '--porcelain'])
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-flight-model.json')
const { rotateByQuat, flightForces } = await import(pathToFileURL(join(spoint, 'src/physics/FlightModel.js')).href)
const rotateInputs = [
  { q: [0, 0, 0.7071067811865476, 0.7071067811865476], v: [1, 0, 0] },
  { q: [0, 0, 2, 2], v: [1, 2, 3] },
  { q: [0.1, 0.2, 0.3, 0.4], v: [-1, 0.5, 2] },
  { q: [0, 0, 0, 1], v: [0, 3, 4] },
  { q: [1, 0, 0, 0], v: [0, 3, 4] },
  { q: [0.5, -0.5, 0.5, 0.5], v: [2, -3, 1] },
]
const flightInputs = [
  { velocity: [20, -1, 3], rotation: [0, 0, 0, 1], opts: {} },
  { velocity: [12, 4, -6], rotation: [0.1, 0.2, 0.3, 0.9], opts: {} },
  { velocity: [3, 0, 2], rotation: [0, 0, 0, 1], opts: { stallSpeed: 5 } },
  { velocity: [10, 0, 1], rotation: [0, 0, 0, 1], opts: { thrust: 500 } },
  { velocity: [0, 0, 0], rotation: [0, 0, 0, 1], opts: {} },
  { velocity: [2, -8, 5], rotation: [0, 0, 0.3826834323650898, 0.9238795325112867], opts: { liftSlope: 3, maxLiftCoeff: 1.2, wingArea: 14, airDensity: 1, parasiteDrag: 0.05, inducedDragFactor: 0.1, thrust: 120, stallSpeed: 2 } },
  { velocity: [-6, 5, 9], rotation: [0.3, -0.4, 0.2, 0.8], opts: { wingArea: 0, thrust: 40 } },
  { velocity: [0, -15, 20], rotation: [0.5, 0, 0, 0.5], opts: { stallSpeed: 30 } },
  { velocity: [7, 2, -4], rotation: [0, 0.7071067811865476, 0, 0.7071067811865476], opts: { maxLiftCoeff: 0.5 } },
]
const rotate = rotateInputs.map((c) => ({ ...c, expect: rotateByQuat(c.q, c.v) }))
const flight = flightInputs.map((c) => {
  const r = flightForces({ velocity: c.velocity, rotation: c.rotation }, c.opts)
  return { ...c, expect: { force: r.force, speed: r.speed, aoa: r.aoa, liftCoeff: r.liftCoeff, stalled: r.stalled } }
})
const statusAfter = git(['status', '--porcelain'])
if (statusAfter !== statusBefore) throw new Error('spoint working tree changed')
const fixture = {
  spoint: rev,
  source: 'spoint ' + rev + ': src/physics/FlightModel.js (rotateByQuat, flightForces)',
  spointStatusBefore: statusBefore,
  spointStatusAfter: statusAfter,
  rotate,
  flight,
}
writeFileSync(outPath, JSON.stringify(fixture, null, 1) + '\n')
console.log(JSON.stringify({ rev, out: outPath, status: 'unchanged', rotateCases: rotate.length, flightCases: flight.length }))
