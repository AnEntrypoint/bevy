import { execFileSync } from 'node:child_process'
import { readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const outDir = join(import.meta.dirname, '..', 'testdata', 'parity')
const WORLD = 'tps-game'
const EXTENT = 512
const RES = 32
const POINT_COUNT = 64
const artifactPath = join(outDir, 'baked-heightfield.json')
const shortPath = join(outDir, 'baked-heightfield-short.json')
const pointsPath = join(outDir, 'baked-heightfield-points.json')

const spointGit = (...args) => execFileSync('git', ['-C', spoint, ...args]).toString()
const rev = spointGit('rev-parse', '--short', 'HEAD').trim()
const statusBefore = spointGit('status', '--porcelain')

execFileSync('node', ['scripts/bake-heightfield.mjs', '--world', WORLD, '--extent', String(EXTENT), '--res', String(RES), '--out', artifactPath], { cwd: spoint, stdio: ['ignore', 'pipe', 'pipe'] })

const { loadBakedHeightField } = await import(pathToFileURL(join(spoint, 'src/terrain/TerrainPhysics.js')).href)
const { nodeBakedSource } = await import(pathToFileURL(join(spoint, 'src/terrain/BakedSourceAdapter.js')).href)
const { resolveTerrainConfig, terrainHashVersionOf } = await import(pathToFileURL(join(spoint, 'src/shared/terrainConfig.js')).href)
const { expandWorldPresets } = await import(pathToFileURL(join(spoint, 'src/shared/worldPresets.js')).href)
const worldMod = await import(pathToFileURL(join(spoint, 'apps/world', WORLD + '.js')).href)
const tcfg = resolveTerrainConfig(expandWorldPresets(worldMod.default || worldMod))
const hashVersion = terrainHashVersionOf(tcfg)

const artifact = JSON.parse(readFileSync(artifactPath, 'utf8'))
const field = await loadBakedHeightField(artifactPath, hashVersion, tcfg, null, nodeBakedSource())
if (!field) throw new Error('spoint loadBakedHeightField returned null for the generated artifact')

const half = EXTENT / 2
const step = EXTENT / (artifact.N - 1)
const span = half + 2 * step
let state = 20260
const random = () => { state = (Math.imul(state, 1664525) + 1013904223) >>> 0; return state / 4294967296 }
const xz = [[-half, -half], [half, half], [0, 0], [half, -half], [-half, half]]
for (let i = 0; i < POINT_COUNT; i++) xz.push([(random() * 2 - 1) * span, (random() * 2 - 1) * span])
const samples = xz.map(([x, z]) => [x, z, field.heightAtLocal(x, z)])

const short = { ...artifact, heights: artifact.heights.slice(0, -1) }
writeFileSync(shortPath, JSON.stringify(short))
let refusal = null
try { await loadBakedHeightField(shortPath, hashVersion, tcfg, null, nodeBakedSource()) } catch (e) { refusal = String(e.message) }
if (refusal === null) throw new Error('spoint accepted the truncated artifact')

const statusAfter = spointGit('status', '--porcelain')
if (statusAfter !== statusBefore) throw new Error('spoint working tree changed during the bake')

const points = {
  spoint: rev,
  world: WORLD,
  bake: { script: 'scripts/bake-heightfield.mjs', extent: EXTENT, res: RES, center: [0, 0], N: artifact.N, hashVersion, cpuSampler: true },
  codeVersion: artifact.codeVersion ?? null,
  spointStatusBefore: statusBefore,
  spointStatusAfter: statusAfter,
  refusalShort: refusal,
  samples,
}
writeFileSync(pointsPath, JSON.stringify(points) + '\n')
console.log(JSON.stringify({ spoint: rev, N: artifact.N, samples: samples.length, refusal }))
