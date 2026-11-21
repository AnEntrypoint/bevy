import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const git = (args) => execFileSync('git', ['-C', spoint, ...args]).toString()
const rev = git(['rev-parse', '--short', 'HEAD']).trim()
const statusBefore = git(['status', '--porcelain'])
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-terrain-mesh-surface.json')
const { createTerrainMeshSurface, renderMeshHeightAt, isMeshSurfaceNoGround, MESH_SURFACE_NO_GROUND } = await import(pathToFileURL(join(spoint, 'src/terrain/TerrainMeshSurface.js')).href)
const { worldToFaceLocal } = await import(pathToFileURL(join(spoint, 'packages/mapspinner/src/planet-orchestrator-cull.js')).href)

const R = 2000
const ANCHOR = 12.5
const OFFSET = 3.25
const norm = (v) => { const l = Math.hypot(v[0], v[1], v[2]); return v.map((c) => c / l) }
const cross = (a, b) => [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
const up = norm([0.6, 0.48, 0.64])
const east = norm(cross([0, 1, 0], up))
const north = cross(up, east)
const baseFrame = { radius: R, anchorHeight: ANCHOR, offsetY: OFFSET, up, east, north, chartEpoch: 7 }
const heightAtDir = (d) => 40 * d[0] * d[1] + 25 * d[2] * d[2] - 10 * d[0] + 3 * d[0] * d[1] * d[2] + 5
const camWorld = (fx, fy) => [0, 1, 2].map((i) => up[i] * (R + ANCHOR) + east[i] * fx + north[i] * fy)
const faceCam = (face) => worldToFaceLocal(face, camWorld(0, 0), R)
const around = (face, tiles) => {
  const [cx, cy] = faceCam(face)
  return tiles.map(([du, dv, l, level]) => ({ face, ox: cx + du, oy: cy + dv, l, level }))
}
const tiles1 = [
  ...around(4, [[-120, -120, 150, 0], [30, -120, 150, 0], [-120, 30, 150, 0], [30, 30, 150, 0], [-300, -40, 80, 0], [300, 300, 400, 0], [900, 900, 150, 0]]),
  ...around(0, [[-100, -100, 120, 0], [20, 20, 120, 0]]),
  ...around(2, [[-100, -80, 140, 0], [40, -40, 100, 0]]),
  { face: 4, ox: 0, oy: 0, l: 0, level: 0 },
  { face: 9, ox: 0, oy: 0, l: 100, level: 0 },
]
const payload = (tiles, gridSize = 4) => ({ count: tiles.length, gridSize, offsets: tiles.flatMap((t) => [t.ox, t.oy, t.l, t.level]), faces: tiles.map((t) => t.face) })
const payloads = [
  payload(tiles1),
  payload(around(4, [[-200, -160, 200, 1], [0, 0, 180, 1], [-60, 60, 90, 0]])),
  { count: 0, gridSize: 4, offsets: [], faces: [] },
  payload(tiles1, 2),
  null,
]
const latticeOf = (spec) => spec === null ? null : {
  ...(spec.chartEpoch === null ? {} : { chartEpoch: spec.chartEpoch }),
  covers: (x, z) => Math.abs(x) <= 500 && Math.abs(z) <= 400,
  heightAtLocal: (x, z) => 0.5 + 0.002 * x - 0.001 * z + 1e-6 * x * z,
}
const Q = [[0, 0], [20, -15], [-60, 40], [90, -100], [-150, 120], [250, 200], [600, -600], [1200, 0], [-40, -40]]
const queries = () => Q.map(([x, z]) => ({ op: 'query', x, z }))
const frameOf = (overrides = {}) => ({ ...baseFrame, ...overrides })
const jsonFrame = (f) => ({ radius: f.radius, anchorHeight: f.anchorHeight, offsetY: f.offsetY, up: f.up, east: f.east, north: f.north, chartEpoch: f.chartEpoch ?? null })
const runCase = (name, spec) => {
  const frame = frameOf(spec.frame)
  let current = payloads[spec.quads]
  const surface = createTerrainMeshSurface({
    frame,
    heightAtDir,
    quads: () => current,
    lattice: latticeOf(spec.lattice ?? null),
    focusRadiusM: spec.focusRadiusM,
    maxQuads: spec.maxQuads,
  })
  const steps = spec.steps.map((step) => {
    switch (step.op) {
      case 'query': return { ...step, expect: surface.heightAtLocal(step.x, step.z) }
      case 'setFocus': surface.setFocus(step.x, step.z); return step
      case 'sync': surface.sync(); return step
      case 'version': return { ...step, expect: surface.version }
      case 'setQuads': current = payloads[step.index]; return step
      default: throw new Error('unknown op ' + step.op)
    }
  })
  return { name, frame: jsonFrame(frame), lattice: spec.lattice ?? null, quads: spec.quads, focusRadiusM: spec.focusRadiusM ?? null, maxQuads: spec.maxQuads ?? null, steps }
}
const cases = [
  runCase('lattice-lazy-query', { frame: {}, lattice: { chartEpoch: 7 }, quads: 0, steps: [...queries(), { op: 'version' }] }),
  runCase('explicit-sync-swap', { frame: {}, lattice: { chartEpoch: 7 }, quads: 0, steps: [
    { op: 'setFocus', x: 0, z: 0 }, { op: 'sync' }, { op: 'version' }, ...queries(),
    { op: 'setQuads', index: 1 }, { op: 'sync' }, { op: 'version' }, ...queries(),
    { op: 'setQuads', index: 3 }, { op: 'sync' }, { op: 'version' }, ...queries(),
    { op: 'setQuads', index: 2 }, { op: 'sync' }, { op: 'version' }, ...queries(),
    { op: 'setQuads', index: 4 }, { op: 'sync' }, { op: 'version' }, ...queries(),
  ] }),
  runCase('focus-move-sync', { frame: {}, lattice: { chartEpoch: 7 }, quads: 0, steps: [
    { op: 'setFocus', x: -150, z: 120 }, { op: 'sync' }, { op: 'version' }, ...queries(),
    { op: 'setFocus', x: 90, z: -100 }, { op: 'sync' }, { op: 'version' }, ...queries(),
  ] }),
  runCase('no-lattice-small-focus', { frame: {}, lattice: null, quads: 0, focusRadiusM: 60, steps: queries() }),
  runCase('lattice-epoch-mismatch', { frame: {}, lattice: { chartEpoch: 8 }, quads: 0, steps: queries() }),
  runCase('frame-epoch-absent', { frame: { chartEpoch: undefined }, lattice: { chartEpoch: 7 }, quads: 0, steps: queries() }),
  runCase('lattice-without-epoch', { frame: {}, lattice: { chartEpoch: null }, quads: 0, steps: queries() }),
  runCase('eviction-one-quad', { frame: {}, lattice: { chartEpoch: 7 }, quads: 0, maxQuads: 1, steps: queries() }),
  runCase('empty-payload', { frame: {}, lattice: { chartEpoch: 7 }, quads: 2, steps: [...queries(), { op: 'sync' }, ...queries()] }),
  runCase('null-payload', { frame: {}, lattice: null, quads: 4, steps: queries() }),
  runCase('grid-two-payload', { frame: {}, lattice: { chartEpoch: 7 }, quads: 3, steps: queries() }),
]
const refusalCase = (name, spec) => {
  const frame = frameOf(spec.frame)
  let error = null
  try {
    createTerrainMeshSurface({ frame, heightAtDir, quads: () => payloads[0], lattice: null, focusRadiusM: spec.focusRadiusM, maxQuads: spec.maxQuads })
  } catch (e) {
    error = e.message
  }
  return { name, frame: jsonFrame(frame), lattice: null, quads: 0, focusRadiusM: spec.focusRadiusM ?? null, maxQuads: spec.maxQuads ?? null, error }
}
const refusals = [
  refusalCase('radius-zero', { frame: { radius: 0 } }),
  refusalCase('radius-negative', { frame: { radius: -4 } }),
  refusalCase('focus-zero', { focusRadiusM: 0 }),
  refusalCase('focus-negative', { focusRadiusM: -3 }),
  refusalCase('max-quads-fraction', { maxQuads: 0.5 }),
  refusalCase('max-quads-zero', { maxQuads: 0 }),
]
const renderSurface = createTerrainMeshSurface({ frame: frameOf(), heightAtDir, quads: () => payloads[0], lattice: latticeOf({ chartEpoch: 7 }), focusRadiusM: undefined, maxQuads: undefined })
const renderFrame = { renderMeshHeightAtLocal: renderSurface.heightAtLocal }
const probeFn = renderMeshHeightAt(renderFrame)
const render = {
  case: { frame: jsonFrame(frameOf()), lattice: { chartEpoch: 7 }, quads: 0, focusRadiusM: null, maxQuads: null },
  absentIsNull: renderMeshHeightAt({}) === null,
  probes: Q.map(([x, z]) => ({ x, z, expect: probeFn(x, z) })),
}
const sentinel = {
  constant: MESH_SURFACE_NO_GROUND,
  cases: [null, 0, 1.5, -2].map((value) => ({ value, expect: isMeshSurfaceNoGround(value) })),
}
const statusAfter = git(['status', '--porcelain'])
if (statusAfter !== statusBefore) throw new Error('spoint working tree changed')
const fixture = {
  spoint: rev,
  source: 'spoint ' + rev + ': src/terrain/TerrainMeshSurface.js (createTerrainMeshSurface, renderMeshHeightAt, isMeshSurfaceNoGround, MESH_SURFACE_NO_GROUND)',
  payloads,
  cases,
  refusals,
  render,
  sentinel,
  spointStatusBefore: statusBefore,
  spointStatusAfter: statusAfter,
}
writeFileSync(outPath, JSON.stringify(fixture, null, 1) + '\n')
const queryResults = cases.flatMap((c) => c.steps).filter((s) => s.op === 'query')
console.log(JSON.stringify({ rev, out: outPath, status: 'unchanged', cases: cases.length, queries: queryResults.length, hits: queryResults.filter((s) => s.expect !== null).length, renderHits: render.probes.filter((p) => p.expect !== null).length, refusals: refusals.map((r) => r.error) }))
