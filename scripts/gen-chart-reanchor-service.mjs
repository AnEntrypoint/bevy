import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-chart-reanchor-service.json')
const rev = execFileSync('git', ['-C', spoint, 'rev-parse', '--short', 'HEAD']).toString().trim()
const statusBefore = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
const spointModule = (rel) => import(pathToFileURL(join(spoint, rel)).href)
const { createPlanetFrame } = await spointModule('src/terrain/PlanetFrame.js')
const { createChartReanchorService } = await spointModule('src/terrain/ChartReanchorService.js')
const { CHART_ANCHORS_PER_FACE } = await spointModule('src/shared/chartAnchor.js')
const { CHART_REANCHOR_HYSTERESIS_DEG, WALKABLE_CHART_LIMIT_DEG } = await spointModule('src/shared/chartReanchorPolicy.js')

const RADIUS = 60
const ANCHOR_DIR = [0, 1, 0]
const heightAt = (d) => 0.5 + 0.25 * d[0] - 0.1 * d[2]
const newFrame = () => createPlanetFrame({ sampler: { radius: RADIUS, hashVersion: 1, heightAt }, anchorDir: ANCHOR_DIR, offsetY: 0, reliefScale: 1 })
const unit = (v) => { const l = Math.hypot(v[0], v[1], v[2]); return v.map((c) => c / l) }
const snap = (f) => ({ radius: f.radius, offsetY: f.offsetY, anchorHeight: f.anchorHeight, chartEpoch: f.chartEpoch, east: [...f.east], up: [...f.up], north: [...f.north] })
const SPREAD_RAD = 100 * Math.PI / 180
const GAIN_GATE_DEG = 100

const frame = newFrame()
const initial = snap(frame)
let players = []
const migrated = []
const terrainMigrated = []
const service = createChartReanchorService({ frame, radius: RADIUS, playerDirs: () => players })
service.onReanchor((event) => migrated.push(event.epoch))
service.addTerrainMigrator({
  gate: ({ decision }) => (decision.kind === 'reanchor' && decision.gainDeg > GAIN_GATE_DEG ? 'terrain-stream-busy' : null),
  migrate: (event) => terrainMigrated.push(event.epoch),
})

const eventJson = (event) => ({
  kind: 'reanchor',
  epoch: event.epoch,
  anchorDir: [...event.anchorDir],
  angleBeforeDeg: event.angleBeforeDeg,
  angleAfterDeg: event.angleAfterDeg,
  tiltRad: event.transfer.tiltRad,
  decisionKind: event.decision.kind,
  centroidDir: [...event.decision.centroidDir],
  gainDeg: event.decision.gainDeg,
  from: snap(event.from),
  to: snap(event.to),
})

const STEPS = [
  { name: 'stay-near-anchor', dirs: [[0, 1, 0.01]] },
  { name: 'refuse-party-spread', dirs: [[1, 0, 0], [Math.cos(SPREAD_RAD), Math.sin(SPREAD_RAD), 0]] },
  { name: 'refuse-players-straddle', dirs: [[1, 0, 0], [-1, 0, 0]] },
  { name: 'refuse-no-length', dirs: [[0, 0, 0]] },
  { name: 'reanchor-far', dirs: [unit([0.9, 0.3, 0.2])] },
  { name: 'stay-after-reanchor', dirs: [unit([0.9, 0.3, 0.2])] },
  { name: 'refuse-terrain-gate', dirs: [unit([-0.6, 0.2, 0.77])] },
  { name: 'reanchor-moderate', dirs: [unit([0.5, 0.5, 0.7])] },
]
const records = STEPS.map((step) => {
  players = step.dirs
  const refusalsBefore = service.refusalCount
  const result = service.step()
  const lastRefusal = service.refusalCount > refusalsBefore ? service.lastRefusal : null
  return {
    name: step.name,
    dirs: step.dirs,
    result: result ? eventJson(result) : null,
    lastRefusal: lastRefusal ? { reason: lastRefusal.reason, epoch: lastRefusal.epoch, worstPlayerAngleDeg: lastRefusal.decision.worstPlayerAngleDeg ?? null } : null,
    reanchorCount: service.reanchorCount,
    refusalCount: service.refusalCount,
    migratorCount: service.migratorCount,
    migrated: migrated.length,
    terrainMigrated: terrainMigrated.length,
    refusalsByReason: service.refusalsByReason,
  }
})

const bare = createChartReanchorService({ frame: newFrame(), radius: RADIUS, playerDirs: () => [[0, 1, 0]] })
let bareError = null
try { bare.step() } catch (error) { bareError = error.message }

const expectKind = (name, want) => { const r = records.find((x) => x.name === name); if (!r) throw new Error(name); if (want === null ? r.result !== null || r.lastRefusal !== null : (want === 'reanchor' ? r.result === null : r.lastRefusal?.reason !== want)) throw new Error(`${name} expected ${want}, got ${JSON.stringify(r)}`) }
expectKind('stay-near-anchor', null)
expectKind('refuse-party-spread', 'party-spread-exceeds-walkable-chart')
expectKind('refuse-players-straddle', 'players-straddle-the-planet')
expectKind('refuse-no-length', 'player-direction-has-no-length')
expectKind('reanchor-far', 'reanchor')
expectKind('stay-after-reanchor', null)
expectKind('refuse-terrain-gate', 'terrain-stream-busy')
expectKind('reanchor-moderate', 'reanchor')
if (!bareError || !bareError.includes('no registered state migrator')) throw new Error('bare service did not throw')

const statusAfter = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
if (statusAfter !== statusBefore) throw new Error('spoint working tree changed')
writeFileSync(outPath, JSON.stringify({ spoint: rev, radius: RADIUS, anchorDir: ANCHOR_DIR, offsetY: 0, anchorsPerFace: CHART_ANCHORS_PER_FACE, hysteresisDeg: CHART_REANCHOR_HYSTERESIS_DEG, walkableLimitDeg: WALKABLE_CHART_LIMIT_DEG, gainGateDeg: GAIN_GATE_DEG, heightProbe: [[0, 1, 0], [0.9, 0.3, 0.2], [-0.6, 0.2, 0.77]].map((d) => heightAt(unit(d))), initial, steps: records, bareError, spointStatusBefore: statusBefore, spointStatusAfter: statusAfter }, null, 1) + '\n')
console.log(JSON.stringify(records.map((r) => [r.name, r.result ? r.result.epoch : null, r.lastRefusal ? r.lastRefusal.reason : null, r.reanchorCount, r.refusalCount, r.migrated, r.terrainMigrated]), null, 0))
console.log(JSON.stringify({ bareError, refusalsByReason: records[records.length - 1].refusalsByReason, walkableLimitDeg: WALKABLE_CHART_LIMIT_DEG }))
