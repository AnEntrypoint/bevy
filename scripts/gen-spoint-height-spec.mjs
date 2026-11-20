import { execFileSync } from 'node:child_process'
import { mkdirSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const mapspinner = join(spoint, 'packages/mapspinner/src')
const outDir = join(import.meta.dirname, '..', 'testdata', 'parity')
const rev = execFileSync('git', ['-C', spoint, 'rev-parse', '--short', 'HEAD']).toString().trim()
const { compileHeightSpec, compileSnoise3 } = await import(pathToFileURL(join(mapspinner, 'tsl/ops-jsgen.js')).href)
const { carveChord2 } = await import(pathToFileURL(join(mapspinner, 'tsl/height-spec.js')).href)

const RES = 8
const PLANET_RADIUS_M = 6360

let seed = 20260
const random = () => { seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0; return seed / 4294967296 }
const signed = () => random() * 2 - 1
const vec = (scale) => [signed() * scale, signed() * scale, signed() * scale]

const hpfFaces = Array.from({ length: 6 }, (_, face) => Array.from({ length: RES * RES * 4 }, (_, i) => {
  const c = i % 4, xy = Math.floor(i / 4), x = xy % RES, y = Math.floor(xy / RES)
  return Math.fround(((face * 131 + x * 17 + y * 29 + c * 7) % 128 - 64) / 128)
}))
const hpfFace = (face) => Float32Array.from(hpfFaces[face])

const params = (overrides) => ({ hpfRes: RES, landBias: -100000, beachShelfM: 0, reliefScale: 0, ...overrides })
const unit = (v) => { const l = Math.hypot(v[0], v[1], v[2]); return v.map((c) => c / l) }
const carve = ({ dir, innerM, outerM, targetH }) => ({ dir: unit(dir), innerChord2: carveChord2(innerM, PLANET_RADIUS_M), outerChord2: carveChord2(outerM, PLANET_RADIUS_M), targetH })

const snoiseCases = [
  { hashVersion: 1, precision: 'f32' },
  { hashVersion: 1, precision: 'f64' },
  { hashVersion: 2, precision: 'f64' },
  { hashVersion: 2, precision: 'f32' },
].map((c) => {
  const points = [[1, 2, 3], [-1, 0, 5], [0, 0, 0], [-2, -3, -4]]
  while (points.length < 24) points.push(vec(40))
  const snoise3 = compileSnoise3({ hashVersion: c.hashVersion, precision: c.precision })
  return { ...c, points, expected: points.map((p) => snoise3(p[0], p[1], p[2])) }
})

const composeSpecs = [
  { hashVersion: 1, precision: 'f32', params: params({}), carves: [] },
  { hashVersion: 1, precision: 'f32', params: params({ beachShelfM: 150, reliefScale: 2.5, landBias: -2500.5 }), carves: [] },
  { hashVersion: 1, precision: 'f64', params: params({ beachShelfM: 30.25 }), carves: [] },
  { hashVersion: 2, precision: 'f64', params: params({}), carves: [
    { dir: [0.3, -0.8, 0.5], innerM: 300, outerM: 900, targetH: 12.5 },
    { dir: [-1, 0.2, 0.1], innerM: 1200, outerM: 2000, targetH: -40.25 },
  ] },
  { hashVersion: 2, precision: 'f32', params: params({ landBias: -2500.5, beachShelfM: 30.25, reliefScale: 0.75 }), carves: [
    { dir: [0.9, 0.1, 0.4], innerM: 150, outerM: 600, targetH: 3.75 },
    { dir: [0.2, 0.9, -0.3], innerM: 500, outerM: 1100, targetH: -8.5 },
    { dir: [-0.5, -0.5, 0.7], innerM: 800, outerM: 1600, targetH: 20.125 },
    { dir: [0.1, -0.2, -0.9], innerM: 60, outerM: 120, targetH: 0.5 },
  ] },
]
const compose = composeSpecs.map((c) => {
  const { composeHeight } = compileHeightSpec({ hashVersion: c.hashVersion, carves: c.carves.map(carve), params: c.params, hpfFace, hpfRes: RES, precision: c.precision })
  const dirs = []
  for (let i = 0; i < 16; i++) dirs.push(vec(i % 3 === 0 ? 3.7 : 1))
  return { hashVersion: c.hashVersion, precision: c.precision, params: c.params, carves: c.carves.map(carve), dirs, expected: dirs.map((d) => composeHeight(d[0], d[1], d[2])) }
})

mkdirSync(outDir, { recursive: true })
const fixture = { source: `spoint ${rev}: packages/mapspinner/src/tsl/ops-jsgen.js (compileSnoise3, compileHeightSpec) and tsl/height-spec.js (carveChord2)`, hpfRes: RES, hpfFaces, snoise3: snoiseCases, compose }
writeFileSync(join(outDir, 'spoint-height-spec.json'), JSON.stringify(fixture, (key, value) => (typeof value === 'number' && key !== 'hashVersion' ? String(value) : value)) + '\n')
const snoiseValues = snoiseCases.reduce((n, c) => n + c.expected.length, 0)
const composeValues = compose.reduce((n, c) => n + c.expected.length, 0)
console.log(`wrote ${snoiseValues} snoise3 and ${composeValues} composeHeight values from spoint ${rev}`)
