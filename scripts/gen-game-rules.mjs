import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-game-rules.json')
const rev = execFileSync('git', ['-C', spoint, 'rev-parse', '--short', 'HEAD']).toString().trim()
const statusBefore = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
const spointModule = (rel) => import(pathToFileURL(join(spoint, rel)).href)

const spawnMod = await spointModule('src/stdlib-apps/spawn-point/index.js')
const shared = await spointModule('apps/tps-game/shared.js')
const serverMod = await spointModule('apps/tps-game/server-app.js')
const clientMod = await spointModule('apps/tps-game/client-app.js')
const combatMod = await spointModule('src/behaviours/combat.js')
const hitscanMod = await spointModule('src/netcode/Hitscan.js')

let clock = 0
Date.now = () => clock
const at = (ms) => { clock = ms }
const quiet = async (fn) => {
  const log = console.log
  console.log = () => {}
  try { return await fn() } finally { console.log = log }
}

const CONFIGS = [
  { name: 'default', config: {} },
  { name: 'red-gizmo', config: { team: 'red', showGizmo: true } },
  { name: 'red-hidden', config: { team: 'red', showGizmo: false } },
  { name: 'null-team', config: { team: null } },
  { name: 'empty-team', config: { team: '' } },
  { name: 'green-undefined-gizmo', config: { team: 'green', showGizmo: undefined } },
]
const spawnSetups = CONFIGS.map(({ name, config }) => {
  const entity = { position: [0, 0, 0] }
  let changed = () => {}
  spawnMod.default.server.setup({ config, entity, onConfigChange: (fn) => { changed = fn } })
  const setup = JSON.parse(JSON.stringify(entity.custom))
  changed({ team: 'blue' })
  const afterBlue = entity.custom._spawnTeam
  changed({})
  const afterNoTeam = entity.custom._spawnTeam
  return { name, config, setup, afterBlue, afterNoTeam }
})

const ENTITIES = [
  { name: 'red', custom: { _spawnPoint: true, _spawnTeam: 'red' } },
  { name: 'any', custom: { _spawnPoint: true, _spawnTeam: 'any' } },
  { name: 'blue', custom: { _spawnPoint: true, _spawnTeam: 'blue' } },
  { name: 'untagged', custom: { _spawnPoint: true } },
  { name: 'empty', custom: { _spawnPoint: true, _spawnTeam: '' } },
  { name: 'plain', custom: {} },
  { name: 'green', custom: { _spawnPoint: true, _spawnTeam: 'green' } },
]
ENTITIES.forEach((e, i) => { e.position = [i * 10, 1, 0 - i * 10] })
const world = { query: (pred) => ENTITIES.filter(pred) }
const TEAM_CASES = [
  { name: 'unset', team: undefined },
  { name: 'any', team: 'any' },
  { name: 'empty', team: '' },
  { name: 'red', team: 'red' },
  { name: 'blue', team: 'blue' },
  { name: 'green', team: 'green' },
  { name: 'mauve', team: 'mauve' },
]
const collectCases = TEAM_CASES.map(({ name, team }) => ({
  name,
  team: team ?? null,
  positions: spawnMod.collectSpawnPoints({ world }, team),
}))

const PU_Y = 10 + 0.6
const AWAY = [-50, PU_Y, 0]

const tagOf = (e) => {
  if (e.op === 'spawn') return ['spawn', e.id]
  if (e.op === 'destroy') return ['destroy', e.id]
  if (e.op === 'send' && e.msg.type === 'buff_applied') return ['buff_applied', e.id, e.msg.duration, e.msg.speed, e.msg.fireRate, e.msg.damage]
  if (e.op === 'send' && e.msg.type === 'buff_expired') return ['buff_expired', e.id]
  return [e.op]
}

const combatCtx = (log, players) => ({
  persisted: () => ({ ready: Promise.resolve(), value: {}, save() {}, flush: async () => {} }),
  world: {
    spawn: (id, props) => log.push({ op: 'spawn', id, position: props.position }),
    destroy: (id) => log.push({ op: 'destroy', id }),
  },
  players: {
    getAll: () => [...players.values()],
    getById: (id) => players.get(id) ?? null,
    send: (id, msg) => log.push({ op: 'send', id, msg: JSON.parse(JSON.stringify(msg)) }),
    setPosition() {},
  },
  network: { broadcast: (msg) => log.push({ op: 'broadcast', msg }) },
  fallFloorY: () => -1e9,
  terrainBodyId: null,
  raycast: () => ({ hit: false }),
  terrainHeightAt: () => 0,
})

