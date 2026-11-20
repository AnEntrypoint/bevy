import { execFileSync } from 'node:child_process'
import { mkdirSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const outDir = join(import.meta.dirname, '..', 'testdata', 'parity')
const rev = execFileSync('git', ['-C', spoint, 'rev-parse', '--short', 'HEAD']).toString().trim()
const { loadPlanetSampler, planetSamplerOptsOf } = await import(pathToFileURL(join(spoint, 'src/terrain/TerrainPhysics.js')).href)

const RADIUS = 6360
const SEED = 0x5eed1234
const POINT_COUNT = 64
const CARVES = [
  { dir: [0.3, -0.8, 0.5], innerM: 300, outerM: 900 },
  { dir: [-1, 0.2, 0.1], innerM: 1200, outerM: 2000 },
]
const HPF_TEX_RES = 64

let state = 20260
const random = () => { state = (Math.imul(state, 1664525) + 1013904223) >>> 0; return state / 4294967296 }
const toDegrees = (radians) => radians * 180 / Math.PI
const latLonOf = (dir) => { const l = Math.hypot(...dir); const u = dir.map((c) => c / l); return { lat: Math.fround(toDegrees(Math.asin(u[1]))), lon: Math.fround(toDegrees(Math.atan2(u[0], u[2]))) } }
const toRadians = (degrees) => degrees * Math.PI / 180
const unitOf = ({ lat, lon }) => [Math.cos(toRadians(lat)) * Math.sin(toRadians(lon)), Math.sin(toRadians(lat)), Math.cos(toRadians(lat)) * Math.cos(toRadians(lon))]

const random_points = Array.from({ length: POINT_COUNT }, () => ({ lat: Math.fround(random() * 170 - 85), lon: Math.fround(random() * 360 - 180) }))
const carve_points = CARVES.flatMap((c) => { const centre = latLonOf(c.dir); return [centre, { lat: Math.fround(centre.lat + 0.5), lon: centre.lon }] })
const points = [...random_points, ...carve_points]

const cases = [
  { name: 'default', tcfg: { radius: RADIUS, seed: SEED } },
  { name: 'carves', tcfg: { radius: RADIUS, seed: SEED, carves: CARVES } },
  { name: 'hpfTexRes', tcfg: { radius: RADIUS, seed: SEED, physics: { hpfTexRes: HPF_TEX_RES } } },
]
const out = []
for (const { name, tcfg } of cases) {
  const opts = planetSamplerOptsOf(tcfg)
  const sampler = await loadPlanetSampler(opts)
  out.push({ name, config: { radius: RADIUS, seed: SEED, carves: opts.carves, hpfTexRes: opts.hpfTexRes ?? 128 }, hashVersion: sampler.hashVersion, heights: points.map((p) => sampler.heightAt(unitOf(p))) })
}
mkdirSync(outDir, { recursive: true })
const fixture = { source: `spoint ${rev}: src/terrain/TerrainPhysics.js planetSamplerOptsOf and loadPlanetSampler, then heightAt at fixed points`, seed: SEED, points, cases: out }
writeFileSync(join(outDir, 'spoint-sampler.json'), JSON.stringify(fixture, (key, value) => (typeof value === 'number' && key !== 'seed' ? String(value) : value)) + '\n')
console.log(`wrote ${out.length} cases of ${points.length} heights from spoint ${rev}; hashVersions ${out.map((c) => c.hashVersion).join(',')}`)
