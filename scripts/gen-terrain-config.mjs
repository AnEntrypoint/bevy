import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-terrain-config.json')
const bakeOutPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-terrain-bake.json')
const git = (args) => execFileSync('git', ['-C', spoint, ...args]).toString()
const rev = git(['rev-parse', '--short', 'HEAD']).trim()
const statusBefore = git(['status', '--porcelain'])
const tc = await import(pathToFileURL(join(spoint, 'src/shared/terrainConfig.js')).href)
const { canonicalJSON } = await import(pathToFileURL(join(spoint, 'src/shared/canonicalJSON.js')).href)

const outcome = (fn) => {
  try {
    return { ok: true, value: fn() }
  } catch (e) {
    return { ok: false, name: e.name, message: e.message }
  }
}
const num = (v) => (Number.isFinite(v) ? v : null)
const configOut = (c) => ({
  seed: num(c.seed),
  radius: num(c.radius),
  reliefScale: num(c.reliefScale),
  anchorDir: c.anchorDir,
  center: c.center,
  enabled: c.enabled,
  hashVersion: c.hashVersion,
  carves: c.carves.map((k) => ({ dir: k.dir, innerM: k.innerM, outerM: k.outerM })),
  carveInputs: c.carveInputs,
  extent: num(c.extent),
  res: num(c.res),
})
const parseOut = (tcfg) => {
  const p = tc.parseTerrainConfig(tcfg)
  return p.ok ? { ok: true, config: configOut(p.config) } : { ok: false, path: p.path, reason: p.reason }
}

const ANCHOR = [0.3, -0.8, 0.5]
const PARSED_CARVE = { dir: [0.3, -0.8, 0.5], innerM: 300, outerM: 900 }
const CARVE_A = { center: [300, -200], falloff: 1000, radius: 500 }
const CARVE_B = { center: [-1200, 900], falloff: 250, radius: 0 }

const hashVersionCases = [{}, null, { hashVersion: 1 }, { hashVersion: 2 }, { hashVersion: null }, { hashVersion: 3 }, { hashVersion: '2' }, { hashVersion: 2.5 }, { hashVersion: 0 }, { hashVersion: true }]
const assertCases = [1, 2, 0, 3, '2', null, 2.5, true]
const overrideCases = [null, '', '1', '2', ' 2 ', '0x2', '3', 'abc', '2.5', ' ']
const carvesCases = [
  { radius: 6360 },
  { radius: 6360, carves: null },
  { radius: 6360, hashVersion: 1, carves: 'bad' },
  { radius: 6360, hashVersion: 1, carves: [CARVE_A] },
  { radius: 6360, carves: 'bad' },
  { radius: 6360, carves: [CARVE_A] },
  { radius: 6360, anchorDir: ANCHOR, carves: [CARVE_A, CARVE_B] },
  { radius: 6360, anchorDir: ANCHOR, carves: [PARSED_CARVE] },
  { radius: 6360, carves: [{ center: [1, 2], falloff: 1, radius: 0 }] },
  { carves: [{ center: [1, 2], falloff: 1, radius: 1 }] },
  { radius: 0, carves: [{ center: [1, 2], falloff: 1, radius: 1 }] },
  { radius: 6360, carves: [{ center: [1], falloff: 1, radius: 1 }] },
  { radius: 6360, carves: [{ center: [1, 2], falloff: 1, radius: -1 }] },
  { radius: 6360, carves: [{ center: [1, 2], falloff: 0, radius: 1 }] },
  { radius: 6360, carves: [null] },
  { radius: 6360, hashVersion: 3, carves: [] },
  { radius: 6360, hashVersion: 1 },
]
const resolveCases = [
  { entities: [{ app: 'terrain', config: { seed: 7, radius: 6360 } }] },
  { entities: [{ app: 'sky', config: { seed: 9 } }], terrain: { seed: 3, radius: 100 } },
  { entities: [{ app: 'terrain', config: null }], terrain: { seed: 5, radius: 90 } },
  { entities: [{ app: 'terrain' }] },
  { entities: [{ app: 'terrain', config: { seed: 1 } }, { app: 'terrain', config: { seed: 2 } }] },
  {},
  null,
]
const parseCases = [
  null,
  [],
  5,
  { seed: 'x' },
  { seed: null },
  { radius: -1 },
  { radius: null },
  { radius: 6360, hashVersion: 3 },
  { radius: 6360, carves: 'x' },
  { carves: [{ center: [1, 2], falloff: 1, radius: 1 }] },
  { radius: 6360, carves: [{ center: [1], falloff: 1, radius: 1 }] },
  { seed: 20260, radius: 6360, reliefScale: 0.5, anchorDir: ANCHOR, center: [10, -5], enabled: false, hashVersion: 2, carves: [CARVE_A] },
  { seed: 1, radius: 90, hashVersion: 1, carves: [{ center: [1, 2], falloff: 4, radius: 3 }], carveInputs: [{ center: [1, 2], falloff: 4, radius: 3 }] },
  {},
  { radius: 1000, minimapExtent: 300, minimapRes: 64, seed: -4.5 },
  { radius: 20000, minimapExtent: 'x', minimapRes: null, seed: 3 },
  { radius: 100000 },
  { radius: 6360, hashVersion: 1 },
  { radius: 6360, enabled: 0 },
]

