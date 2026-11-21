import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-chart-reanchor-policy.json')
const rev = execFileSync('git', ['-C', spoint, 'rev-parse', '--short', 'HEAD']).toString().trim()
const statusBefore = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
const spointModule = (rel) => import(pathToFileURL(join(spoint, rel)).href)
const { createPlanetFrame } = await spointModule('src/terrain/PlanetFrame.js')
const { createChartAnchorLattice, chartAnchorForDir } = await spointModule('src/shared/chartAnchor.js')
const { CHART_REANCHOR_HYSTERESIS_DEG, WALKABLE_CHART_LIMIT_DEG, playerCentroidDir, chartAnchorDecision } = await spointModule('src/shared/chartReanchorPolicy.js')

const RADIUS = 60
const ANCHOR_DIR = [0, 1, 0]
const heightAt = (d) => 0.5 + 0.25 * d[0] - 0.1 * d[2]
const unit = (v) => { const l = Math.hypot(v[0], v[1], v[2]); return v.map((c) => c / l) }
const newFrame = (anchorDir) => createPlanetFrame({ sampler: { radius: RADIUS, hashVersion: 1, heightAt }, anchorDir, offsetY: 0, reliefScale: 1 })
const SPREAD = 100 * Math.PI / 180
const lattice = createChartAnchorLattice(RADIUS)

const CENTROID_CASES = [
  { name: 'empty', dirs: [] },
  { name: 'two-unit-dirs', dirs: [[1, 0, 0], [0, 1, 0]] },
  { name: 'unnormalised-dirs', dirs: [[2, 0, 0], [0, 0.5, 0]] },
  { name: 'straddle', dirs: [[1, 0, 0], [-1, 0, 0]] },
  { name: 'no-length', dirs: [[1, 0, 0], [0, 0, 0]] },
]
const playerCentroid = CENTROID_CASES.map((c) => {
  try {
    const d = playerCentroidDir(c.dirs)
    return { ...c, kind: d === null ? 'none' : 'ok', centroidDir: d }
  } catch (e) {
    return { ...c, kind: 'error', errorName: e.name, reason: e.reason ?? null }
  }
})

const DECISION_CASES = [
  { name: 'no-players', up: ANCHOR_DIR, dirs: [], expect: 'stay:no-players' },
  { name: 'players-straddle', up: ANCHOR_DIR, dirs: [[1, 0, 0], [-1, 0, 0]], expect: 'refusal:players-straddle-the-planet' },
  { name: 'no-length-direction', up: ANCHOR_DIR, dirs: [[0, 0, 0]], expect: 'refusal:player-direction-has-no-length' },
  { name: 'party-spread', up: ANCHOR_DIR, dirs: [[1, 0, 0], [Math.cos(SPREAD), Math.sin(SPREAD), 0]], expect: 'refusal:party-spread-exceeds-walkable-chart' },
  { name: 'anchor-unchanged', up: chartAnchorForDir(lattice, [0, 1, 0.01]), dirs: [[0, 1, 0.01]], expect: 'stay:anchor-unchanged' },
  { name: 'gain-below-hysteresis', up: unit([0.9, 0.3, 0.2]), dirs: [[0, 1, 0]], hysteresisDeg: 90, expect: 'stay:gain-below-hysteresis' },
  { name: 'reanchor-single-player', up: ANCHOR_DIR, dirs: [unit([0.9, 0.3, 0.2])], expect: 'reanchor' },
  { name: 'reanchor-two-players', up: ANCHOR_DIR, dirs: [[0, 1, 0.2], [0.2, 1, 0]], expect: 'reanchor' },
]
const decisions = DECISION_CASES.map((c) => {
  const frame = newFrame(c.up)
  const hysteresisDeg = c.hysteresisDeg ?? CHART_REANCHOR_HYSTERESIS_DEG
  const r = chartAnchorDecision({ lattice, frame, dirs: c.dirs, hysteresisDeg, walkableLimitDeg: WALKABLE_CHART_LIMIT_DEG })
  const got = r.kind === 'reanchor' ? 'reanchor' : r.kind + ':' + r.reason
  if (got !== c.expect) throw new Error(c.name + ': got ' + got + ' expected ' + c.expect)
  return { name: c.name, up: c.up, dirs: c.dirs, hysteresisDeg, walkableLimitDeg: WALKABLE_CHART_LIMIT_DEG, result: r }
})

const statusAfter = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
if (statusAfter !== statusBefore) throw new Error('spoint working tree changed')
writeFileSync(outPath, JSON.stringify({
  spoint: rev, planetRadius: RADIUS,
  hysteresisDeg: CHART_REANCHOR_HYSTERESIS_DEG, walkableLimitDeg: WALKABLE_CHART_LIMIT_DEG,
  playerCentroid, decisions,
  spointStatusBefore: statusBefore, spointStatusAfter: statusAfter,
}, null, 1) + '\n')
console.log(JSON.stringify({ centroid: playerCentroid.map((c) => [c.name, c.kind]), decisions: decisions.map((d) => [d.name, d.result.kind, d.result.reason ?? null]) }))
