import { writeParity, spointImport } from './lib/spoint.mjs'

await writeParity(async ({ spoint, rev }) => {
  const { seaLevelDirOf, seaLevelXZOfDir, reanchoredSeaLevelXZ } = await spointImport(spoint, 'src/terrain/ChartLocalPoint.js')
  const { anchorBasis } = await spointImport(spoint, 'src/terrain/PlanetFrame.js')

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

  return { 'spoint-chart-local-point.json': { spoint: rev, from: FROM, to: TO, grid, dirs } }
})
