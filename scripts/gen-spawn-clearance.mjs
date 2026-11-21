import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-spawn-clearance.json')
const rev = execFileSync('git', ['-C', spoint, 'rev-parse', '--short', 'HEAD']).toString().trim()
const statusBefore = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
const spointModule = (rel) => import(pathToFileURL(join(spoint, rel)).href)
const { evaluateSpawnPoint, pickClearSpawnPose } = await spointModule('apps/tps-game/respawn-clearance.js')
const { capsuleRadiusM, capsuleHeightM } = await spointModule('src/apps/AppGameplay.js')
const { DEFAULT_HITBOX } = await spointModule('src/netcode/Hitscan.js')

const MIN_SAFE_DISTANCE_M = 25
const xzKey = (x, z) => x + ',' + z

const CANDIDATES = [
  { name: 'open-a', sp: [0, 10, 0], terrainY: 10, feetY: 10, footprint: [] },
  { name: 'open-b', sp: [40, 10, 0], terrainY: 10, feetY: 10, footprint: [] },
  { name: 'open-c', sp: [-40, 10, 0], terrainY: 10, feetY: 10, footprint: [] },
  { name: 'open-d', sp: [10, 10, 10], terrainY: 10, feetY: 10, footprint: [] },
  { name: 'intruding-a', sp: [5, 10, 5], terrainY: 10, feetY: 10, footprint: ['intruding'] },
  { name: 'void-a', sp: [-5, 10, 5], terrainY: 10, feetY: 10, footprint: ['void'] },
  { name: 'two-blockers', sp: [-70, 10, -70], terrainY: 10, feetY: 10, footprint: ['intruding', 'void'] },
  { name: 'no-surface', sp: [60, 10, 60], terrainY: 10, feetY: null, footprint: [] },
  { name: 'mismatch-high', sp: [70, 10, 0], terrainY: 10, feetY: 13, footprint: [] },
  { name: 'mismatch-low', sp: [80, 10, 0], terrainY: 10, feetY: 6, footprint: [] },
  { name: 'lifted-ok', sp: [90, 10, 0], terrainY: 10, feetY: 9.5, footprint: [] },
]
const byName = new Map(CANDIDATES.map((c) => [c.name, c]))
const byXZ = new Map(CANDIDATES.map((c) => [xzKey(c.sp[0], c.sp[2]), c]))
const at = (x, z) => {
  const c = byXZ.get(xzKey(x, z))
  if (!c) throw new Error('no scene candidate at ' + xzKey(x, z))
  return c
}
const footprintFor = (c) => c.footprint.map((kind) => kind === 'intruding'
  ? { kind, x: c.sp[0] + 0.5, z: c.sp[2], offsetM: 0.5, surfaceY: c.feetY + 0.3, intrusionM: 0.3 }
  : { kind, x: c.sp[0] - 0.5, z: c.sp[2], offsetM: 0.5, surfaceY: null, intrusionM: null })

const PLAYERS = {
  near: { id: 1, state: { position: [0.5, 10, 0] } },
  high: { id: 2, state: { position: [0, 13, 0] } },
  stateless: { id: 3, state: null },
  far: { id: 4, state: { position: [-100, 10, 100] } },
}
const ALL_PLAYERS = ['near', 'high', 'stateless', 'far']
const playerList = (keys) => keys.map((k) => PLAYERS[k])
const playerRecord = (p) => ({ id: p.id, position: p.state ? p.state.position : null })

const scene = (players) => ({
  spawnGeometry: {
    DEFAULT_HITBOX,
    capsuleRadiusM,
    capsuleHeightM,
    spawnSurfaceY: (sp) => at(sp[0], sp[2]).feetY,
    footprintBlockers: (sp) => footprintFor(at(sp[0], sp[2])),
  },
  terrainHeightAt: (x, z) => at(x, z).terrainY,
  players: { getAll: () => players },
})

const EVAL_VARIANTS = [{ suffix: '', exclude: [] }, { suffix: '-exclude-near', exclude: [1] }]
const blockerRecord = (b) => (b.kind === 'player' ? { kind: b.kind, playerId: b.playerId } : { kind: b.kind })
const evaluations = CANDIDATES.flatMap((c) => EVAL_VARIANTS.map((v) => {
  const r = evaluateSpawnPoint(scene(playerList(ALL_PLAYERS)), c.sp, DEFAULT_HITBOX, (p) => v.exclude.includes(p.id))
  return { name: c.name + v.suffix, sp: c.sp, excludeIds: v.exclude, feetY: r.feetY, pose: r.pose, blockers: r.blockers.map(blockerRecord) }
}))

