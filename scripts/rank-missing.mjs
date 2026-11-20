import { existsSync, readFileSync, writeFileSync } from 'node:fs'
import { join, relative, resolve, dirname } from 'node:path'

const root = join(import.meta.dirname, '..')
const spoint = process.argv[2] ?? '/workspace/spoint'
const DEFAULT_ENTRIES = ['apps/tps-game/index.js', 'apps/tps-game/client-app.js', 'apps/tps-game/server-app.js', 'apps/tps-game/shared.js', 'src/terrain/TerrainPhysics.js']
const lines = readFileSync(join(root, 'docs', 'parity-matrix.tsv'), 'utf8').split('\n').filter(Boolean)
const header = lines[0].split('\t')
const rows = lines.slice(1).map((line) => Object.fromEntries(header.map((h, i) => [h, line.split('\t')[i] ?? ''])))

const IMPORT = /(?:import|export)\s+(?:[^'"]*?\s+from\s+)?['"](\.{1,2}\/[^'"]+)['"]/g
const closureOf = (entries) => {
  const seen = new Set()
  const queue = entries.map((e) => resolve(spoint, e))
  while (queue.length) {
    let file = queue.shift()
    if (!existsSync(file)) { if (existsSync(file + '.js')) file = file + '.js'; else if (existsSync(join(file, 'index.js'))) file = join(file, 'index.js'); else continue }
    if (seen.has(file)) continue
    seen.add(file)
    for (const m of readFileSync(file, 'utf8').matchAll(IMPORT)) queue.push(resolve(dirname(file), m[1]))
  }
  return new Set([...seen].map((f) => relative(spoint, f)))
}
const defaultGame = closureOf(DEFAULT_ENTRIES)

const ASSET_PIPELINE = /corpus|bake|glb|gltf|ktx2|cluster|manifest|decimat|degenerate|mip|meshlet|octahedral|draco|codec|validator|conformance|texture|material variant|material convergence|atlas/i
const CATEGORIES = [
  ['rendering', /render|shader|material|camera|light|sky|fog|shadow|atmosphere|water|bloom|postprocess|frustum|particle|draw|hud|overlay/i],
  ['physics', /physics|collider|collision|rapier|raycast|rigid|sweep|trimesh|softbody|fluid|projectile|impulse|buoyan|joint|gravity|trigger|body|character/i],
  ['input', /input|keyboard|mouse|pointer|gamepad|touch|keybind|control|aim|movement|\bpad\b/i],
  ['networking', /netcode|lockstep|rollback|protocol|lightyear|websocket|multiplayer|msgpack|replicat|consensus|desync|transport|relay|peer|socket|control frame|wire/i],
  ['world', /terrain|planet|globe|heightfield|height|chart|carve|vegetation|grass|tree|placement|weather|water|ocean|biome|climate|sector|chunk|navmesh|pathfind|sea|cube|spline|cave|fire|wind|cloud|ground|surface|tile|spawn/i],
]
const CATEGORY_ORDER = [...CATEGORIES.map(([name]) => name), 'asset-pipeline', 'other']
const categoryOf = (row) => {
  const text = row.feature + ' ' + row.source
  if (ASSET_PIPELINE.test(text)) return 'asset-pipeline'
  const hit = CATEGORIES.find(([, re]) => re.test(text))
  return hit ? hit[0] : 'other'
}

const IMPURE = /document\.|window\.|navigator\.|THREE\.|GPUDevice|WebGPU|getContext\(|\bcanvas\b|from 'three|from "three|ShaderMaterial|Mesh\(|requestAnimationFrame|createElement\(/
const sourcePaths = (row) => row.source.match(/[\w./-]+\.(?:m?js|cjs|ts)\b/g) || []
const exportsOf = (text) => new Set([...text.matchAll(/export\s+(?:async\s+)?(?:function\*?|const|let|class)\s+([A-Za-z_$][\w$]*)/g)].map((m) => m[1]))
const reachable = (row) => {
  const words = new Set(row.feature.match(/\b[a-z][A-Za-z0-9]{3,}\b|\b[A-Z][A-Za-z0-9]{3,}\b/g) || [])
  return sourcePaths(row).some((rel) => {
    const file = join(spoint, rel)
    if (!existsSync(file)) return false
    const text = readFileSync(file, 'utf8')
    if (IMPURE.test(text)) return false
    const exported = exportsOf(text)
    return [...words].some((w) => exported.has(w))
  })
}
const inDefaultGame = (row) => sourcePaths(row).some((rel) => defaultGame.has(rel))

const ranked = rows.filter((row) => row.outcome === 'missing').map((row) => ({
  n: Number(row.n), tier: inDefaultGame(row) ? 0 : 1, category: categoryOf(row), reachable: reachable(row), feature: row.feature, source: row.source,
})).sort((a, b) => a.tier - b.tier || CATEGORY_ORDER.indexOf(a.category) - CATEGORY_ORDER.indexOf(b.category) || Number(b.reachable) - Number(a.reachable) || a.n - b.n)
  .map((row, i) => ({ rank: i + 1, ...row }))

writeFileSync(join(root, 'docs', 'RANKED_MISSING.tsv'), ['rank\ttier\tn\tcategory\treachable\tfeature\tsource', ...ranked.map((r) => [r.rank, r.tier === 0 ? 'default-game' : 'other', r.n, r.category, r.reachable ? 'yes' : 'no', r.feature, r.source].join('\t'))].join('\n') + '\n')
console.log(JSON.stringify({ defaultGameFiles: defaultGame.size, missing: ranked.length, reachable: ranked.filter((r) => r.reachable).length, defaultGameMissing: ranked.filter((r) => r.tier === 0).length, defaultGameReachable: ranked.filter((r) => r.tier === 0 && r.reachable).length }))
for (const r of ranked.filter((x) => x.reachable).slice(0, 12)) console.log(JSON.stringify({ rank: r.rank, n: r.n, tier: r.tier, category: r.category, feature: r.feature.slice(0, 120), source: r.source.slice(0, 70) }))
