import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-brush-stroke.json')
const gitStatus = () => execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
const rev = execFileSync('git', ['-C', spoint, 'rev-parse', '--short', 'HEAD']).toString().trim()
const statusBefore = gitStatus()
const { STROKE_KINDS, BIOME_STROKE_FIELDS, heightStrokeIsLoadable, biomeStrokeIsLoadable, heightStrokeReplay, biomeStrokeReplay, replayStrokes } = await import(pathToFileURL(join(spoint, 'src/terrain/brushStroke.js')).href)

const encode = (value) => JSON.parse(JSON.stringify(value, (_, item) => (typeof item === 'number' && !Number.isFinite(item) ? String(item) : item)))
const withoutField = (object, field) => {
  const copy = { ...object }
  delete copy[field]
  return copy
}
const BASE_FIELDS = ['x', 'z', 'strength']
const FIELD_VALUES = { x: 3, z: -2, strength: 0.5, radius: 4, targetHeight: 1.25, halfWidth: 3, halfDepth: 2, targetDepth: -0.5 }
const validHeight = (brush) => {
  const stroke = { brush }
  for (const field of [...BASE_FIELDS, ...STROKE_KINDS[brush]]) stroke[field] = FIELD_VALUES[field]
  return stroke
}

const heightInputs = []
for (const brush of Object.keys(STROKE_KINDS)) {
  const valid = validHeight(brush)
  const fields = [...BASE_FIELDS, ...STROKE_KINDS[brush]]
  const lastKindField = STROKE_KINDS[brush][STROKE_KINDS[brush].length - 1]
  heightInputs.push([brush + ' valid', valid])
  for (const field of fields) heightInputs.push([brush + ' without ' + field, withoutField(valid, field)])
  heightInputs.push([brush + ' non-finite x', { ...valid, x: Infinity }])
  heightInputs.push([brush + ' NaN ' + lastKindField, { ...valid, [lastKindField]: NaN }])
  heightInputs.push([brush + ' string ' + lastKindField, { ...valid, [lastKindField]: '4' }])
}
heightInputs.push(['unknown brush', { ...validHeight('raise'), brush: 'bogus' }])
heightInputs.push(['brush absent', withoutField(validHeight('raise'), 'brush')])
heightInputs.push(['null input', null])
heightInputs.push(['number input', 3])
heightInputs.push(['string input', 'raise'])

const BIOME_TARGET = { temp: 0.3, humidity: 0.6, erosion: 0.2 }
const validBiome = () => ({ x: 3, z: -2, radius: 4, strength: 0.5, target: { ...BIOME_TARGET } })
const biomeInputs = [['valid', validBiome()]]
for (const field of BIOME_STROKE_FIELDS) biomeInputs.push(['without ' + field, withoutField(validBiome(), field)])
biomeInputs.push(['non-finite x', { ...validBiome(), x: Infinity }])
biomeInputs.push(['NaN radius', { ...validBiome(), radius: NaN }])
biomeInputs.push(['target without erosion', { ...validBiome(), target: withoutField(BIOME_TARGET, 'erosion') }])
biomeInputs.push(['target NaN temp', { ...validBiome(), target: { ...BIOME_TARGET, temp: NaN } }])
biomeInputs.push(['target absent', withoutField(validBiome(), 'target')])
biomeInputs.push(['null input', null])
biomeInputs.push(['number input', 3])

const judge = (inputs, check) => inputs.map(([name, input]) => ({ name, input: encode(input), result: check(input) }))

const baseHeight = (x) => 0.25 + 0.1 * x
const MIXED = [
  { brush: 'raise', x: 1, z: 2, radius: 3, strength: 0.4 },
  { brush: 'lower', x: -1, z: 2, radius: 2.5, strength: -0.3 },
  { brush: 'smooth', x: 0, z: 0, radius: 4, strength: 0.2 },
  { brush: 'flatten', x: 5, z: -3, radius: 6, targetHeight: 1.5, strength: 0.7 },
  { brush: 'rectangularCarve', x: -4, z: 1, halfWidth: 2, halfDepth: 3, targetDepth: -1, strength: 0.9 },
  { x: 2, z: 2, radius: 3, strength: 0.5, target: { temp: 0.3, humidity: 0.6, erosion: 0.2 } },
  { brush: 'raise', x: NaN, z: 0, radius: 1, strength: 0.5 },
  { brush: 'flatten', x: 1, z: 1, radius: 2, strength: 0.5 },
  { brush: 'bogus', x: 0, z: 0, radius: 1, strength: 0.5 },
  null,
  { x: 1, z: 1, radius: 0, strength: 0.5, target: { temp: 0.5, humidity: 0.5, erosion: 0.5 } },
  { x: 4, z: -4, radius: 5, strength: 0, target: { temp: 0.9, humidity: 0.1, erosion: 0.2 } },
  { x: 4, z: -4, radius: 5, strength: 0.6, target: { temp: 0.9, humidity: 0.1 } },
  { brush: 'rectangularCarve', x: 3, z: 3, halfWidth: 1, halfDepth: 1, strength: 0.5 },
]

const recordingHeightField = () => {
  const calls = []
  const hd = {
    applySmoothBrush: (...args) => calls.push({ call: 'applySmoothBrush', args }),
    applyFlattenBrush: (base, ...args) => calls.push({ call: 'applyFlattenBrush', args }),
    applyRectangularCarve: (base, ...args) => calls.push({ call: 'applyRectangularCarve', args }),
    applyRaiseBrush: (...args) => calls.push({ call: 'applyRaiseBrush', args }),
  }
  return { hd, calls }
}
const recordingBiomeField = () => {
  const calls = []
  const bo = { applyPaintBrush: (...args) => calls.push({ call: 'applyPaintBrush', args }) }
  return { bo, calls }
}

const withBase = recordingHeightField()
const withBaseCount = replayStrokes(MIXED, heightStrokeReplay(withBase.hd, baseHeight))
const withoutBase = recordingHeightField()
const withoutBaseCount = replayStrokes(MIXED, heightStrokeReplay(withoutBase.hd, null))
const paint = recordingBiomeField()
const paintCount = replayStrokes(MIXED, biomeStrokeReplay(paint.bo))
const nonArray = {
  nullInput: replayStrokes(null, heightStrokeReplay(withoutBase.hd, baseHeight)),
  stringInput: replayStrokes('nope', biomeStrokeReplay(paint.bo)),
}

if (gitStatus() !== statusBefore) throw new Error('spoint working tree changed')
writeFileSync(outPath, JSON.stringify({
  spoint: rev,
  strokeKinds: STROKE_KINDS,
  biomeStrokeFields: BIOME_STROKE_FIELDS,
  loadability: { height: judge(heightInputs, heightStrokeIsLoadable), biome: judge(biomeInputs, biomeStrokeIsLoadable) },
  replay: {
    mixed: encode(MIXED),
    heightWithBase: { count: withBaseCount, calls: withBase.calls },
    heightWithoutBase: { count: withoutBaseCount, calls: withoutBase.calls },
    biome: { count: paintCount, calls: paint.calls },
    nonArray,
  },
}, null, 1) + '\n')
console.log(JSON.stringify({ rev, height: heightInputs.length, biome: biomeInputs.length, withBaseCount, withoutBaseCount, paintCount, nonArray }))
