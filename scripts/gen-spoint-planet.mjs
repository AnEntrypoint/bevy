import { execFileSync } from 'node:child_process'
import { mkdirSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const mapspinner = join(spoint, 'packages/mapspinner/src')
const outDir = join(import.meta.dirname, '..', 'testdata', 'parity')
const rev = execFileSync('git', ['-C', spoint, 'rev-parse', '--short', 'HEAD']).toString().trim()
const { createHeightSampler } = await import(pathToFileURL(join(mapspinner, 'height-cpu.js')).href)
const { createAnchorField } = await import(pathToFileURL(join(mapspinner, 'anchor-field.js')).href)
const { DEFAULT_TERRAIN_HASH_VERSION } = await import(pathToFileURL(join(spoint, 'src/shared/terrainConfig.js')).href)

const SEED = 0x5eed1234
const HASH_VERSION = DEFAULT_TERRAIN_HASH_VERSION
const RES = 128
const BAKE_MAX_LEVEL = 7
const RADII = [64, 6360]
const SAMPLE_INDICES = [0, RES - 1, (RES - 1) * RES, RES * RES - 1, 64 * RES + 64, 5 * RES + 77, 100 * RES + 3, 33 * RES + 120]
const DIR_COUNT = 32

let state = 20260
const random = () => { state = (Math.imul(state, 1664525) + 1013904223) >>> 0; return state / 4294967296 }
const direction = () => [random() * 2 - 1, random() * 2 - 1, random() * 2 - 1]

const fnv1a = (values) => {
  let hash = 0xcbf29ce484222325n
  for (const byte of new Uint8Array(Float32Array.from(values).buffer)) {
    hash = ((hash ^ BigInt(byte)) * 0x100000001b3n) & 0xffffffffffffffffn
  }
  return hash.toString(16).padStart(16, '0')
}

const af = createAnchorField({ seed: SEED })
const faces = Array.from({ length: 6 }, (_, face) => {
  const values = new Float32Array(RES * RES)
  for (let y = 0; y < RES; y++) {
    const fv = y / (RES - 1)
    for (let x = 0; x < RES; x++) values[y * RES + x] = af.sampleUV(face, x / (RES - 1), fv, BAKE_MAX_LEVEL).seaBias
  }
  return values
})
const dirs = Array.from({ length: DIR_COUNT }, () => direction())
const fixture = {
  source: `spoint ${rev}: packages/mapspinner/src/height-cpu.js (createHeightSampler heightAt, hashVersion from terrainConfig) over the anchor-field sampleUV seaBias bake at BAKE_MAX_LEVEL ${BAKE_MAX_LEVEL}`,
  seed: SEED,
  faceDigests: faces.map(fnv1a),
  faceSamples: faces.flatMap((values, face) => SAMPLE_INDICES.map((index) => ({ face, index, value: values[index] }))),
  radii: RADII.map((radius) => {
    const sampler = createHeightSampler({ radius, seed: SEED, hashVersion: HASH_VERSION, anchorField: af })
    return { radius, dirs, heights: dirs.map((d) => sampler.heightAt(d)) }
  }),
}
mkdirSync(outDir, { recursive: true })
writeFileSync(join(outDir, 'spoint-planet.json'), JSON.stringify(fixture, (key, value) => (typeof value === 'number' && key !== 'seed' ? String(value) : value)) + '\n')
console.log(`wrote ${faces.length} face digests, ${fixture.faceSamples.length} seaBias samples and ${RADII.length} radii of ${DIR_COUNT} heights from spoint ${rev}`)
