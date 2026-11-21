import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { writeParity, spointImport } from './lib/spoint.mjs'

const RADIUS = 63600
const SEED = 1337
const RELIEF_SCALE = 0.001
const ANCHOR_DIR = [-0.641, 0.2558, 0.7237]
const CHUNK_SOURCE = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-planet-veg-placement.json')

const unit = (v) => { const l = Math.hypot(...v); return v.map((c) => c / l) }
const plain = (v) => Array.from(v)
const sampledChunks = () => JSON.parse(readFileSync(CHUNK_SOURCE, 'utf8')).chunks.map(({ role, face, i, j }) => ({ role, face, i, j }))
const RING_M = 320
const SCAN_POINTS = 6000
const ROCK_RICH_CHUNKS = 8
const RING_RICH_CHUNKS = 4
const ROCK_CONFIG = { rocks: true, rockRenderDistance: 320, rockMaxInstances: 12000, rockColliders: true, rockColliderRadius: 32, rockColliderCap: 128 }

await writeParity(async ({ spoint, rev }) => {
  const im = (rel) => spointImport(spoint, rel)
  const { createHeightSampler } = await im('packages/mapspinner/src/height-cpu.js')
  const { createAnchorField } = await im('packages/mapspinner/src/anchor-field.js')
  const { DEFAULT_TERRAIN_HASH_VERSION } = await im('src/shared/terrainConfig.js')
  const { createPlanetFrame } = await im('src/terrain/PlanetFrame.js')
  const { latticeFor, climateAt } = await im('src/terrain/PlacementChart.js')
  const { ROCK, ROCK_SCALE_TABLE, ROCK_SQUASH_TABLE, ROCK_SCALE_LEVELS, ROCK_SQUASH_LEVELS, placementsForRockChunk } = await im('src/terrain/RockPlacement.js')
  const { createCachedAnchorField } = await im('src/terrain/ClimateCache.js')

  const anchorField = createAnchorField({ seed: SEED })
  const sampler = createHeightSampler({ radius: RADIUS, seed: SEED, reliefScale: RELIEF_SCALE, hashVersion: DEFAULT_TERRAIN_HASH_VERSION, anchorField })
  const lattice = latticeFor({ radius: RADIUS }, ROCK)
  const frameAt = (dir) => createPlanetFrame({ sampler, anchorDir: dir, offsetY: 0, reliefScale: RELIEF_SCALE })
  const centreOf = (key) => lattice.chunkCentreDir(key, [0, 0, 0])

  const record = (frame, p) => {
    const world = frame.localToWorld(p.x, p.y, p.z)
    return {
      x: p.x, y: p.y, z: p.z, type: p.type, scale: p.scale, yaw: p.yaw, squash: p.squash, variant: p.variant,
      normal: plain(p.normal), tiltQuat: plain(p.tiltQuat), rockId: p.rockId, world: plain(world), dir: unit(plain(world)),
    }
  }
  const placeChunk = (key) => {
    const dir = centreOf(key)
    const frame = frameAt(dir)
    const field = createCachedAnchorField(anchorField, frame)
    return { dir, frame, field, list: placementsForRockChunk(key, frame, field, SEED) }
  }

  const keyTable = Object.fromEntries([...readFileSync(join(spoint, 'src/terrain/RockPlacement.js'), 'utf8').matchAll(/\b(K_[A-Z0-9_]+) = (\d+)/g)].map((m) => [m[1], Number(m[2])]))

  const scanCounts = new Map()
  for (let n = 0; n < SCAN_POINTS; n++) {
    const y = 1 - 2 * (n + 0.5) / SCAN_POINTS
    const r = Math.sqrt(1 - y * y)
    const a = n * 2.399963229728653
    const key = lattice.chunkKeyOfDir(Math.cos(a) * r, y, Math.sin(a) * r)
    if (!scanCounts.has(key)) scanCounts.set(key, placeChunk(key).list.length)
  }
  const playerDir = unit(ANCHOR_DIR)
  const ring = lattice.ringAroundDir(...playerDir, RING_M).slice().map((key) => [key, placeChunk(key).list.length])
  const byCount = (counts) => [...counts].sort((a, b) => b[1] - a[1] || a[0] - b[0])
  const rockChunk = (role) => ([key]) => { const [face, i, j] = lattice.decodeChunk(key, [0, 0, 0]); return { role, face, i, j } }
  const rockRich = [...byCount(scanCounts).slice(0, ROCK_RICH_CHUNKS).map(rockChunk('rock-rich')), ...byCount(ring).slice(0, RING_RICH_CHUNKS).map(rockChunk('rock-ring'))]

  const chunks = [...sampledChunks(), ...rockRich].map(({ role, face, i, j }) => {
    const key = lattice.chunkKey(face, i, j)
    const { dir, frame, field, list } = placeChunk(key)
    const climate = climateAt(field, 0, 0, dir)
    return {
      role, key, face, i, j, centreDir: plain(dir), centreElevationM: frame.anchorHeight,
      climate: { temp: climate.temp, humidity: climate.humidity, erosion: climate.erosion, seaBias: climate.seaBias },
      placedCount: list.length, instances: list.map((p) => record(frame, p)),
    }
  })

  return {
    'spoint-planet-rock-placement.json': {
      rev,
      rule: 'chunks are sampled (the chunk coordinates of the veg placement fixture plus the richest rock chunks of a deterministic Fibonacci-sphere scan and of the anchor ring); every rock inside a sampled chunk is recorded, never a subset; rocks have no thinning, resident equals placed within the ring up to rockMaxInstances',
      planet: { radius: RADIUS, seed: SEED, reliefScale: RELIEF_SCALE, anchorDir: ANCHOR_DIR, hashVersion: DEFAULT_TERRAIN_HASH_VERSION },
      rockConfig: ROCK_CONFIG,
      rock: ROCK,
      scaleLevels: ROCK_SCALE_LEVELS,
      squashLevels: ROCK_SQUASH_LEVELS,
      scaleTable: plain(ROCK_SCALE_TABLE),
      squashTable: plain(ROCK_SQUASH_TABLE),
      keyTable,
      chunkCount: chunks.length,
      chunks,
      ring: { radiusM: RING_M, fields: ['key', 'placed'], placed: ring.reduce((s, r) => s + r[1], 0), chunks: ring },
    },
  }
})
