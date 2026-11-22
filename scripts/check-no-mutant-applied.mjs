import { readFileSync, readdirSync } from 'node:fs'
import { execFileSync } from 'node:child_process'
import { join } from 'node:path'

const files = process.argv.slice(2)
const rows = []
for (const name of readdirSync('scripts/mutants').filter((f) => f.endsWith('.tsv'))) {
  for (const line of readFileSync(join('scripts/mutants', name), 'utf8').split('\n').filter(Boolean)) rows.push(line.split('\t'))
}
const stub = 'mutant() { printf "%s\\x1f" "$@"; printf "\\n"; }; eval "$(grep "^mutant " scripts/phase-b.sh)"'
for (const line of execFileSync('bash', ['-c', stub], { encoding: 'utf8', maxBuffer: 1 << 26 }).split('\n').filter(Boolean)) rows.push(line.split('\x1f'))
const applied = rows.filter(([, file, old, replacement]) => (files.length === 0 || files.includes(file)) && !replacement.includes(old) && readFileSync(file, 'utf8').includes(replacement) && !readFileSync(file, 'utf8').includes(old))
console.log(JSON.stringify({ rows: rows.length, applied: applied.length }))
for (const [name, file] of applied) console.log('APPLIED ' + name + ' in ' + file)
process.exit(applied.length === 0 ? 0 : 1)
