import { writeParity, spointImport } from './lib/spoint.mjs'

const SEED = 1337
const RADII = [63600, 6360000]
const ANCHOR_DIR = [-0.641, 0.2558, 0.7237]
const BAKE_RES = 128
const BAKE_MAX_LEVEL = 7
const SPREAD_COUNT = 40
const SEAM_EPSILON = 1e-6
const BAKE_NODES = [[0, 0], [127, 0], [0, 127], [127, 127], [64, 64], [5, 77], [100, 3], [33, 120]]
const BAND_TILES = [[0, 0], [1, 2], [5, 3]]
const FACE_COUNT = 6
const CHANNELS = 4

const unit = (a) => { const n = Math.hypot(...a); return a.map((c) => c / n) }
const goldenAngle = Math.PI * (3 - Math.sqrt(5))
const spread = Array.from({ length: SPREAD_COUNT }, (_, i) => {
  const y = 1 - (2 * (i + 0.5)) / SPREAD_COUNT
  const r = Math.sqrt(1 - y * y)
  return [Math.cos(goldenAngle * i) * r, y, Math.sin(goldenAngle * i) * r]
})
const axes = [[1, 0, 0], [-1, 0, 0], [0, 1, 0], [0, -1, 0], [0, 0, 1], [0, 0, -1]]
const signs = [-1, 1]
const corners = signs.flatMap((x) => signs.flatMap((y) => signs.map((z) => [x, y, z])))
const edgeMids = [0, 1, 2].flatMap((free) => signs.flatMap((a) => signs.map((b) => {
  const [i, j] = [0, 1, 2].filter((k) => k !== free)
  const d = [0, 0, 0]
  d[i] = a
  d[j] = b
  return d
})))
const seamPairs = edgeMids.flatMap((m) => {
  const [i, j] = [0, 1, 2].filter((k) => m[k] !== 0)
  return [i, j].map((k) => m.map((c, n) => (n === k ? c * (1 + SEAM_EPSILON) : c)))
})
const named = [
  ['anchor', ANCHOR_DIR],
  ...axes.map((d, i) => ['face-centre-or-pole-' + i, d]),
  ...edgeMids.map((d, i) => ['edge-mid-' + i, d]),
  ...corners.map((d, i) => ['cube-corner-' + i, d]),
  ...seamPairs.map((d, i) => ['seam-offset-' + i, d]),
  ...spread.map((d, i) => ['spread-' + i, d]),
].map(([name, d]) => ({ name, dir: unit(d) }))

await writeParity(async ({ spoint, rev }) => {
  const { createHeightSampler } = await spointImport(spoint, 'packages/mapspinner/src/height-cpu.js')
  const { createAnchorField } = await spointImport(spoint, 'packages/mapspinner/src/anchor-field.js')
  const { hpfFaceSampler } = await spointImport(spoint, 'packages/mapspinner/src/hpf-lattice.js')
  const { DEFAULT_TERRAIN_HASH_VERSION } = await spointImport(spoint, 'src/shared/terrainConfig.js')

  const field = createAnchorField({ seed: SEED })
  const channelsOf = (s) => [s.seaBias, s.elevAmp, s.temp, s.humidity, s.erosion, s.roughness]

  const perRadius = (radius) => {
    const baked = Array.from({ length: FACE_COUNT }, () => [])
    const recording = {
      ...field,
      sampleUV(face, fu, fv, maxBandLevel) {
        const s = field.sampleUV(face, fu, fv, maxBandLevel)
        if (maxBandLevel !== undefined) baked[face].push(s.seaBias, s.elevAmp, s.temp, s.humidity)
        return s
      },
    }
    const sampler = createHeightSampler({ radius, seed: SEED, hashVersion: DEFAULT_TERRAIN_HASH_VERSION, anchorField: recording })
    const heights = named.map(({ dir }) => sampler.heightAt(dir))
    const buffers = baked.map((b, face) => {
      if (b.length !== BAKE_RES * BAKE_RES * CHANNELS) throw new Error('face ' + face + ' bake recorded ' + b.length)
      return Float32Array.from(b)
    })
    const samplers = buffers.map((b) => hpfFaceSampler(b, BAKE_RES))
    const directions = named.map(({ name, dir }, i) => {
      const { face, fu, fv } = field.dirToFaceUV(dir)
      const hpf = Array.from(samplers[face](fu, fv, [0, 0, 0, 0]))
      return { name, dir, face, fu, fv, height: heights[i], continuous: channelsOf(field.sampleDir(dir)), hpf }
    })
    const bakeNodes = buffers.flatMap((b, face) => BAKE_NODES.map(([x, y]) => ({
      face, x, y,
      baked: Array.from(b.subarray((y * BAKE_RES + x) * CHANNELS, (y * BAKE_RES + x + 1) * CHANNELS)),
      continuous: channelsOf(field.sampleUV(face, x / (BAKE_RES - 1), y / (BAKE_RES - 1), BAKE_MAX_LEVEL)),
    })))
    return { radius, reliefScale: radius / 63600000, directions, bakeNodes }
  }

  const bands = field.BANDS.map((_, band) => Array.from({ length: FACE_COUNT }, (_, face) => BAND_TILES.map(([tx, ty]) => {
    const n = 1 << field.BANDS[band].level
    const x = tx % n
    const y = ty % n
    const p = field.baseParams(band, face, x, y)
    return { band, face, tx: x, ty: y, params: [p.seaBias, p.elevAmp, p.temp, p.humidity, p.erosion, p.roughness] }
  }))).flat(2)

  return {
    'spoint-planet-climate.json': {
      source: 'spoint ' + rev + ': packages/mapspinner/src/anchor-field.js createAnchorField (sampleDir, sampleUV, baseParams), anchor-field-bands.js BANDS, height-cpu.js createHeightSampler (heightAt and its own baked HPF face buffers captured through a recording anchor field), hpf-lattice.js hpfFaceSampler; all run unchanged',
      spoint: rev,
      seed: SEED,
      hashVersion: DEFAULT_TERRAIN_HASH_VERSION,
      anchorDir: ANCHOR_DIR,
      channelOrder: ['seaBias', 'elevAmp', 'temp', 'humidity', 'erosion', 'roughness'],
      hpfChannelOrder: ['seaBias', 'elevAmp', 'temp', 'humidity'],
      bakeRes: BAKE_RES,
      bakeMaxLevel: BAKE_MAX_LEVEL,
      bandLevels: field.BANDS.map((b) => b.level),
      bands,
      radii: RADII.map(perRadius),
    },
  }
}, { indent: 0 })
