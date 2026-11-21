import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-brush-result.json')
const gitStatus = () => execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
const rev = execFileSync('git', ['-C', spoint, 'rev-parse', '--short', 'HEAD']).toString().trim()
const statusBefore = gitStatus()
const { brushPrecondition, brushApplied, STRENGTH_REQUIRES } = await import(pathToFileURL(join(spoint, 'src/terrain/brushResult.js')).href)

const attempt = (run) => {
  try {
    return { threw: false, value: run() }
  } catch (err) {
    return { threw: true, message: err.message }
  }
}

const PAINT_CLAUSES = ['x is finite', 'z is finite', 'radius is finite and positive']
const PRECONDITIONS = [
  { name: 'all clauses hold', requires: PAINT_CLAUSES, holds: [true, true, true] },
  { name: 'middle clause false', requires: PAINT_CLAUSES, holds: [true, false, true] },
  { name: 'first clause false', requires: PAINT_CLAUSES, holds: [false, true, true] },
  { name: 'strength clause false', requires: STRENGTH_REQUIRES, holds: [false] },
  { name: 'length mismatch throws', requires: PAINT_CLAUSES, holds: [true, true] },
]
const APPLIED = [
  { name: 'zero touched without stroke', strokes: ['s0'], touched: 0, stroke: undefined },
  { name: 'zero touched ignores stroke', strokes: [], touched: 0, stroke: 's1' },
  { name: 'positive touched with stroke', strokes: ['s0'], touched: 3, stroke: 's1' },
  { name: 'positive touched without stroke throws', strokes: [], touched: 2, stroke: undefined },
  { name: 'fractional touched throws', strokes: [], touched: 1.5, stroke: 's1' },
  { name: 'negative touched throws', strokes: [], touched: -1, stroke: 's1' },
]

const preconditionCases = PRECONDITIONS.map(({ name, requires, holds }) => ({
  name,
  input: { requires: [...requires], holds: [...holds] },
  result: attempt(() => brushPrecondition(requires, holds)),
}))
const appliedCases = APPLIED.map(({ name, strokes, touched, stroke }) => {
  const target = [...strokes]
  const input = { strokes: [...strokes], touched, stroke: stroke ?? null }
  const result = attempt(() => brushApplied(target, touched, stroke))
  return { name, input, result, strokesAfter: target }
})

if (gitStatus() !== statusBefore) throw new Error('spoint working tree changed')
writeFileSync(outPath, JSON.stringify({ spoint: rev, strengthRequires: [...STRENGTH_REQUIRES], preconditions: preconditionCases, applied: appliedCases }, null, 1) + '\n')
console.log(JSON.stringify({ rev, preconditions: preconditionCases.length, applied: appliedCases.length }))
