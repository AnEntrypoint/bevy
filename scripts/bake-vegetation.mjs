import { createHash } from 'node:crypto'
import { register } from 'node:module'
import { copyFileSync, mkdirSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

export const BRANCH_CAP = 6000
export const LEAF_CAP = 1400
export const MIN_LEAF_ALPHA_TEST = 0.5
export const PARTS = [
  { part: 'branch', tier: 0, key: 'branchGeo0' },
  { part: 'leaf', tier: 0, key: 'leafGeo0' },
  { part: 'branch', tier: 1, key: 'b1' },
  { part: 'leaf', tier: 1, key: 'l1' },
  { part: 'branch', tier: 2, key: 'b2' },
  { part: 'leaf', tier: 2, key: 'l2' },
  { part: 'shadow', tier: 2, key: 'b2shadow' },
]
export const TIER_RATIO = { branch: [null, 0.28, 0.07], leaf: [null, 0.30, 0.09], shadow: [null, null, 0.07] }

const assetsOf = (spoint) => join(spoint, 'node_modules/@dgreenheck/ez-tree/src/lib/assets')
const barkMaps = (type) => Object.fromEntries(['color', 'normal', 'ao', 'roughness'].map((m) => [m, `bark/${type}_${m}_1k.jpg`]))
const leafMap = (type) => `leaves/${type}_color.png`
const slugOf = (name) => name.toLowerCase().replace(/ /g, '_')
const triangles = (geo) => geo.index.count / 3

function stubBrowserForReference(spoint) {
  const sourceRoot = pathToFileURL(spoint).href
  const hooks = `export async function resolve(s, c, n) { return n(s.startsWith('/src/') ? '${sourceRoot}' + s : s, c) }`
  register('data:text/javascript,' + encodeURIComponent(hooks))
  const element = () => ({ addEventListener() {}, removeEventListener() {}, style: {}, set src(v) {}, get src() { return '' } })
  globalThis.document = { createElementNS: element, createElement: element }
}

export async function referenceSpecies(spoint) {
  stubBrowserForReference(spoint)
  const build = await import(pathToFileURL(join(spoint, 'client/core/VegetationBuild.js')).href)
  const { Tree } = await build.loadEzTree()
  const consoleError = console.error
  console.error = () => {}
  const species = []
  for (const name of Object.keys(build.PRESET)) {
    const source = build.buildSpecies(name, Tree, 0)
    const chain = await build.speciesLodChain(name, 0, Tree)
    species.push({ name, preset: build.PRESET[name], targetH: build.TARGET_H[name], source, chain })
  }
  console.error = consoleError
  return { species, refusals: build.leafCapRefusals() }
}

const attribute = (geo, name) => geo.attributes[name].array
const indexOf = (geo) => Uint32Array.from(geo.index.array)

function describeMaterials(entry) {
  const { bark, leaves } = entry.source.tree.options
  return {
    bark: { type: bark.type, tint: bark.tint, textureScale: [bark.textureScale.x, bark.textureScale.y], flatShading: bark.flatShading, maps: barkMaps(bark.type) },
    leaf: { type: leaves.type, tint: leaves.tint, alphaTest: Math.max(leaves.alphaTest, MIN_LEAF_ALPHA_TEST), map: leafMap(leaves.type) },
  }
}

function encodeSpecies(entry) {
  const { chain } = entry
  const blobs = []
  let offset = 0
  const placedAt = new Map()
  const place = (typed) => {
    const bytes = Buffer.from(typed.buffer, typed.byteOffset, typed.byteLength)
    const identity = createHash('sha1').update(bytes).digest('hex')
    if (placedAt.has(identity)) return placedAt.get(identity)
    const at = offset
    placedAt.set(identity, at)
    blobs.push(bytes)
    offset += bytes.length
    return at
  }
  const parts = PARTS.map(({ part, tier, key }) => {
    const geo = chain[key]
    return {
      part, tier,
      vertices: geo.attributes.position.count,
      indices: geo.index.count,
      positions: place(attribute(geo, 'position')),
      normals: place(attribute(geo, 'normal')),
      uvs: place(attribute(geo, 'uv')),
      indexData: place(indexOf(geo)),
    }
  })
  const { treeBox, treeSph } = chain
  const header = JSON.stringify({
    name: entry.name, preset: entry.preset, targetH: entry.targetH,
    width: chain.dims.width, height: chain.dims.height,
    boxMin: treeBox.min.toArray(), boxMax: treeBox.max.toArray(),
    sphereCentre: treeSph.center.toArray(), sphereRadius: treeSph.radius,
    ...describeMaterials(entry), parts,
  })
  const padded = Buffer.from(header + ' '.repeat((4 - (header.length % 4)) % 4))
  const prefix = Buffer.alloc(8)
  prefix.write('VEG1', 0, 'ascii')
  prefix.writeUInt32LE(padded.length, 4)
  return Buffer.concat([prefix, padded, ...blobs])
}

export function textureFiles(species) {
  const used = new Set()
  for (const entry of species) {
    const { bark, leaf } = describeMaterials(entry)
    Object.values(bark.maps).forEach((f) => used.add(f))
    used.add(leaf.map)
  }
  return [...used].sort()
}

async function bake() {
  const spoint = process.argv[2] ?? '/workspace/spoint'
  const out = process.argv[3] ?? join(import.meta.dirname, '..', 'assets', 'planet', 'vegetation')
  const { species } = await referenceSpecies(spoint)
  for (const sub of ['bark', 'leaves']) mkdirSync(join(out, sub), { recursive: true })
  for (const entry of species) writeFileSync(join(out, slugOf(entry.name) + '.veg'), encodeSpecies(entry))
  for (const file of textureFiles(species)) copyFileSync(join(assetsOf(spoint), file), join(out, file))
  console.log(JSON.stringify({ out, species: species.length, textures: textureFiles(species).length }))
}

export { describeMaterials, slugOf, triangles }

if (process.argv[1] === fileURLToPath(import.meta.url)) await bake()
