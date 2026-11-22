import { writeFixture } from './lib/spoint.mjs'
import { BRANCH_CAP, LEAF_CAP, PARTS, TIER_RATIO, describeMaterials, referenceSpecies, triangles } from './bake-vegetation.mjs'

const fnv1a = (typed) => {
  let hash = 2166136261
  for (const byte of new Uint8Array(typed.buffer, typed.byteOffset, typed.byteLength)) hash = Math.imul(hash ^ byte, 16777619) >>> 0
  return hash
}

const boundsOf = (positions) => {
  const min = [Infinity, Infinity, Infinity]
  const max = [-Infinity, -Infinity, -Infinity]
  for (let i = 0; i < positions.length; i++) {
    min[i % 3] = Math.min(min[i % 3], positions[i])
    max[i % 3] = Math.max(max[i % 3], positions[i])
  }
  return { min, max }
}

const targetTris = (part, tier, sourceTris) => {
  if (tier === 0) return part === 'branch' ? BRANCH_CAP : LEAF_CAP
  return Math.floor(sourceTris * TIER_RATIO[part][tier])
}

const sourceTrisOf = (entry, part, tier) => {
  if (tier === 0) return triangles(part === 'leaf' ? entry.source.leafGeo : entry.source.branchGeo)
  return triangles(part === 'leaf' ? entry.chain.leafGeo0 : entry.chain.branchGeo0)
}

const partRow = (entry, { part, tier, key }) => {
  const geo = entry.chain[key]
  const attrs = geo.attributes
  const sourceTris = sourceTrisOf(entry, part, tier)
  return {
    part, tier,
    sourceTris,
    targetTris: targetTris(part, tier, sourceTris),
    deliveredTris: triangles(geo),
    vertices: attrs.position.count,
    ...boundsOf(attrs.position.array),
    hashes: { positions: fnv1a(attrs.position.array), normals: fnv1a(attrs.normal.array), uvs: fnv1a(attrs.uv.array), indices: fnv1a(Uint32Array.from(geo.index.array)) },
  }
}

const sourceHeightOf = ({ branchGeo, leafGeo }) => {
  const ys = [branchGeo, leafGeo].map((g) => boundsOf(g.attributes.position.array)).flatMap(({ min, max }) => [min[1], max[1]])
  return Math.max(...ys.filter((_, i) => i % 2 === 1)) - Math.min(...ys.filter((_, i) => i % 2 === 0))
}

await writeFixture('spoint-planet-species', async ({ spoint, rev }) => {
  const { species, refusals } = await referenceSpecies(spoint)
  return {
    rev,
    caps: { branch: BRANCH_CAP, leaf: LEAF_CAP },
    ratios: TIER_RATIO,
    species: species.map((entry) => ({
      name: entry.name,
      preset: entry.preset,
      targetH: entry.targetH,
      width: entry.chain.dims.width,
      height: entry.chain.dims.height,
      sourceHeight: sourceHeightOf(entry.source),
      boxMin: entry.chain.treeBox.min.toArray(),
      boxMax: entry.chain.treeBox.max.toArray(),
      sphereCentre: entry.chain.treeSph.center.toArray(),
      sphereRadius: entry.chain.treeSph.radius,
      ...describeMaterials(entry),
      parts: PARTS.map((spec) => partRow(entry, spec)),
    })),
    leafCapRefusals: refusals,
  }
})
