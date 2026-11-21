import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-biome-override.json')
const gitStatus = () => execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
const rev = execFileSync('git', ['-C', spoint, 'rev-parse', '--short', 'HEAD']).toString().trim()
const statusBefore = gitStatus()
const { createBiomeOverride, loadBiomeOverride, CELL_M, PAINT_REQUIRES, BIOME_PRESETS, BIOME_NAMES } = await import(pathToFileURL(join(spoint, 'src/terrain/BiomeOverride.js')).href)

const encode = (value) => JSON.parse(JSON.stringify(value, (_, item) => (typeof item === 'number' && !Number.isFinite(item) ? String(item) : item)))

const PAINTS = [
  { x: 0, z: 0, radius: 20, strength: 1, target: BIOME_PRESETS.desert },
  { x: 30, z: 4, radius: 12, strength: 0.6, target: BIOME_PRESETS.wetland },
  { x: 14, z: -10, radius: 9, strength: 0.8, target: { temp: 1.4, humidity: -0.2, erosion: 0.5 } },
  { x: NaN, z: 0, radius: 5, strength: 0.5, target: BIOME_PRESETS.tundra },
  { x: -22, z: 6, radius: 10, strength: 0, target: BIOME_PRESETS.forest },
  { x: -40, z: -40, radius: 7, strength: 0.9, target: BIOME_PRESETS.grassland },
  { x: 5, z: 40, radius: 6, strength: 1.5, target: { temp: 0.2, humidity: 0.2, erosion: 0.9 } },
]
const SAMPLE_POINTS = [[0, 0], [7.9, 0.1], [12, 4], [30, 4], [14, -10], [-22, 6], [-40, -40], [5, 40], [100, 100], [-200, -200]]
const baseField = {
  climateUsesLocalXZ: true,
  climateAtLocal(x, z) {
    if (z < -30) return null
    return { temp: 0.3 + 0.002 * x, humidity: 0.5 - 0.001 * z, erosion: 0.4, seaBias: 0.25 }
  },
}
const overridePoints = (api) => SAMPLE_POINTS.map(([x, z]) => api.overrideAt(x, z))
const climatePoints = (climate) => SAMPLE_POINTS.map(([x, z]) => climate.climateAtLocal(x, z, null))

const painted = createBiomeOverride(PAINTS)
const wrapped = painted.wrapClimateField(baseField)
const emptyWrapped = createBiomeOverride([]).wrapClimateField(baseField)
const direct = createBiomeOverride([])
const DIRECT_CALLS = [
  { name: 'zero radius', args: [0, 0, 0, BIOME_PRESETS.desert, 1] },
  { name: 'target missing erosion', args: [0, 0, 5, { temp: 0.5, humidity: 0.5 }, 1] },
  { name: 'zero strength', args: [0, 0, 5, BIOME_PRESETS.desert, 0] },
  { name: 'accepted', args: [0, 0, 9, BIOME_PRESETS.desert, 0.5] },
]
const directResults = DIRECT_CALLS.map(({ name, args }) => ({ name, args: encode(args), result: direct.applyPaintBrush(...args) }))
const directAfter = { cellCount: direct.cellCount, strokeCount: direct.strokeCount }
direct.clear()
const directCleared = { cellCount: direct.cellCount, strokeCount: direct.strokeCount, overrideAt: direct.overrideAt(0, 0) }

const loadJson = { version: 1, cellM: CELL_M, strokes: PAINTS }
const loaded = loadBiomeOverride(loadJson)
const roundTrip = loadBiomeOverride(loaded.toJSON())
const nullLoaded = loadBiomeOverride(null)
const textLoaded = loadBiomeOverride({ strokes: 'nope' })

if (gitStatus() !== statusBefore) throw new Error('spoint working tree changed')
writeFileSync(outPath, JSON.stringify({
  spoint: rev,
  cellM: CELL_M,
  paintRequires: PAINT_REQUIRES,
  presets: BIOME_PRESETS,
  names: BIOME_NAMES,
  samplePoints: SAMPLE_POINTS,
  paints: encode(PAINTS),
  paint: {
    cellCount: painted.cellCount,
    strokeCount: painted.strokeCount,
    overrideAt: overridePoints(painted),
    toJSON: painted.toJSON(),
    wrap: { climateUsesLocalXZ: wrapped.climateUsesLocalXZ, samples: climatePoints(wrapped) },
    emptyWrap: { climateUsesLocalXZ: emptyWrapped.climateUsesLocalXZ, samples: climatePoints(emptyWrapped) },
    direct: { calls: directResults, after: directAfter, cleared: directCleared },
  },
  load: {
    input: encode(loadJson),
    cellCount: loaded.cellCount,
    strokeCount: loaded.strokeCount,
    overrideAt: overridePoints(loaded),
    roundTrip: { cellCount: roundTrip.cellCount, strokeCount: roundTrip.strokeCount },
    nullInput: { cellCount: nullLoaded.cellCount, strokeCount: nullLoaded.strokeCount, overrideAt: overridePoints(nullLoaded) },
    nonArrayStrokes: { cellCount: textLoaded.cellCount, strokeCount: textLoaded.strokeCount },
  },
}, null, 1) + '\n')
console.log(JSON.stringify({ rev, cellCount: painted.cellCount, strokeCount: painted.strokeCount, direct: directResults.map((r) => r.result) }))
