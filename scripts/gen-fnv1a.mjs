import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const git = (args) => execFileSync('git', ['-C', spoint, ...args]).toString()
const rev = git(['rev-parse', '--short', 'HEAD']).trim()
const statusBefore = git(['status', '--porcelain'])
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-fnv1a.json')
const { FNV1A_32_OFFSET_BASIS, FNV1A_32_PRIME, fnv1aString, fnv1aBytes, fnv1aStepString, fnv1aStepBytes, fnv1aStepFloat32, fnv1aStepFloat64 } = await import(pathToFileURL(join(spoint, 'src/shared/fnv1a.js')).href)
const u32 = (n) => n >>> 0
const fixture = {
  spoint: rev,
  source: 'spoint ' + rev + ': src/shared/fnv1a.js (32-bit FNV-1a; string steps hash UTF-16 code units)',
  offsetBasis: FNV1A_32_OFFSET_BASIS,
  prime: FNV1A_32_PRIME,
  string: ['', 'a', 'spoint', 'héllo', '日本語', '😀', 'a\u0000b', 'terrain 20260', 'é'].map((input) => ({ input, expect: u32(fnv1aString(input)) })),
  bytes: [[], [0], [97, 255], [0xc3, 0xa9], [0xde, 0xad, 0xbe, 0xef], [0, 128, 127, 1]].map((input) => ({ input, expect: u32(fnv1aBytes(Uint8Array.from(input))) })),
  stepString: [{ hash: 0xdeadbeef, input: 'x' }, { hash: 12345, input: 'é😀' }].map(({ hash, input }) => ({ hash, input, expect: u32(fnv1aStepString(hash, input)) })),
  stepBytes: [{ hash: 12345, input: [1, 2, 3] }, { hash: 0xffffffff, input: [200] }].map(({ hash, input }) => ({ hash, input, expect: u32(fnv1aStepBytes(hash, Uint8Array.from(input))) })),
  stepFloat32: [{ hash: FNV1A_32_OFFSET_BASIS, input: 0.5 }, { hash: FNV1A_32_OFFSET_BASIS, input: -2.25 }, { hash: 7, input: 1e10 }, { hash: FNV1A_32_OFFSET_BASIS, input: 3.4e38 }].map(({ hash, input }) => ({ hash, input, expect: u32(fnv1aStepFloat32(hash, input)) })),
  stepFloat64: [{ hash: FNV1A_32_OFFSET_BASIS, input: 0.5, byteOffset: 0 }, { hash: FNV1A_32_OFFSET_BASIS, input: -2.25, byteOffset: 3 }, { hash: 9, input: 1e300, byteOffset: 0 }, { hash: 9, input: 0, byteOffset: 255 }].map(({ hash, input, byteOffset }) => ({ hash, input, byteOffset, expect: u32(fnv1aStepFloat64(hash, input, byteOffset)) })),
}

const statusAfter = git(['status', '--porcelain'])
if (statusAfter !== statusBefore) throw new Error('spoint working tree changed')
fixture.spointStatusBefore = statusBefore
fixture.spointStatusAfter = statusAfter
writeFileSync(outPath, JSON.stringify(fixture, null, 1) + '\n')
console.log(JSON.stringify({ rev, out: outPath, status: 'unchanged' }))
