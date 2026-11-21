process.env.SPOINT_NO_WATCH = '1'
process.env.SPOINT_SKIP_PREWARM = '1'
import { execFileSync } from 'node:child_process'
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-lattice-surface.json')
const gitStatus = () => execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
const rev = execFileSync('git', ['-C', spoint, 'rev-parse', '--short', 'HEAD']).toString().trim()
const statusBefore = gitStatus()
const { encodeHeightfield } = await import(pathToFileURL(join(spoint, 'packages/mapspinner/src/heightfield-codec.js')).href)
const { loadLatticeSurface } = await import(pathToFileURL(join(spoint, 'src/terrain/LatticeSurface.js')).href)
const { terrainHashVersionOf, terrainBakeKey } = await import(pathToFileURL(join(spoint, 'src/shared/terrainConfig.js')).href)

const tcfg = { radius: 64, reliefScale: 0.001, anchorDir: [0, 1, 0], hashVersion: 2, seed: 7 }
const hashVersion = terrainHashVersionOf(tcfg)
const bakeKey = terrainBakeKey(tcfg)

const base = {
  N: 5, extent: 4, center: [0, 0], anchorDir: [0, 1, 0], radius: 64, reliefScale: 0.001, anchorHeight: 1.5,
  seed: 7, hashVersion: 2, codeVersion: '0123456789ab',
  sectors: { gridS: 3, nodesPerSector: 2, bits: 8 },
  sectorMin: [-3.5, -2, -1.25, 0, 0.5, 1, 2.75, 3, 4],
  sectorMax: [0, 1, 1.5, 2.5, 3, 4.25, 5, 6.5, 8],
  q: Array.from({ length: 25 }, (_, i) => (i * 37) % 256),
}
const bits16 = { ...base, sectors: { gridS: 3, nodesPerSector: 2, bits: 16 }, q: Array.from({ length: 25 }, (_, i) => (i * 2731) % 65536) }

const encoded = (artifact) => Buffer.from(encodeHeightfield(artifact))
const withHeaderField = (bytes, field, value) => {
  const hlen = bytes.readUInt32LE(4)
  const header = JSON.parse(bytes.subarray(8, 8 + hlen).toString('utf8'))
  header[field] = value
  const hbytes = Buffer.from(JSON.stringify(header), 'utf8')
  const pad = (4 - ((8 + hbytes.length) & 3)) & 3
  const padded = Buffer.concat([hbytes, Buffer.alloc(pad, 0x20)])
  const prefix = Buffer.alloc(8)
  prefix.writeUInt32LE(bytes.readUInt32LE(0), 0)
  prefix.writeUInt32LE(padded.length, 4)
  return Buffer.concat([prefix, padded, bytes.subarray(8 + hlen)])
}
const baseBytes = encoded(base)
const CASES = [
  { name: 'accepts a matching bake', bytes: baseBytes, frameEpoch: 0, expect: true },
  { name: 'accepts a bake whose terrainKey is the world bake key', bytes: encoded({ ...base, terrainKey: bakeKey }), frameEpoch: 0, expect: true },
  { name: 'refuses a terrainKey from another world', bytes: encoded({ ...base, terrainKey: 'feedface' }), frameEpoch: 0, expect: false },
  { name: 'refuses a radius mismatch', bytes: encoded({ ...base, radius: 65 }), frameEpoch: 0, expect: false },
  { name: 'refuses a reliefScale mismatch', bytes: encoded({ ...base, reliefScale: 0.002 }), frameEpoch: 0, expect: false },
  { name: 'refuses a null reliefScale against a set reliefScale', bytes: encoded({ ...base, reliefScale: null }), frameEpoch: 0, expect: false },
  { name: 'refuses a different anchorDir', bytes: encoded({ ...base, anchorDir: [1, 0, 0] }), frameEpoch: 0, expect: false },
  { name: 'accepts an anchorDir with the same direction at another length', bytes: encoded({ ...base, anchorDir: [0, 2, 0] }), frameEpoch: 0, expect: true },
  { name: 'refuses a hashVersion mismatch', bytes: encoded({ ...base, hashVersion: 1 }), frameEpoch: 0, expect: false },
  { name: 'refuses a bake when the frame is at another chart epoch', bytes: baseBytes, frameEpoch: 4, expect: false },
  { name: 'accepts a header chartEpoch that the frame is at', bytes: withHeaderField(baseBytes, 'chartEpoch', 4), frameEpoch: 4, expect: true },
  { name: 'refuses a header chartEpoch the frame is not at', bytes: withHeaderField(baseBytes, 'chartEpoch', 4), frameEpoch: 0, expect: false },
  { name: 'accepts a 16-bit sectorized bake', bytes: encoded(bits16), frameEpoch: 0, expect: true },
  { name: 'refuses a truncated buffer', bytes: baseBytes.subarray(0, baseBytes.length - 1), frameEpoch: 0, expect: false },
  { name: 'refuses a bad magic', bytes: Buffer.concat([Buffer.from([0x00]), baseBytes.subarray(1)]), frameEpoch: 0, expect: false },
  { name: 'refuses a buffer shorter than the prefix', bytes: Buffer.from([1, 2, 3, 4]), frameEpoch: 0, expect: false },
]

