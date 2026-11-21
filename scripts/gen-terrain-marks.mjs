import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-terrain-marks.json')
const gitStatus = () => execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
const rev = execFileSync('git', ['-C', spoint, 'rev-parse', '--short', 'HEAD']).toString().trim()
const statusBefore = gitStatus()
const { TERRAIN_MARK_KINDS, wrapHeightWithMarks } = await import(pathToFileURL(join(spoint, 'src/terrain/TerrainMarks.js')).href)

const encode = (value) => JSON.parse(JSON.stringify(value, (_, item) => (typeof item === 'number' && !Number.isFinite(item) ? String(item) : item)))
const outcome = (fn) => {
  try {
    return { ok: fn() }
  } catch (e) {
    if (typeof e.reasonCode !== 'string') throw e
    return { error: { reasonCode: e.reasonCode, index: e.index } }
  }
}
const HEIGHT = (x, z, yGuess) => 1.5 + 0.5 * x - 0.25 * z + 0.125 * yGuess
const POINTS = [[0, 0, 0], [0.25, 0.25, 1], [-2.5, 3, 0.5], [1, -1, -2], [3.5, 4.5, 4], [-3.75, -0.25, 0], [5, 5, 1], [-5, -5, 0.5], [1.5, 1.5, -1], [0.5, -4, 2], [-4, 0.25, 0], [2, -2, 3]]
const runCase = (name, specs) => {
  const run = outcome(() => {
    const height = wrapHeightWithMarks(specs, HEIGHT)
    return POINTS.map(([x, z, yGuess]) => ({ x, z, yGuess, height: height(x, z, yGuess) }))
  })
  return { name, specs, error: run.error ?? null, points: run.ok ?? [] }
}

