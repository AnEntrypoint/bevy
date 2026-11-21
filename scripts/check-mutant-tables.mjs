import { readFileSync, readdirSync } from 'node:fs'
import { execFileSync } from 'node:child_process'
import { join } from 'node:path'

const dir = process.argv[2] ?? 'scripts/mutants'
const problems = []
let total = 0
for (const name of readdirSync(dir).filter((f) => f.endsWith('.tsv'))) {
  for (const [index, line] of readFileSync(join(dir, name), 'utf8').split('\n').filter(Boolean).entries()) {
    const cells = line.split('\t')
    total++
    const [mutant, file, old] = cells
    if (cells.some((cell) => cell === '')) { problems.push(name + ':' + (index + 1) + ' has an empty field'); continue }
    if (cells.length !== 5) { problems.push(name + ':' + (index + 1) + ' has ' + cells.length + ' columns'); continue }
    const count = readFileSync(file, 'utf8').split(old).length - 1
    if (count !== 1) problems.push(name + ' ' + mutant + ': old text occurs ' + count + ' times in ' + file)
  }
}
const stub = 'mutant() { printf "%s\\x1f" "$@"; printf "\\n"; }; eval "$(grep "^mutant " scripts/phase-b.sh)"'
for (const line of execFileSync('bash', ['-c', stub], { encoding: 'utf8', maxBuffer: 1 << 26 }).split('\n').filter(Boolean)) {
  const [name, file, old] = line.split('\x1f')
  total++
  const count = readFileSync(file, 'utf8').split(old).length - 1
  if (count !== 1) problems.push('phase-b.sh ' + name + ': old text occurs ' + count + ' times in ' + file)
}
console.log(JSON.stringify({ mutants: total, problems: problems.length }))
for (const p of problems) console.log('PROBLEM ' + p)
process.exit(problems.length === 0 ? 0 : 1)