const PICK_CASES = [
  { name: 'first-clear-no-players', spawn: ['open-a', 'open-b', 'open-c'], players: [], exclude: [] },
  { name: 'intruding-skipped', spawn: ['intruding-a', 'open-b'], players: [], exclude: [] },
  { name: 'void-skipped', spawn: ['void-a', 'open-c'], players: [], exclude: [] },
  { name: 'least-blocked-strict-minimum', spawn: ['two-blockers', 'void-a', 'no-surface'], players: [], exclude: [] },
  { name: 'no-surface-pose-unlifted', spawn: ['no-surface'], players: [], exclude: [] },
  { name: 'surface-mismatch-first-of-tie', spawn: ['mismatch-high', 'mismatch-low'], players: [], exclude: [] },
  { name: 'lifted-surface-clear', spawn: ['lifted-ok', 'mismatch-low'], players: [], exclude: [] },
  { name: 'safe-pool-skips-near-spawn', spawn: ['open-a', 'open-b', 'open-c'], players: ['near', 'far'], exclude: [] },
  { name: 'unsafe-pool-sorted-by-nearest', spawn: ['open-a', 'intruding-a', 'void-a', 'open-d'], players: ['near'], exclude: [] },
  { name: 'unsafe-pool-all-blocked', spawn: ['open-a', 'intruding-a', 'void-a'], players: ['near'], exclude: [] },
  { name: 'min-safe-distance-keeps-input-order', spawn: ['intruding-a', 'void-a', 'open-a'], players: ['near'], exclude: [], minSafeDistanceM: 5 },
  { name: 'player-overlap-skips-spawn', spawn: ['open-a', 'open-c'], players: ['near'], exclude: [], minSafeDistanceM: 0 },
  { name: 'exclude-drops-player-from-ordering', spawn: ['open-a', 'open-b'], players: ['near'], exclude: [1] },
  { name: 'stateless-player-ignored', spawn: ['open-a', 'open-b'], players: ['stateless'], exclude: [] },
  { name: 'player-above-height-does-not-block', spawn: ['open-a'], players: ['high'], exclude: [] },
  { name: 'refusal-no-spawn-points', spawn: [], players: [], exclude: [] },
  { name: 'refusal-no-finite-spawn-point', spawn: [[NaN, 10, 0]], players: [], exclude: [] },
]

const runPick = (c) => {
  const spawn = c.spawn.map((s) => (typeof s === 'string' ? byName.get(s).sp : s))
  const players = playerList(c.players)
  const minSafeDistance = c.minSafeDistanceM ?? MIN_SAFE_DISTANCE_M
  const original = console.error
  let logged = null
  console.error = (message) => { logged = message }
  let result
  try {
    const pose = pickClearSpawnPose(scene(players), spawn, { exclude: (p) => c.exclude.includes(p.id), minSafeDistance, hitbox: DEFAULT_HITBOX })
    result = { kind: 'pose', pose, allBlocked: logged !== null }
  } catch (e) {
    result = { kind: 'refusal', errorName: e.name, error: e.message }
  } finally {
    console.error = original
  }
  return { name: c.name, spawn, players: players.map(playerRecord), excludeIds: c.exclude, minSafeDistanceM: minSafeDistance, result }
}
const picks = PICK_CASES.map(runPick)

const statusAfter = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
if (statusAfter !== statusBefore) throw new Error('spoint working tree changed')
const replacer = (_, v) => (typeof v === 'number' && !Number.isFinite(v) ? String(v) : v)
writeFileSync(outPath, JSON.stringify({
  spoint: rev,
  hitbox: DEFAULT_HITBOX,
  respawnLiftM: evaluations[0].pose[1] - evaluations[0].feetY,
  players: ALL_PLAYERS.map((k) => playerRecord(PLAYERS[k])),
  scene: CANDIDATES.map((c) => ({ name: c.name, sp: c.sp, terrainY: c.terrainY, feetY: c.feetY, footprint: c.footprint })),
  evaluations,
  picks,
  spointStatusBefore: statusBefore,
  spointStatusAfter: statusAfter,
}, replacer, 1) + '\n')
console.log(JSON.stringify({ evaluations: evaluations.length, picks: picks.map((p) => [p.name, p.result.kind, p.result.allBlocked ?? p.result.error]) }))
