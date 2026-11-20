import { execFileSync } from 'node:child_process'
import { mkdirSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const out = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-ballistics-quat.json')
const ballistics = await import(pathToFileURL(join(spoint, 'src/shared/ballistics.js')).href)
const quat = await import(pathToFileURL(join(spoint, 'src/shared/quat.js')).href)
const rev = execFileSync('git', ['-C', spoint, 'rev-parse', '--short', 'HEAD']).toString().trim()

const cases = [
  { kind: 'solve', from: [0, 0, 0], to: [10, 0, 0], speed: 20, g: 9.81, lead: null },
  { kind: 'solve', from: { x: 0, y: 2, z: 0 }, to: { x: 30, y: -1, z: 12 }, speed: 25, g: 9.81, lead: null },
  { kind: 'solve', from: [0, 0, 0], to: [1000, 0, 0], speed: 5, g: 9.81, lead: null },
  { kind: 'solve', from: [1, 1, 1], to: [1, 1, 1], speed: 10, g: 9.81, lead: null },
  { kind: 'solve', from: [0, 0, 0], to: [40, 3, 0], speed: 30, g: 9.81, lead: [0, 0, 5] },
  { kind: 'solve', from: { x: -5, y: 1, z: 2 }, to: { x: 20, y: 4, z: -6 }, speed: 18, g: 9.81, lead: { x: -2, y: 0, z: 3 } },
  { kind: 'solve', from: [0, 0, 0], to: [25, 10, 25], speed: 22, g: 9.81, lead: [1, 0, -1] },
  { kind: 'elevation', speed: 10, g: 9.81, distance: 5, rise: 0 },
  { kind: 'elevation', speed: 20, g: 9.81, distance: 0, rise: 3 },
  { kind: 'elevation', speed: 5, g: 9.81, distance: 100, rise: 0 },
  { kind: 'elevation', speed: 20, g: 9.81, distance: 30, rise: 2 },
  { kind: 'rotate', q: [0, 0, 0.7071067811865476, 0.7071067811865476], v: [1, 0, 0] },
  { kind: 'rotate', q: [0, 0, 2, 2], v: [1, 2, 3] },
  { kind: 'rotate', q: [0.1, 0.2, 0.3, 0.4], v: [-1, 0.5, 2] },
  { kind: 'rotate', q: [1, 0, 0, 0], v: [0, 3, 4] },
]

const run = (c) => {
  if (c.kind === 'solve') return ballistics.solveShot(c.from, c.to, c.speed, c.g, c.lead)
  if (c.kind === 'elevation') return ballistics.elevation(c.speed, c.g, c.distance, c.rise)
  return quat.rotate(c.q, c.v)
}

mkdirSync(dirname(out), { recursive: true })
writeFileSync(out, JSON.stringify({ source: `spoint ${rev}: src/shared/ballistics.js solveShot and elevation, src/shared/quat.js rotate`, cases, expected: cases.map(run) }, null, 2) + '\n')
console.log(`wrote ${out} with ${cases.length} cases from spoint ${rev}`)
