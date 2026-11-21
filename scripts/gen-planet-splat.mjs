import { writeParity, spointImport } from './lib/spoint.mjs'

const SPREAD_DIRS = 24
const WARP_DIRS = 100
const HASH_VERSION = 2
const WARP_FREQS = [1100, 2580]
const LEVEL = { neutral: [0.5, 0.5], dryHot: [0.8, 0.1], coolWet: [0.2, 0.9] }

const range = (lo, hi, step) => Array.from({ length: Math.round((hi - lo) / step) + 1 }, (_, i) => +(lo + i * step).toFixed(6))
const union = (...lists) => [...new Set(lists.flat())].sort((a, b) => a - b)
const goldenAngle = Math.PI * (3 - Math.sqrt(5))
const fibonacci = (count) => Array.from({ length: count }, (_, i) => {
  const y = 1 - (2 * (i + 0.5)) / count
  const r = Math.sqrt(1 - y * y)
  return [Math.cos(goldenAngle * i) * r, y, Math.sin(goldenAngle * i) * r]
})

const heightsSweep = union(range(-600, 5000, 25), range(-20, 40, 0.5), range(80, 112, 0.5), range(115, 200, 1), range(0, 20, 0.25))
const slopesSweep = [0, 0.1, 0.2, 0.3, 0.45, 0.6, 0.8]
const slopeAxis = range(0, 1, 0.01)
const slopeHeights = [-5, 2, 10, 50, 100, 150, 300]
const climateTemps = union(range(0, 1, 0.05), [0.42, 0.52, 0.62])
const climateHumids = range(0, 1, 0.05)
const climateHeights = [0.5, 5, 30, 300]
const climateSlopes = [0.05, 0.3]
const spreadHeights = [-5, 0, 3, 8, 15, 40, 90, 100, 120, 150, 180, 200]
const spreadSlopes = [0, 0.3, 0.6]
const spreadClimates = [LEVEL.neutral, LEVEL.dryHot]

await writeParity(async ({ spoint, rev }) => {
  const { createSplatWeights } = await spointImport(spoint, 'packages/mapspinner/src/splat-weights.js')
  const { compileSnoise3 } = await spointImport(spoint, 'packages/mapspinner/src/tsl/ops-jsgen.js')
  const { TERRAIN_DEFAULTS } = await spointImport(spoint, 'packages/mapspinner/src/terrain-defaults.js')

  const splatWeights = createSplatWeights({ hashVersion: HASH_VERSION })
  const snoise3 = compileSnoise3({ hashVersion: HASH_VERSION })
  const sweepDirs = fibonacci(2).map((d) => { const n = Math.hypot(...d); return d.map((c) => c / n) })
  const spreadDirs = fibonacci(SPREAD_DIRS)
  const warpDirs = fibonacci(WARP_DIRS)

  const rows = []
  const record = (dirIndex, dirs, h, slope, temp, humid) => {
    const w = splatWeights(dirs[dirIndex], h, slope, temp, humid)
    rows.push([dirIndex, h, slope, temp, humid, w.grass, w.rock, w.sand, w.snow])
  }
  const sweeps = []
  const section = (name, dirs, fill) => {
    const start = rows.length
    fill(dirs)
    sweeps.push({ name, dirs: dirs.length, start, count: rows.length - start })
  }

  section('height', sweepDirs, (dirs) => {
    for (const d of dirs.keys()) for (const [t, hu] of Object.values(LEVEL)) for (const s of slopesSweep) for (const h of heightsSweep) record(d, dirs, h, s, t, hu)
  })
  section('slope', sweepDirs, (dirs) => {
    for (const d of dirs.keys()) for (const [t, hu] of Object.values(LEVEL)) for (const h of slopeHeights) for (const s of slopeAxis) record(d, dirs, h, s, t, hu)
  })
  section('climate', sweepDirs.slice(0, 1), (dirs) => {
    for (const h of climateHeights) for (const s of climateSlopes) for (const t of climateTemps) for (const hu of climateHumids) record(0, dirs, h, s, t, hu)
  })
  section('spread', spreadDirs, (dirs) => {
    for (const d of dirs.keys()) for (const [t, hu] of spreadClimates) for (const s of spreadSlopes) for (const h of spreadHeights) record(d, dirs, h, s, t, hu)
  })

  const warp = warpDirs.map((d) => WARP_FREQS.map((f) => snoise3(d[0] * f, d[1] * f, d[2] * f)))

  return {
    'spoint-planet-splat.json': {
      source: 'spoint ' + rev + ': packages/mapspinner/src/splat-weights.js createSplatWeights (hash version 2) run unchanged, with snoise3 from tsl/ops-jsgen.js compileSnoise3 for the two band warp frequencies',
      spoint: rev,
      hashVersion: HASH_VERSION,
      constants: {
        beachTop: TERRAIN_DEFAULTS.beachTop,
        beachWidth: TERRAIN_DEFAULTS.beachWidth,
        bandWarp: TERRAIN_DEFAULTS.bandWarp,
        snowEdges: TERRAIN_DEFAULTS.snowEdges,
        slopeRock: TERRAIN_DEFAULTS.slopeRock,
      },
      rowOrder: ['dir', 'height', 'slope', 'temp', 'humid', 'grass', 'rock', 'sand', 'snow'],
      sweepDirs,
      spreadDirs,
      sweeps,
      rows,
      warpFreqs: WARP_FREQS,
      warpDirs,
      warp,
    },
  }
}, { indent: 0, status: false })
