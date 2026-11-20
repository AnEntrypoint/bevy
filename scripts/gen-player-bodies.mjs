import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-player-bodies.json')
const rev = execFileSync('git', ['-C', spoint, 'rev-parse', '--short', 'HEAD']).toString().trim()
const statusBefore = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
const { PhysicsWorld } = await import(pathToFileURL(join(spoint, 'src/physics/World.js')).href)
const { PhysicsIntegration } = await import(pathToFileURL(join(spoint, 'src/netcode/PhysicsIntegration.js')).href)

const CONFIG = { capsuleRadius: 0.4, capsuleHalfHeight: 0.9, ankleClearance: 0.1, playerMass: 80, gravity: [0, -9.81, 0] }
const messages = [
  { type: 'spawn', playerId: 1, radius: 0.4, position: [2, 1.3, 1] },
  { type: 'spawn', playerId: 2, radius: 0, position: [-1, 1.3, 0] },
  { type: 'spawn', playerId: 1, radius: 0.5, position: [2, 1.3, 1] },
  { type: 'reconnect', oldId: 2, newId: 3, radius: 0.4, position: [-3, 1.3, 0] },
  { type: 'leave', playerId: 1 },
]

const apply = (pi, m) => {
  if (m.type === 'spawn') {
    pi.addPlayerCollider(m.playerId, m.radius)
    pi.setPlayerPosition(m.playerId, m.position)
  } else if (m.type === 'reconnect') {
    if (m.oldId !== m.newId) pi.removePlayerCollider(m.oldId)
    pi.addPlayerCollider(m.newId, m.radius)
    pi.setPlayerPosition(m.newId, m.position)
  } else {
    pi.removePlayerCollider(m.playerId)
  }
}
const round = (v) => v.map((c) => Number(c.toFixed(9)))
const observe = (pi) => {
  const bodies = [...pi.playerBodies.entries()].map(([id, d]) => { const shape = pi.physicsWorld && d.charId != null ? pi.physicsWorld._charMgr._charShapes.get(d.charId) : null; return { id, hasCharacter: d.charId != null, onGround: d.onGround, radius: shape ? shape.radius : null } })
  const positions = [...pi.playerBodies.keys()].map((id) => { const p = pi.getPlayerPosition(id); return { id, p: p ? round(p) : null } })
  const characterCount = pi.physicsWorld ? pi.physicsWorld._charMgr.characters.size : 0
  return { bodies, positions, crouch: [...pi._crouchStates.keys()], characterCount }
}

const scenarios = []
for (const name of ['world', 'noWorld']) {
  const physicsWorld = name === 'world' ? new PhysicsWorld({ gravity: CONFIG.gravity }) : null
  if (physicsWorld) await physicsWorld.init()
  const pi = new PhysicsIntegration({ ...CONFIG, physicsWorld })
  const steps = []
  for (const m of messages) {
    apply(pi, m)
    steps.push({ message: m, ...observe(pi) })
  }
  scenarios.push({ name, physicalHalfHeight: pi._physicalHalfHeight, steps })
}

const statusAfter = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
if (statusAfter !== statusBefore) throw new Error('spoint working tree changed')
writeFileSync(outPath, JSON.stringify({ spoint: rev, config: CONFIG, messages, scenarios, spointStatusBefore: statusBefore, spointStatusAfter: statusAfter }, null, 2) + String.fromCharCode(10))
console.log(JSON.stringify(scenarios, null, 1))
