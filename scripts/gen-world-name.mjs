import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const git = (args) => execFileSync('git', ['-C', spoint, ...args]).toString()
const rev = git(['rev-parse', '--short', 'HEAD']).trim()
const statusBefore = git(['status', '--porcelain'])
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-world-name.json')
const { isWorldName, worldFileRel, worldFileCandidates, defaultWorldNameOf, WORLD_DIR, WORLD_INDEX, WORLD_FIXTURES_DIR } = await import(pathToFileURL(join(spoint, 'src/shared/worldName.js')).href)
const outcome = (fn) => {
  try {
    return { ok: true, value: fn() }
  } catch (e) {
    return { ok: false, name: e.name, message: e.message }
  }
}
const nameInputs = ['index', 'a-b_C9', 'ABC123_-z', 'x'.repeat(128), 'x'.repeat(129), '', 'a b', 'a.b', 'é', 'x\n', 'a/b', '../x', null, 5, ['a']]
const moduleInputs = [null, {}, { defaultWorld: 'town' }, { defaultWorld: 'bad name' }, { defaultWorld: null }, { defaultWorld: 7 }, { defaultWorld: '' }, { defaultWorld: 'x'.repeat(128) }, { defaultWorld: 'x'.repeat(129) }]
const fixture = {
  spoint: rev,
  source: 'spoint ' + rev + ': src/shared/worldName.js (isWorldName, worldFileRel, worldFileCandidates, defaultWorldNameOf)',
  worldDir: WORLD_DIR,
  worldIndex: WORLD_INDEX,
  worldFixturesDir: WORLD_FIXTURES_DIR,
  isWorldName: nameInputs.map((input) => ({ input, expect: isWorldName(input) })),
  worldFileRel: nameInputs.map((input) => ({ input, expect: outcome(() => worldFileRel(input)) })),
  worldFileCandidates: nameInputs.map((input) => ({ input, expect: outcome(() => worldFileCandidates(input)) })),
  defaultWorldNameOf: moduleInputs.map((input) => ({ input, expect: outcome(() => defaultWorldNameOf(input)) })),
}

const statusAfter = git(['status', '--porcelain'])
if (statusAfter !== statusBefore) throw new Error('spoint working tree changed')
fixture.spointStatusBefore = statusBefore
fixture.spointStatusAfter = statusAfter
writeFileSync(outPath, JSON.stringify(fixture, null, 1) + '\n')
console.log(JSON.stringify({ rev, out: outPath, status: 'unchanged' }))
