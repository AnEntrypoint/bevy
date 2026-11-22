import { writeParity, spointImport } from './lib/spoint.mjs'

const AMPLITUDE = 1
const DIRECTION = [0.8, 0.6]
const TIMES = [0, 3.0, 11.3]
const CAMERA = [0, -40]
const GRID = 8

await writeParity(async ({ spoint, rev }) => {
  const { DataUtils } = await spointImport(spoint, 'node_modules/three/build/three.module.js')
  const field = await spointImport(spoint, 'client/core/GrassWindField.js')
  const cells = TIMES.map((t) => {
    const f = field.createGrassWindField()
    f.update(t, ...CAMERA, AMPLITUDE, ...DIRECTION, field.GRASS_WIND_FIELD_REACH_M + field.GRASS_WIND_FIELD_DROP_MARGIN_M)
    const data = f.texture.image.data
    const origin = f.uniforms.uFieldOrigin.value
    const list = []
    for (let row = 0; row < GRID; row++) for (let col = 0; col < GRID; col++) {
      const j = 24 + col * 2, i = 11 + row * 3
      const o = (i * f.cells + j) * 4
      list.push({ col, row, x: origin.x + (j + 0.5) * f.cellM, z: origin.y + (i + 0.5) * f.cellM, dx: DataUtils.fromHalfFloat(data[o]), dz: DataUtils.fromHalfFloat(data[o + 1]) })
    }
    return list
  })
  return {
    'spoint-planet-grass-wind.json': {
      rev,
      rule: 'createGrassWindField updated at each time around the camera, then a strided 8 by 8 block of its half-float texels decoded with DataUtils.fromHalfFloat',
      amplitude: AMPLITUDE,
      direction: DIRECTION,
      times: TIMES,
      cells,
      constants: { SWAY_BASE: field.GRASS_SWAY_BASE, SWAY_FLOW: field.GRASS_SWAY_FLOW },
    },
  }
})
