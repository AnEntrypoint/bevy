import { readFileSync, existsSync } from 'node:fs'
import { execFileSync } from 'node:child_process'
import { join } from 'node:path'

const table = process.argv[2] ?? 'docs/planet-parity.md'
const spoint = process.argv[3] ?? '/workspace/spoint'
const ref = /([\w@.][\w@.\-/]*\.(?:js|mjs|cjs|glsl|wgsl|json|rs)):(\d+)(?:-(\d+))?/g
const statusOf = () => execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
const before = statusOf()
const files = new Map()
const linesOf = (root, path) => {
  const key = join(root, path)
  if (!files.has(key)) files.set(key, existsSync(key) ? readFileSync(key, 'utf8').split('\n') : null)
  return files.get(key)
}
const tokensOf = (text) => [...new Set(text.toLowerCase().match(/[a-z_][a-z0-9_]{2,}|\d+(?:\.\d+)?/g) ?? [])]
const rowsOf = (text) => text.split('\n').filter((l) => /^\| [a-z]{3}:[A-Za-z0-9._-]+ \| /.test(l) && !/^\| [a-z]{3}:nf/.test(l)).map((l) => l.slice(2, -2).split(' | '))
const WINDOW = 2
const failures = []
let rows = 0
let refs = 0
for (const cells of rowsOf(readFileSync(table, 'utf8'))) {
  const [id, value, source] = cells.length === 7 ? [cells[0], cells[2], cells[3]] : cells.length === 3 ? cells : [null]
  if (!source || /^spec\b/.test(source)) continue
  rows++
  const cited = [...source.matchAll(ref)]
  if (cited.length === 0) { failures.push(id + ' no source line'); continue }
  const seen = []
  let missing = null
  for (const [, path, from, to] of cited) {
    const lines = linesOf(spoint, path)
    const last = Number(to ?? from)
    if (!lines || last > lines.length || Number(from) < 1) { missing = path + ':' + from; break }
    refs++
    for (let n = Math.max(1, Number(from) - WINDOW); n <= Math.min(lines.length, last + WINDOW); n++) seen.push(lines[n - 1].toLowerCase())
  }
  if (missing) { failures.push(id + ' line does not exist ' + missing); continue }
  const haystack = seen.join('\n')
  if (!tokensOf(value).some((t) => haystack.includes(t))) failures.push(id + ' value not on the cited line: ' + value.slice(0, 60))
}
if (statusOf() !== before) failures.push('spoint working tree changed')
console.log(JSON.stringify({ rows, refs, failures: failures.length }))
for (const f of failures) console.log('FAIL ' + f)
process.exit(failures.length === 0 && rows > 0 ? 0 : 1)
