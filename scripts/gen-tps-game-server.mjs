import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-tps-game-server.json')
const git = (args) => execFileSync('git', ['-C', spoint, ...args]).toString()
const rev = git(['rev-parse', '--short', 'HEAD']).trim()
const statusBefore = git(['status', '--porcelain'])
const spointModule = (rel) => import(pathToFileURL(join(spoint, rel)).href)
const { tpsGameServer } = await spointModule('apps/tps-game/server-app.js')
const { stampSender, senderOf } = await spointModule('src/sdk/appEventSender.js')
const { EMOTE_CLIPS, POWERUP_DEFS, POWERUP_RESPAWN_MS } = await spointModule('apps/tps-game/shared.js')
const { defineCombat, DEFAULT_COMBAT } = await spointModule('src/behaviours/combat.js')

let clock = 0
Date.now = () => clock

const clone = (v) => JSON.parse(JSON.stringify(v))
const s = (at, sender, code) => ({ at, sender, code })
const EMOTE_CODES = [...EMOTE_CLIPS.keys()]
const EMOTE_SCENARIOS = [
  { name: 'every-named-emote', steps: EMOTE_CODES.map((code, i) => s(1000 * (i + 1), 1, code)) },
  { name: 'rejected-codes-leave-the-limit-open', steps: [s(5000, 1, 'Wave'), s(5000, 1, 'wave '), s(5000, 1, ''), s(5000, 1, 'constructor'), s(5000, 1, '__proto__'), s(5000, 1, 'toString'), s(5000, 1, 5), s(5000, 1, null), s(5000, 1, ['wave']), s(5000, 1, 'wave')] },
  { name: 'rejected-senders', steps: [s(3000, null, 'wave'), s(3000, 0, 'wave'), s(3000, -3, 'wave'), s(3000, 1.5, 'wave'), s(3000, '1', 'wave'), s(3000, 'abc', 'wave'), s(3000, 2, 'wave')] },
  { name: 'rate-limit-window', steps: [s(1000, 1, 'wave'), s(1500, 1, 'dance'), s(1700, 1, 'nod'), s(1799, 1, 'victory'), s(1800, 1, 'meditate'), s(2000, 1, 'sit'), s(2599, 1, 'confused'), s(2600, 1, 'jumpingjacks')] },
  { name: 'first-emote-near-clock-start', steps: [s(0, 1, 'wave'), s(500, 1, 'dance'), s(800, 1, 'nod'), s(1000, 1, 'victory')] },
  { name: 'limit-is-per-player', steps: [s(1000, 1, 'wave'), s(1000, 2, 'dance'), s(1400, 1, 'nod'), s(1400, 2, 'nod'), s(1200, 3, 'sit'), s(1800, 2, 'nod')] },
]

const emoteScenarios = EMOTE_SCENARIOS.map(({ name, steps }) => {
  const calls = []
  const ctx = {
    state: { lastEmoteAt: new Map() },
    senderOf: (message) => senderOf(message),
    players: { playAnimation: (playerId, clip, opts) => calls.push({ playerId, clip, loop: opts.loop }) },
  }
  return {
    name,
    steps: steps.map(({ at, sender, code }) => {
      clock = at
      const msg = { type: 'emote', code }
      if (sender !== null) stampSender(msg, { id: sender, fromClient: true })
      calls.length = 0
      tpsGameServer.onMessage(ctx, msg)
      return { at, sender, code, played: calls.map((c) => ({ ...c })) }
    }),
  }
})

