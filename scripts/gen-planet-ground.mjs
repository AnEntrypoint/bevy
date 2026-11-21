import { writeParity, spointImport } from './lib/spoint.mjs'

const groundHeight = (x, z) => (x < -450 ? null : 3 + 0.02 * x - 0.015 * z + 0.0001 * x * z)
const RAYS = [
  { origin: [0, 10, 0], direction: [0, -1, 0], maxDistance: 50, stepM: 0.5, cellM: 1 },
  { origin: [5, 20, -8], direction: [0.3, -1, 0.2], maxDistance: 100, stepM: 0.5, cellM: 1 },
  { origin: [-100, 30, 40], direction: [1, -0.5, 0], maxDistance: 300, stepM: 0.25, cellM: 2 },
  { origin: [0, 1, 0], direction: [0, -1, 0], maxDistance: 50, stepM: 0.5, cellM: 1 },
  { origin: [0, 50, 0], direction: [0, 1, 0], maxDistance: 100, stepM: 0.5, cellM: 1 },
  { origin: [0, 10, 0], direction: [0, -1, 0], maxDistance: 3, stepM: 0.5, cellM: 1 },
  { origin: [0, 10, 0], direction: [0, -4, 0], maxDistance: 50, stepM: 0.5, cellM: 1 },
  { origin: [0, 10, 0], direction: [0, 0, 0], maxDistance: 50, stepM: 0.5, cellM: 1 },
  { origin: [0, 10, 0], direction: [0, -1, 0], maxDistance: 0, stepM: 0.5, cellM: 1 },
  { origin: [0, 10, 0], direction: [0, -1, 0], maxDistance: -5, stepM: 0.5, cellM: 1 },
  { origin: [-500, 10, 0], direction: [1, -1, 0], maxDistance: 200, stepM: 0.5, cellM: 1 },
  { origin: [-10, 3.5, 0], direction: [1, 0, 0], maxDistance: 40, stepM: 0.5, cellM: 1 },
]

await writeParity(async ({ spoint, rev, statusBefore }) => {
  const pf = await spointImport(spoint, 'src/terrain/PlanetFrame.js')
  const frame = { groundHeightLocal: (x, z) => groundHeight(x, z) }
  const raycast = RAYS.map((ray) => ({ ...ray, hit: pf.raycastGroundLocal(frame, ray.origin, ray.direction, ray.maxDistance, { stepM: ray.stepM, cellM: ray.cellM }) }))

  const guardHeightFn = (x, z) => { if (z > 300) throw new Error('boom'); if (x > 100) throw new pf.SurfaceSolveError(x, z, 1e-4, 64, 0.5); return x < -400 ? null : 3 + 0.02 * x - 0.015 * z }
  const guarded = [[0, 0], [-500, 0], [150, 0], [0, 400], [20, -30]].map(([x, z]) => {
    let value = null, rethrows = false
    try { value = pf.guardedGroundHeight('witness', guardHeightFn)(x, z) } catch (e) { rethrows = e.message === 'boom' }
    return { x, z, value, rethrows }
  })

  return {
    'spoint-planet-ground.json': { spoint: rev, module: 'src/terrain/PlanetFrame.js', groundHeight: 'mock: x < -450 gives null, otherwise 3 + 0.02 x - 0.015 z + 0.0001 x z', spointStatusBefore: statusBefore, spointStatusAfter: statusBefore, raycast, guarded },
  }
}, { indent: 0, status: false })