const V = {
  flatten: { kind: 'flatten', center: [0, 0], radius: 2, falloff: 1, height: 4 },
  pond: { kind: 'pond', center: [3, 1], radius: 1.5, falloff: 0.5, level: -1 },
  lake: { kind: 'lake', center: [-2, 2], radius: 1, falloff: 0, level: 0.5 },
  sea: { kind: 'sea', center: [0, -3], radius: 2, falloff: 1, level: -0.5 },
  river: { kind: 'river', points: [[-4, 0], [0, 0], [4, 1]], width: 1, falloff: 0.5, depth: 0.75 },
  path: { kind: 'path', points: [[-3, -3, 0.5], [3, -3, 2]], width: 1, falloff: 0.5 },
  road: { kind: 'road', points: [[0, 4, 1], [2, 4, 1.5], [4, 2, 1]], width: 0.5, falloff: 0 },
  clear: { kind: 'clear', center: [5, 5], radius: 1.5 },
  hole: { kind: 'hole', center: [-5, -5], radius: 1 },
  grade: { kind: 'grade', center: [1, 1], radius: 3, falloff: 1, level: 2, slope: 0.25, maxGrade: 0.5 },
  spline: { kind: 'spline', points: [[-1, -4, 1], [1, -4, 3]], width: 0.5, falloff: 1 },
  liquid: { kind: 'liquid', center: [0, 0], radius: 6, level: 0 },
}
const CASES = [
  ['empty list', []],
  ['null specs', null],
  ...Object.keys(V).map((kind) => ['kind ' + kind, [V[kind]]]),
  ['all kinds in one list', Object.values(V)],
  ['pond then flatten', [V.pond, V.flatten]],
  ['flatten then pond', [V.flatten, V.pond]],
  ['clear wins over later flatten', [{ kind: 'clear', center: [0, 0], radius: 2 }, { kind: 'flatten', center: [0, 0], radius: 3, falloff: 1, height: 9 }]],
  ['river ignores a third point coordinate', [{ kind: 'river', points: [[0, 0, 9], [4, 0, 9]], width: 1, falloff: 0, depth: 1 }]],
  ['grade slope equal to maxGrade', [{ ...V.grade, slope: 0.5 }]],
  ['not a list string', 'x'],
  ['not a list object', { kind: 'flatten' }],
  ['null mark', [null]],
  ['number mark', [3]],
  ['string mark', ['flatten']],
  ['valid then number mark', [V.flatten, 3]],
  ['array mark', [[]]],
  ['missing kind', [{ center: [0, 0], radius: 1, falloff: 0, height: 1 }]],
  ['kind not a string', [{ ...V.flatten, kind: 3 }]],
  ['kind null', [{ ...V.flatten, kind: null }]],
  ['kind misspelled', [{ ...V.flatten, kind: 'Flatten' }]],
  ['kind constructor', [{ ...V.flatten, kind: 'constructor' }]],
  ['kind __proto__', [{ ...V.flatten, kind: '__proto__' }]],
  ['kind toString', [{ ...V.flatten, kind: 'toString' }]],
  ['flatten radius zero', [{ ...V.flatten, radius: 0 }]],
  ['flatten radius negative', [{ ...V.flatten, radius: -1 }]],
  ['flatten radius string NaN', [{ ...V.flatten, radius: 'NaN' }]],
  ['flatten radius string Infinity', [{ ...V.flatten, radius: 'Infinity' }]],
  ['flatten falloff negative', [{ ...V.flatten, falloff: -0.5 }]],
  ['flatten falloff zero accepted', [{ ...V.flatten, falloff: 0 }]],
  ['flatten height missing', [{ kind: 'flatten', center: [0, 0], radius: 1, falloff: 0 }]],
  ['flatten center short', [{ ...V.flatten, center: [1] }]],
  ['flatten center long', [{ ...V.flatten, center: [1, 2, 3] }]],
  ['flatten center string coordinate', [{ ...V.flatten, center: [1, '2'] }]],
  ['flatten center null', [{ ...V.flatten, center: null }]],
  ['flatten center checked before radius', [{ ...V.flatten, center: [1], radius: 0 }]],
  ['pond level missing', [{ kind: 'pond', center: [0, 0], radius: 1, falloff: 0 }]],
  ['lake center string', [{ ...V.lake, center: '00' }]],
  ['sea level Infinity string', [{ ...V.sea, level: 'Infinity' }]],
  ['river one point', [{ ...V.river, points: [[0, 0]] }]],
  ['river points missing', [{ kind: 'river', width: 1, falloff: 0, depth: 1 }]],
  ['river points string', [{ ...V.river, points: 'ab' }]],
  ['river point too short', [{ ...V.river, points: [[0, 0], [1]] }]],
  ['river point is string', [{ ...V.river, points: [[0, 0], 'ab'] }]],
  ['river point coordinate null', [{ ...V.river, points: [[0, 0], [1, null]] }]],
  ['river width zero', [{ ...V.river, width: 0 }]],
  ['river depth negative', [{ ...V.river, depth: -1 }]],
  ['path level missing', [{ ...V.path, points: [[0, 0], [1, 1, 1]] }]],
  ['path level non-number', [{ ...V.path, points: [[0, 0, 1], [1, 1, 'x']] }]],
  ['path point with four entries', [{ ...V.path, points: [[0, 0, 1, 5], [1, 1, 1]] }]],
  ['path width zero', [{ ...V.path, width: 0 }]],
  ['road falloff negative', [{ ...V.road, falloff: -1 }]],
  ['clear radius zero', [{ kind: 'clear', center: [0, 0], radius: 0 }]],
  ['hole center short', [{ ...V.hole, center: [0] }]],
  ['grade too steep', [{ ...V.grade, slope: 2 }]],
  ['grade too steep negative', [{ ...V.grade, slope: -1 }]],
  ['grade maxGrade negative', [{ ...V.grade, maxGrade: -1 }]],
  ['grade falloff negative with steep slope', [{ ...V.grade, falloff: -1, slope: 2 }]],
  ['grade slope string', [{ ...V.grade, slope: 'x' }]],
  ['steep grade after valid mark', [V.flatten, { ...V.grade, slope: 2 }]],
  ['spline width negative', [{ ...V.spline, width: -1 }]],
  ['spline falloff zero', [{ ...V.spline, falloff: 0 }]],
  ['spline one point', [{ ...V.spline, points: [[0, 0, 1]] }]],
  ['liquid radius zero', [{ ...V.liquid, radius: 0 }]],
  ['liquid center string', [{ ...V.liquid, center: 'x' }]],
  ['liquid level missing', [{ kind: 'liquid', center: [0, 0], radius: 1 }]],
  ['first refusal wins', [{ ...V.flatten, radius: 0 }, { kind: 'nope' }]],
  ['refusal index after accepted marks', [V.pond, V.river, { ...V.clear, radius: -2 }]],
]
const out = {
  spoint: rev,
  kinds: [...TERRAIN_MARK_KINDS],
  cases: CASES.map(([name, specs]) => runCase(name, specs)),
}
if (gitStatus() !== statusBefore) throw new Error('spoint working tree changed')
writeFileSync(outPath, JSON.stringify(encode(out), null, 1) + '\n')
console.log(JSON.stringify({ rev, kinds: out.kinds.length, cases: out.cases.length, refusals: out.cases.filter((c) => c.error).length, withHeights: out.cases.filter((c) => !c.error).length }))
