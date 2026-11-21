import { registerHooks } from 'node:module'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'
import { writeFixture, spointImport } from './lib/spoint.mjs'

const MAPSPINNER = 'packages/mapspinner/src/'
const SCALAR_SHIM = pathToFileURL(join(import.meta.dirname, 'lib', 'tsl-scalar.mjs')).href
const SCALAR_PARENT = 'sea-waves-tsl.js?scalar'
const PLANET_RADIUS = 6360000
const NOISE_PROBES = [[0, 0], [0.5, 0.25], [-3.7, 12.1], [100.3, -57.9], [2.999, 3.001], [-0.001, -0.001], [1234.567, 890.123]]
const OCTAVE_CHOPPIES = [0, 0.3, 0.5, 0.9, 1]
const HEIGHT_POINTS = [[0.3, 0.7], [3.7, -2.2], [-41.3, 18.8], [250.5, 120.25], [-777.7, 333.3], [1023.9, 1.1]]
const HEIGHT_TIMES = [0, 1.7, 13.37, 600]
const HEIGHT_WINDOWS = [[0, 8], [0, 2], [2, 6], [6, 8], [3, 4]]
const HEIGHT_OCEANS = [[1, 0.5], [0.5, 0.2], [2, 0.9]]
const MEAN_OCEANS = [[0.5, 1], [0.2, 0.5], [0.9, 2]]
const GATE_HEIGHT = { amplitude: 2500, kx: 3, phase: 1, ky: 2.2, tilt: 800, base: -300 }
const LEAVES = [
  [0, 6, 3, 5], [0, 11, 1000, 1500], [0, 12, 2000, 3000], [0, 12, 2001, 3000], [0, 12, 2000, 3001], [0, 12, 2001, 3001], [0, 14, 8000, 12000],
  [0, 11, 1000, 1500], [2, 11, 1024, 1024], [2, 9, 256, 256], [2, 13, 4100, 4100], [4, 11, 900, 1100], [4, 10, 500, 700], [4, 15, 33000, 33000],
  [1, 11, 2047, 2047], [3, 11, 0, 0], [5, 12, 1234, 4321], [5, 12, 1235, 4321], [5, 3, 1, 6], [4, 11, 1000, 1000], [2, 12, 2050, 2051],
]
const CONSTANT_FILES = {
  'tsl/sea-waves-tsl.js': 'SEA_OCTAVES|SEA_FREQ|SEA_SPEED|SEA_AMP_M|SEA_OCTAVE_ANGLE|SEA_FREQ_LF',
  'sea-waves.js': 'OCTAVES|BASE_AMP_M|PHASE_STEP|AMP_GAIN|CHOPPY_TO_ONE|GRID|GRID_STEP',
  'tsl/water-tsl.js': 'WATER_GRID|WATER_GRID_SKIRT|WATER_MAX_LEVEL|WATER_KEY_SPAN|QUARTER_PI|SLOPE_EPS_M|SLOPE_EPS_LF_M|MARCH_NEAR_M|MARCH_STEPS|BISECT_MAX|WAVE_TILE_M|UNDERWATER_BELOW_M|LAND_ABOVE_SEA_DISCARD_M|DEFAULT_OCEAN',
}

registerHooks({
  resolve: (specifier, context, next) => (specifier === 'three/tsl' && context.parentURL?.endsWith(SCALAR_PARENT) ? { url: SCALAR_SHIM, shortCircuit: true } : next(specifier, context)),
})

const evaluate = (expression) => new Function('return (' + expression + ')')()

function constants(spoint, file, names) {
  const text = readFileSync(join(spoint, MAPSPINNER, file), 'utf8')
  const out = {}
  for (const line of text.split('\n')) {
    const m = line.match(new RegExp('^(?:export )?const (' + names + ') = (.+)$'))
    if (m) out[m[1]] = evaluate(m[2])
  }
  return out
}

const gateHeight = ([x, y, z]) => GATE_HEIGHT.amplitude * Math.sin(GATE_HEIGHT.kx * x + GATE_HEIGHT.phase) * Math.cos(GATE_HEIGHT.ky * y) + GATE_HEIGHT.tilt * z + GATE_HEIGHT.base

