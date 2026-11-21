import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'
const spoint = process.argv[2] ?? '/workspace/spoint'
const git = (args) => execFileSync('git', ['-C', spoint, ...args]).toString()
const rev = git(['rev-parse', '--short', 'HEAD']).trim()
const statusBefore = git(['status', '--porcelain'])
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-fire-stage-map.json')
const { createFireStageMap, burnMaskWindow } = await import(pathToFileURL(join(spoint, 'src/shared/fire/fireStageMap.js')).href)
const { fnv1aStepBytes, FNV1A_32_OFFSET_BASIS } = await import(pathToFileURL(join(spoint, 'src/shared/fnv1a.js')).href)

const kernel = {
  tileGeneration: 0,
  serials: new Map(),
  tileIndexOf(face, ti, tj) { return (face + ti + tj) % 4 === 3 ? -1 : (face * 8 + ti) * 8 + tj },
  tileSerialOf(t) { return this.serials.get(t) ?? 1 },
  readTileStage(t, out, offset) {
    const s = this.tileSerialOf(t)
    for (let k = 0; k < 256; k++) out[offset + k] = (t * 37 + s * 11 + k * 5) & 255
  },
  stateCodeAt(face, I, J) { return (face * 3 + I * 5 + J * 7) % 3 },
}
const lattice = { cellsPerFace: 24, cellOfPlacementId(id) { return { face: id % 3, I: (id * 5) % 24, J: (id * 7) % 24 } } }
const SLOTS = 4
const map = createFireStageMap({ kernel, lattice, slotsAcross: SLOTS, slotsDown: SLOTS })
const hashData = () => fnv1aStepBytes(FNV1A_32_OFFSET_BASIS, map.data) >>> 0
const exec = (s) => {
  switch (s.op) {
    case 'acquire': return map.acquire(s.face, s.ti, s.tj)
    case 'release': return map.release(s.face, s.ti, s.tj)
    case 'slotOfCell': return map.slotValueOfCell(s.face, s.I, s.J)
    case 'slotOfTrunk': return map.slotValueOfTrunk(s.id)
    case 'texel': { const o = map.texelOf(s.value, { x: 0, y: 0 }); return { x: o.x, y: o.y } }
    case 'blockOrigin': return map.blockOrigin(s.slot)
    case 'sync': {
      const changed = map.sync()
      return { changed, hash: hashData(), watched: map.watchedTiles, stats: { ...map.stats }, dirty: Array.from(map.dirtySlots.slice(0, changed)).sort((a, b) => a - b) }
    }
    case 'generation': kernel.tileGeneration = s.value; return null
    case 'serial': kernel.serials.set(s.t, s.value); return null
    case 'burnMask': {
      const out = new Uint8Array(s.w * s.h)
      const lit = burnMaskWindow(kernel, s.face, s.I0, s.J0, s.w, s.h, out)
      return { lit, out: Array.from(out) }
    }
    default: throw new Error('unknown op ' + s.op)
  }
}
const steps = []
for (const face of [0, 1]) for (const ti of [0, 1, 2]) for (const tj of [0, 1, 2]) steps.push({ op: 'acquire', face, ti, tj })
steps.push({ op: 'sync' })
steps.push({ op: 'slotOfCell', face: 0, I: 0, J: 0 }, { op: 'slotOfCell', face: 0, I: 9, J: 17 }, { op: 'slotOfCell', face: 1, I: 23, J: 2 }, { op: 'slotOfCell', face: 1, I: 8, J: 8 }, { op: 'slotOfCell', face: 1, I: 20, J: 20 })
steps.push({ op: 'slotOfTrunk', id: 0 }, { op: 'slotOfTrunk', id: 1 }, { op: 'slotOfTrunk', id: 5 }, { op: 'slotOfTrunk', id: 9 }, { op: 'slotOfTrunk', id: 14 })
steps.push({ op: 'texel', value: 0 }, { op: 'texel', value: 63 }, { op: 'texel', value: 64 }, { op: 'texel', value: 1000 })
steps.push({ op: 'blockOrigin', slot: 0 }, { op: 'blockOrigin', slot: 3 }, { op: 'blockOrigin', slot: 4 }, { op: 'blockOrigin', slot: 15 })
steps.push({ op: 'burnMask', face: 0, I0: 2, J0: 3, w: 5, h: 4 }, { op: 'burnMask', face: 2, I0: 0, J0: 0, w: 3, h: 3 })
steps.push({ op: 'sync' })
steps.push({ op: 'serial', t: 0, value: 5 }, { op: 'sync' })
steps.push({ op: 'release', face: 0, ti: 0, tj: 0 }, { op: 'release', face: 0, ti: 0, tj: 0 }, { op: 'acquire', face: 2, ti: 0, tj: 0 }, { op: 'sync' })
steps.push({ op: 'generation', value: 1 }, { op: 'sync' })
steps.push({ op: 'slotOfCell', face: 2, I: 3, J: 3 }, { op: 'acquire', face: 1, ti: 2, tj: 2 }, { op: 'release', face: 1, ti: 0, tj: 0 }, { op: 'sync' }, { op: 'acquire', face: 1, ti: 2, tj: 2 }, { op: 'sync' })
steps.push({ op: 'burnMask', face: 1, I0: 20, J0: 20, w: 4, h: 4 })
const fixture = {
  rev,
  slotsAcross: SLOTS,
  slotsDown: SLOTS,
  lattice: { cellsPerFace: lattice.cellsPerFace },
  refusedConstruct: (() => { try { createFireStageMap({ kernel, lattice: { cellsPerFace: 4, cellOfPlacementId: lattice.cellOfPlacementId } }); return null } catch (e) { return { error: e.message } } })(),
  steps: steps.map((s) => {
    try { return { ...s, expect: exec(s) } } catch (e) { return { ...s, expect: { error: e.message } } }
  }),
}
const statusAfter = git(['status', '--porcelain'])
if (statusAfter !== statusBefore) throw new Error('spoint working tree changed')
fixture.spointStatusBefore = statusBefore
fixture.spointStatusAfter = statusAfter
writeFileSync(outPath, JSON.stringify(fixture, null, 1) + '\n')
console.log(JSON.stringify({ rev, out: outPath, status: 'unchanged', steps: fixture.steps.length, errors: fixture.steps.filter((s) => s.expect && s.expect.error).length, refusedConstruct: fixture.refusedConstruct }))
