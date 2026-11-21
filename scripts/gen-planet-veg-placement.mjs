import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { writeParity, spointImport } from './lib/spoint.mjs'

const RADIUS = 63600
const SEED = 1337
const RELIEF_SCALE = 0.001
const ANCHOR_DIR = [-0.641, 0.2558, 0.7237]
const PLAYER_RING_M = 680
const RENDER_DISTANCE_M = 640
const SCAN_POINTS = 6000
const POLAR_POINTS = 120
const POLAR_Y = 0.995
const SEAM_STEPS = 17
const SEAM_NUDGE = 0.0004
const ARID_HUMIDITY = 0.28
const COLD_TEMP = 0.3
const SOIL_MIN_ELEV_M = 15
const KEY_TABLE_SOURCES = ['VegPlacement.js', 'RockPlacement.js', 'GrassPlacement.js']
const VEGETATION_CONFIG = { enabled: true, seed: 1337, renderDistance: 640, treeline: 4000, densityScale: 1.0, maxInstances: 30000 }

const unit = (v) => { const l = Math.hypot(...v); return v.map((c) => c / l) }
const round = (v) => Array.from(v)

await writeParity(async ({ spoint, rev }) => {
  const im = (rel) => spointImport(spoint, rel)
  const { createHeightSampler } = await im('packages/mapspinner/src/height-cpu.js')
  const { createAnchorField } = await im('packages/mapspinner/src/anchor-field.js')
  const { DEFAULT_TERRAIN_HASH_VERSION } = await im('src/shared/terrainConfig.js')
  const { createPlanetFrame } = await im('src/terrain/PlanetFrame.js')
  const { latticeFor, climateAt } = await im('src/terrain/PlacementChart.js')
  const { VEG, SPECIES, placementsForChunk, reliefMarginScaleOf } = await im('src/terrain/VegPlacement.js')
  const { createCachedAnchorField } = await im('src/terrain/ClimateCache.js')

  const anchorField = createAnchorField({ seed: SEED })
  const sampler = createHeightSampler({ radius: RADIUS, seed: SEED, reliefScale: RELIEF_SCALE, hashVersion: DEFAULT_TERRAIN_HASH_VERSION, anchorField })
  const lattice = latticeFor({ radius: RADIUS }, VEG)
  const frameAt = (dir) => createPlanetFrame({ sampler, anchorDir: dir, offsetY: 0, reliefScale: RELIEF_SCALE })
  const centreOf = (key) => lattice.chunkCentreDir(key, [0, 0, 0])
  const faceIJ = (key) => lattice.decodeChunk(key, [0, 0, 0])

  const keyTable = Object.fromEntries(KEY_TABLE_SOURCES.map((file) => [file, Object.fromEntries(
    [...readFileSync(join(spoint, 'src/terrain', file), 'utf8').matchAll(/\b(K_[A-Z0-9_]+) = (\d+)/g)].map((m) => [m[1], Number(m[2])]))]))

  const vegetationSource = readFileSync(join(spoint, 'client/core/Vegetation.js'), 'utf8')
  const thinningText = ['function vegKeepProb', 'function _treeCoin'].map((head) => {
    const from = vegetationSource.indexOf(head)
    return vegetationSource.slice(from, vegetationSource.indexOf('\n  }\n', from) + 5)
  }).join('\n')
  const { vegKeepProb, treeCoin } = new Function('FALLOFF_NEAR', 'FALLOFF_FLOOR', 'renderDistance', '_falloffSpan', thinningText + '\nreturn { vegKeepProb, treeCoin: _treeCoin }')(96, 0.15, RENDER_DISTANCE_M, RENDER_DISTANCE_M - 96)

  const playerDir = unit(ANCHOR_DIR)
  const playerWorld = frameAt(playerDir).localToWorld(0, 0, 0)

  const survey = (key) => {
    const dir = centreOf(key)
    const frame = frameAt(dir)
    const field = createCachedAnchorField(anchorField, frame)
    const climate = climateAt(field, 0, 0, dir)
    return { key, elev: frame.anchorHeight, temp: climate.temp, humidity: climate.humidity, placed: placementsForChunk(key, frame, field, SEED).length }
  }

  const fibonacci = Array.from({ length: SCAN_POINTS }, (_, i) => {
    const y = 1 - 2 * (i + 0.5) / SCAN_POINTS
    const r = Math.sqrt(1 - y * y)
    const a = i * 2.399963229728653
    return [Math.cos(a) * r, y, Math.sin(a) * r]
  })
  const scanned = new Map()
  for (const dir of fibonacci) {
    const key = lattice.chunkKeyOfDir(...dir)
    if (!scanned.has(key)) scanned.set(key, survey(key))
  }
  const candidates = [...scanned.values()]

  const polarCandidates = new Map()
  for (const sign of [1, -1]) {
    for (let i = 0; i < POLAR_POINTS; i++) {
      const a = i * 2.399963229728653
      const r = Math.sqrt(1 - POLAR_Y * POLAR_Y)
      const key = lattice.chunkKeyOfDir(Math.cos(a) * r, sign * POLAR_Y, Math.sin(a) * r)
      if (!polarCandidates.has(key)) polarCandidates.set(key, survey(key))
    }
  }

  const chosen = []
  const take = (role, key) => { if (!chosen.some((c) => c.key === key)) chosen.push({ role, key }) }
  const pick = (role, count, filter, score, pool = candidates) => pool.filter(filter)
    .sort((a, b) => score(b) - score(a) || a.key - b.key).filter((c) => !chosen.some((k) => k.key === c.key))
    .slice(0, count).forEach((c) => take(role, c.key))

  const anchorKey = lattice.chunkKeyOfDir(...playerDir)
  const [anchorFace, anchorI, anchorJ] = faceIJ(anchorKey)
  take('anchor', anchorKey)
  for (const [di, dj] of [[1, 0], [-1, 0], [0, 1], [0, -1]]) take('anchor-neighbour', lattice.chunkKey(anchorFace, anchorI + di, anchorJ + dj))
  pick('forest', 4, (c) => c.placed > 0, (c) => c.placed)
  pick('arid', 2, (c) => c.humidity < ARID_HUMIDITY && c.elev > SOIL_MIN_ELEV_M, (c) => c.placed)
  pick('cold', 2, (c) => c.temp < COLD_TEMP && c.elev > SOIL_MIN_ELEV_M, (c) => c.placed)
  pick('beach', 2, (c) => c.elev > 0 && c.elev <= SOIL_MIN_ELEV_M, (c) => c.placed)
  pick('soil-fade', 2, (c) => c.elev > SOIL_MIN_ELEV_M && c.elev <= 40, (c) => c.placed)
  pick('below-sea', 1, (c) => c.elev < 0 && c.elev > -40, (c) => -Math.abs(c.elev + 10))
  pick('deep-ocean', 1, (c) => c.elev < -200, (c) => -c.elev)
  pick('bare-high', 2, (c) => c.placed === 0 && c.elev > 100, (c) => c.elev)
  pick('upland', 2, (c) => c.elev > 120 && c.elev <= 250, (c) => c.placed)
  pick('polar-land', 2, (c) => c.elev > SOIL_MIN_ELEV_M, (c) => c.placed, [...polarCandidates.values()])
  pick('treeline-high', 3, (c) => c.elev > 250, (c) => c.elev)
  take('pole-north', lattice.chunkKeyOfDir(0, 1, 0))
  take('pole-south', lattice.chunkKeyOfDir(0, -1, 0))

  const seamPairs = []
  for (const [a, b] of [[0, 1], [0, 2], [1, 2]]) {
    for (const sa of [1, -1]) {
      for (const sb of [1, -1]) {
        for (let s = 0; s < SEAM_STEPS; s++) {
          const dir = [0, 0, 0]
          dir[a] = sa
          dir[b] = sb
          dir[3 - a - b] = -0.8 + 1.6 * s / (SEAM_STEPS - 1)
          const nudged = (sign) => { const d = dir.slice(); d[a] *= 1 + sign * SEAM_NUDGE; return lattice.chunkKeyOfDir(...unit(d)) }
          const pair = [nudged(1), nudged(-1)]
          if (faceIJ(pair[0])[0] === faceIJ(pair[1])[0]) continue
          const infos = pair.map((key) => scanned.get(key) ?? survey(key))
          if (infos.every((i) => i.elev > SOIL_MIN_ELEV_M)) seamPairs.push({ pair, placed: infos[0].placed + infos[1].placed })
        }
      }
    }
  }
  seamPairs.sort((x, y) => y.placed - x.placed || x.pair[0] - y.pair[0])
  seamPairs.slice(0, 2).forEach(({ pair }, n) => pair.forEach((key) => take('seam-' + n, key)))

  const record = (frame, p) => {
    const world = frame.localToWorld(p.x, p.y, p.z)
    const dist = Math.hypot(world[0] - playerWorld[0], world[1] - playerWorld[1], world[2] - playerWorld[2])
    const coin = treeCoin(p.trunkId)
    return {
      x: p.x, y: p.y, z: p.z, species: p.species, shape: p.shape, scale: p.scale, yaw: p.yaw, windPhase: p.windPhase,
      tint: round(p.tint), tiltQuat: round(p.tiltQuat), normal: round(p.normal), trunkId: p.trunkId,
      world: round(world), dir: unit(round(world)), playerDistanceM: dist, thinCoin: coin, keepProb: vegKeepProb(dist),
      resident: dist <= PLAYER_RING_M && (dist <= 96 || coin < vegKeepProb(dist)),
    }
  }

  const chunks = chosen.map(({ role, key }) => {
    const [face, i, j] = faceIJ(key)
    const dir = centreOf(key)
    const frame = frameAt(dir)
    const field = createCachedAnchorField(anchorField, frame)
    const climate = climateAt(field, 0, 0, dir)
    const instances = placementsForChunk(key, frame, field, SEED).map((p) => record(frame, p))
    return {
      role, key, face, i, j, centreDir: round(dir),
      frame: { anchorDir: round(frame.anchorDir), up: round(frame.up), east: round(frame.east), north: round(frame.north), anchorHeight: frame.anchorHeight, anchorSurfaceWorld: round(frame.anchorSurfaceWorld), reliefScale: frame.reliefScale, reliefMarginScale: reliefMarginScaleOf(frame) },
      centreElevationM: frame.anchorHeight,
      climate: { temp: climate.temp, humidity: climate.humidity, erosion: climate.erosion, seaBias: climate.seaBias },
      placedCount: instances.length,
      residentCount: instances.filter((r) => r.resident).length,
      instances,
    }
  })

  const ringKeys = lattice.ringAroundDir(...playerDir, PLAYER_RING_M).slice()
  const ring = ringKeys.map((key) => {
    const frame = frameAt(centreOf(key))
    const field = createCachedAnchorField(anchorField, frame)
    const list = placementsForChunk(key, frame, field, SEED)
    const resident = list.filter((p) => record(frame, p).resident).length
    return [key, list.length, resident]
  })

  return {
    'spoint-planet-veg-placement.json': {
      rev,
      rule: 'chunks are sampled (role-selected by a deterministic Fibonacci-sphere scan plus anchor neighbours, poles and seam pairs); every instance inside a sampled chunk is recorded, never a subset',
      planet: { radius: RADIUS, seed: SEED, reliefScale: RELIEF_SCALE, anchorDir: ANCHOR_DIR, hashVersion: DEFAULT_TERRAIN_HASH_VERSION },
      vegetationConfig: VEGETATION_CONFIG,
      veg: VEG,
      species: SPECIES,
      treelineM: VEG.TREELINE * reliefMarginScaleOf({ reliefScale: RELIEF_SCALE }),
      treelineFadeM: VEG.TREELINE_FADE * reliefMarginScaleOf({ reliefScale: RELIEF_SCALE }),
      keyTable,
      thinning: { source: 'client/core/Vegetation.js vegKeepProb and _treeCoin text extracted unchanged', nearM: 96, floor: 0.15, renderDistanceM: RENDER_DISTANCE_M, playerDirection: playerDir, playerWorld: round(playerWorld), residentRadiusM: PLAYER_RING_M },
      scan: { points: SCAN_POINTS, distinctChunks: candidates.length, maxElevationM: Math.max(...candidates.map((c) => c.elev)), minElevationM: Math.min(...candidates.map((c) => c.elev)) },
      chunkCount: chunks.length,
      chunks,
      ring: { radiusM: PLAYER_RING_M, fields: ['key', 'placed', 'resident'], placed: ring.reduce((s, r) => s + r[1], 0), resident: ring.reduce((s, r) => s + r[2], 0), chunks: ring },
    },
  }
})
