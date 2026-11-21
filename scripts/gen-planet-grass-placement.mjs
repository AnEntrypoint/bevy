import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { writeParity, spointImport } from './lib/spoint.mjs'

const RADIUS = 63600
const SEED = 1337
const RELIEF_SCALE = 0.001
const ANCHOR_DIR = [-0.641, 0.2558, 0.7237]
const CHUNK_SOURCE = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-planet-veg-placement.json')
const RING_M = 88
const BLADE_ROLES = ['anchor', 'upland']
const BLADE_FIELDS = ['q', 'x', 'y', 'z', 'scale', 'yaw', 'nx', 'ny', 'nz', 'tint', 'windPhase', 'shadow', 'groundR', 'groundG', 'groundB']
const GRASS_CONFIG = { grassRenderDistance: 88 }

const unit = (v) => { const l = Math.hypot(...v); return v.map((c) => c / l) }
const plain = (v) => Array.from(v)
const sampledChunks = () => JSON.parse(readFileSync(CHUNK_SOURCE, 'utf8')).chunks.map(({ role, face, i, j }) => ({ role, face, i, j }))

await writeParity(async ({ spoint, rev }) => {
  const im = (rel) => spointImport(spoint, rel)
  const { createHeightSampler } = await im('packages/mapspinner/src/height-cpu.js')
  const { createAnchorField } = await im('packages/mapspinner/src/anchor-field.js')
  const { DEFAULT_TERRAIN_HASH_VERSION } = await im('src/shared/terrainConfig.js')
  const { createPlanetFrame } = await im('src/terrain/PlanetFrame.js')
  const { latticeFor, climateAt } = await im('src/terrain/PlacementChart.js')
  const { GRASS, GRASS_DENSITY, placementsForGrassChunk, grassBladeEdgeM } = await im('src/terrain/GrassPlacement.js')
  const { createCachedAnchorField } = await im('src/terrain/ClimateCache.js')

  const anchorField = createAnchorField({ seed: SEED })
  const sampler = createHeightSampler({ radius: RADIUS, seed: SEED, reliefScale: RELIEF_SCALE, hashVersion: DEFAULT_TERRAIN_HASH_VERSION, anchorField })
  const lattice = latticeFor({ radius: RADIUS }, GRASS)
  const frameAt = (dir) => createPlanetFrame({ sampler, anchorDir: dir, offsetY: 0, reliefScale: RELIEF_SCALE })

  const placeChunk = (key) => {
    const dir = lattice.chunkCentreDir(key, [0, 0, 0])
    const frame = frameAt(dir)
    const field = createCachedAnchorField(anchorField, frame)
    return { dir, frame, field, list: placementsForGrassChunk(key, frame, field, SEED) }
  }

  const keyTable = Object.fromEntries([...readFileSync(join(spoint, 'src/terrain/GrassPlacement.js'), 'utf8').matchAll(/\b(K_[A-Z0-9_]+) = (\d+)/g)].map((m) => [m[1], Number(m[2])]))
  const fullRoles = new Set(BLADE_ROLES)

  const chunks = sampledChunks().map(({ role, face, i, j }) => {
    const key = lattice.chunkKey(face, i, j)
    const { dir, frame, field, list } = placeChunk(key)
    const climate = climateAt(field, 0, 0, dir)
    const recordBlades = fullRoles.delete(role)
    return {
      role, key, face, i, j, centreDir: plain(dir), centreElevationM: frame.anchorHeight,
      climate: { temp: climate.temp, humidity: climate.humidity, erosion: climate.erosion, seaBias: climate.seaBias },
      placedCount: list.length,
      meanEdgeM: list.length ? list.reduce((sum, b) => sum + grassBladeEdgeM(b.q), 0) / list.length : 0,
      blades: recordBlades ? list.map((b) => [...BLADE_FIELDS.map((f) => b[f]), ...plain(frame.localToWorld(b.x, b.y, b.z))]) : null,
    }
  })

  const ring = lattice.ringAroundDir(...unit(ANCHOR_DIR), RING_M).slice().map((key) => [key, placeChunk(key).list.length])

  return {
    'spoint-planet-grass-placement.json': {
      rev,
      rule: 'chunks are sampled (the chunk coordinates of the veg placement fixture); blade arrays are recorded in full for the chunks whose role is in bladeRoles (first chunk of each role, all blades, never a subset) and only counts for the others, because a chunk holds up to 16 x 16 x 79 blades',
      planet: { radius: RADIUS, seed: SEED, reliefScale: RELIEF_SCALE, anchorDir: ANCHOR_DIR, hashVersion: DEFAULT_TERRAIN_HASH_VERSION },
      grassConfig: GRASS_CONFIG,
      grass: GRASS,
      density: GRASS_DENSITY,
      keyTable,
      bladeRoles: BLADE_ROLES,
      bladeFields: [...BLADE_FIELDS, 'worldX', 'worldY', 'worldZ'],
      chunkCount: chunks.length,
      chunks,
      ring: { radiusM: RING_M, fields: ['key', 'placed'], placed: ring.reduce((s, r) => s + r[1], 0), chunks: ring },
    },
  }
}, { indent: 0 })
