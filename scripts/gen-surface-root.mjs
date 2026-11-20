import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-surface-root.json')
const rev = execFileSync('git', ['-C', spoint, 'rev-parse', '--short', 'HEAD']).toString().trim()
const statusBefore = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
const pf = await import(pathToFileURL(join(spoint, 'src/terrain/PlanetFrame.js')).href)

const enc = (v) => (Number.isNaN(v) ? 'NaN' : v === Infinity ? 'Infinity' : v === -Infinity ? '-Infinity' : v)
const residuals = [0, 1e-5, -1e-4, 1e-4, 1.0001e-4, NaN, Infinity, -Infinity, 2e-4]
const tolerances = [1e-4, 0, -1, NaN, Infinity, 1e-4]
const surfaceRoot = residuals.flatMap((residualM) => tolerances.map((toleranceM) => ({
  residualM: enc(residualM), toleranceM: enc(toleranceM), within: pf.isSurfaceRootWithin(residualM, toleranceM),
})))
const R = 6360
const dropPairs = [[0, R], [1, R], [40000, R], [R * R, R], [R * R + 1, R], [100, 0.5], [0.25, 0.5], [1e-6, 1e-3], [0, 0], [5, 1]]
const dropBelowTangent = dropPairs.map(([r2, sphereRadius]) => ({ r2, sphereRadius, value: pf.sphereDropBelowTangent(r2, sphereRadius) }))

const statusAfter = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
if (statusAfter !== statusBefore) throw new Error('spoint working tree changed')
writeFileSync(outPath, JSON.stringify({ spoint: rev, surfaceRoot, dropBelowTangent, spointStatusBefore: statusBefore, spointStatusAfter: statusAfter }, null, 1) + String.fromCharCode(10))
console.log(JSON.stringify({ spoint: rev, surfaceRoot: surfaceRoot.length, drop: dropBelowTangent.length, within: surfaceRoot.filter((c) => c.within).length }))
