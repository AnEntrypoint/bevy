import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-spline-carve.json')
const gitStatus = () => execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
const rev = execFileSync('git', ['-C', spoint, 'rev-parse', '--short', 'HEAD']).toString().trim()
const statusBefore = gitStatus()
const { sampleSpline, carveSpline, isCarved, createSplineCarveLayer, createCarvedClimateFilter } = await import(pathToFileURL(join(spoint, 'src/terrain/SplineCarve.js')).href)

const SPLINE_OPEN = [[0, 0], [7, 3], [12, -4], [18, 1], [25, 6]]
const SPLINE_LOOP = [[2, 2], [10, -3], [4, -12], [-6, -5]]
const SPLINE_ROAD = [[-30, 10], [-5, 8], [14, 14]]

const SAMPLE_CASES = [
  { name: 'open', controlPoints: SPLINE_OPEN, stepSize: 1.3, closed: false },
  { name: 'closed', controlPoints: SPLINE_LOOP, stepSize: 2, closed: true },
  { name: 'single_point', controlPoints: [[1, 1]], stepSize: 1, closed: false },
]

const grid = (n) => Array.from({ length: n * n }, (_, i) => 3 + 0.05 * (i % n) - 0.03 * Math.floor(i / n))

const CARVE_CASES = [
  { name: 'river_bank', width: 24, spacing: 1.5, cornerX: -4.5, cornerZ: -2, controlPoints: SPLINE_OPEN, carveWidth: 6, carveDepth: 2.5, bankWidth: 3, nanIndices: [40] },
  { name: 'river_no_bank', width: 24, spacing: 1.5, cornerX: -2, cornerZ: -4, controlPoints: SPLINE_LOOP, carveWidth: 4, carveDepth: 1.25, bankWidth: 0, nanIndices: [] },
  { name: 'degenerate_width', width: 1, spacing: 1.5, cornerX: 0, cornerZ: 0, controlPoints: SPLINE_OPEN, carveWidth: 4, carveDepth: 1, bankWidth: 0, nanIndices: [] },
  { name: 'bad_spacing', width: 4, spacing: 0, cornerX: 0, cornerZ: 0, controlPoints: SPLINE_OPEN, carveWidth: 4, carveDepth: 1, bankWidth: 0, nanIndices: [] },
  { name: 'bad_carve_width', width: 4, spacing: 1, cornerX: 0, cornerZ: 0, controlPoints: SPLINE_OPEN, carveWidth: 0, carveDepth: 1, bankWidth: 0, nanIndices: [] },
]

const carveRecord = (c) => {
  const heightsIn = grid(c.width)
  const heights = heightsIn.slice()
  for (const i of c.nanIndices) heights[i] = NaN
  const r = carveSpline(heights, c.width, c.spacing, c.cornerX, c.cornerZ, c.controlPoints, c.carveWidth, c.carveDepth, { bankWidth: c.bankWidth })
  return {
    ...c,
    heightsIn,
    heights: Array.from(r.heights, (v) => (Number.isFinite(v) ? v : null)),
    cellKeys: [...r.carvedCells].sort((a, b) => a - b),
  }
}
const carveResults = CARVE_CASES.map(carveRecord)

const riverCells = new Set(carveResults[0].cellKeys)
const CELL_PAIRS = [[0, 0], [1, 0], [2, 0], [3, 1], [-1, -1], [0, -2], [5, 5], [-4, 3], [100, -100], [1, 1]]
const isCarvedRecord = {
  cases: CELL_PAIRS.map(([cx, cz]) => ({ cx, cz, carved: isCarved(riverCells, cx, cz) })),
  emptyCarved: isCarved(new Set(), 0, 0),
}

const filter = createCarvedClimateFilter(riverCells)
const climateRecord = {
  enabled: filter !== null,
  emptyEnabled: createCarvedClimateFilter(new Set()) !== null,
  cases: CELL_PAIRS.flatMap(([cx, cz]) => ['temperate', null].map((climate) => ({ climate, cx, cz, out: filter(climate, cx, cz) }))),
}

const LAYER_BASE = (x, z) => (x < -20 ? NaN : 10 + 0.1 * x - 0.05 * z)
const LAYER_CARVES = [
  { controlPoints: SPLINE_OPEN, width: 6, depth: 2.5, kind: 'river', opts: { seed: 7, widthVariance: 0.3, bankErosion: 1.5 } },
  { controlPoints: SPLINE_ROAD, width: 3, depth: 1, kind: 'road', opts: {} },
  { controlPoints: SPLINE_OPEN, width: 4, depth: 1, kind: 'road', opts: {} },
  { controlPoints: [[0, 0]], width: 4, depth: 1, kind: 'river', opts: {} },
  { controlPoints: SPLINE_LOOP, width: 5, depth: 2, kind: 'lava', opts: {} },
  { controlPoints: SPLINE_LOOP, width: 5, depth: 2, kind: 'river', opts: { seed: 19, widthVariance: 0.2, bankErosion: 0.8 } },
]
const layer = createSplineCarveLayer()
const layerCarves = LAYER_CARVES.map((c) => {
  const opts = { seed: 0, widthVariance: 0, bankErosion: 0, ...c.opts }
  const { touched } = layer.carve(c.controlPoints, c.width, c.depth, c.kind, LAYER_BASE, opts)
  return { controlPoints: c.controlPoints, width: c.width, depth: c.depth, kind: c.kind, opts, touched }
})
const SAMPLE_XS = [-25, -8, -4, 0, 3, 6, 10, 15, 20, 26]
const SAMPLE_ZS = [-8, -6, -2, 0, 2, 5, 9, 12]
const emptyLayer = createSplineCarveLayer()
const layerRecord = {
  carves: layerCarves,
  cellCount: layer.cellCount,
  splineCount: layer.splineCount,
  samples: SAMPLE_XS.flatMap((x) => SAMPLE_ZS.map((z) => ({ x, z, delta: layer.deltaAt(x, z), kind: layer.kindAt(x, z) }))),
  empty: { delta: emptyLayer.deltaAt(0, 0), kind: emptyLayer.kindAt(0, 0) },
}

const result = {
  spoint: rev,
  sampleSpline: SAMPLE_CASES.map((c) => ({ ...c, points: sampleSpline(c.controlPoints, c.stepSize, c.closed).map((p) => [p.x, p.z]) })),
  carveSpline: carveResults,
  isCarved: isCarvedRecord,
  carvedClimateFilter: climateRecord,
  splineCarveLayer: layerRecord,
}

if (gitStatus() !== statusBefore) throw new Error('spoint working tree changed')
writeFileSync(outPath, JSON.stringify(result, null, 1) + '\n')
console.log(JSON.stringify({
  rev,
  sampleCounts: result.sampleSpline.map((s) => [s.name, s.points.length]),
  carve: carveResults.map((c) => [c.name, c.cellKeys.length, c.heights.slice(0, 3)]),
  isCarved: isCarvedRecord.cases.map((c) => c.carved),
  climateEnabled: climateRecord.enabled,
  layer: { touched: layerCarves.map((c) => c.touched), cellCount: layer.cellCount, splineCount: layer.splineCount },
}))
