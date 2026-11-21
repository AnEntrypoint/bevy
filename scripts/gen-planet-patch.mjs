import { writeParity, spointImport } from './lib/spoint.mjs'

const GRID_SIZE = 9
const RADII = [63600, 6360000]
const LEVELS = [0, 6, 13, 22]
const NODE_SPECS = [[0, 0, 0], [1, 0.5, 0.5], [2, 1, 1], [3, 0.3, 0.7], [4, 0.9, 0.1], [5, 0.5, 0.5]]
const NODES_PER_LEVEL = 3
const SEED = 0x5eed1234
const QUARTER_PI = 0.7853981634
const SKIRT_MIN_M = 30.0
const SKIRT_PATCH_FRAC = 0.06

const sub = (a, b) => a.map((c, k) => c - b[k])
const cross = (a, b) => [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
const dot = (a, b) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
const unit = (a) => { const n = Math.hypot(...a); return a.map((c) => c / n) }

await writeParity(async ({ spoint, rev, statusBefore }) => {
  const { buildGridGeometry } = await spointImport(spoint, 'packages/mapspinner/src/patch-grid-geometry.js')
  const { FACE_FRAME } = await spointImport(spoint, 'packages/mapspinner/src/planet-orchestrator-cull.js')
  const { createHeightSampler } = await spointImport(spoint, 'packages/mapspinner/src/height-cpu.js')
  const { createAnchorField } = await spointImport(spoint, 'packages/mapspinner/src/anchor-field.js')
  const { RELIEF_DESIGN_RADIUS_M } = await spointImport(spoint, 'packages/mapspinner/src/terrain-defaults.js')
  const { DEFAULT_TERRAIN_HASH_VERSION } = await spointImport(spoint, 'src/shared/terrainConfig.js')

  const grid = buildGridGeometry(GRID_SIZE)
  const layout = Array.from({ length: grid.vertices.length / 3 }, (_, n) => Array.from(grid.vertices.subarray(3 * n, 3 * n + 3)))
  const anchorField = createAnchorField({ seed: SEED })

  const patchCase = (R, sampler, [face, fx, fy], level) => {
    const cells = 2 ** level
    const tx = Math.min(cells - 1, Math.floor(fx * cells))
    const ty = Math.min(cells - 1, Math.floor(fy * cells))
    const size = (2 * R) / cells
    const ox = -R + tx * size
    const oy = -R + ty * size
    const F = FACE_FRAME[face]
    const step = size * (1 / GRID_SIZE)
    const direction = (ax, ay) => {
      const fl = [Math.tan((ax / R) * QUARTER_PI) * R, Math.tan((ay / R) * QUARTER_PI) * R]
      return unit([0, 1, 2].map((k) => F.u[k] * fl[0] + F.v[k] * fl[1] + F.c[k] * R))
    }
    const tap = (ax, ay) => {
      const d = direction(ax, ay)
      const h = sampler.heightAt(d)
      return { d, h, w: d.map((c) => c * (R + h)) }
    }
    const reliefScale = R / RELIEF_DESIGN_RADIUS_M
    const skirtM = Math.max(size * SKIRT_PATCH_FRAC, (reliefScale > 0 ? reliefScale : 1) * SKIRT_MIN_M)
    const positions = []
    const normals = []
    const heights = []
    for (const [px, py, ring] of layout) {
      const ax = ox + px * size
      const ay = oy + py * size
      const c = tap(ax, ay)
      const pu = tap(ax + step, ay)
      const pv = tap(ax, ay + step)
      const n0 = unit(cross(sub(pu.w, c.w), sub(pv.w, c.w)))
      normals.push(...(dot(n0, c.d) < 0 ? n0.map((v) => -v) : n0))
      positions.push(...c.d.map((v, k) => v * (R + c.h - (ring > 0.5 ? skirtM : 0))))
      heights.push(c.h)
    }
    const centre = direction(ox + 0.5 * size, oy + 0.5 * size).map((c) => c * R)
    return { radius: R, face, level, x: tx, y: ty, size, skirtM, origin: centre, positions, normals, minElevation: Math.min(...heights), maxElevation: Math.max(...heights) }
  }

  const cases = RADII.flatMap((R, ri) => {
    const sampler = createHeightSampler({ radius: R, seed: SEED, hashVersion: DEFAULT_TERRAIN_HASH_VERSION, anchorField })
    return LEVELS.flatMap((level, li) => Array.from({ length: NODES_PER_LEVEL }, (_, i) => patchCase(R, sampler, NODE_SPECS[(li * NODES_PER_LEVEL + i + ri) % NODE_SPECS.length], level)))
  })

  return {
    'spoint-planet-patch.json': {
      source: 'spoint ' + rev + ': packages/mapspinner/src/patch-grid-geometry.js (buildGridGeometry, run unchanged), the position and normal rules of packages/mapspinner/src/tsl/terrain-material-tsl.js positionNode written out in node (a TSL graph cannot run on the CPU), heights from height-cpu.js createHeightSampler',
      gridSize: GRID_SIZE,
      layout,
      indices: Array.from(grid.indices),
      cases,
    },
  }
}, { indent: 0 })