async function waterSurface(spoint) {
  const three = await spointImport(spoint, 'node_modules/three/build/three.webgpu.js')
  const tsl = await spointImport(spoint, 'node_modules/three/build/three.tsl.js')
  const { FACE_FRAME } = await spointImport(spoint, MAPSPINNER + 'anchor-field-bands.js')
  const { createWaterTSL } = await spointImport(spoint, MAPSPINNER + 'tsl/water-tsl.js')
  const vector = ([x, y, z]) => new three.Vector3(x, y, z)
  const terrainUniforms = {
    defRadius: tsl.uniform(PLANET_RADIUS),
    camDir: tsl.uniform(vector([0, 1, 0])),
    camAlt: tsl.uniform(0),
    camRender: tsl.uniform(vector([0, 0, 0])),
    east: tsl.uniform(vector([1, 0, 0])),
    up: tsl.uniform(vector([0, 1, 0])),
    north: tsl.uniform(vector([0, 0, 1])),
  }
  const frameArray = (axis) => tsl.uniformArray(FACE_FRAME.map((frame) => vector(frame[axis])), 'vec3')
  const water = createWaterTSL({
    spec: { composeHeight: () => tsl.float(0) },
    terrainUniforms,
    faceU: frameArray('u'),
    faceV: frameArray('v'),
    faceC: frameArray('c'),
    heightAt: gateHeight,
    sculpt: null,
  })
  const offsets = []
  const faces = []
  for (const [face, level, x, y] of LEAVES) {
    const l = (2 * PLANET_RADIUS) / 2 ** level
    offsets.push(-PLANET_RADIUS + x * l, -PLANET_RADIUS + y * l, l, level)
    faces.push(face)
  }
  const kept = water.setQuads(offsets, faces, LEAVES.length)
  const geometry = water.mesh.geometry
  const keptLeaves = []
  for (let i = 0; i < kept; i++) {
    const [ox, oy, l, level] = Array.from(geometry.attributes.iOffset.array.slice(i * 4, i * 4 + 4))
    keptLeaves.push([Math.round(geometry.attributes.iFace.array[i]), level, Math.round((ox + PLANET_RADIUS) / l), Math.round((oy + PLANET_RADIUS) / l)])
  }
  return { gate: GATE_HEIGHT, radius: PLANET_RADIUS, input: LEAVES, kept: keptLeaves }
}

writeFixture('spoint-planet-waves', async ({ spoint, rev }) => {
  const seaTsl = await import(pathToFileURL(join(spoint, MAPSPINNER, 'tsl/sea-waves-tsl.js')).href + '?scalar')
  const scalar = await import(SCALAR_SHIM)
  const { seaHeightMean } = await spointImport(spoint, MAPSPINNER + 'sea-waves.js')
  const { buildGridGeometry } = await spointImport(spoint, MAPSPINNER + 'patch-grid-geometry.js')
  const slopeEps = constants(spoint, 'tsl/water-tsl.js', 'SLOPE_EPS_M|SLOPE_EPS_LF_M')
  const { float, int, vec2 } = scalar
  const at = ([x, y]) => vec2(x, y)
  const height = (p, t, start, end, amp, choppy) => seaTsl.seaHeight(at(p), float(t), int(start), int(end), float(amp), float(choppy)).c[0]
  const heightLF = (p, t, amp, choppy) => seaTsl.seaHeightLF(at(p), float(t), float(amp), float(choppy)).c[0]
  const heights = []
  const slopes = []
  const lowFrequency = []
  for (const p of HEIGHT_POINTS) for (const t of HEIGHT_TIMES) for (const [amp, choppy] of HEIGHT_OCEANS) {
    for (const [start, end] of HEIGHT_WINDOWS) {
      const h = height(p, t, start, end, amp, choppy)
      heights.push({ p, t, start, end, amp, choppy, h })
      const hx = height([p[0] + slopeEps.SLOPE_EPS_M, p[1]], t, start, end, amp, choppy)
      const hy = height([p[0], p[1] + slopeEps.SLOPE_EPS_M], t, start, end, amp, choppy)
      slopes.push({ p, t, start, end, amp, choppy, s: [(hx - h) / slopeEps.SLOPE_EPS_M, (hy - h) / slopeEps.SLOPE_EPS_M] })
    }
    const lf = heightLF(p, t, amp, choppy)
    const lx = heightLF([p[0] + slopeEps.SLOPE_EPS_LF_M, p[1]], t, amp, choppy)
    const ly = heightLF([p[0], p[1] + slopeEps.SLOPE_EPS_LF_M], t, amp, choppy)
    lowFrequency.push({ p, t, amp, choppy, h: lf, s: [(lx - lf) / slopeEps.SLOPE_EPS_LF_M, (ly - lf) / slopeEps.SLOPE_EPS_LF_M] })
  }
  const grid = buildGridGeometry(constants(spoint, 'tsl/water-tsl.js', 'WATER_GRID').WATER_GRID, false)
  return {
    spointRev: rev,
    source: 'sea-waves.js seaHeightMean and patch-grid-geometry.js buildGridGeometry run unchanged; sea-waves-tsl.js seaNoise seaOctave seaHeight seaHeightLF run unchanged on the scalar stand-in scripts/lib/tsl-scalar.mjs; water-tsl.js createWaterTSL setQuads run unchanged on three nodes; slopes transcribe the slopeAt differences of water-tsl.js with its extracted eps',
    constants: Object.fromEntries(Object.entries(CONSTANT_FILES).map(([file, names]) => [file, constants(spoint, file, names)])),
    noise: NOISE_PROBES.map((p) => ({ p, v: seaTsl.seaNoise(at(p)).c[0] })),
    octave: NOISE_PROBES.flatMap((p) => OCTAVE_CHOPPIES.map((choppy) => ({ p, choppy, v: seaTsl.seaOctave(at(p), float(choppy)).c[0] }))),
    height: heights,
    slope: slopes,
    heightLF: lowFrequency,
    mean: MEAN_OCEANS.map(([choppy, amp]) => ({ choppy, amp, v: seaHeightMean(choppy, amp) })),
    grid: { vertices: Array.from(grid.vertices), indices: Array.from(grid.indices) },
    surface: await waterSurface(spoint),
  }
})
