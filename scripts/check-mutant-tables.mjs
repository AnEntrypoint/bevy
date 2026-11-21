import { readFileSync, readdirSync } from 'node:fs'
import { join } from 'node:path'

const dir = process.argv[2] ?? 'scripts/mutants'
const problems = []
let total = 0
for (const name of readdirSync(dir).filter((f) => f.endsWith('.tsv'))) {
  for (const [index, line] of readFileSync(join(dir, name), 'utf8').split('\n').filter(Boolean).entries()) {
    const cells = line.split('\t')
    total++
    const [mutant, file, old] = cells
    if (cells.length !== 5) { problems.push(name + ':' + (index + 1) + ' has ' + cells.length + ' columns'); continue }
    const count = readFileSync(file, 'utf8').split(old).length - 1
    if (count !== 1) problems.push(name + ' ' + mutant + ': old text occurs ' + count + ' times in ' + file)
  }
}
console.log(JSON.stringify({ mutants: total, problems: problems.length }))
for (const p of problems) console.log('PROBLEM ' + p)
process.exit(problems.length === 0 ? 0 : 1)
