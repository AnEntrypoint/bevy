import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'
const spoint = process.argv[2] ?? '/workspace/spoint'
const git = (args) => execFileSync('git', ['-C', spoint, ...args]).toString()
const rev = git(['rev-parse', '--short', 'HEAD']).trim()
const statusBefore = git(['status', '--porcelain'])
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-fire-terrain.json')
const { createFirebreakFuel } = await import(pathToFileURL(join(spoint, 'src/behaviours/fireTerrain.js')).href)

const SAMPLERS = {
  heightAt: { ramp: (x, z) => 0.5 * x - 0.25 * z },
  seaLevelAt: { zero: () => 0 },
  kindAt: { band: (x, z) => (x < -6 ? 'road' : z > 8 ? 'river' : x > 14 ? 'rock' : x > 9 ? 'grass' : null) },
  rock: { edge: (x) => x > 25 },
}
const named = (slot, name) => (name === null ? null : SAMPLERS[slot][name])
const FACES = [-3, 0, 1, 2]
const FRAME_VECTORS = { up: [0, 0, 1], east: [1, 0, 0], north: [0, 1, 0] }
const lattice = {
  cellCentreDir(face, I, J, out) {
    const x = I - 3, y = J - 3, z = 2 + face
    const n = Math.sqrt(x * x + y * y + z * z)
    out[0] = x / n; out[1] = y / n; out[2] = z / n
  },
}
const baseClassAt = (face, I, J) => (I + 2 * J + face + 8) % 4
const NONE = { heightAt: null, seaLevelAt: null, kindAt: null }
const ALL = { heightAt: 'ramp', seaLevelAt: 'zero', kindAt: 'band' }
const cases = [
  { name: 'all-breaks', radius: 40, anchorHeight: 0.5, breaks: { water: true, kinds: ['road', 'river'], cleared: [{ center: [10, 5], radiusM: 9 }], rock: 'edge', ...NONE }, terrain: ALL },
  { name: 'water-only', radius: 40, anchorHeight: 0.5, breaks: { water: true, kinds: [], cleared: [], rock: null, ...NONE }, terrain: { heightAt: 'ramp', seaLevelAt: 'zero', kindAt: null } },
  { name: 'kinds-only', radius: 40, anchorHeight: 0.5, breaks: { water: false, kinds: ['river'], cleared: [], rock: null, ...NONE }, terrain: { heightAt: null, seaLevelAt: null, kindAt: 'band' } },
  { name: 'none', radius: 40, anchorHeight: 0.5, breaks: { water: false, kinds: [], cleared: [], rock: null, ...NONE }, terrain: NONE },
  { name: 'rock-only-no-samplers', radius: 40, anchorHeight: 0.5, breaks: { water: false, kinds: [], cleared: [], rock: 'edge', ...NONE }, terrain: NONE },
  { name: 'breaks-override-terrain', radius: 40, anchorHeight: 0.5, breaks: { water: true, kinds: ['road'], cleared: [], rock: 'edge', heightAt: 'ramp', seaLevelAt: 'zero', kindAt: 'band' }, terrain: NONE },
  { name: 'anchor-high', radius: 40, anchorHeight: 6, breaks: { water: true, kinds: ['road', 'river'], cleared: [{ center: [12, -4], radiusM: 6 }, { center: [-8, 3], radiusM: 4 }], rock: null, ...NONE }, terrain: ALL },
  { name: 'refused-radius-zero', radius: 0, anchorHeight: 0.5, breaks: { water: true, kinds: ['road'], cleared: [], rock: null, ...NONE }, terrain: ALL },
  { name: 'refused-radius-negative', radius: -5, anchorHeight: 0.5, breaks: { water: false, kinds: [], cleared: [], rock: null, ...NONE }, terrain: NONE },
  { name: 'refused-water-no-sea-level', radius: 40, anchorHeight: 0.5, breaks: { water: true, kinds: [], cleared: [], rock: null, ...NONE }, terrain: { heightAt: 'ramp', seaLevelAt: null, kindAt: 'band' } },
  { name: 'refused-water-no-height', radius: 40, anchorHeight: 0.5, breaks: { water: true, kinds: [], cleared: [], rock: null, ...NONE }, terrain: { heightAt: null, seaLevelAt: 'zero', kindAt: null } },
  { name: 'refused-kinds-no-kindAt', radius: 40, anchorHeight: 0.5, breaks: { water: false, kinds: ['road'], cleared: [], rock: null, ...NONE }, terrain: { heightAt: 'ramp', seaLevelAt: 'zero', kindAt: null } },
]
const run = (c) => {
  const breaks = {
    water: c.breaks.water, kinds: new Set(c.breaks.kinds), cleared: c.breaks.cleared,
    rock: named('rock', c.breaks.rock ?? null), heightAt: named('heightAt', c.breaks.heightAt ?? null),
    seaLevelAt: named('seaLevelAt', c.breaks.seaLevelAt ?? null), kindAt: named('kindAt', c.breaks.kindAt ?? null),
  }
  const terrain = { heightAt: named('heightAt', c.terrain.heightAt), seaLevelAt: named('seaLevelAt', c.terrain.seaLevelAt), kindAt: named('kindAt', c.terrain.kindAt) }
  const frame = { radius: c.radius, anchorHeight: c.anchorHeight, ...FRAME_VECTORS }
  const classAt = createFirebreakFuel({ baseClassAt, breaks, frame, lattice, terrain })
  const classes = []
  for (const face of FACES) for (let I = 0; I < 7; I++) for (let J = 0; J < 7; J++) classes.push(classAt(face, I, J))
  return classes
}
const outcome = (c) => {
  try {
    return { ok: true, classes: run(c) }
  } catch (e) {
    return { ok: false, name: e.name, message: e.message }
  }
}
const fixture = {
  rev,
  frame: FRAME_VECTORS,
  grid: { faces: FACES, I: [0, 6], J: [0, 6] },
  cases: cases.map((c) => ({ ...c, expect: outcome(c) })),
}
const statusAfter = git(['status', '--porcelain'])
if (statusAfter !== statusBefore) throw new Error('spoint working tree changed')
fixture.spointStatusBefore = statusBefore
fixture.spointStatusAfter = statusAfter
writeFileSync(outPath, JSON.stringify(fixture, null, 1) + '\n')
console.log(JSON.stringify({ rev, out: outPath, status: 'unchanged', cases: cases.length, refused: fixture.cases.filter((c) => !c.expect.ok).length }))
