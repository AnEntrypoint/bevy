import { writeParity, spointImport } from './lib/spoint.mjs'

const POINTS = [[0, 0, 0], [0.5, 0.2, -0.3], [1.2, -0.7, 0.4], [-0.9, 1.1, 0.8], [0.1, -0.05, 0.02], [2, 0.5, -1.5], [-0.3, -1.4, 0.6], [0.7, 0.7, 0.7], [-1.1, 0.4, -0.2], [0.25, 0.9, -0.6]]
const SEEDS = [1337, 1337 + 7919, 42, 0, 987654]
const MESHES = [{ seed: 42, res: 8 }, { seed: 7919, res: 12 }, { seed: 1337, res: 16 }]

await writeParity(async ({ spoint, rev }) => {
  const { makeRockSDF, marchRockSurface, generateRockHullData, ROCK_MESH_RES } = await spointImport(spoint, 'src/terrain/RockShapes.js')
  const sdfSamples = SEEDS.map((seed) => {
    const sdf = makeRockSDF(seed)
    return { seed, values: POINTS.map(([x, y, z]) => sdf(x, y, z)) }
  })
  const meshes = MESHES.map(({ seed, res }) => {
    const mesh = marchRockSurface(res, makeRockSDF(seed))
    return { seed, res, vc: mesh.vc, ic: mesh.ic, triangles: mesh.ic / 3, positions: Array.from(mesh.positions), indices: Array.from(mesh.indices) }
  })
  const HULL = { numShapes: 6, baseSeed: 1337, hullRes: ROCK_MESH_RES }
  const shapes = generateRockHullData(HULL.numShapes, HULL.baseSeed, HULL.hullRes).map((h) => ({
    vCount: h.vCount, ic: h.indices.length, avgRadius: h.avgRadius, squashRatio: h.squashRatio, bottomY: h.bottomY, height: h.height,
    positions: Array.from(h.positions), indices: Array.from(h.indices),
  }))
  return { 'spoint-rock-shapes.json': { spoint: rev, points: POINTS, sdfSamples, meshes, hull: { ...HULL, shapes } } }
})
