import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const git = (args) => execFileSync('git', ['-C', spoint, ...args]).toString()
const rev = git(['rev-parse', '--short', 'HEAD']).trim()
const statusBefore = git(['status', '--porcelain'])
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-canonical-json.json')
const { canonicalJSON } = await import(pathToFileURL(join(spoint, 'src/shared/canonicalJSON.js')).href)
const cases = [
  { name: 'nestedKeyOrder', input: { zeta: { beta: [1, 2.5, -0.25, 1e-7, 1e-6, 1e20, 1e21, 123456789.125], alpha: null }, alpha: true, '10': 'ten', '2': 'two', '01': 'lead', '4294967295': 'max', '4294967294': 'maxIndex', '': 'empty', 'é': 'e-acute', '😀': 'emoji', '�': 'replacement' } },
  { name: 'stringEscapes', input: { s: 'a"b\\c\n\r\t\b\f\u0001\u001f\u007f é 😀  ' } },
  { name: 'arraysAndEmpties', input: [[], {}, [{}], [[]], null, true, false, 'x'] },
  { name: 'nonFinite', input: { n: NaN, i: Infinity, m: -Infinity, arr: [NaN, 1] } },
  { name: 'undefinedDropped', input: { a: undefined, b: 1, c: [undefined, 2] } },
  { name: 'numberFormats', input: [0, -0, 1, -1, 0.1, 100, 1e21, 1.7976931348623157e308, 5e-324, 2 ** 53, 0.000001, 1234.5e-2] },
  { name: 'topLevelNull', input: null },
  { name: 'topLevelString', input: 'x' },
]
const fixture = {
  spoint: rev,
  source: 'spoint ' + rev + ': src/shared/canonicalJSON.js (JSON.stringify with sorted-key replacer)',
  nonFiniteEncoding: 'NaN, Infinity and -Infinity serialise as null; the fixture input stores the JSON.stringify form (null), so the bevy input is null',
  undefinedEncoding: 'undefined object members are omitted and undefined array items are null; the fixture input stores the JSON.stringify form',
  cases: cases.map(({ name, input }) => ({ name, input, expect: canonicalJSON(input) })),
}

const statusAfter = git(['status', '--porcelain'])
if (statusAfter !== statusBefore) throw new Error('spoint working tree changed')
fixture.spointStatusBefore = statusBefore
fixture.spointStatusAfter = statusAfter
writeFileSync(outPath, JSON.stringify(fixture, null, 1) + '\n')
console.log(JSON.stringify({ rev, out: outPath, status: 'unchanged' }))
