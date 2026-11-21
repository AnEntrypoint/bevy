import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-climate-cache.json')
const rev = execFileSync('git', ['-C', spoint, 'rev-parse', '--short', 'HEAD']).toString().trim()
const statusBefore = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
const { createCachedAnchorField, SECTOR_M } = await import(pathToFileURL(join(spoint, 'src/terrain/ClimateCache.js')).href)

const RADIUS = 6360
const unit = (v) => { const l = Math.hypot(v[0], v[1], v[2]); return v.map((c) => c / l) }
const frame = {
  radius: RADIUS,
  localToDir: (x, z) => unit([x / RADIUS, 1, z / RADIUS]),
}

let calls = 0
const anchorField = {
  sampleDir(dir) {
    calls++
    if (dir[0] < -0.02) return null
    return { temp: 15 + 20 * dir[1], humidity: 0.5 + 0.3 * dir[2], erosion: 0.2 + 0.1 * dir[0], seaBias: dir[0] * dir[1] }
  },
}

const cached = createCachedAnchorField(anchorField, frame)
const plan = [
  { x: 0, z: 0 },
  { x: 0, z: 0 },
  { x: 0.5, z: 0.25 },
  { x: 3, z: -2 },
  { x: 100, z: -40 },
  { x: 100, z: -40 },
  { x: -300, z: 200 },
  { x: -300, z: 200 },
  { x: 0, z: 0, dir: [0, 0, 1] },
  { x: 0.5, z: 0.25 },
  { x: -1200, z: 900 },
  { x: 2000, z: 1500, dir: [0.3, 0.5, 0.8] },
]
const queries = plan.map((q) => {
  const climate = q.dir ? cached.climateAtLocal(q.x, q.z, q.dir) : cached.climateAtLocal(q.x, q.z)
  return { x: q.x, z: q.z, dir: q.dir ?? null, climate, size: cached.size, calls }
})

const statusAfter = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
if (statusAfter !== statusBefore) throw new Error('spoint working tree changed')
writeFileSync(outPath, JSON.stringify({ spoint: rev, sectorM: SECTOR_M, radius: RADIUS, queries, callsTotal: calls, spointStatusBefore: statusBefore, spointStatusAfter: statusAfter }, null, 1) + String.fromCharCode(10))
console.log(JSON.stringify({ spoint: rev, sectorM: SECTOR_M, queries: queries.length, sizes: queries.map((q) => q.size), calls: queries.map((q) => q.calls) }))