const buffOf = (b) => (b ? { expiresAt: b.expiresAt, speed: b.speed, fireRate: b.fireRate, damage: b.damage } : null)

const runCombat = async (spec, startAt, steps) => {
  const log = []
  const players = new Map([[1, { id: 1, state: { position: [0, 0, 0], health: 100, velocity: [0, 0, 0], onGround: true } }]])
  at(startAt)
  const combat = combatMod.defineCombat(spec, combatCtx(log, players))
  await quiet(() => combat.setup())
  const setup = log.splice(0).map(tagOf)
  const trace = steps.map((step) => {
    at(startAt + step.at)
    const p = players.get(1)
    if (step.pos) p.state.position = [...step.pos]
    if (step.health !== undefined) p.state.health = step.health
    combat.tick(step.dt)
    return { label: step.label, at: startAt + step.at, health: p.state.health, buff: buffOf(combat.buffOf(1)), events: log.splice(0).map(tagOf) }
  })
  return { spawnPoints: combat.spawnPoints, setup, trace }
}

const T0 = 1000000
const MAIN_SPEC = {
  powerups: shared.POWERUP_DEFS,
  powerupRespawnMs: shared.POWERUP_RESPAWN_MS,
  scoreboardKey: 'scoreboard',
  spawnPoints: [[0, 10, 0], [10, 10, 0], [20, 10, 0], [30, 10, 0]],
}
const MAIN_STEPS = [
  { label: 'pickup-damage-dz1', at: 0, dt: 0, pos: [10, PU_Y, 1] },
  { label: 'no-respawn-before-15s', at: 14999, dt: 0, pos: AWAY },
  { label: 'respawn-damage-at-15s', at: 15000, dt: 0 },
  { label: 'regen-while-buffed', at: 19000, dt: 0.5, health: 50 },
  { label: 'expired-at-20s', at: 20000, dt: 1 },
  { label: 'pickup-speed-dz1.7-boundary', at: 20001, dt: 0, pos: [20, PU_Y, 1.7] },
  { label: 'outside-dz1.75', at: 20002, dt: 0, pos: [30, PU_Y, 1.75] },
  { label: 'pickup-rapid-center', at: 20003, dt: 0, pos: [30, PU_Y, 0] },
  { label: 'speed-not-yet-respawned', at: 35000, dt: 0, pos: AWAY },
  { label: 'speed-respawn-at-35001', at: 35001, dt: 0 },
  { label: 'rapid-respawn-at-35003', at: 35003, dt: 0 },
  { label: 'rapid-expired-at-40003', at: 40003, dt: 0 },
]
const MAIN = await runCombat(MAIN_SPEC, T0, MAIN_STEPS)

const T1 = 2000000
const DEFAULT_SPEC = {
  powerups: [
    { type: 'quick', color: 0, emissive: 0, respawnMs: 4000, buff: { duration: 5, speedMultiplier: 2 } },
    { type: 'plain', buff: { duration: 3 } },
  ],
  spawnPoints: [[0, 10, 0], [10, 10, 0], [20, 10, 0]],
}
const DEFAULT_STEPS = [
  { label: 'quick-pickup', at: 0, dt: 0, pos: [10, PU_Y, 0] },
  { label: 'quick-away-before-4s', at: 3999, dt: 0, pos: AWAY },
  { label: 'quick-respawn-4s', at: 4000, dt: 0 },
  { label: 'quick-expired-5s', at: 5000, dt: 0 },
  { label: 'plain-pickup', at: 5001, dt: 0, pos: [20, PU_Y, 0] },
  { label: 'plain-away-before-15s', at: 20000, dt: 0, pos: AWAY },
  { label: 'plain-respawn-15s', at: 20001, dt: 0 },
]
const DEFAULTS = await runCombat(DEFAULT_SPEC, T1, DEFAULT_STEPS)

const REFUSALS = [
  { name: 'duration-nan', powerups: [{ type: 'bad', buff: { duration: NaN } }] },
  { name: 'empty-type', powerups: [{ type: '', buff: { duration: 1 } }] },
  { name: 'multiplier-string', powerups: [{ type: 'bad', buff: { duration: 1, damageMultiplier: 'x' } }] },
].map(({ name, powerups }) => {
  try {
    combatMod.defineCombat({ powerups }, combatCtx([], new Map()))
    return { name, error: null }
  } catch (e) {
    return { name, error: e.message }
  }
})

