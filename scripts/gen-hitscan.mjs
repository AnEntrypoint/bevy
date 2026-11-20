import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-hitscan.json')
const rev = execFileSync('git', ['-C', spoint, 'rev-parse', '--short', 'HEAD']).toString().trim()
const statusBefore = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
const hs = await import(pathToFileURL(join(spoint, 'src/netcode/Hitscan.js')).href)
const shared = await import(pathToFileURL(join(spoint, 'apps/tps-game/shared.js')).href)

const RANGE = 100
shared.bindCombat({
  normalizeShotDirection: hs.normalizeShotDirection,
  rayVsCapsule: hs.rayVsCapsule,
  hitHeightRatio: hs.hitHeightRatio,
  DEFAULT_HITBOX: hs.DEFAULT_HITBOX,
  DEFAULT_COMBAT: { range: RANGE, headshotZone: 0.75 },
  readHealth: (p) => ({ known: p.hp !== undefined, hp: p.hp }),
})
const attempt = (fn) => { try { return { out: fn(), error: null } } catch (e) { return { out: null, error: e.name } } }
const players = [
  { id: 'self', position: [0, 1.7, 5], hp: 100 },
  { id: 'dead', position: [0, 0.8, 10], hp: 0 },
  { id: 'side', position: [2, 0.8, 20], hp: 100 },
  { id: 'near', position: [0.1, 0.8, 20], hp: 100 },
  { id: 'far', position: [0, 0.8, 200], hp: 100 },
]
const direct = [
  { origin: [0, 0, 0], direction: [1, 1, 0], range: 10, tp: [3, 0.1, 0] },
  { origin: [0, 1.7, 0], direction: [0, 0, 1], range: 100, tp: [0.1, 0.8, 20] },
  { origin: [0, 1.7, 0], direction: [0, 0, 1], range: 100, tp: [2, 0.8, 20] },
  { origin: [0, 1.7, 0], direction: [0, 0, 1], range: 5, tp: [0.1, 0.8, 20] },
  { origin: [0, 1.7, 0], direction: [0, 0, 0], range: 100, tp: [0.1, 0.8, 20] },
].map((c) => {
  const r = attempt(() => { const h = hs.rayVsCapsule(c.origin, c.direction, c.range, c.tp, hs.DEFAULT_HITBOX); return h ? { proj: h.proj, dot: h.dot } : null })
  return { ...c, ...r }
})
const predict = [
  { origin: [0, 1.7, 0], direction: [0, 0, 1], selfId: 'self', zone: 0.75 },
  { origin: [0, 1.7, 0], direction: [0, 0, 1], selfId: 'self', zone: 0.4 },
  { origin: [0, 1.7, 0], direction: [0, 0, 1], selfId: 'near', zone: 0.4 },
  { origin: [0, 1.7, 0], direction: [0, 0, 0], selfId: 'self', zone: 0.75 },
  { origin: [0, 1.7, 0], direction: [1, 0, 0], selfId: 'self', zone: 0.75 },
].map((c) => ({ ...c, ...attempt(() => shared.predictHit(c.origin, c.direction, players, c.selfId, c.zone)) }))

const statusAfter = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
if (statusAfter !== statusBefore) throw new Error('spoint working tree changed')
writeFileSync(outPath, JSON.stringify({ spoint: rev, range: RANGE, players, direct, predict, spointStatusBefore: statusBefore, spointStatusAfter: statusAfter }, null, 1) + '\n')
console.log(JSON.stringify({ spoint: rev, direct: direct.map((c) => [c.out && c.out.dot, c.error]), predict: predict.map((c) => [c.out, c.error]) }))
