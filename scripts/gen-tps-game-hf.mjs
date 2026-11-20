import { createHash } from 'node:crypto'
import { execFileSync } from 'node:child_process'
import { readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const outDir = join(import.meta.dirname, '..', 'testdata', 'parity')
const WORLD = 'tps-game'
const EXTENT = 256
const RES = 1
const SECTOR = 16
const BITS = 16
const POINT_COUNT = 64
const hfPath = join(outDir, 'tps-game.hf')
const pointsPath = join(outDir, 'tps-game-hf-points.json')
const shippedPath = join(spoint, 'apps/world', WORLD + '.hf')

const spointGit = (...args) => execFileSync('git', ['-C', spoint, ...args]).toString()
const rev = spointGit('rev-parse', '--short', 'HEAD').trim()
const statusBefore = spointGit('status', '--porcelain')
const shipped = readFileSync(shippedPath)
const shippedHeaderLength = shipped.readUInt32LE(4)
const shippedHeader = JSON.parse(shipped.subarray(8, 8 + shippedHeaderLength).toString('utf8'))
if (shippedHeader.N !== EXTENT / RES + 1 || shippedHeader.extent !== EXTENT || shippedHeader.sectors.nodesPerSector !== SECTOR || shippedHeader.sectors.bits !== BITS) throw new Error('bake parameters do not match the shipped header: ' + JSON.stringify(shippedHeader.sectors))

execFileSync('node', ['scripts/bake-heightfield.mjs', '--world', WORLD, '--extent', String(EXTENT), '--res', String(RES), '--sector', String(SECTOR), '--bits', String(BITS), '--out', hfPath], { cwd: spoint, stdio: ['ignore', 'pipe', 'pipe'] })
const baked = readFileSync(hfPath)
const sha = (b) => createHash('sha256').update(b).digest('hex')

const { loadBakedHeightField } = await import(pathToFileURL(join(spoint, 'src/terrain/TerrainPhysics.js')).href)
const { nodeBakedSource } = await import(pathToFileURL(join(spoint, 'src/terrain/BakedSourceAdapter.js')).href)
const { resolveTerrainConfig, terrainHashVersionOf } = await import(pathToFileURL(join(spoint, 'src/shared/terrainConfig.js')).href)
const { expandWorldPresets } = await import(pathToFileURL(join(spoint, 'src/shared/worldPresets.js')).href)
const { decodeHeightfield } = await import(pathToFileURL(join(spoint, 'packages/mapspinner/src/heightfield-codec.js')).href)
const worldMod = await import(pathToFileURL(join(spoint, 'apps/world', WORLD + '.js')).href)
const tcfg = resolveTerrainConfig(expandWorldPresets(worldMod.default || worldMod))
const hashVersion = terrainHashVersionOf(tcfg)
const field = await loadBakedHeightField(hfPath, hashVersion, tcfg, null, nodeBakedSource())
if (!field) throw new Error('spoint loadBakedHeightField returned null for the baked .hf')

const half = EXTENT / 2
const span = half + 2 * (EXTENT / (shippedHeader.N - 1))
let state = 20261
const random = () => { state = (Math.imul(state, 1664525) + 1013904223) >>> 0; return state / 4294967296 }
const xz = [[-half, -half], [half, half], [0, 0], [half, -half], [-half, half]]
for (let i = 0; i < POINT_COUNT; i++) xz.push([(random() * 2 - 1) * span, (random() * 2 - 1) * span])
const samples = xz.map(([x, z]) => [x, z, field.heightAtLocal(x, z)])

const badMagic = Buffer.from(baked); badMagic[0] ^= 0xff
const truncated = baked.subarray(0, baked.length - 1)
const codeOf = (buf) => { try { decodeHeightfield(buf.buffer.slice(buf.byteOffset, buf.byteOffset + buf.byteLength)); return null } catch (e) { return e.code } }
const refusals = { badMagic: codeOf(badMagic), truncated: codeOf(truncated) }
if (!refusals.badMagic || !refusals.truncated) throw new Error('spoint codec accepted a refused buffer: ' + JSON.stringify(refusals))

const statusAfter = spointGit('status', '--porcelain')
if (statusAfter !== statusBefore) throw new Error('spoint working tree changed during the bake')

writeFileSync(pointsPath, JSON.stringify({
  spoint: rev,
  world: WORLD,
  bake: { script: 'scripts/bake-heightfield.mjs', extent: EXTENT, res: RES, sector: SECTOR, bits: BITS, center: [0, 0], N: shippedHeader.N, hashVersion, cpuSampler: true },
  shipped: { path: 'apps/world/tps-game.hf', sha256: sha(shipped), bytes: shipped.length, codeVersion: shippedHeader.codeVersion, terrainKey: shippedHeader.terrainKey },
  baked: { sha256: sha(baked), bytes: baked.length, identicalToShipped: sha(baked) === sha(shipped) },
  spointStatusBefore: statusBefore,
  spointStatusAfter: statusAfter,
  refusals,
  samples,
}) + '\n')
console.log(JSON.stringify({ spoint: rev, N: shippedHeader.N, samples: samples.length, identical: sha(baked) === sha(shipped), bakedBytes: baked.length, shippedBytes: shipped.length, refusals }))