const SERVER_ENTITIES = [
  { position: [5, 6, 7], custom: { _spawnPoint: true } },
  { position: [1, 1, 1], custom: {} },
]
const FIRE_CASES = [
  { name: 'absent', config: {} },
  { name: 'disabled', config: { fire: { enabled: false, radius: 1 } } },
  { name: 'enabled-string', config: { fire: { enabled: 'true', radius: 1 } } },
  { name: 'enabled-default', config: { fire: { enabled: true } } },
  { name: 'enabled-tuned', config: { fire: { enabled: true, radius: 1000, seed: 7, stepTicks: 10, leadTicks: 3, regrowSteps: 50 } } },
]
const SHOOTERS = [1, 255, 256, 300, 65537]
const fireOut = []
for (const c of FIRE_CASES) {
  const record = { fire: undefined, combat: undefined, rayBlocked: [], ignite: [] }
  const ctx = {
    config: c.config,
    state: {},
    world: { query: (pred) => SERVER_ENTITIES.filter(pred) },
    combat: { DEFAULT_COMBAT: combatMod.DEFAULT_COMBAT },
    defineCombat: (spec) => {
      record.combat = spec
      return { setup: async () => {}, flush: async () => {}, tick() {}, handle: () => false }
    },
    defineFire: (spec) => {
      record.fire = spec
      return {
        rayBlocked: (o, d, dist) => { record.rayBlocked.push([o, d, dist]); return true },
        ignite: (p, s) => { record.ignite.push([p, s]) },
        tick() {},
      }
    },
    onShutdown() {},
  }
  await quiet(() => serverMod.tpsGameServer.setup(ctx))
  const blocked = record.combat.shotBlocked ? record.combat.shotBlocked(ctx, [0, 1, 2], [0, 0, 1], 4) : null
  const seeds = SHOOTERS.map((s) => {
    if (!record.combat.onWorldHit) return null
    const before = record.ignite.length
    record.combat.onWorldHit(ctx, { shooterId: s, position: [1, 2, 3] })
    return record.ignite[before] ? record.ignite[before][1] : null
  })
  fireOut.push({
    name: c.name,
    config: c.config,
    spawnPoints: record.combat.spawnPoints,
    powerupCount: record.combat.powerups.length,
    powerupRespawnMs: record.combat.powerupRespawnMs,
    fireSpec: record.fire ? JSON.parse(JSON.stringify(record.fire)) : null,
    rayBlockedArgs: record.rayBlocked[0] ?? null,
    rayBlockedResult: blocked,
    ignitePositions: record.ignite.map((r) => r[0]),
    ignitionSeeds: seeds,
    shooterIds: SHOOTERS,
  })
}

