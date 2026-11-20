import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-hud.json')
const rev = execFileSync('git', ['-C', spoint, 'rev-parse', '--short', 'HEAD']).toString().trim()
const statusBefore = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
const shared = await import(pathToFileURL(join(spoint, 'apps/tps-game/shared.js')).href)
const app = await import(pathToFileURL(join(spoint, 'src/apps/AppGameplay.js')).href)
const { tpsGameClient } = await import(pathToFileURL(join(spoint, 'apps/tps-game/client-app.js')).href)
shared.bindCombat(app.COMBAT_API)
const cfg = shared.combatConfig()

const NOW = 100000
const PLAYER = 7
const elements = () => ({
  _elCross: { style: {} },
  _elHit: { style: {} },
  _elHitMarks: [{ style: {} }, { style: {} }],
  _elKill: { style: {}, textContent: undefined },
  _elDmgDealt: { style: {}, textContent: undefined },
  _elDmgTaken: { style: {}, textContent: undefined },
  _elShield: { style: {} },
  _elVig: { style: {} },
})

const cases = [
  { name: 'idle', tps: {} },
  { name: 'hit-marker-rtt', tps: { hitMarkerTime: NOW - 100 }, rtt: 40 },
  { name: 'hit-marker-expired', tps: { hitMarkerTime: NOW - 160 }, rtt: 0 },
  { name: 'headshot-marker', tps: { headshotMarkerTime: NOW - 200 }, rtt: 0 },
  { name: 'kill-multi-rtt', tps: { killTime: NOW - 1000, streak: 4 }, rtt: 300 },
  { name: 'kill-triple-expired', tps: { killTime: NOW - 1600, streak: 3 }, rtt: 0 },
  { name: 'kill-double', tps: { killTime: NOW - 300, streak: 2 }, rtt: 0 },
  { name: 'kill-single', tps: { killTime: NOW - 100, streak: 1 }, rtt: 0 },
  { name: 'recoil-on', tps: { lastShootTime: NOW - 100 } },
  { name: 'recoil-off', tps: { lastShootTime: NOW - 200 } },
  { name: 'dmg-dealt', tps: { dmgDealtTime: NOW - 200, lastDamageDealt: 25 } },
  { name: 'dmg-dealt-expired', tps: { dmgDealtTime: NOW - 600, lastDamageDealt: 25 } },
  { name: 'dmg-taken-low-hp', tps: { lastHitTime: NOW - 300, lastDamageTaken: 12 }, hp: 25 },
  { name: 'dmg-flash', tps: { lastHitTime: NOW - 100, lastDamageTaken: 12 }, hp: 100 },
  { name: 'shield-on', tps: { spawnShieldUntil: NOW + 1000 } },
  { name: 'shield-off', tps: {} },
  { name: 'reload-half', tps: { reloading: true, reloadEndTime: NOW + 500, ammo: 0 } },
  { name: 'boost', tps: { boost: { expiresAt: NOW + 2500, fireRate: 2 } } },
  { name: 'fall-punch', tps: { _wasOnGround: false, _fallVy: -10 }, onGround: true },
  { name: 'low-hp-vignette', tps: {}, hp: 10 },
  { name: 'ammo-low', tps: { ammo: 3 } },
  { name: 'hit-marker-rtt-pad', tps: { hitMarkerTime: NOW - 170 }, rtt: 40 },
  { name: 'kill-confirm-cap', tps: { killTime: NOW - 2300, streak: 1 }, rtt: 2000 },
  { name: 'kill-triple', tps: { killTime: NOW - 300, streak: 3 }, rtt: 0 },
  { name: 'reload-round-up', tps: { reloading: true, reloadEndTime: NOW + 1003, ammo: 0 } },
]

const realNow = Date.now
const results = []
for (const c of cases) {
  Date.now = () => NOW
  const captured = { stats: [], punch: [], ui: null }
  const tps = {
    ammo: cfg.magazineSize, magazineSize: cfg.magazineSize, reloading: false, reloadEndTime: null,
    boost: null, flash: null, flashOff: 0, lastShootTime: 0, hitMarkerTime: 0, headshotMarkerTime: 0,
    killTime: 0, streak: 1, kills: 0, dmgDealtTime: 0, lastDamageDealt: 0, lastHitTime: 0, lastDamageTaken: 0,
    spawnShieldUntil: 0, isAiming: false, _wasOnGround: undefined, _fallVy: 0, overlay: {}, ...elements(), ...c.tps,
  }
  const hp = c.hp ?? 100
  const engine = {
    _tps: tps, playerId: PLAYER,
    hud: { stat: (k, v) => captured.stats.push([k, v]) },
    players: { setAiming() {} },
    cam: { punch: (v) => captured.punch.push(v) },
    client: {
      getRTT: () => c.rtt ?? 0,
      state: { players: [{ id: PLAYER, health: hp, position: [0, 1.3, 0], velocity: [0, 0, 0], onGround: c.onGround ?? true }] },
      getLocalState: () => null,
    },
  }
  tpsGameClient.onFrame(1 / 60, engine)
  tpsGameClient.render({
    h: {}, state: { map: 'arena', mode: 'dm' }, engine, entity: { position: [0, 1.3, 0] },
    kit: { renderGameHud: (h, args) => { captured.ui = args; return 'ui' } },
  })
  const stat = (k) => captured.stats.filter(([key]) => key === k).map(([, v]) => v).pop()
  results.push({
    name: c.name,
    inputs: { now: NOW, rtt: c.rtt ?? 0, hp, onGround: c.onGround ?? true, tps: c.tps },
    outputs: {
      ammoStat: stat('ammo'), reloadingStat: stat('reloading'),
      crosshairTransform: tps._elCross.style.transform ?? null,
      hitOpacity: tps._elHit.style.opacity ?? null, hitColor: tps._elHitMarks[0].style.background ?? null,
      killText: tps._elKill.textContent ?? null, killOpacity: tps._elKill.style.opacity ?? null,
      dmgDealtText: tps._elDmgDealt.textContent ?? null, dmgDealtOpacity: tps._elDmgDealt.style.opacity ?? null,
      dmgTakenText: tps._elDmgTaken.textContent ?? null, dmgTakenOpacity: tps._elDmgTaken.style.opacity ?? null,
      shieldOpacity: tps._elShield.style.opacity ?? null, vignetteOpacity: tps._elVig.style.opacity ?? null,
      punch: captured.punch,
      ui: captured.ui,
    },
  })
}
Date.now = realNow

const statusAfter = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
if (statusAfter !== statusBefore) throw new Error('spoint working tree changed')
writeFileSync(outPath, JSON.stringify({ spoint: rev, now: NOW, combat: { magazineSize: cfg.magazineSize, reloadTime: cfg.reloadTime, spawnInvulnMs: cfg.spawnInvulnMs }, cases: results, spointStatusBefore: statusBefore, spointStatusAfter: statusAfter }, null, 2) + String.fromCharCode(10))
console.log(results.map((r) => r.name + ' ' + JSON.stringify(r.outputs)).join(String.fromCharCode(10)))
