import { writeParity, spointImport } from './lib/spoint.mjs'

await writeParity(async ({ spoint, rev }) => {
  const { TERRAIN_FIELD_NAMES, FIELD_LAYER_NAMES, PAINTED_HEIGHT_FIELD, NATURAL_MASK_FIELD } = await spointImport(spoint, 'src/terrain/fieldPaint.js')

  return {
    'spoint-field-names.json': {
      spoint: rev,
      module: 'src/terrain/fieldPaint.js',
      terrainFieldNames: [...TERRAIN_FIELD_NAMES],
      fieldLayerNames: [...FIELD_LAYER_NAMES],
      paintedHeightField: PAINTED_HEIGHT_FIELD,
      naturalMaskField: NATURAL_MASK_FIELD,
    },
  }
}, { indent: 0 })
