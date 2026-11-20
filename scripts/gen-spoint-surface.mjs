import { execFileSync } from 'node:child_process'
import { mkdirSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const mapspinner = join(spoint, 'packages/mapspinner/src')
const outDir = join(import.meta.dirname, '..', 'testdata', 'parity')
const rev = execFileSync('git', ['-C', spoint, 'rev-parse', '--short', 'HEAD']).toString().trim()
const { createHeightSampler } = await import(pathToFileURL(join(mapspinner, 'height-cpu.js')).href)
const { createPlanetFrame } = await import(pathToFileURL(join(spoint, 'src/terrain/PlanetFrame.js')).href)
const { surfaceAlongDir } = await import(pathToFileURL(join(spoint, 'src/terrain/PlacementChart.js')).href)

const SEED = 0x5eed1234
const RADIUS = 6360
const HASH_VERSION = 2
const POINT_COUNT = 64

let state = 20260
const random = () => { state = (Math.imul(state, 1664525) + 1013904223) >>> 0; return state / 4294967296 }
const toRadians = (degrees) => degrees * Math.PI / 180
const unitOf = (lat, lon) => [Math.cos(toRadians(lat)) * Math.sin(toRadians(lon)), Math.sin(toRadians(lat)), Math.cos(toRadians(lat)) * Math.cos(toRadians(lon))]

const sampler = createHeightSampler({ radius: RADIUS, seed: SEED, hashVersion: HASH_VERSION })
const frame = createPlanetFrame({ sampler, anchorDir: [0, 1, 0], offsetY: 0 })
const out = [0, 0, 0]
const points = Array.from({ length: POINT_COUNT }, () => {
  const lat = Math.fround(random() * 170 - 85)
  const lon = Math.fround(random() * 360 - 180)
  const [dx, dy, dz] = unitOf(lat, lon)
  const r = surfaceAlongDir(frame, dx, dy, dz, 0, out)
  return { lat, lon, r }
})
mkdirSync(outDir, { recursive: true })
const fixture = { source: `spoint ${rev}: packages/mapspinner/src/terrain/PlacementChart.js surfaceAlongDir (cpu-solve branch) with PlanetFrame createPlanetFrame and the height-cpu sampler at hashVersion ${HASH_VERSION}`, seed: SEED, radius: RADIUS, points }
writeFileSync(join(outDir, 'spoint-surface.json'), JSON.stringify(fixture, (key, value) => (typeof value === 'number' && key !== 'seed' ? String(value) : value)) + '\n')
console.log(`wrote ${points.length} surfaceAlongDir points from spoint ${rev}`)
