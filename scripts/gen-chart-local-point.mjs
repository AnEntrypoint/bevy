import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const rev = execFileSync('git', ['-C', spoint, 'rev-parse', '--short', 'HEAD']).toString().trim()
const statusBefore = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
const spointModule = (rel) => import(pathToFileURL(join(spoint, rel)).href)
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-chart-local-point.json')
const { seaLevelDirOf, seaLevelXZOfDir, reanchoredSeaLevelXZ } = await spointModule('src/terrain/ChartLocalPoint.js')
const { anchorBasis } = await spointModule('src/terrain/PlanetFrame.js')

const chartOf = (radius, anchorDir) => {
  const { up, east, north } = anchorBasis(anchorDir)
  return { radius, east: [...east], up: [...up], north: [...north] }
}
const FROM = chartOf(6360, [0, 1, 0])
const TO = chartOf(5000, [0.36, 0.8, 0.48])
const GRID = [-2500, -1250, 0, 1250, 2500]
const points = GRID.flatMap((x) => GRID.map((z) => ({ x, z })))
const grid = points.map(({ x, z }) => ({
  x,
  z,
  dir: seaLevelDirOf(FROM, x, z),
  xz: seaLevelXZOfDir(FROM, seaLevelDirOf(FROM, x, z)),
  reanchoredXZ: reanchoredSeaLevelXZ(FROM, TO, x, z),
}))
const DIRS = [[1, 0, 0], [0, 1, 0], [0, 0, -1], [0.3, -0.4, 0.8660254037844386]]
const dirs = DIRS.map((dir) => ({ dir, fromXZ: seaLevelXZOfDir(FROM, dir), toXZ: seaLevelXZOfDir(TO, dir) }))

const statusAfter = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
if (statusAfter !== statusBefore) throw new Error('spoint working tree changed')
writeFileSync(outPath, JSON.stringify({ spoint: rev, from: FROM, to: TO, grid, dirs, spointStatusBefore: statusBefore, spointStatusAfter: statusAfter }, null, 1) + '\n')
console.log(JSON.stringify({ rows: grid.length, dirs: dirs.length, sample: grid[12] }))
