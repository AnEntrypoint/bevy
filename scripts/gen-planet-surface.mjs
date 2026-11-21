import { writeParity, spointImport } from './lib/spoint.mjs'

const RADIUS = 6360
const smooth = (d) => 40 + 25 * d[0] - 15 * d[1] + 10 * d[2] * d[2]
const KINDS = {
  smooth,
  step: (d) => 40 + 60 * (d[0] > 0.3 ? 1 : 0),
  nullish: (d) => (d[0] > 0.9 ? null : smooth(d)),
  null: () => null,
  zigzag: (d) => 40 + 50 * (Math.floor(d[2] * 2000) & 1),
}
const CASES = [
  { x: 0, z: 0, kind: 'smooth', tol: 1e-3, guess: null },
  { x: 100, z: 50, kind: 'smooth', tol: 1e-3, guess: null },
  { x: -300, z: 200, kind: 'smooth', tol: 1e-4, guess: null },
  { x: 2000, z: -1500, kind: 'smooth', tol: 1e-4, guess: 5 },
  { x: 1e-3, z: 1e-3, kind: 'smooth', tol: 1e-4, guess: null },
  { x: -5000, z: 3000, kind: 'smooth', tol: 1e-3, guess: null },
  { x: 6400, z: 0, kind: 'smooth', tol: 1e-3, guess: null },
  { x: 0, z: 0, kind: 'smooth', tol: 0, guess: null },
  { x: 0, z: 0, kind: 'smooth', tol: -1, guess: null },
  { x: 5500, z: 0, kind: 'nullish', tol: 1e-3, guess: null },
  { x: 0, z: 0, kind: 'step', tol: 1e-3, guess: null },
  { x: 3000, z: 0, kind: 'step', tol: 1e-6, guess: null },
  { x: 6300, z: 0, kind: 'nullish', tol: 1e-3, guess: null },
  { x: 6300, z: 0, kind: 'nullish', tol: 1e-3, guess: -6000 },
  { x: 1, z: 1, kind: 'null', tol: 1e-3, guess: null },
  { x: 3000, z: 0, kind: 'step', tol: 1e-12, guess: null },
  { x: 1500, z: 0, kind: 'step', tol: 1e-12, guess: null },
  { x: 3000, z: 0, kind: 'step', tol: 1e-14, guess: 60 },
  { x: 100, z: 50, kind: 'zigzag', tol: 1e-3, guess: null },
  { x: 3000, z: 2000, kind: 'zigzag', tol: 1e-3, guess: null },
  { x: 0, z: 0, kind: 'zigzag', tol: 1e-6, guess: null },
  { x: -4000, z: 1000, kind: 'zigzag', tol: 1e-6, guess: null },
  { x: 100, z: 50, kind: 'smooth', tol: 1e-300, guess: null },
  { x: 3000, z: 0, kind: 'step', tol: 1e-300, guess: null },
]

await writeParity(async ({ spoint, rev, statusBefore }) => {
  const pf = await spointImport(spoint, 'src/terrain/PlanetFrame.js')
  const sampler = { radius: RADIUS, hashVersion: 2, heightAt: smooth }
  const frame = pf.createPlanetFrame({ sampler, anchorDir: [0, 1, 0], offsetY: 0, reliefScale: 0.001 })
  const cases = CASES.map((c) => {
    try {
      const y = frame.solveSurfaceY(c.x, c.z, KINDS[c.kind], c.tol, c.guess ?? undefined)
      return { ...c, y, error: null }
    } catch (e) {
      return { ...c, y: null, error: e.constructor.name }
    }
  })
  return {
    'spoint-planet-surface.json': { spoint: rev, module: 'src/terrain/PlanetFrame.js', radius: RADIUS, anchorDir: [0, 1, 0], sampler: 'smooth: 40 + 25 x - 15 y + 10 z z', anchorHeight: frame.anchorHeight, kinds: 'null: always null, zigzag: 40 + 50 (floor(z 2000) odd), smooth, step: 40 + 60 (x > 0.3), nullish: null when x > 0.9 else smooth', spointStatusBefore: statusBefore, spointStatusAfter: statusBefore, cases },
  }
}, { indent: 0, status: false })
