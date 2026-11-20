import { execFileSync } from 'node:child_process'
import { createRequire } from 'node:module'
import { mkdirSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const outDir = join(import.meta.dirname, '..', 'testdata', 'parity')
const rev = execFileSync('git', ['-C', spoint, 'rev-parse', '--short', 'HEAD']).toString().trim()
const require = createRequire(join(spoint, 'package.json'))
const imported = await import(pathToFileURL(require.resolve('meshoptimizer')).href)
const meshopt = imported.MeshoptEncoder ? imported : imported.default
const { MeshoptEncoder, MeshoptDecoder } = meshopt
await MeshoptEncoder.ready
await MeshoptDecoder.ready

const vertexCount = 8
const vertexStride = 12
const positions = new Float32Array(vertexCount * 3)
for (let i = 0; i < vertexCount; i++) {
  positions[i * 3] = i * 0.5
  positions[i * 3 + 1] = Math.sin(i) * 2
  positions[i * 3 + 2] = i * i * 0.1
}
const indices = Uint16Array.from([0, 1, 2, 2, 1, 3, 4, 5, 6, 6, 5, 7, 0, 4, 1, 1, 4, 5])
const sourceVertices = new Uint8Array(positions.buffer)
const sourceIndices = new Uint8Array(indices.buffer)
const encodedVertices = MeshoptEncoder.encodeVertexBuffer(sourceVertices, vertexCount, vertexStride)
const encodedIndices = MeshoptEncoder.encodeIndexBuffer(sourceIndices, indices.length, 2)
const decodedVertices = new Uint8Array(vertexCount * vertexStride)
MeshoptDecoder.decodeVertexBuffer(decodedVertices, vertexCount, vertexStride, encodedVertices)
const decodedIndices = new Uint8Array(indices.length * 2)
MeshoptDecoder.decodeIndexBuffer(decodedIndices, indices.length, 2, encodedIndices)
if (Buffer.compare(Buffer.from(decodedVertices), Buffer.from(sourceVertices)) !== 0) throw new Error('spoint vertex decode does not reproduce its input')
const triangles = (list) => { const out = []; for (let t = 0; t < list.length; t += 3) { const tri = [list[t], list[t + 1], list[t + 2]]; const r = tri.indexOf(Math.min(...tri)); out.push(tri.slice(r).concat(tri.slice(0, r)).join(',')) } return out.sort().join('|') }
if (triangles(indices) !== triangles(new Uint16Array(decodedIndices.buffer))) throw new Error('spoint index decode does not describe the same triangles')

const align4 = (n) => (n + 3) & ~3
const indexOffset = align4(encodedVertices.length)
const bin = Buffer.alloc(align4(indexOffset + encodedIndices.length))
Buffer.from(encodedVertices).copy(bin, 0)
Buffer.from(encodedIndices).copy(bin, indexOffset)

const json = {
  asset: { version: '2.0' },
  extensionsUsed: ['EXT_meshopt_compression'],
  extensionsRequired: ['EXT_meshopt_compression'],
  buffers: [{ byteLength: bin.length }],
  bufferViews: [
    { buffer: 0, byteLength: vertexCount * vertexStride, byteStride: vertexStride, extensions: { EXT_meshopt_compression: { buffer: 0, byteOffset: 0, byteLength: encodedVertices.length, byteStride: vertexStride, count: vertexCount, mode: 'ATTRIBUTES' } } },
    { buffer: 0, byteLength: indices.length * 2, byteStride: 2, extensions: { EXT_meshopt_compression: { buffer: 0, byteOffset: indexOffset, byteLength: encodedIndices.length, byteStride: 2, count: indices.length, mode: 'TRIANGLES' } } },
  ],
}
const jsonBytes = Buffer.from(JSON.stringify(json).padEnd(align4(JSON.stringify(json).length), ' '))
const total = 12 + 8 + jsonBytes.length + 8 + bin.length
const glb = Buffer.alloc(total)
glb.writeUInt32LE(0x46546C67, 0)
glb.writeUInt32LE(2, 4)
glb.writeUInt32LE(total, 8)
glb.writeUInt32LE(jsonBytes.length, 12)
glb.writeUInt32LE(0x4E4F534A, 16)
jsonBytes.copy(glb, 20)
const binHeader = 20 + jsonBytes.length
glb.writeUInt32LE(bin.length, binHeader)
glb.writeUInt32LE(0x004E4942, binHeader + 4)
bin.copy(glb, binHeader + 8)

mkdirSync(outDir, { recursive: true })
writeFileSync(join(outDir, 'meshopt-views.glb'), glb)
writeFileSync(join(outDir, 'meshopt-views.json'), JSON.stringify({ source: `spoint ${rev} node_modules/meshoptimizer MeshoptDecoder.decodeVertexBuffer and decodeIndexBuffer`, views: [Array.from(decodedVertices), Array.from(decodedIndices)] }) + '\n')
console.log(`wrote meshopt-views.glb ${total} bytes and meshopt-views.json from spoint ${rev}`)
