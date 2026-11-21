import { writeFixture, spointImport } from './lib/spoint.mjs'

const RADII = [63600, 6360000]
const ANCHOR = [0.6, 0, 0.8]
const OFFSET_Y = 5
const BODIES = [[0, 0], [37.3, -91.7], [1500, -900]]
const smooth = (d) => 40 + 25 * d[0] - 15 * d[1] + 10 * d[2] * d[2]
const PHYSICS = { extent: 256, resolution: 2 }
const MM_PER_M = 1000

await writeFixture('spoint-planet-collider', async ({ spoint, rev }) => {
  const { colliderLatticeOf } = await spointImport(spoint, 'src/shared/colliderLattice.js')
  const { sampleTerrainGridChunked } = await spointImport(spoint, 'src/terrain/HeightfieldStreamer.js')
  const { createPlanetFrame } = await spointImport(spoint, 'src/terrain/PlanetFrame.js')
  const lattice = colliderLatticeOf({ physics: PHYSICS })
  const half = lattice.extentM / 2
  const cases = []
  for (const radius of RADII) {
    const frame = createPlanetFrame({ sampler: { radius, hashVersion: 2, heightAt: smooth }, anchorDir: ANCHOR, offsetY: OFFSET_Y, reliefScale: 0.001 })
    for (const body of BODIES) {
      const snap = (c) => Math.round((c - half) / lattice.spacingM) * lattice.spacingM
      const origin = [snap(body[0]), snap(body[1])]
      const grid = await sampleTerrainGridChunked({ heightFn: (x, z) => frame.groundHeightLocal(x, z), N: lattice.N, spacing: lattice.spacingM, cornerX: origin[0], cornerZ: origin[1] })
      cases.push({ radius, body, origin, n: lattice.N, spacing: lattice.spacingM, heightsMm: Array.from(grid.samples, (h) => Math.round(h * MM_PER_M)) })
    }
  }
  return { spoint: rev, modules: ['src/shared/colliderLattice.js', 'src/terrain/HeightfieldStreamer.js', 'src/terrain/PlanetFrame.js'], physics: PHYSICS, anchorDir: ANCHOR, offsetY: OFFSET_Y, reliefScale: 0.001, coverRadius: lattice.extentM * 0.4, sampler: 'smooth: 40 + 25 x - 15 y + 10 z z', cases }
})
