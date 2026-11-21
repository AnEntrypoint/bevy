process.env.SPOINT_NO_WATCH = '1'
process.env.SPOINT_SKIP_PREWARM = '1'
import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-baked-field-load.json')
const gitStatus = () => execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
const rev = execFileSync('git', ['-C', spoint, 'rev-parse', '--short', 'HEAD']).toString().trim()
const statusBefore = gitStatus()
const { encodeHeightfield, decodeHeightfield } = await import(pathToFileURL(join(spoint, 'packages/mapspinner/src/heightfield-codec.js')).href)
const { loadBakedHeightField, BakedSourceMissingError } = await import(pathToFileURL(join(spoint, 'src/terrain/TerrainPhysics.js')).href)
const { terrainHashVersionOf, terrainBakeKey, terrainBakeSliceText } = await import(pathToFileURL(join(spoint, 'src/shared/terrainConfig.js')).href)

const tcfg = { radius: 64, reliefScale: 0.001, anchorDir: [0, 1, 0], hashVersion: 2, seed: 7 }
const hashVersion = terrainHashVersionOf(tcfg)
const terrain = { hashVersion, bakeKey: terrainBakeKey(tcfg), bakeSliceText: terrainBakeSliceText(tcfg), radius: tcfg.radius, reliefScale: tcfg.reliefScale, anchorDir: tcfg.anchorDir }

const base = {
  N: 5, extent: 4, center: [0, 0], anchorDir: [0, 1, 0], radius: 64, reliefScale: 0.001, anchorHeight: 1.5,
  seed: 7, hashVersion: 2, codeVersion: '0123456789ab',
  sectors: { gridS: 3, nodesPerSector: 2, bits: 8 },
  sectorMin: [-3.5, -2, -1.25, 0, 0.5, 1, 2.75, 3, 4],
  sectorMax: [0, 1, 1.5, 2.5, 3, 4.25, 5, 6.5, 8],
  q: Array.from({ length: 25 }, (_, i) => (i * 37) % 256),
}
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
const withLength = (bytes, hlen) => { const out = Buffer.from(bytes); out.writeUInt32LE(hlen, 4); return out }
const rawHeader = (text) => { const body = Buffer.from(text); const prefix = Buffer.alloc(8); prefix.writeUInt32LE(0x48464d31, 0); prefix.writeUInt32LE(body.length, 4); return Buffer.concat([prefix, body]) }
const baseBytes = encoded(base)
const CODE = '0123456789ab'
const ALL = ['readBytes', 'heightfieldCodec', 'bakeCodeVersion']
const jsonBase = { N: 3, extent: 2, center: [0, 0], heights: [0, 1, 2, 3, 4, 5, 6, 7, 8], codeVersion: CODE, hashVersion: 2, radius: 64, reliefScale: 0.001, anchorDir: [0, 1, 0], chartEpoch: 0 }
const jsonText = (value) => Buffer.from(JSON.stringify(value))
const hf = (over) => ({ url: 'case.hf', supplied: ALL, bytes: baseBytes, bakeCode: CODE, hashVersion, frameEpoch: 0, expect: 'loaded', ...over })
const json = (over) => ({ url: 'case.json', supplied: ALL, bytes: jsonText(jsonBase), bakeCode: CODE, hashVersion, frameEpoch: 0, expect: 'loaded', ...over })
const text = (s) => Buffer.from(s)

const CASES = [
  hf({ name: 'an empty url reads nothing', url: '', expect: 'absent' }),
  hf({ name: 'a source without readBytes is refused', supplied: [], expect: 'missing' }),
  hf({ name: 'a source without heightfieldCodec is refused', supplied: ['readBytes', 'bakeCodeVersion'], expect: 'missing' }),
  hf({ name: 'a source without bakeCodeVersion is refused', supplied: ['readBytes', 'heightfieldCodec'], expect: 'missing' }),
  hf({ name: 'a null source is refused', supplied: null, expect: 'missing' }),
  hf({ name: 'an absent file reads as absent', bytes: null, expect: 'absent' }),
  hf({ name: 'a matching bake loads with its code version verified' }),
  hf({ name: 'a bake loads unverified when the runtime has no bake code', bakeCode: null }),
  hf({ name: 'an upper-case extension is read as a heightfield', url: 'CASE.HF' }),
  hf({ name: 'a stale bake is an error', bakeCode: 'ffffffffffff', expect: 'error' }),
  hf({ name: 'a hashVersion mismatch is ignored with a warning', hashVersion: 1, expect: 'ignored' }),
  hf({ name: 'a bake without a code version is ignored when the runtime has no bake code', bakeCode: null, bytes: withHeaderField(baseBytes, 'codeVersion', undefined), expect: 'ignored' }),
  hf({ name: 'a radius mismatch is ignored', bytes: encoded({ ...base, radius: 65 }), expect: 'ignored' }),
  hf({ name: 'a frame at another chart epoch is ignored', frameEpoch: 4, expect: 'ignored' }),
  hf({ name: 'a terrainKey from another world is ignored', bytes: encoded({ ...base, terrainKey: 'feedface' }), expect: 'ignored' }),
  hf({ name: 'a buffer shorter than the header prefix is an error', bytes: Buffer.from([1, 2, 3, 4]), expect: 'error' }),
  hf({ name: 'a bad magic is an error', bytes: Buffer.concat([Buffer.from([0x00]), baseBytes.subarray(1)]), expect: 'error' }),
  hf({ name: 'a header length past the file is an error', bytes: withLength(baseBytes, 9999), expect: 'error' }),
  hf({ name: 'an unreadable header is an error', bytes: rawHeader('{{{'), expect: 'error', engineDetail: '[baked-heightfield] case.hf: unreadable header: ' }),
  hf({ name: 'a header N below 2 is an error', bytes: withHeaderField(baseBytes, 'N', 1), expect: 'error' }),
  hf({ name: 'a string N is an error', bytes: withHeaderField(baseBytes, 'N', '5'), expect: 'error' }),
  hf({ name: 'a header without sectors is an error', bytes: withHeaderField(baseBytes, 'sectors', undefined), expect: 'error' }),
  hf({ name: 'a sectors object without gridS is an error', bytes: withHeaderField(baseBytes, 'sectors', { bits: 8, nodesPerSector: 2 }), expect: 'error' }),
  hf({ name: 'sectors.bits other than 8 or 16 is an error', bytes: withHeaderField(baseBytes, 'sectors', { bits: 12, gridS: 3, nodesPerSector: 2 }), expect: 'error' }),
  hf({ name: 'a file shorter than its header describes is an error', bytes: baseBytes.subarray(0, baseBytes.length - 1), expect: 'error' }),
  hf({ name: 'a file longer than its header describes is an error', bytes: Buffer.concat([baseBytes, Buffer.alloc(3)]), expect: 'error' }),
  json({ name: 'a heights json bake loads with its code version verified' }),
  json({ name: 'a json bake that is not valid JSON is an error', bytes: text('{'), expect: 'error', engineDetail: '[baked-heightfield] case.json: not valid JSON: ' }),
  json({ name: 'a json null is an error', bytes: text('null'), expect: 'error' }),
  json({ name: 'a json array has no N and is an error', bytes: text('[]'), expect: 'error' }),
  json({ name: 'a json N of zero is an error', bytes: jsonText({ ...jsonBase, N: 0 }), expect: 'error' }),
  json({ name: 'a json without heights is an error', bytes: jsonText({ ...jsonBase, heights: undefined }), expect: 'error' }),
  json({ name: 'a json with sectors and no heights or sectors.q names both', bytes: jsonText({ N: 3, sectors: { bits: 8, gridS: 1, nodesPerSector: 3 } }), expect: 'error' }),
  json({ name: 'a json heights array of the wrong length is an error', bytes: jsonText({ ...jsonBase, heights: jsonBase.heights.slice(1) }), expect: 'error' }),
  json({ name: 'a json bake with a hashVersion mismatch is ignored', hashVersion: 1, expect: 'ignored' }),
]

