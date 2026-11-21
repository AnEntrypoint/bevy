import { writeParity, spointImport } from './lib/spoint.mjs'

const RADIUS = 6360
const OFFSET_Y = 5
const ANCHOR = [0.6, 0, 0.8]
const RELIEF_SCALE = 0.001
const smooth = (d) => 40 + 25 * d[0] - 15 * d[1] + 10 * d[2] * d[2]
const POINTS = [[0, 0], [100, 50], [-300, 200], [2000, -1500], [0.5, -0.25], [-1200, 900], [4000, -4000], [6400, 0]]

await writeParity(async ({ spoint, rev }) => {
  const pf = await spointImport(spoint, 'src/terrain/PlanetFrame.js')
  const frame = pf.createPlanetFrame({ sampler: { radius: RADIUS, hashVersion: 2, heightAt: smooth }, anchorDir: ANCHOR, offsetY: OFFSET_Y, reliefScale: RELIEF_SCALE })
  const groundHeightLocal = POINTS.map(([x, z]) => {
    try {
      return { x, z, y: frame.groundHeightLocal(x, z), error: null }
    } catch (e) {
      return { x, z, y: null, error: e.constructor.name }
    }
  })
  return {
    'spoint-planet-ground-local.json': { spoint: rev, module: 'src/terrain/PlanetFrame.js', radius: RADIUS, offsetY: OFFSET_Y, anchorDir: ANCHOR, reliefScale: frame.reliefScale, sampler: 'smooth: 40 + 25 x - 15 y + 10 z z', groundHeightLocal },
  }
}, { indent: 0 })
