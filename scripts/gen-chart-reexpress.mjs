import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-chart-reexpress.json')
const gitStatus = () => execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
const rev = execFileSync('git', ['-C', spoint, 'rev-parse', '--short', 'HEAD']).toString().trim()
const statusBefore = gitStatus()
const { createChartTransfer } = await import(pathToFileURL(join(spoint, 'src/shared/chartTransfer.js')).href)
const { createReexpressPass, reexpressYawQuat, reexpressLook } = await import(pathToFileURL(join(spoint, 'src/shared/chartReexpress.js')).href)

const mul = (a, b) => a.map((row) => [0, 1, 2].map((c) => row[0] * b[0][c] + row[1] * b[1][c] + row[2] * b[2][c]))
const rotZ = (a) => [[Math.cos(a), -Math.sin(a), 0], [Math.sin(a), Math.cos(a), 0], [0, 0, 1]]
const rotX = (a) => [[1, 0, 0], [0, Math.cos(a), -Math.sin(a)], [0, Math.sin(a), Math.cos(a)]]
const R = mul(rotZ(0.3), rotX(-0.2))
const axis = (i) => [R[0][i], R[1][i], R[2][i]]

const from = { radius: 60, offsetY: 0.5, anchorHeight: 0.25, east: [1, 0, 0], up: [0, 1, 0], north: [0, 0, 1] }
const to = { radius: 60, offsetY: 0.125, anchorHeight: 0.75, east: axis(0), up: axis(1), north: axis(2) }
const transfer = createChartTransfer(from, to)

const halfQuat = (h) => [0.1, Math.sin(h), -0.05, Math.cos(h)]
const YAW_HALF_ANGLES = [-0.6, -0.1, 0.25, 0.7, 1.1]
const yawQuats = YAW_HALF_ANGLES.map(halfQuat)
const yawQuatResults = yawQuats.map((q) => { const c = [...q]; reexpressYawQuat(transfer, c); return c })

const looks = [{ yaw: 0.4, pitch: -0.2 }, { yaw: -2.1, pitch: 0.5 }, { yaw: 1.3, pitch: null }, { yaw: null, pitch: 0.35 }, { yaw: null, pitch: null }]
const lookResults = looks.map((h) => { const c = { ...h }; reexpressLook(transfer, c); return c })

const POINTS = [[0, 0, 0], [1, 2, 3], [-4, 0.5, 6]]
const VECTORS = [[1, 0, 0], [0, 1, 0], [0.3, -0.7, 2]]
const QUATS = YAW_HALF_ANGLES.slice(0, 3).map(halfQuat)
const HOLDERS = looks.slice(0, 3)
const LISTS = [[1, 2], [3]]
const SLOTS = [[1.5, -2, 0.25, 4]]

const pass = createReexpressPass(transfer)
const pointResults = POINTS.map((p) => { const c = [...p]; pass.point(c); return c })
const vectorResults = VECTORS.map((v) => { const c = [...v]; pass.vector(c); return c })
const quatResults = QUATS.map((q) => { const c = [...q]; pass.yawRotation(c); return c })
const holderResults = HOLDERS.map((h) => { const c = { ...h }; pass.look(c); return c })
const listLengths = LISTS.map((l) => { const c = [...l]; pass.clear(c); return c.length })
const slotResults = SLOTS.map((s) => { const c = [...s]; pass.pushSlot(c); return c })
const repeatedPoint = [...POINTS[0]]
pass.point(repeatedPoint)
pass.point(repeatedPoint)

const fixture = {
  spointRev: rev,
  from,
  to,
  tiltRad: transfer.tiltRad,
  yawQuat: { inputs: yawQuats, results: yawQuatResults },
  look: { inputs: looks, results: lookResults },
  pass: {
    points: POINTS, pointResults,
    vectors: VECTORS, vectorResults,
    quats: QUATS, quatResults,
    holders: HOLDERS, holderResults,
    lists: LISTS, listLengths,
    slots: SLOTS, slotResults,
    repeatedPoint: POINTS[0], pointRepeat: repeatedPoint,
    distinctTargets: pass.distinctTargets,
  },
}

if (gitStatus() !== statusBefore) throw new Error('spoint working tree changed during the witness')
writeFileSync(outPath, JSON.stringify(fixture, null, 2) + '\n')
console.log('wrote ' + outPath + ' from spoint ' + rev + ' distinctTargets=' + pass.distinctTargets)
