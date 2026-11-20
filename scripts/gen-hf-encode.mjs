import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-hf-encode.json')
const rev = execFileSync('git', ['-C', spoint, 'rev-parse', '--short', 'HEAD']).toString().trim()
const statusBefore = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
const { encodeHeightfield } = await import(pathToFileURL(join(spoint, 'packages/mapspinner/src/heightfield-codec.js')).href)

const base = { extent: 4, center: [0, 0], anchorDir: [0, 1, 0], radius: 64, reliefScale: 0.001, anchorHeight: 1.5, seed: 7, hashVersion: 2, terrainKey: 'abcd1234', codeVersion: '0123456789ab' }
const CASES = [
  { ...base, N: 3, sectors: { gridS: 2, nodesPerSector: 2, bits: 8 }, sectorMin: [0, 0.5, 1, 1.5], sectorMax: [1, 2, 3, 5], q: [0, 1, 2, 3, 4, 5, 6, 7, 255] },
  { ...base, codeVersion: 'abcdef', N: 3, sectors: { gridS: 2, nodesPerSector: 2, bits: 8 }, sectorMin: [0, 0.25, 1, 2], sectorMax: [1, 1.5, 3, 4], q: [9, 8, 7, 6, 5, 4, 3, 2, 1] },
  { ...base, N: 5, extent: 8, radius: 6360, reliefScale: null, anchorHeight: -12.25, seed: 1337, terrainKey: 'feedface', sectors: { gridS: 3, nodesPerSector: 2, bits: 16 }, sectorMin: [-3.5, -2, -1.25, 0, 0.5, 1, 2.75, 3, 4], sectorMax: [0, 1, 1.5, 2.5, 3, 4.25, 5, 6.5, 8], q: Array.from({ length: 25 }, (_, i) => (i * 2731) % 65536) },
]
const cases = CASES.map((artifact) => {
  const bytes = Buffer.from(encodeHeightfield(artifact))
  const headerLength = bytes.readUInt32LE(4)
  const header = bytes.subarray(8, 8 + headerLength).toString('utf8').replace(/ +$/, '')
  return { header, pad: headerLength - header.length, bits: artifact.sectors.bits, sectorMin: artifact.sectorMin, sectorMax: artifact.sectorMax, q: artifact.q, bytesHex: bytes.toString('hex') }
})
if (!cases.some((c) => c.pad > 0)) throw new Error('no case exercises the header padding')

const statusAfter = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
if (statusAfter !== statusBefore) throw new Error('spoint working tree changed')
writeFileSync(outPath, JSON.stringify({ spoint: rev, module: 'packages/mapspinner/src/heightfield-codec.js', spointStatusBefore: statusBefore, spointStatusAfter: statusAfter, cases }) + '\n')
console.log(JSON.stringify({ spoint: rev, cases: cases.map((c) => ({ bits: c.bits, pad: c.pad, bytes: c.bytesHex.length / 2 })) }))
