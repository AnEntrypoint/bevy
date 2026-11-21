import { writeParity, spointImport } from './lib/spoint.mjs'

await writeParity(async ({ spoint, rev }) => {
  const { TRI_TABLE } = await spointImport(spoint, 'src/terrain/RockTriTableShared.js')
  if (TRI_TABLE.length !== 256) throw new Error('TRI_TABLE has ' + TRI_TABLE.length + ' entries')
  return { 'spoint-rock-tri-table.json': { spoint: rev, module: 'src/terrain/RockTriTableShared.js', triTable: TRI_TABLE } }
}, { indent: 0 })
