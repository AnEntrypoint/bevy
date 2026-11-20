import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-placement-lattice.json')
const rev = execFileSync('git', ['-C', spoint, 'rev-parse', '--short', 'HEAD']).toString().trim()
const statusBefore = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
const pl = await import(pathToFileURL(join(spoint, 'src/terrain/PlacementLattice.js')).href)

const enc = (v) => (Number.isNaN(v) ? 'NaN' : v)
const unit = (v) => { const l = Math.hypot(v[0], v[1], v[2]); return [v[0] / l, v[1] / l, v[2] / l] }
const faceDirs = [[1, 0, 0], [-1, 0, 0], [0, 1, 0], [0, -1, 0], [0, 0, 1], [0, 0, -1], [1, 1, 0], [1, 1, 1], [0.3, 0.5, 0.8], [-0.2, -0.9, 0.1], [0.7, -0.7, 0.1], [0.1, 0.2, -0.9], [0, 0, 0]]
const ringDirs = [[1, 0, 0], [0.3, 0.5, 0.8], [-0.2, -0.9, 0.1], [0.7, 0.7, 0.7]].map(unit)

const lattices = [[6360, 16, 4], [1000, 8, 2], [6360, 16, 1], [2500, 10, 2.5]].map(([radius, chunkM, cellsPerChunk]) => {
  const L = pl.createPlacementLattice(radius, chunkM, cellsPerChunk)
  const cpf = L.chunksPerFace
  const cpfCells = L.cellsPerFace
  const faces = [0, 1, 2, 3, 4, 5]
  const corners = [[0, 0], [3, 5], [cpf - 1, cpf - 1], [Math.floor(cpf / 2), 2]]
  const chunkKeys = faces.flatMap((face) => corners.map(([ci, cj]) => [face, ci, cj, L.chunkKey(face, ci, cj)]))
  const keys = chunkKeys.map((r) => r[3])
  const cellFuFv = [[0, 0], [1.5, 7], [cpfCells / 2, cpfCells / 3], [cpfCells, cpfCells]]
  return {
    radius, chunkM, cellsPerChunk,
    scalars: { chunksPerFace: L.chunksPerFace, cellsPerFace: L.cellsPerFace, cellAngle: L.cellAngle, chunkAngle: L.chunkAngle, cellM: L.cellM, faceCount: L.faceCount },
    faceOf: faceDirs.map((d) => [d, pl.cubeFaceOf(d[0], d[1], d[2])]),
    cellDir: faces.flatMap((face) => cellFuFv.map(([fu, fv]) => [face, fu, fv, L.cellDir(face, fu, fv, [0, 0, 0])])),
    area: cellFuFv.map(([fu, fv]) => [fu, fv, L.cellAreaOverTarget(fu, fv)]),
    chunkKeys,
    decode: keys.map((key) => [key, L.decodeChunk(key, [0, 0, 0])]),
    keyOfDir: ringDirs.map((d) => [d, L.chunkKeyOfDir(d[0], d[1], d[2])]),
    centreDir: keys.slice(0, 3).map((key) => [key, L.chunkCentreDir(key, [0, 0, 0])]),
    cornerDir: keys.slice(0, 3).flatMap((key) => [[0, 0], [1, 1], [0.5, 0]].map(([cu, cv]) => [key, cu, cv, L.chunkCornerDir(key, cu, cv, [0, 0, 0])])),
    cellIds: faces.flatMap((face) => [[0, 0], [1, 2], [cpfCells - 1, cpfCells - 1]].map(([i, j]) => [face, i, j, L.cellId(face, i, j)])),
    hashRows: faces.flatMap((face) => [0, 7, cpfCells - 1].map((i) => [face, i, L.hashRow(face, i)])),
    rings: ringDirs.flatMap((d) => [200, 60].map((radiusM) => [d, radiusM, L.ringAroundDir(d[0], d[1], d[2], radiusM)])),
  }
})

const errors = [[0, 16, 4], [6360, 0, 4], [6360, 16, 0.5], [NaN, 16, 4]].map(([radius, chunkM, cellsPerChunk]) => {
  try {
    pl.createPlacementLattice(radius, chunkM, cellsPerChunk)
    return null
  } catch (e) {
    return { radius: enc(radius), chunkM, cellsPerChunk, message: e.message }
  }
}).filter(Boolean)

const statusAfter = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
if (statusAfter !== statusBefore) throw new Error('spoint working tree changed')
writeFileSync(outPath, JSON.stringify({ spoint: rev, lattices, errors, spointStatusBefore: statusBefore, spointStatusAfter: statusAfter }, null, 1) + String.fromCharCode(10))
console.log(JSON.stringify({ spoint: rev, lattices: lattices.length, rings: lattices.map((l) => l.rings.map((r) => r[2].length)), errors: errors.length }))
