import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-terrain-config.json')
const git = (args) => execFileSync('git', ['-C', spoint, ...args]).toString()
const rev = git(['rev-parse', '--short', 'HEAD']).trim()
const statusBefore = git(['status', '--porcelain'])
const tc = await import(pathToFileURL(join(spoint, 'src/shared/terrainConfig.js')).href)

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

const statusAfter = git(['status', '--porcelain'])
if (statusAfter !== statusBefore) throw new Error('spoint working tree changed')
fixture.spointStatusBefore = statusBefore
fixture.spointStatusAfter = statusAfter
writeFileSync(outPath, JSON.stringify(fixture, null, 1) + '\n')
console.log(JSON.stringify({ rev, hashVersionOf: hashVersionCases.length, assertHashVersion: assertCases.length, hashOverride: overrideCases.length, carvesOf: carvesCases.length, resolve: resolveCases.length, parse: parseCases.length }))
