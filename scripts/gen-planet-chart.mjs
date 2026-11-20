import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-planet-chart.json')
const rev = execFileSync('git', ['-C', spoint, 'rev-parse', '--short', 'HEAD']).toString().trim()
const statusBefore = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
const pf = await import(pathToFileURL(join(spoint, 'src/terrain/PlanetFrame.js')).href)

const RADIUS = 6360
const OFFSET_Y = 5
const ANCHOR = [0.6, 0, 0.8]
const smooth = (d) => 40 + 25 * d[0] - 15 * d[1] + 10 * d[2] * d[2]
const frame = pf.createPlanetFrame({ sampler: { radius: RADIUS, hashVersion: 2, heightAt: smooth }, anchorDir: ANCHOR, offsetY: OFFSET_Y, reliefScale: 0.001 })

const localToWorld = [[0, 0, 0], [100, 5, -50], [-2000, -30, 1500]].map(([x, y, z]) => ({ x, y, z, out: frame.localToWorld(x, y, z) }))
const localToDir = [[100, 50, null], [100, 50, 20], [-2000, 1500, -30]].map(([x, z, y]) => ({ x, z, y, out: frame.localToDir(x, z, y ?? undefined) }))
const groundHeightLocal = [[0, 0], [100, 50], [-300, 200], [2000, -1500]].map(([x, z]) => {
  try {
    return { x, z, y: frame.groundHeightLocal(x, z), error: null }
  } catch (e) {
    return { x, z, y: null, error: e.constructor.name }
  }
})

const statusAfter = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
if (statusAfter !== statusBefore) throw new Error('spoint working tree changed')
writeFileSync(outPath, JSON.stringify({ spoint: rev, module: 'src/terrain/PlanetFrame.js', radius: RADIUS, offsetY: OFFSET_Y, anchorDir: ANCHOR, sampler: 'smooth: 40 + 25 x - 15 y + 10 z z', anchorHeight: frame.anchorHeight, anchorSurfaceWorld: frame.anchorSurfaceWorld, localToWorld, localToDir, groundHeightLocal, spointStatusBefore: statusBefore, spointStatusAfter: statusAfter }) + '\n')
console.log(JSON.stringify({ spoint: rev, anchorHeight: frame.anchorHeight, groundHeightLocal: groundHeightLocal.map((c) => [c.x, c.z, c.y, c.error]) }))
