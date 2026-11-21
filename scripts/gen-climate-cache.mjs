import { writeParity, spointImport } from './lib/spoint.mjs'

await writeParity(async ({ spoint, rev }) => {
  const { createCachedAnchorField, SECTOR_M } = await spointImport(spoint, 'src/terrain/ClimateCache.js')

  const RADIUS = 6360
  const unit = (v) => { const l = Math.hypot(v[0], v[1], v[2]); return v.map((c) => c / l) }
  const frame = {
    radius: RADIUS,
    localToDir: (x, z) => unit([x / RADIUS, 1, z / RADIUS]),
  }

  let calls = 0
  const anchorField = {
    sampleDir(dir) {
      calls++
      if (dir[0] < -0.02) return null
      return { temp: 15 + 20 * dir[1], humidity: 0.5 + 0.3 * dir[2], erosion: 0.2 + 0.1 * dir[0], seaBias: dir[0] * dir[1] }
    },
  }

  const cached = createCachedAnchorField(anchorField, frame)
  const plan = [
    { x: 0, z: 0 },
    { x: 0, z: 0 },
    { x: 0.5, z: 0.25 },
    { x: 3, z: -2 },
    { x: 100, z: -40 },
    { x: 100, z: -40 },
    { x: -300, z: 200 },
    { x: -300, z: 200 },
    { x: 0, z: 0, dir: [0, 0, 1] },
    { x: 0.5, z: 0.25 },
    { x: -1200, z: 900 },
    { x: 2000, z: 1500, dir: [0.3, 0.5, 0.8] },
  ]
  const queries = plan.map((q) => {
    const climate = q.dir ? cached.climateAtLocal(q.x, q.z, q.dir) : cached.climateAtLocal(q.x, q.z)
    return { x: q.x, z: q.z, dir: q.dir ?? null, climate, size: cached.size, calls }
  })

  return {
    'spoint-climate-cache.json': {
      spoint: rev,
      sectorM: SECTOR_M,
      radius: RADIUS,
      queries,
      callsTotal: calls,
    },
  }
})
