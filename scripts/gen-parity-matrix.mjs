import { readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

const root = join(import.meta.dirname, '..')
const tsvPath = join(root, 'docs', 'parity-matrix.tsv')
const docPath = join(root, 'docs', 'PARITY_MATRIX.md')
const HEADER = ['n', 'section', 'feature', 'source', 'spoint_status', 'outcome', 'bevy_evidence_or_cause', 'item_or_reason']
const COLUMNS = ['#', 'Section', 'Feature', 'spoint source', 'spoint status', 'Outcome', 'bevy evidence or cause', 'Item or reason']
const COUNTED = ['missing', 'not applicable', 'blocked', 'present', 'candidate-only']

const lines = readFileSync(tsvPath, 'utf8').split('\n').filter(Boolean)
const header = lines[0].split('\t')
if (HEADER.some((name, i) => header[i] !== name)) throw new Error('parity-matrix.tsv header changed: ' + header.join(' | '))
const rows = lines.slice(1).map((line) => {
  const cells = line.split('\t')
  if (cells.length !== HEADER.length) throw new Error('row ' + cells[0] + ' has ' + cells.length + ' cells')
  return cells
})

const prose = readFileSync(docPath, 'utf8').split('\n')
const countsAt = prose.findIndex((line) => line.startsWith('Counts: '))
if (countsAt < 0) throw new Error('PARITY_MATRIX.md has no Counts line to keep the prose above')

const tally = (name) => rows.filter((cells) => cells[5] === name).length
const remainingPath = join(root, 'docs', 'REMAINING_PARITY.md')
const open = rows.filter((cells) => cells[5] !== 'not applicable' && cells[5] !== 'present').length
const totals = 'Totals: open ' + open + ', present ' + tally('present') + ', candidate-only ' + tally('candidate-only') + ', blocked ' + tally('blocked') + '.'
const counts = 'Counts: ' + rows.length + ' rows. ' + COUNTED.map((name) => name + ' ' + tally(name)).join(', ') + '.'
const table = ['| ' + COLUMNS.join(' | ') + ' |', '|' + COLUMNS.map(() => '---').join('|') + '|', ...rows.map((cells) => '| ' + cells.map((cell) => cell.replaceAll('|', '\\|')).join(' | ') + ' |')]
const out = [...prose.slice(0, countsAt), counts, '', ...table, ''].join('\n')

const remaining = readFileSync(remainingPath, 'utf8').split('\n')
const totalsAt = remaining.findIndex((line) => line.startsWith('Totals: open '))
if (totalsAt < 0) throw new Error('REMAINING_PARITY.md has no Totals line')
remaining[totalsAt] = totals
const remainingOut = remaining.join('\n')
if (process.argv.includes('--check')) {
  const same = readFileSync(docPath, 'utf8') === out && readFileSync(remainingPath, 'utf8') === remainingOut
  console.log(same ? 'parity docs match the TSV' : 'parity docs differ from the TSV')
  process.exit(same ? 0 : 1)
}
writeFileSync(docPath, out)
writeFileSync(remainingPath, remainingOut)
console.log(counts)
console.log(totals)
