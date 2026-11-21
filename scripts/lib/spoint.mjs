import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

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

export async function writeParity(build, { indent = 1, status = true } = {}) {
  const spoint = process.argv[2] ?? '/workspace/spoint'
  const git = (args) => execFileSync('git', ['-C', spoint, ...args]).toString()
  const rev = git(['rev-parse', '--short', 'HEAD']).trim()
  const statusBefore = git(['status', '--porcelain'])
  const outputs = await build({ spoint, rev, statusBefore })
  if (git(['status', '--porcelain']) !== statusBefore) throw new Error('spoint working tree changed')
  for (const [name, value] of Object.entries(outputs)) {
    const body = typeof value === 'string' || Buffer.isBuffer(value)
      ? value
      : JSON.stringify(status ? { ...value, spointStatusBefore: statusBefore, spointStatusAfter: statusBefore } : value, null, indent) + '\n'
    writeFileSync(join(import.meta.dirname, '..', '..', 'testdata', 'parity', name), body)
  }
  console.log(JSON.stringify({ rev, outputs: Object.keys(outputs) }))
}

export function spointImport(spoint, rel) {
  return import(pathToFileURL(join(spoint, rel)).href)
}

export function spointNode(spoint, args) {
  return execFileSync('node', args, { cwd: spoint, stdio: ['ignore', 'pipe', 'pipe'] })
}