const radicalInverse = (k, b) => { let f = 1, r = 0; while (k > 0) { f /= b; r += f * (k % b); k = Math.floor(k / b) } return r }
const PROBES = 24
const num = (v) => (Number.isNaN(v) ? 'NaN' : v)
const cwd = process.cwd()
const dir = mkdtempSync(join(tmpdir(), 'lattice-surface-'))
const cases = []
try {
  process.chdir(dir)
  for (const [index, c] of CASES.entries()) {
    const file = `case-${index}.hf`
    writeFileSync(join(dir, file), c.bytes)
    const frame = { chartEpoch: c.frameEpoch }
    const surface = await loadLatticeSurface({ url: `/${file}`, tcfg, frame })
    const record = { name: c.name, bytesHex: c.bytes.toString('hex'), frameEpoch: c.frameEpoch, loaded: surface !== null }
    if (record.loaded !== c.expect) throw new Error(`spoint loaded=${record.loaded} for "${c.name}", expected ${c.expect}`)
    if (surface) {
      const points = Array.from({ length: PROBES }, (_, k) => [(radicalInverse(k + 1, 2) - 0.5) * surface.extent * 1.3, (radicalInverse(k + 1, 3) - 0.5) * surface.extent * 1.3])
      record.N = surface.N
      record.extent = surface.extent
      record.stepM = surface.stepM
      record.chartEpoch = surface.chartEpoch
      record.liveEpoch = surface.liveEpoch
      record.kind = surface.kind
      record.probes = points.map(([x, z]) => ({ x, z, covers: surface.covers(x, z), height: num(surface.heightAtLocal(x, z)) }))
      frame.chartEpoch = c.frameEpoch + 2
      record.staleProbes = points.map(([x, z]) => ({ covers: surface.covers(x, z), height: num(surface.heightAtLocal(x, z)) }))
      frame.chartEpoch = c.frameEpoch
    }
    cases.push(record)
  }
} finally {
  process.chdir(cwd)
  rmSync(dir, { recursive: true, force: true })
}

const statusAfter = gitStatus()
if (statusAfter !== statusBefore) throw new Error('spoint working tree changed')
writeFileSync(outPath, JSON.stringify({ spoint: rev, module: 'src/terrain/LatticeSurface.js', spointStatusBefore: statusBefore, spointStatusAfter: statusAfter, terrain: { radius: tcfg.radius, reliefScale: tcfg.reliefScale, anchorDir: [0, 1, 0], hashVersion, bakeKey }, cases }) + '\n')
console.log(JSON.stringify({ spoint: rev, hashVersion, bakeKey, cases: cases.map((c) => [c.name, c.loaded, c.probes ? c.probes.filter((p) => p.covers).length : 0]) }))
