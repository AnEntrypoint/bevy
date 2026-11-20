import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-airborne-reuse.json')
const rev = execFileSync('git', ['-C', spoint, 'rev-parse', '--short', 'HEAD']).toString().trim()
const statusBefore = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
const tp = await import(pathToFileURL(join(spoint, 'src/terrain/TerrainPhysics.js')).href)

const heightFn = (x, z) => 3 + 0.25 * x - 0.1 * z
const frame = { chartEpoch: 0 }
const players = new Map()
const reuse = tp.createAirborneGroundReuse({ heightFn, frame, playerManager: { players } })
const enc = (v) => (v === Infinity ? 'Infinity' : v === -Infinity ? '-Infinity' : v)
const encStats = (s) => Object.fromEntries(Object.entries(s).map(([k, v]) => [k, enc(v)]))
const ops = []
const expect = []
let key = 0
const run = (op, fn) => {
  ops.push(op)
  const y = fn()
  expect.push({ y: y === undefined ? null : enc(y), stats: encStats(reuse.stats()) })
}
const add = (id, position, velocity, onGround) => {
  key += 1
  run({ op: 'add', id, key, position: [...position], velocity: [...velocity], onGround }, () => {
    players.set(id, { state: { position: [...position], velocity: [...velocity], onGround } })
  })
}
const move = (id, position, velocity, onGround) => run({ op: 'move', id, position: [...position], velocity: [...velocity], onGround }, () => {
  const s = players.get(id).state
  s.position = [...position]
  s.velocity = [...velocity]
  s.onGround = onGround
})
const remove = (id) => run({ op: 'remove', id }, () => { players.delete(id) })
const height = (x, z) => run({ op: 'height', x, z }, () => reuse.heightAt(x, z))
const setEpoch = (value) => run({ op: 'epoch', value }, () => { frame.chartEpoch = value })
const setEnabled = (value) => run({ op: 'enabled', value }, () => reuse.setEnabled(value))
const setVerify = (value) => run({ op: 'verify', value }, () => reuse.setVerifyReuse(value))
const setOverride = (value) => run({ op: 'override', value }, () => reuse.setDistanceOverrideM(value))
const resetStats = () => run({ op: 'reset' }, () => reuse.resetStats())

add('p', [0, 6, 0], [5, -30, 0], false)
add('g', [40, 5, 40], [0, 0, 0], true)
add('b', [0, 6, 0], [0, -4, 0], false)
height(0, 0)
let px = 0
for (let i = 0; i < 3; i++) {
  px += 0.09
  move('p', [px, 6, 0], [5, -30, 0], false)
  height(px, 0)
}
setEpoch(1)
height(px, 0)
setEpoch(0)
let py = 6
for (let i = 0; i < 40; i++) {
  px += 0.001
  py -= 0.1
  move('p', [px, py, 0], [5, -30, 0], false)
  height(px, 0)
  if (i % 8 === 7) height(40, 40)
}
setEpoch(1)
height(px, 0)
setEpoch(0)
move('p', [px, 4, 0], [5, 2, 0], false)
height(px, 0)
setVerify(true)
for (let i = 0; i < 6; i++) {
  px += 0.02
  move('p', [px, 4 + i * 0.01, 0], [5, 2, 0], false)
  height(px, 0)
}
setVerify(false)
setOverride(0.5)
for (let i = 0; i < 4; i++) {
  px += 0.01
  move('p', [px, 4, 0], [5, 2, 0], false)
  height(px, 0)
}
setOverride(null)
setEnabled(false)
height(px, 0)
setEnabled(true)
remove('p')
height(px, 0)
add('p', [px, 9, 0], [5, -2, 0], false)
resetStats()
height(px, 0)
move('p', [px + 0.01, 9, 0], [5, -2, 0], false)
height(px + 0.01, 0)

const statusAfter = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
if (statusAfter !== statusBefore) throw new Error('spoint working tree changed')
writeFileSync(outPath, JSON.stringify({ spoint: rev, ops, expect, spointStatusBefore: statusBefore, spointStatusAfter: statusAfter }, null, 1) + '\n')
const reused = expect[expect.length - 1].stats.reused
console.log(JSON.stringify({ spoint: rev, ops: ops.length, reused }))
