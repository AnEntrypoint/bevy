import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'

export async function writeFixture(name, build) {
  const spoint = process.argv[2] ?? '/workspace/spoint'
  const git = (args) => execFileSync('git', ['-C', spoint, ...args]).toString()
  const rev = git(['rev-parse', '--short', 'HEAD']).trim()
  const statusBefore = git(['status', '--porcelain'])
  const outPath = join(import.meta.dirname, '..', '..', 'testdata', 'parity', `${name}.json`)
  const fixture = await build({ spoint, rev })
  const statusAfter = git(['status', '--porcelain'])
  if (statusAfter !== statusBefore) throw new Error('spoint working tree changed')
  fixture.spointStatusBefore = statusBefore
  fixture.spointStatusAfter = statusAfter
  writeFileSync(outPath, JSON.stringify(fixture, null, 1) + '\n')
  console.log(JSON.stringify({ rev, out: outPath, status: 'unchanged' }))
}