const radicalInverse = (k, b) => { let f = 1, r = 0; while (k > 0) { f /= b; r += f * (k % b); k = Math.floor(k / b) } return r }
const PROBES = 8
const num = (v) => (Number.isNaN(v) ? 'NaN' : v)
const toArrayBuffer = (buf) => buf.buffer.slice(buf.byteOffset, buf.byteOffset + buf.byteLength)
const sourceOf = (c) => {
  if (c.supplied === null) return null
  const fns = {
    readBytes: async () => (c.bytes === null ? null : toArrayBuffer(c.bytes)),
    heightfieldCodec: async () => ({ decodeHeightfield }),
    bakeCodeVersion: async () => c.bakeCode,
  }
  return Object.fromEntries(c.supplied.map((name) => [name, fns[name]]))
}
const runCase = async (c) => {
  const warnings = []
  const warn = console.warn
  console.warn = (...args) => { warnings.push(args.map(String).join(' ')) }
  let outcome
  try {
    const field = await loadBakedHeightField(c.url, c.hashVersion, tcfg, { chartEpoch: c.frameEpoch }, sourceOf(c))
    if (field === null) {
      outcome = { kind: warnings.length === 0 ? 'absent' : 'ignored' }
    } else {
      const points = Array.from({ length: PROBES }, (_, k) => [(radicalInverse(k + 1, 2) - 0.5) * field.extent * 1.3, (radicalInverse(k + 1, 3) - 0.5) * field.extent * 1.3])
      outcome = {
        kind: 'loaded', N: field.N, extent: field.extent, chartEpoch: field.chartEpoch, codeVersionVerified: field.codeVersionVerified === true,
        probes: points.map(([x, z]) => ({ x, z, covers: field.covers(x, z), height: num(field.heightAtLocal(x, z)) })),
      }
    }
  } catch (e) {
    if (e instanceof BakedSourceMissingError) outcome = { kind: 'missing', message: e.message, errorName: e.name, errorCode: e.code, missing: e.missing }
    else outcome = { kind: 'error', message: e.message }
  } finally {
    console.warn = warn
  }
  return { ...outcome, warnings }
}

const cases = []
for (const c of CASES) {
  const result = await runCase(c)
  if (result.kind !== c.expect) throw new Error('spoint gave ' + result.kind + ' for "' + c.name + '", expected ' + c.expect)
  if (c.engineDetail) {
    if (!result.message.startsWith(c.engineDetail)) throw new Error('spoint message "' + result.message + '" for "' + c.name + '" lacks the expected head')
    result.message = c.engineDetail
    result.messageIsPrefix = true
  }
  cases.push({
    name: c.name, url: c.url, supplied: c.supplied ?? [], bytesHex: c.bytes === null ? null : c.bytes.toString('hex'),
    bakeCode: c.bakeCode, hashVersion: c.hashVersion, frameEpoch: c.frameEpoch, ...result,
  })
}

const statusAfter = gitStatus()
if (statusAfter !== statusBefore) throw new Error('spoint working tree changed')
writeFileSync(outPath, JSON.stringify({ spoint: rev, module: 'src/terrain/TerrainPhysics.js', spointStatusBefore: statusBefore, spointStatusAfter: statusAfter, terrain, cases }, null, 1) + '\n')
console.log(JSON.stringify({ spoint: rev, hashVersion, cases: cases.map((c) => [c.name, c.kind]) }))
