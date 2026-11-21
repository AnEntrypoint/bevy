import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-rock-tri-table.json')
const rev = execFileSync('git', ['-C', spoint, 'rev-parse', '--short', 'HEAD']).toString().trim()
const statusBefore = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
const { TRI_TABLE } = await import(pathToFileURL(join(spoint, 'src/terrain/RockTriTableShared.js')).href)
if (TRI_TABLE.length !== 256) throw new Error('TRI_TABLE has ' + TRI_TABLE.length + ' entries')
const statusAfter = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
if (statusAfter !== statusBefore) throw new Error('spoint working tree changed')
writeFileSync(outPath, JSON.stringify({ spoint: rev, module: 'src/terrain/RockTriTableShared.js', triTable: TRI_TABLE, spointStatusBefore: statusBefore, spointStatusAfter: statusAfter }) + '\n')
console.log(JSON.stringify({ spoint: rev, entries: TRI_TABLE.length }))
