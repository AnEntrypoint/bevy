import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-height-delta.json')
const gitStatus = () => execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
const rev = execFileSync('git', ['-C', spoint, 'rev-parse', '--short', 'HEAD']).toString().trim()
const statusBefore = gitStatus()
const { CELL_M, RAISE_REQUIRES, FLATTEN_REQUIRES, CARVE_REQUIRES, SMOOTH_REQUIRES, createHeightDelta, loadHeightDelta } = await import(pathToFileURL(join(spoint, 'src/terrain/HeightDelta.js')).href)

const encode = (value) => JSON.parse(JSON.stringify(value, (_, item) => (typeof item === 'number' && !Number.isFinite(item) ? String(item) : item)))
const finiteOrNull = (value) => (Number.isFinite(value) ? value : null)
const baseHeight = (x, z) => (x > 16 ? NaN : 2 + 0.5 * Math.sin(0.3 * x) + 0.25 * Math.cos(0.4 * z) + 0.005 * x * z)

const points = []
for (let i = 0; i <= 10; i++) {
  for (let j = 0; j <= 10; j++) points.push({ x: -8 + i * 2.25, z: -8 + j * 2.25 })
}
const sample = (hd) => points.map((p) => hd.deltaAt(p.x, p.z))
const state = (hd) => ({ cellCount: hd.cellCount, strokeCount: hd.strokeCount, toJSON: hd.toJSON(), values: sample(hd) })

const plan = [
  { op: 'raise', x: 4.5, z: 4.5, radius: 3, strength: 2 },
  { op: 'raise', x: 10.2, z: 3.7, radius: 2, strength: -1.5 },
  { op: 'smooth', x: 5, z: 5, radius: 3, strength: 0.5 },
  { op: 'flatten', x: 14, z: 8, radius: 4, targetHeight: 3, strength: 0.8 },
  { op: 'rectangularCarve', x: -6, z: 2, halfWidth: 3, halfDepth: 2, targetDepth: -1, strength: 1.2 },
  { op: 'smooth', x: 2, z: 9, radius: 2.5, strength: 2 },
  { op: 'raise', x: 1, z: 1, radius: 0, strength: 1 },
  { op: 'flatten', x: 3, z: 3, radius: 2, targetHeight: 1, strength: 0 },
  { op: 'flatten', x: 3, z: 3, radius: 2, targetHeight: 1, strength: NaN },
  { op: 'rectangularCarve', useBase: false, x: 0, z: 0, halfWidth: 1, halfDepth: 1, targetDepth: 0, strength: 1 },
  { op: 'rectangularCarve', x: 0, z: 0, halfWidth: 1, halfDepth: 1, targetDepth: 0, strength: 0 },
  { op: 'flatten', x: 18, z: 0, radius: 1, targetHeight: 2, strength: 0.5 },
  { op: 'raise', x: -3, z: 12, radius: 1.5, strength: 0.75 },
  { op: 'smooth', x: 0, z: 0, radius: 1, strength: 0 },
  { op: 'rectangularCarve', x: -2, z: -2, halfWidth: 2.5, halfDepth: 1.5, targetDepth: 0.5, strength: 0.6 },
].map((step) => ({ useBase: true, ...step }))

const apply = (hd, step, base) => {
  if (step.op === 'raise') return hd.applyRaiseBrush(step.x, step.z, step.radius, step.strength)
  if (step.op === 'smooth') return hd.applySmoothBrush(step.x, step.z, step.radius, step.strength)
  if (step.op === 'flatten') return hd.applyFlattenBrush(base, step.x, step.z, step.radius, step.targetHeight, step.strength)
  if (step.op === 'rectangularCarve') return hd.applyRectangularCarve(base, step.x, step.z, step.halfWidth, step.halfDepth, step.targetDepth, step.strength)
  throw new Error('unknown op ' + step.op)
}

const api = createHeightDelta([], baseHeight)
const planRecords = plan.map((step) => {
  const result = apply(api, step, step.useBase ? baseHeight : null)
  return { step, result, cellCount: api.cellCount, strokeCount: api.strokeCount }
})
const final = state(api)
const reloaded = loadHeightDelta(JSON.parse(JSON.stringify(api.toJSON())), baseHeight)
const roundTrip = state(reloaded)

const flattenRecord = { x: 14, z: 8, radius: 4, strength: 0.8, targetHeight: 3, brush: 'flatten' }
const LOAD_CASES = [
  { name: 'null json', json: null, useBase: true },
  { name: 'empty strokes', json: { version: 1, cellM: 1, strokes: [] }, useBase: true },
  { name: 'malformed strokes skipped', json: { version: 1, cellM: 1, strokes: [{ x: 1, z: 2, strength: 1, brush: 'raise' }, { x: 0, z: 0, radius: 2, strength: 1, brush: 'bogus' }, null, { x: 0, z: 0, radius: 2, strength: 1, brush: 'flatten' }] }, useBase: true },
  { name: 'flatten needs base', json: { version: 1, cellM: 1, strokes: [flattenRecord] }, useBase: false },
  { name: 'flatten with base', json: { version: 1, cellM: 1, strokes: [flattenRecord] }, useBase: true },
  { name: 'strength clamps on load', json: { version: 1, cellM: 1, strokes: [{ x: 2, z: 2, radius: 3, strength: 5, brush: 'smooth' }, { x: -4, z: 1, radius: 3, strength: 0.5, brush: 'raise' }, { x: 3, z: -2, radius: 2, strength: 0.25, brush: 'lower' }] }, useBase: true },
]
const loadCases = LOAD_CASES.map(({ name, json, useBase }) => ({ name, json, useBase, ...state(loadHeightDelta(json, useBase ? baseHeight : null)) }))

if (gitStatus() !== statusBefore) throw new Error('spoint working tree changed')
writeFileSync(outPath, JSON.stringify(encode({
  spoint: rev,
  cellM: CELL_M,
  requires: { raise: RAISE_REQUIRES, flatten: FLATTEN_REQUIRES, rectangularCarve: CARVE_REQUIRES, smooth: SMOOTH_REQUIRES },
  points,
  baseValues: points.map((p) => finiteOrNull(baseHeight(p.x, p.z))),
  plan: planRecords,
  final,
  roundTrip,
  loadCases,
}), null, 1) + '\n')
if (gitStatus() !== statusBefore) throw new Error('spoint working tree changed')
console.log(JSON.stringify({ rev, points: points.length, plan: plan.length, cellCount: final.cellCount, strokeCount: final.strokeCount, loadCases: loadCases.length, roundTripMatches: JSON.stringify(roundTrip.toJSON) === JSON.stringify(final.toJSON) }))
