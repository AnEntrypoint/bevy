import { writeParity, spointImport } from './lib/spoint.mjs'

const ANCHORS = [[0, 1, 0], [-0.641, 0.2558, 0.7237], [1, 0, 0], [0.3, -0.8, 0.5], [0.2, 0.9, -0.4]]
const RADII = [6360, 63600]
const PLANE = [[0, 0], [1200, -800], [-2500, 900], [0.5, 0.25], [-40, 17]]
const FRAMES = [
  { offsetY: 0, anchorHeight: 0, radius: 6360 },
  { offsetY: -3.5, anchorHeight: 82.68697197035642, radius: 63600 },
  { offsetY: 1.25, anchorHeight: 10, radius: 0 },
  { offsetY: 2, anchorHeight: null, radius: 6360 },
]
const ELEVATIONS = [0, 12.5, -4, 300]
const nullable = (v) => (v === null || v === undefined || !Number.isFinite(v) ? null : v)

await writeParity(async ({ spoint, rev, statusBefore }) => {
  const pf = await spointImport(spoint, 'src/terrain/PlanetFrame.js')
  const basis = ANCHORS.map((anchor) => { const b = pf.anchorBasis(anchor); return { anchor, up: b.up, east: b.east, north: b.north } })
  const tangent = []
  for (const anchor of ANCHORS) for (const radius of RADII) for (const [x, z] of PLANE) tangent.push({ anchor, radius, x, z, dir: pf.tangentLocalToDir(pf.anchorBasis(anchor), radius, x, z) })
  const frames = FRAMES.map((frame) => {
    const f = { offsetY: frame.offsetY ?? NaN, anchorHeight: frame.anchorHeight ?? NaN, radius: frame.radius }
    const cases = []
    for (const [x, z] of PLANE) {
      const y = 3.25
      cases.push({ x, z, y, elevation: 0, waterline: nullable(pf.waterlineLocalY(f, x, z)), elevationAtLocal: nullable(pf.elevationAtLocal(f, x, y, z)) })
      for (const elevation of ELEVATIONS) cases.push({ x, z, y, elevation, surface: nullable(pf.sphereSurfaceLocalY(f, x, z, elevation)), elevationAtLocal: nullable(pf.elevationAtLocal(f, x, y, z)) })
    }
    return { frame, cases }
  })
  return {
    'spoint-planet-frame.json': { spoint: rev, module: 'src/terrain/PlanetFrame.js', spointStatusBefore: statusBefore, spointStatusAfter: statusBefore, basis, tangent, frames },
  }
}, { indent: 0, status: false })
