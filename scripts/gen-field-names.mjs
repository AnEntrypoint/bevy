import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-field-names.json')
const rev = execFileSync('git', ['-C', spoint, 'rev-parse', '--short', 'HEAD']).toString().trim()
const statusBefore = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
const { TERRAIN_FIELD_NAMES, FIELD_LAYER_NAMES, PAINTED_HEIGHT_FIELD, NATURAL_MASK_FIELD } = await import(pathToFileURL(join(spoint, 'src/terrain/fieldPaint.js')).href)
const statusAfter = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
if (statusAfter !== statusBefore) throw new Error('spoint working tree changed')
writeFileSync(outPath, JSON.stringify({ spoint: rev, module: 'src/terrain/fieldPaint.js', terrainFieldNames: [...TERRAIN_FIELD_NAMES], fieldLayerNames: [...FIELD_LAYER_NAMES], paintedHeightField: PAINTED_HEIGHT_FIELD, naturalMaskField: NATURAL_MASK_FIELD, spointStatusBefore: statusBefore, spointStatusAfter: statusAfter }) + '\n')
console.log(JSON.stringify({ spoint: rev, terrainFieldNames: TERRAIN_FIELD_NAMES, fieldLayerNames: FIELD_LAYER_NAMES, paintedHeightField: PAINTED_HEIGHT_FIELD, naturalMaskField: NATURAL_MASK_FIELD }))
