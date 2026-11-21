import { writeParity, spointImport } from './lib/spoint.mjs'

const RADIUS = 6360
const CASES = [
  { c: -0.9, A: -5000, B: 0, x: 0, z: 3000, tol: 1e-3 },
  { c: -0.9, A: -5000, B: 0, x: 0, z: 3000, tol: 1e-9 },
  { c: -0.9, A: -5000, B: 40000, x: 0, z: 3000, tol: 1e-9 },
  { c: -0.9, A: -40000, B: 0, x: 0, z: 0, tol: 1e-3 },
]

await writeParity(async ({ spoint, rev, statusBefore }) => {
  const pf = await spointImport(spoint, 'src/terrain/PlanetFrame.js')
  const cases = CASES.map((k) => {
    const h = (d) => (d[0] > k.c ? k.A : k.B)
    const frame = pf.createPlanetFrame({ sampler: { radius: RADIUS, hashVersion: 2, heightAt: h }, anchorDir: [0, 1, 0], offsetY: 0, reliefScale: 0.001 })
    try {
      const y = frame.solveSurfaceY(k.x, k.z, h, k.tol)
      const residual = pf.sphereSurfaceLocalY(frame, k.x, k.z, h(frame.localToDir(k.x, k.z, y))) - y
      return { ...k, anchorHeight: frame.anchorHeight, y, residual, error: null }
    } catch (e) {
      return { ...k, anchorHeight: frame.anchorHeight, y: null, residual: null, error: e.name }
    }
  })
  return {
    'spoint-solve-paths.json': { spoint: rev, radius: RADIUS, anchorDir: [0, 1, 0], sampler: 'split: A when d[0] > c, else B', spointStatusBefore: statusBefore, spointStatusAfter: statusBefore, cases },
  }
}, { indent: 0, status: false })