const runScoreboard = ({ name, snapshot, events }) => {
  const players = new Map()
  const writes = []
  const captured = []
  const traceValues = snapshot === 'every'
  let store = null
  let nextHit = null
  let nextRatio = 0
  const persisted = (key, initial) => {
    store = {
      value: clone(initial),
      ready: Promise.resolve(),
      save() { writes.push(traceValues ? { op: 'save', key, value: clone(store.value) } : { op: 'save', key }) },
      flush() { writes.push({ op: 'flush', key }); return Promise.resolve() },
    }
    return store
  }
  const ctx = {
    persisted,
    players: {
      getById: (id) => players.get(id) ?? null,
      getAll: () => [...players.values()],
      send: () => {},
      setPosition: () => {},
    },
    network: { broadcast: () => {} },
    world: { spawn: () => {}, destroy: () => {}, sendToEntity: () => {} },
    raycast: () => ({ hit: false }),
    terrainBodyId: 0,
    terrainHeightAt: () => 0,
    fallFloorY: () => 0,
    pickSpawnPoint: (points) => points[0],
    combat: {
      normalizeShotDirection: (d) => d,
      resolveFireRequest: (_lag, _id, pos) => ({ origin: pos, viewTick: null }),
      buildLiveIndex: () => null,
      findHitSpatial: () => nextHit,
      hitHeightRatio: () => nextRatio,
      DEFAULT_HITBOX: { radiusSq: 1, centerHeight: 1 },
      recordHit: () => {},
      DEFAULT_COMBAT,
    },
    lagCompensator: null,
    time: { tick: 1 },
  }
  const spec = {
    powerups: POWERUP_DEFS,
    powerupRespawnMs: POWERUP_RESPAWN_MS,
    scoreboardKey: 'scoreboard',
    config: DEFAULT_COMBAT,
    spawnPoints: [[0, 3, 0]],
    onHit: (_c, info) => captured.push({ handler: 'onHit', shooterId: info.shooterId, targetId: info.target.id, damage: info.damage, headshot: info.headshot, health: info.health }),
    onKill: (_c, info) => captured.push({ handler: 'onKill', shooterId: info.shooterId, targetId: info.targetId, damage: info.damage, headshot: info.headshot, streak: info.streak, killerKills: info.stats.kills }),
    onDeath: (_c, info) => captured.push({ handler: 'onDeath', playerId: info.playerId, cause: info.cause }),
  }
  const combat = defineCombat(spec, ctx)
  const sendMessage = (msg, sender, fromClient) => {
    stampSender(msg, { id: sender, fromClient })
    combat.handle(msg)
  }
  const dispatch = {
    join: (ev) => {
      const player = { id: ev.id, name: ev.name ?? undefined, state: { health: 100, position: [0, 0, 0], velocity: [0, 0, 0], onGround: true } }
      if (ev.peer !== null) player.socket = { _peerId: ev.peer }
      players.set(ev.id, player)
      sendMessage({ type: 'player_join' }, ev.id, false)
    },
    hit: (ev) => {
      const target = players.get(ev.target)
      nextHit = { target, tp: target.state.position, rewound: false, proj: target.state.position }
      nextRatio = ev.ratio
      sendMessage({ type: 'fire', direction: [0, 0, -1] }, ev.shooter, true)
    },
    fall: (ev) => {
      const player = players.get(ev.id)
      player.state.onGround = false
      player.state.position = [0, -100, 0]
      combat.tick(ev.dt)
    },
    tick: (ev) => combat.tick(ev.dt),
    leave: (ev) => sendMessage({ type: 'player_leave' }, ev.id, false),
  }
  const rows = events.map((ev) => {
    clock = ev.at
    dispatch[ev.event](ev)
    const row = { ...ev, handlers: captured.splice(0), writes: writes.splice(0) }
    if (traceValues) {
      row.board = clone(store.value)
      row.stats = Object.fromEntries([1, 2, 3].map((id) => [String(id), combat.statsOf(id)]))
    }
    return row
  })
  return { name, rows, finalBoard: clone(store.value) }
}

const TRIM_IDS = Array.from({ length: 202 }, (_, i) => i + 1)
const SCOREBOARD_SCENARIOS = [
  {
    name: 'kills-deaths-fall-and-rejoin',
    snapshot: 'every',
    events: [
      { event: 'join', at: 1000, id: 1, peer: null, name: 'Ada' },
      { event: 'join', at: 1000, id: 2, peer: 'p2', name: 'Bo' },
      { event: 'join', at: 1000, id: 3, peer: null, name: null },
      { event: 'hit', at: 2000, shooter: 1, target: 2, ratio: 0.2 },
      { event: 'hit', at: 2500, shooter: 1, target: 2, ratio: 0.9 },
      { event: 'hit', at: 2800, shooter: 1, target: 3, ratio: 0.9 },
      { event: 'hit', at: 3000, shooter: 2, target: 3, ratio: 0.9 },
      { event: 'hit', at: 3500, shooter: 1, target: 2, ratio: 0.9 },
      { event: 'fall', at: 4000, id: 1, dt: 0.6 },
      { event: 'tick', at: 4600, dt: 0.1 },
      { event: 'leave', at: 5000, id: 2 },
      { event: 'join', at: 5100, id: 2, peer: 'p2', name: 'Bo' },
      { event: 'hit', at: 5200, shooter: 3, target: 2, ratio: 0.2 },
    ],
  },
  {
    name: 'trim-to-max-entries',
    snapshot: 'final',
    events: [
      ...TRIM_IDS.map((id) => ({ event: 'join', at: 1000, id, peer: 'q' + id, name: 'N' + id })),
      ...TRIM_IDS.map((id) => ({ event: 'leave', at: 10000 + 10 * id, id })),
    ],
  },
]
const scoreboardScenarios = SCOREBOARD_SCENARIOS.map((sc) => runScoreboard(sc))

const fixture = {
  spoint: rev,
  source: 'spoint ' + rev + ': apps/tps-game/server-app.js (emoteOf, EMOTE_RATE_LIMIT_MS, onMessage emote branch) with apps/tps-game/shared.js EMOTE_CLIPS; Date.now is replaced by a controlled clock',
  emotes: [...EMOTE_CLIPS].map(([code, clip]) => ({ code, clip })),
  scenarios: emoteScenarios,
  scoreboard: {
    source: 'spoint ' + rev + ': src/behaviours/combat.js defineCombat (fire, applyKill, killByFall, persistStat, trimScoreboard, handle player_join and player_leave) with apps/tps-game/shared.js POWERUP_DEFS and DEFAULT_COMBAT; ctx is a stub whose persisted store is in memory, records each save and flush, and resolves ready with no saved record; hit detection is a fixed input; Date.now is replaced by a controlled clock',
    scenarios: scoreboardScenarios,
  },
}

const statusAfter = git(['status', '--porcelain'])
if (statusAfter !== statusBefore) throw new Error('spoint working tree changed')
fixture.spointStatusBefore = statusBefore
fixture.spointStatusAfter = statusAfter
writeFileSync(outPath, JSON.stringify(fixture, null, 1) + '\n')
console.log(JSON.stringify({ rev, out: outPath, status: 'unchanged', scoreboardRows: scoreboardScenarios.map((x) => x.rows.length) }))
