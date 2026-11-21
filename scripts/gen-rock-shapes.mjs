import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const rev = execFileSync('git', ['-C', spoint, 'rev-parse', '--short', 'HEAD']).toString().trim()
const statusBefore = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
const spointModule = (rel) => import(pathToFileURL(join(spoint, rel)).href)
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-rock-shapes.json')
const { makeRockSDF, marchRockSurface, generateRockHullData, ROCK_MESH_RES } = await spointModule('src/terrain/RockShapes.js')

const POINTS = [[0, 0, 0], [0.5, 0.2, -0.3], [1.2, -0.7, 0.4], [-0.9, 1.1, 0.8], [0.1, -0.05, 0.02], [2, 0.5, -1.5], [-0.3, -1.4, 0.6], [0.7, 0.7, 0.7], [-1.1, 0.4, -0.2], [0.25, 0.9, -0.6]]
const SEEDS = [1337, 1337 + 7919, 42, 0, 987654]
const sdfSamples = SEEDS.map((seed) => {
  const sdf = makeRockSDF(seed)
  return { seed, values: POINTS.map(([x, y, z]) => sdf(x, y, z)) }
})

const MESHES = [{ seed: 42, res: 8 }, { seed: 7919, res: 12 }, { seed: 1337, res: 16 }]
const meshes = MESHES.map(({ seed, res }) => {
  const mesh = marchRockSurface(res, makeRockSDF(seed))
  return { seed, res, vc: mesh.vc, ic: mesh.ic, triangles: mesh.ic / 3, positions: Array.from(mesh.positions), indices: Array.from(mesh.indices) }
})

const HULL = { numShapes: 6, baseSeed: 1337, hullRes: ROCK_MESH_RES }
const shapes = generateRockHullData(HULL.numShapes, HULL.baseSeed, HULL.hullRes).map((h) => ({
  vCount: h.vCount, ic: h.indices.length, avgRadius: h.avgRadius, squashRatio: h.squashRatio, bottomY: h.bottomY, height: h.height,
  positions: Array.from(h.positions), indices: Array.from(h.indices),
}))

const statusAfter = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
if (statusAfter !== statusBefore) throw new Error('spoint working tree changed')
writeFileSync(outPath, JSON.stringify({ spoint: rev, points: POINTS, sdfSamples, meshes, hull: { ...HULL, shapes }, spointStatusBefore: statusBefore, spointStatusAfter: statusAfter }, null, 1) + '\n')
console.log(JSON.stringify({ rev, sdf0: sdfSamples[0].values.slice(0, 3), meshes: meshes.map((m) => [m.seed, m.res, m.vc, m.ic]), hulls: shapes.map((h) => [h.vCount, h.ic, h.avgRadius, h.squashRatio, h.bottomY, h.height]) }))