const fixture = {
  spoint: rev,
  source: 'spoint ' + rev + ': src/shared/terrainConfig.js (terrainHashVersionOf, assertTerrainHashVersion, terrainCarvesOf, resolveTerrainConfig, parseTerrainConfig, parseTerrainHashOverride)',
  hashVersionOf: hashVersionCases.map((tcfg) => ({ tcfg, expect: outcome(() => tc.terrainHashVersionOf(tcfg)) })),
  assertHashVersion: assertCases.map((v) => ({ v, expect: outcome(() => tc.assertTerrainHashVersion(v)) })),
  hashOverride: overrideCases.map((raw) => ({ raw, expect: outcome(() => tc.parseTerrainHashOverride(raw)) })),
  carvesOf: carvesCases.map((tcfg) => ({ tcfg, expect: outcome(() => tc.terrainCarvesOf(tcfg)) })),
  resolve: resolveCases.map((worldDef) => ({ worldDef, expect: outcome(() => tc.resolveTerrainConfig(worldDef)) })),
  parse: parseCases.map((tcfg) => ({ tcfg, expect: parseOut(tcfg) })),
}

const descriptorOut = (d) => (d === null ? null : { base: d.base, center: d.center, extent: num(d.extent) })
const bakeParamsOut = (p) => ({
  radius: num(p.radius),
  reliefScale: num(p.reliefScale),
  hashVersion: p.hashVersion,
  carves: p.carves.map((k) => ({ dir: k.dir, innerM: k.innerM, outerM: k.outerM })),
  anchorDir: p.anchorDir,
  extent: num(p.extent),
  res: num(p.res),
  center: p.center,
})
const descriptorCases = [
  ['town', { seed: 20260, radius: 6360, center: [10, -5], anchorDir: ANCHOR, carves: [CARVE_A, CARVE_B] }],
  ['town', { seed: -4.5, radius: 20000 }],
  ['town', { seed: 7, radius: 6360, hashVersion: 1, carves: [CARVE_A] }],
  ['town', { seed: 2147483653, radius: 6360, minimapExtent: 300, minimapRes: 64 }],
  ['town', { seed: 3, radius: 1000, minimapExtent: 300, minimapRes: 64 }],
  ['town', { seed: 5 }],
  ['town', { seed: 7, radius: 6360, enabled: false }],
  ['town', { seed: 7, radius: 6360, enabled: 0 }],
  ['town', { radius: 6360 }],
  ['bad name', { seed: 1, radius: 90 }],
  [null, { seed: 1, radius: 90 }],
  ['town', null],
  ['town', { seed: 1, radius: -5 }],
  ['town', { seed: 9, radius: 6360, hashVersion: 3 }],
  ['town', { seed: 9, radius: 6360, carves: 'x' }],
]
const bakeParamsCases = [
  { seed: 20260, radius: 6360, reliefScale: 0.5, hashVersion: 2, carves: [CARVE_A], center: [10, -5], anchorDir: ANCHOR },
  { seed: 1, radius: 90, hashVersion: 1, carves: [CARVE_A] },
  { seed: 4 },
  { radius: 6360, anchorDir: ANCHOR, carves: [PARSED_CARVE, CARVE_B] },
  { seed: 3, radius: 6360, hashVersion: 3 },
]
const bakeKeyCases = [
  { seed: 20260, radius: 6360, reliefScale: 0.5, anchorDir: ANCHOR, center: [10, -5], hashVersion: 2, carves: [CARVE_A] },
  { seed: 7, radius: 6360 },
  { seed: 7, radius: 90, hashVersion: 1, carves: [CARVE_A] },
  {},
  { seed: -4.5, radius: 6360, carveInputs: [CARVE_B], carves: [CARVE_A] },
  { seed: 1, radius: 6360, anchorDir: ANCHOR, carves: [PARSED_CARVE] },
  { seed: 3, radius: 6360, hashVersion: 3 },
]
const seedCases = [
  { worldDef: { name: 'town', entities: [{ app: 'terrain', config: { seed: 7, radius: 6360, carves: [CARVE_A], bakedHeightfield: { big: [1, 2] }, vegetation: { seed: 7, density: 0.4 } } }] }, seed: 20260 },
  { worldDef: { terrain: { radius: 90 } }, seed: 3 },
  { worldDef: { entities: [{ app: 'terrain', custom: { seed: 5, radius: 9 } }, { app: 'sky', config: { seed: 9 } }, { app: 'terrain', config: null }] }, seed: 11 },
  { worldDef: { terrain: { seed: 4, radius: 9 } }, seed: 4 },
  { worldDef: { entities: [{ app: 'terrain', custom: { seed: 'x' } }] }, seed: 2 },
  { worldDef: null, seed: 1 },
  { worldDef: { terrain: { seed: 1, radius: 6360, vegetation: 'none' } }, seed: 2 },
  { worldDef: { terrain: { seed: 1 } }, seed: 2.5 },
  { worldDef: { terrain: { seed: 1 } }, seed: '7' },
  { worldDef: { terrain: { seed: 1 } }, seed: null },
]
const bakeFixture = {
  spoint: rev,
  source: 'spoint ' + rev + ': src/shared/terrainConfig.js (minimapDescriptor, minimapBakeParams, terrainBakeKey, withTerrainSeed); canonicalJSON.js for the withTerrainSeed text',
  minimapDescriptor: descriptorCases.map(([worldId, tcfg]) => ({ worldId, tcfg, expect: outcome(() => descriptorOut(tc.minimapDescriptor(worldId, tcfg))) })),
  minimapBakeParams: bakeParamsCases.map((tcfg) => ({ tcfg, expect: outcome(() => bakeParamsOut(tc.minimapBakeParams(tcfg))) })),
  terrainBakeKey: bakeKeyCases.map((tcfg) => ({ tcfg, expect: outcome(() => tc.terrainBakeKey(tcfg)) })),
  withTerrainSeed: seedCases.map(({ worldDef, seed }) => ({ worldDef, seed, expect: outcome(() => canonicalJSON(tc.withTerrainSeed(worldDef, seed))) })),
}

const statusAfter = git(['status', '--porcelain'])
if (statusAfter !== statusBefore) throw new Error('spoint working tree changed')
fixture.spointStatusBefore = statusBefore
fixture.spointStatusAfter = statusAfter
bakeFixture.spointStatusBefore = statusBefore
bakeFixture.spointStatusAfter = statusAfter
writeFileSync(outPath, JSON.stringify(fixture, null, 1) + '\n')
writeFileSync(bakeOutPath, JSON.stringify(bakeFixture, null, 1) + '\n')
console.log(JSON.stringify({ rev, hashVersionOf: hashVersionCases.length, assertHashVersion: assertCases.length, hashOverride: overrideCases.length, carvesOf: carvesCases.length, resolve: resolveCases.length, parse: parseCases.length, minimapDescriptor: descriptorCases.length, minimapBakeParams: bakeParamsCases.length, terrainBakeKey: bakeKeyCases.length, withTerrainSeed: seedCases.length }))