const engineFor = (log) => {
  const local = { id: 1, position: [0, 10, 0], velocity: [0, 0, 0], onGround: true }
  class PointLight {
    constructor() {
      this.color = { setHex() {} }
      this.position = { set() {} }
      this.intensity = 0
      this.distance = 0
    }
  }
  const engine = {
    THREE: { PointLight },
    scene: { add() {} },
    playerId: 1,
    combat: { ...hitscanMod, DEFAULT_COMBAT: combatMod.DEFAULT_COMBAT, readHealth: combatMod.readHealth },
    client: {
      state: { players: [] },
      getLocalState: () => local,
      getRTT: () => 0,
      sendFire: (m) => log.push({ op: 'fire', origin: m.origin, direction: m.direction }),
      sendReload: () => log.push({ op: 'reload' }),
      sendEmote() {},
      recordKnockback: (dir, k, t) => log.push({ op: 'knockback', dir, k, t }),
    },
    cam: { getAimDirection: () => [0.6, 0, -0.8], punch() {} },
    players: { getAnimator: () => null, setExpression() {}, setAiming() {} },
    decals: { spawnTracer: (a, b) => log.push({ op: 'tracer', a, b }) },
    hud: { stat() {} },
    local,
  }
  return engine
}
const clientTag = (e) => {
  if (e.op === 'fire') return ['fire', e.origin, e.direction]
  if (e.op === 'tracer') return ['tracer', e.a, e.b]
  if (e.op === 'knockback') return ['knockback', e.dir, e.k, e.t]
  return [e.op]
}
const CLIENT_C0 = 3000000
const clientRun = () => {
  const log = []
  const engine = engineFor(log)
  clientMod.tpsGameClient.setup(engine)
  const tps = engine._tps
  const results = []
  const step = (label, now, fn) => {
    at(now)
    const mark = log.length
    fn()
    results.push({
      label,
      now,
      ammo: tps.ammo,
      reloading: tps.reloading,
      reloadEndTime: tps.reloadEndTime ?? null,
      lastShootTime: tps.lastShootTime,
      lastReloadTime: tps.lastReloadTime,
      boost: tps.boost ? { expiresAt: tps.boost.expiresAt, fireRate: tps.boost.fireRate } : null,
      velocity: [...engine.local.velocity],
      events: log.slice(mark).map(clientTag),
    })
  }
  const shoot = () => clientMod.tpsGameClient.onInput({ shoot: true }, engine)
  const reload = () => clientMod.tpsGameClient.onInput({ reload: true }, engine)
  const event = (p) => () => clientMod.tpsGameClient.onEvent(p, engine)
  const frame = (dt) => () => clientMod.tpsGameClient.onFrame(dt, engine)
  const C0 = CLIENT_C0
  step('shoot-first', C0, shoot)
  step('shoot-at-100ms-gate-closed', C0 + 100, shoot)
  step('shoot-after-100ms', C0 + 101, shoot)
  step('buff-rapid-applied', C0 + 101, event({ type: 'buff_applied', duration: 20, speed: 1.5, fireRate: 2, damage: 2 }))
  step('shoot-boosted-at-50ms-gate-closed', C0 + 151, shoot)
  step('shoot-boosted-after-50ms', C0 + 152, shoot)
  step('reload-request', C0 + 152, reload)
  step('reload-request-inside-100ms', C0 + 200, reload)
  step('reload-request-after-100ms', C0 + 253, reload)
  step('reload-start', C0 + 260, event({ type: 'reload_start', duration: 2000 }))
  step('shoot-while-reloading', C0 + 261, shoot)
  step('reload-request-while-reloading', C0 + 400, reload)
  step('frame-mid-reload', C0 + 500, frame(0.016))
  step('reload-complete', C0 + 2260, event({ type: 'reload_complete', ammo: 30 }))
  let t = C0 + 2300
  for (let k = 0; k < 31; k++) {
    step('drain-' + k, t, shoot)
    t += 60
  }
  step('boost-lapsed-frame', C0 + 20101, frame(0.016))
  step('shoot-empty-after-drain', C0 + 20200, shoot)
  step('respawn-refill', C0 + 20300, event({ type: 'respawn', ammo: 30, invulnMs: 1500 }))
  step('shoot-unboosted-first', C0 + 20301, shoot)
  step('shoot-unboosted-at-100ms-gate-closed', C0 + 20401, shoot)
  step('hit-taken-knockback', C0 + 20500, event({ type: 'hit', target: 1, damage: 20, dir: [1, 0, 0], knockback: 4 }))
  return results
}
const CLIENT = clientRun()

const statusAfter = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
if (statusAfter !== statusBefore) throw new Error('spoint working tree changed')

const replacer = (_, v) => (typeof v === 'number' && !Number.isFinite(v) ? String(v) : v)
const out = {
  spoint: rev,
  spawn: {
    setups: spawnSetups,
    entities: ENTITIES.map((e) => ({ name: e.name, position: e.position, custom: e.custom })),
    collect: collectCases,
  },
  powerups: {
    defs: shared.POWERUP_DEFS,
    respawnMs: shared.POWERUP_RESPAWN_MS,
    main: { startAt: T0, ...MAIN },
    defaults: { startAt: T1, ...DEFAULTS },
    refusals: REFUSALS,
  },
  fire: { spec: shared.FIRE_SPEC, cases: fireOut },
  client: { startAt: CLIENT_C0, steps: CLIENT },
  spointStatusBefore: statusBefore,
  spointStatusAfter: statusAfter,
}
writeFileSync(outPath, JSON.stringify(out, replacer, 1) + '\n')
console.log(JSON.stringify({ spawn: spawnSetups.length, collect: collectCases.length, main: MAIN.trace.length, defaults: DEFAULTS.trace.length, refusals: REFUSALS.map((r) => r.error), fire: fireOut.map((f) => [f.name, f.fireSpec !== null, f.ignitionSeeds]), client: CLIENT.length }))
