import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-chart-transfer.json')
const rev = execFileSync('git', ['-C', spoint, 'rev-parse', '--short', 'HEAD']).toString().trim()
const statusBefore = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
const { PhysicsWorld } = await import(pathToFileURL(join(spoint, 'src/physics/World.js')).href)
const { verticalVelocity } = await import(pathToFileURL(join(spoint, 'src/shared/characterStep.js')).href)

const GRAVITY = -9.81
const DT = 1 / 60
const RADIUS = 0.4
const HALF_HEIGHT = 0.9
const IDENTITY = [0, 0, 0, 1]
const TILT_RAD = 0.02
const TRANSLATION = [0.5, 0, -0.25]
const round = (v) => v.map((c) => Number(c.toFixed(9)))
const { sin, cos } = Math
const transfer = {
  tiltRad: TILT_RAD,
  point(a, out) {
    const x = a[0], y = a[1], z = a[2]
    const ry = cos(TILT_RAD) * y - sin(TILT_RAD) * z
    const rz = sin(TILT_RAD) * y + cos(TILT_RAD) * z
    out[0] = x + TRANSLATION[0]
    out[1] = ry + TRANSLATION[1]
    out[2] = rz + TRANSLATION[2]
    return out
  },
  vec(a, out) {
    const x = a[0], y = a[1], z = a[2]
    out[0] = x
    out[1] = cos(TILT_RAD) * y - sin(TILT_RAD) * z
    out[2] = sin(TILT_RAD) * y + cos(TILT_RAD) * z
    return out
  },
}

const w = new PhysicsWorld({ gravity: [0, GRAVITY, 0] })
await w.init()
w.addStaticBox([50, 0.5, 50], [0, -0.5, 0], IDENTITY)
w.addStaticBox([0.5, 2, 20], [5, 2, 0], IDENTITY)
w.addStaticBox([1.5, 0.15, 6], [-3, 0.15, 0], IDENTITY)

const cast = [
  { name: 'grounded-floor', start: [0, 2.3, 0], settle: 40, velocity: [0, 0.0005, 0] },
  { name: 'grounded-tilt-bound', start: [-10, 2.3, 0], settle: 40, velocity: [1, 0.017, 0] },
  { name: 'airborne', start: [1, 3, 0], settle: 5, velocity: [1, 2, 0.5] },
]
const states = new Map()
for (const c of cast) {
  const id = w.addPlayerCharacter(RADIUS, HALF_HEIGHT, [...c.start], 80)
  states.set(id, { name: c.name, id, p: [...c.start], v: [0, 0, 0], onGround: false, velocity: c.velocity })
  for (let i = 0; i < c.settle; i++) {
    const st = states.get(id)
    const vy = verticalVelocity(st.v[1], st.onGround, GRAVITY, DT)
    w.setCharacterVelocity(id, [0, vy, 0])
    w.updateCharacter(id, DT)
    w.readCharacterPosition(id, st.p)
    w.readCharacterVelocity(id, st.v)
    st.onGround = w.getCharacterGroundState(id)
  }
  w.setCharacterVelocity(id, c.velocity)
}

const mgr = w._charMgr
const snapshot = (id) => {
  const st = states.get(id)
  const p = [0, 0, 0], v = [0, 0, 0]
  w.readCharacterPosition(id, p)
  w.readCharacterVelocity(id, v)
  const g = mgr._groundNormals.get(id)
  const normal = g && g[3] ? round([g[0], g[1], g[2]]) : null
  return { p: round(p), v: round(v), onGround: w.getCharacterGroundState(id), normal, walls: mgr._walls.get(id)?.[0] ?? 0, name: st.name }
}
const before = [...states.keys()].map(snapshot)
const counts = mgr.applyChartTransfer(transfer)
const after = [...states.keys()].map(snapshot)

const statusAfter = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
if (statusAfter !== statusBefore) throw new Error('spoint working tree changed')
const cases = before.map((b, i) => ({ name: b.name, before: { p: b.p, v: b.v, onGround: b.onGround, normal: b.normal, walls: b.walls }, after: { p: after[i].p, v: after[i].v, onGround: after[i].onGround, normal: after[i].normal, walls: after[i].walls } }))
writeFileSync(outPath, JSON.stringify({ spoint: rev, transfer: { tiltRad: TILT_RAD, translation: TRANSLATION }, counts: { characters: counts.characters, groundedUpwardClamped: counts.groundedUpwardClamped }, cases, spointStatusBefore: statusBefore, spointStatusAfter: statusAfter }, null, 2) + String.fromCharCode(10))
console.log(JSON.stringify({ counts, cases }, null, 1))
