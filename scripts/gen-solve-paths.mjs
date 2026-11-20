import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-solve-paths.json')
const rev = execFileSync('git', ['-C', spoint, 'rev-parse', '--short', 'HEAD']).toString().trim()
const statusBefore = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
const pf = await import(pathToFileURL(join(spoint, 'src/terrain/PlanetFrame.js')).href)

const RADIUS = 6360
const CASES = [
  { c: -0.9, A: -5000, B: 0, x: 0, z: 3000, tol: 1e-3 },
  { c: -0.9, A: -5000, B: 0, x: 0, z: 3000, tol: 1e-9 },
  { c: -0.9, A: -5000, B: 40000, x: 0, z: 3000, tol: 1e-9 },
  { c: -0.9, A: -40000, B: 0, x: 0, z: 0, tol: 1e-3 },
]
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

const statusAfter = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
if (statusAfter !== statusBefore) throw new Error('spoint working tree changed')
writeFileSync(outPath, JSON.stringify({ spoint: rev, radius: RADIUS, anchorDir: [0, 1, 0], sampler: 'split: A when d[0] > c, else B', spointStatusBefore: statusBefore, spointStatusAfter: statusAfter, cases }) + '\n')
console.log(JSON.stringify({ spoint: rev, cases: cases.map((c) => [c.tol, c.y, c.residual, c.error]) }))
