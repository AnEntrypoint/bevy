import { writeParity, spointImport } from './lib/spoint.mjs'

await writeParity(async ({ spoint, rev }) => {
  const { createFieldLattice, tiltDegBetween, RESET_SURFACE_STEP_BOUND_M } = await spointImport(spoint, 'src/terrain/FieldLattice.js')
  const { createPlanetFrame } = await spointImport(spoint, 'src/terrain/PlanetFrame.js')
  const { snapshotChart } = await spointImport(spoint, 'src/shared/chartAnchor.js')

  const PLANET_RADIUS = 60
  const OFFSET_Y = 0.25
  const RELIEF_SCALE = 1
  const ANCHOR_UP = [0.3, 0.9, -0.2]
  const LIVE_DIR = [0.9, 0.3, 0.2]
  const TILT_PAIRS = [[[0, 1, 0], [0, 1, 0]], [[0, 1, 0], [1, 0, 0]], [[0, 1, 0], [0, -1, 0]], [[1, 0, 0], [0, 0, 1]], [[0.6, 0.8, 0], [0, 1, 0]]]
  const POINTS = [[0, 0], [1, 2], [-4, 0.5], [3, -2.5]]
  const CORNERS = [[0, 0], [8, -8], [-16, 4]]

  const sampler = { radius: PLANET_RADIUS, hashVersion: 1, heightAt: (d) => 0.5 + 0.25 * d[0] - 0.1 * d[2] }
  const liveHeightFn = (x, z) => 0.3 + 0.05 * x - 0.02 * z
  const liveFrame = createPlanetFrame({ sampler, anchorDir: LIVE_DIR, offsetY: OFFSET_Y, reliefScale: RELIEF_SCALE })
  const lattice = createFieldLattice({ sampler, offsetY: OFFSET_Y, reliefScale: RELIEF_SCALE, anchor: { up: ANCHOR_UP }, liveFrame, liveHeightFn })

  const result = {
    spoint: rev,
    planetRadius: PLANET_RADIUS,
    offsetY: OFFSET_Y,
    reliefScale: RELIEF_SCALE,
    anchorUp: ANCHOR_UP,
    liveDir: LIVE_DIR,
    resetSurfaceStepBoundM: RESET_SURFACE_STEP_BOUND_M,
    anchor: lattice.anchor,
    live: snapshotChart(liveFrame),
    tiltDeg: lattice.tiltDeg,
    tiltDegBetween: TILT_PAIRS.map(([a, b]) => ({ a, b, deg: tiltDegBetween(a, b) })),
    placements: CORNERS.map(([cornerX, cornerZ]) => {
      const p = lattice.placement(cornerX, cornerZ)
      return { corner: [cornerX, cornerZ], position: p.position, rotation: p.rotation }
    }),
    toLattice: POINTS.map((point) => ({ point, out: lattice.toLattice(point[0], point[1]) })),
    toChart: POINTS.map((point) => ({ point, out: lattice.toChart(point[0], point[1]) })),
    heightFn: POINTS.map((point) => ({ point, height: lattice.heightFn(point[0], point[1]) })),
  }

  return { 'spoint-field-lattice.json': result }
}, { indent: 1, status: false })
