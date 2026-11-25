import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const rev = execFileSync('git', ['-C', spoint, 'rev-parse', '--short', 'HEAD']).toString().trim()
const statusBefore = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
const spointModule = (rel) => import(pathToFileURL(join(spoint, rel)).href)
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-cave-sdf.json')
const { makeSphereCaveSDF, makeCylinderCaveSDF, makeTunnelCaveSDF, polygonizeCaveSDF, createCaveCarveLayer, loadCaveCarveLayer, createCaveVolume, polygonizeCaveVolume } = await spointModule('src/terrain/CaveSDF.js')

const PROBE_POINTS = [[0, 0, 0], [0.5, 0.2, -0.3], [1.2, -0.7, 0.4], [-0.9, 1.1, 0.8], [0.1, -0.05, 0.02], [2, 0.5, -1.5], [-0.3, -1.4, 0.6], [0.7, 0.7, 0.7], [-1.1, 0.4, -0.2], [0.25, 0.9, -0.6]]
const probe = (sdf) => PROBE_POINTS.map(([x, y, z]) => sdf(x, y, z))
const AXES = ['x', 'y', 'z']
const sdfProbes = {
  points: PROBE_POINTS,
  sphere: { cx: 0.1, cy: -0.2, cz: 0.3, radius: 0.7, values: probe(makeSphereCaveSDF(0.1, -0.2, 0.3, 0.7)) },
  cylinders: AXES.map((axis) => ({ cx: 0.1, cy: -0.2, cz: 0.3, radius: 0.6, halfHeight: 0.5, axis, values: probe(makeCylinderCaveSDF(0.1, -0.2, 0.3, 0.6, 0.5, axis)) })),
  tunnels: [...AXES.map((axis) => ({ seed: 7, radius: 0.4, length: 2, axis, warpAmp: 0.15, warpFreq: 2.5 })), { seed: 123456789, radius: 0.3, length: 1.5, axis: 'y', warpAmp: 0.3, warpFreq: 4 }].map((t) => ({ ...t, values: probe(makeTunnelCaveSDF(t.seed, t.radius, t.length, t.axis, t.warpAmp, t.warpFreq)) })),
}

const RES = 10
const checksumOf = (positions) => positions.reduce((sum, p) => sum + Math.round(p * 1000), 0)
const meshOf = (name, sdf) => {
  const mesh = polygonizeCaveSDF(sdf, RES)
  const positions = Array.from(mesh.positions)
  const indices = Array.from(mesh.indices)
  return { name, res: RES, vc: mesh.vc, ic: mesh.ic, triangles: mesh.ic / 3, checksum: checksumOf(positions), positions, indices }
}
const meshes = [
  meshOf('sphere', makeSphereCaveSDF(0, 0, 0, 1)),
  meshOf('cylinder-y', makeCylinderCaveSDF(0, 0, 0, 1, 0.6, 'y')),
  meshOf('tunnel-y', makeTunnelCaveSDF(7, 1, 2.5, 'y', 0.15, 2.5)),
]

const SPECS = [
  { worldX: 10, worldY: 0, worldZ: -5, worldRadius: 3, kind: 'sphere' },
  { worldX: -4, worldY: 1, worldZ: 2, worldRadius: 2.5, kind: 'cylinder', halfHeight: 1.5, axis: 'y' },
  { worldX: 0, worldY: -2, worldZ: 6, worldRadius: 4, kind: 'tunnel', length: 12, axis: 'x', seed: 9, warpAmp: 0.2, warpFreq: 3 },
  { worldX: 1, worldY: 1, worldZ: 1, worldRadius: 2, kind: 'cylinder', halfHeight: 1.2, axis: 'z' },
  { worldX: 3, worldY: 0, worldZ: 3, worldRadius: 2, kind: 'tunnel' },
  { worldX: 0, worldY: 0, worldZ: 0, worldRadius: -1, kind: 'sphere' },
  { worldX: 0, worldY: 0, worldZ: 0, worldRadius: 1, kind: 'cylinder', halfHeight: 0 },
  { worldX: 'a', worldY: 0, worldZ: 0, worldRadius: 1, kind: 'sphere' },
  { worldX: 2, worldY: 2, worldZ: 2, worldRadius: 1, kind: 'tunnel', length: 0 },
]
const layer = createCaveCarveLayer()
const addResults = SPECS.map((spec) => layer.addCaveVolume(spec))
const layerJson = JSON.parse(JSON.stringify(layer.toJSON()))
const loaded = loadCaveCarveLayer(layerJson)
const loadedJson = JSON.parse(JSON.stringify(loaded.toJSON()))
const HEIGHT_PROBES = [[0, 0, 0.5], [3, 3, 1], [-4, 2, 0.5], [1.5, -0.2, 2], [10, 0, 0], [0, -2, -0.5], [2, 2, -3], [-3, 1, 1.4], [6, -1, 0], [0.5, 0.5, 0.1]]
const heightDelta = HEIGHT_PROBES.map(([x, z, y]) => layer.heightDeltaAt(x, z, y))
const heightDeltaLoaded = HEIGHT_PROBES.map(([x, z, y]) => loaded.heightDeltaAt(x, z, y))
if (JSON.stringify(heightDelta) !== JSON.stringify(heightDeltaLoaded)) throw new Error('loaded layer disagrees with the original layer')

const anchorOf = ({ worldX, worldY, worldZ, worldRadius }) => ({ worldX, worldY, worldZ, worldRadius })
const shapeOf = ({ worldX, worldY, worldZ, worldRadius, ...shape }) => shape
const WORLD_INDICES = [0, 1, 2, 3, 4]
const worldMeshes = WORLD_INDICES.map((i) => {
  const { anchor, spec } = { anchor: anchorOf(SPECS[i]), spec: shapeOf(SPECS[i]) }
  const volume = createCaveVolume(anchor.worldX, anchor.worldY, anchor.worldZ, anchor.worldRadius, spec)
  const mesh = polygonizeCaveVolume(volume, RES)
  return { name: 'world-' + spec.kind + '-' + i, anchor, spec, res: RES, vc: mesh.vc, ic: mesh.ic, triangles: mesh.ic / 3, positions: Array.from(mesh.positions), indices: Array.from(mesh.indices) }
})
const worldOutside = [5, 6, 8].map((i) => {
  const { worldRadius } = anchorOf(SPECS[i])
  const spec = shapeOf(SPECS[i])
  let threw = false
  try {
    createCaveVolume(0, 0, 0, worldRadius, spec)
  } catch (error) {
    threw = error instanceof TypeError
  }
  return { worldRadius, spec, threw }
})

const statusAfter = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
if (statusAfter !== statusBefore) throw new Error('spoint working tree changed')
writeFileSync(outPath, JSON.stringify({ spoint: rev, sdfProbes, meshes, worldMeshes, worldOutside, carve: { specs: SPECS, addResults, volumeCount: layer.volumeCount, layerJson, loadedVolumeCount: loaded.volumeCount, loadedJson, heightProbes: HEIGHT_PROBES, heightDelta }, spointStatusBefore: statusBefore, spointStatusAfter: statusAfter }, null, 1) + '\n')
console.log(JSON.stringify({ meshes: meshes.map((m) => [m.name, m.vc, m.ic, m.checksum]), world: worldMeshes.map((m) => [m.name, m.vc, m.ic]), worldOutside: worldOutside.map((c) => c.threw), addResults, heightDelta }))
