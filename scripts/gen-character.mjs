import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-character.json')
const rev = execFileSync('git', ['-C', spoint, 'rev-parse', '--short', 'HEAD']).toString().trim()
const statusBefore = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
const { PhysicsWorld } = await import(pathToFileURL(join(spoint, 'src/physics/World.js')).href)
const { verticalVelocity } = await import(pathToFileURL(join(spoint, 'src/shared/characterStep.js')).href)

const GRAVITY = -9.81
const DT = 1 / 60
const RADIUS = 0.4
const HALF_HEIGHT = 0.9
const JUMP_SPEED = 5
const IDENTITY = [0, 0, 0, 1]
const idle = { cmd: [0, 0, 0] }
const repeat = (n, step) => Array.from({ length: n }, () => ({ ...step }))

const scenarios = [
  { name: 'fall', start: [0, 2.3, 0], steps: repeat(40, idle) },
  { name: 'walk-step', start: [-10, 1.3, 0], steps: [...repeat(10, idle), ...repeat(330, { cmd: [2, 0, 0] })] },
  { name: 'wall-slide', start: [2, 1.3, 0], steps: [...repeat(10, idle), ...repeat(60, { cmd: [3, 0, 0] }), ...repeat(60, { cmd: [3, 0, -2] })] },
  { name: 'jump', start: [0, 1.3, 0], steps: [...repeat(10, idle), { cmd: [0, 0, 0], jump: true }, ...repeat(90, idle)] },
  { name: 'crouch', start: [0, 1.3, 0], steps: [...repeat(10, idle), { cmd: [0, 0, 0], crouch: true }, ...repeat(30, idle), { cmd: [0, 0, 0], crouch: false }, ...repeat(20, idle)] },
  { name: 'crouch-airborne', start: [0, 1.3, 0], steps: [...repeat(10, idle), { cmd: [0, 0, 0], jump: true }, ...repeat(5, idle), { cmd: [0, 0, 0], crouch: true }, ...repeat(60, idle)] },
]

const buildScene = async () => {
  const w = new PhysicsWorld({ gravity: [0, GRAVITY, 0] })
  await w.init()
  w.addStaticBox([50, 0.5, 50], [0, -0.5, 0], IDENTITY)
  w.addStaticBox([0.5, 2, 20], [5, 2, 0], IDENTITY)
  w.addStaticBox([1.5, 0.15, 6], [-3, 0.15, 0], IDENTITY)
  return w
}

const round = (v) => v.map((c) => Number(c.toFixed(9)))
const results = []
for (const scenario of scenarios) {
  const w = await buildScene()
  const id = w.addPlayerCharacter(RADIUS, HALF_HEIGHT, [...scenario.start], 80)
  const state = { p: [...scenario.start], v: [0, 0, 0], onGround: false }
  const trace = []
  for (const step of scenario.steps) {
    if (step.jump && state.onGround) state.v[1] = JUMP_SPEED
    if (step.crouch !== undefined) w.setCharacterCrouch(id, step.crouch)
    const vy = verticalVelocity(state.v[1], state.onGround, GRAVITY, DT)
    w.setCharacterVelocity(id, [step.cmd[0], vy, step.cmd[2]])
    w.updateCharacter(id, DT)
    const p = [0, 0, 0], v = [0, 0, 0]
    w.readCharacterPosition(id, p)
    w.readCharacterVelocity(id, v)
    state.p = p
    state.v = v
    state.onGround = w.getCharacterGroundState(id)
    const n = [0, 0, 0]
    const normalOk = state.onGround && w.readCharacterGroundNormal(id, n)
    const walls = []
    const wallCount = w.readCharacterWallNormals(id, walls) ?? 0
    trace.push({
      p: round(p), v: round(v), onGround: state.onGround,
      normal: normalOk ? round(n) : null,
      wallCount, walls: round(walls.slice(0, 2 * wallCount)),
    })
  }
  results.push({ name: scenario.name, start: scenario.start, steps: scenario.steps.map((s) => ({ cmd: s.cmd, jump: !!s.jump, crouch: s.crouch ?? null })), trace })
  console.log(scenario.name, 'final', trace[trace.length - 1].p.join(','), 'grounded', trace[trace.length - 1].onGround)
}

const statusAfter = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
if (statusAfter !== statusBefore) throw new Error('spoint working tree changed')
writeFileSync(outPath, JSON.stringify({ spoint: rev, dt: DT, gravity: GRAVITY, radius: RADIUS, halfHeight: HALF_HEIGHT, jumpSpeed: JUMP_SPEED, scene: ['floor top y=0 x±50', 'wall x 4.5..5.5 height 4 z±20', 'step top y=0.3 x -4.5..-1.5 z±6'], scenarios: results, spointStatusBefore: statusBefore, spointStatusAfter: statusAfter }) + String.fromCharCode(10))
