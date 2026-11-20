import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-terrain-stream.json')
const rev = execFileSync('git', ['-C', spoint, 'rev-parse', '--short', 'HEAD']).toString().trim()
const statusBefore = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
const tp = await import(pathToFileURL(join(spoint, 'src/terrain/HeightfieldStreamer.js')).href)

const INTERVAL = 300
const realSetTimeout = globalThis.setTimeout
let pendingTick = null
globalThis.setTimeout = (fn, ms, ...rest) => (ms === INTERVAL ? ((pendingTick = fn), 0) : realSetTimeout(fn, ms, ...rest))

let log = []
let nextId = 0
const physics = {
  addHeightField(samples, n, spacing, origin) {
    nextId += 1
    log.push({ e: 'add', n, spacing: spacing[0], origin: [origin[0], origin[2]], id: nextId })
    return nextId
  },
  removeBody(id) { log.push({ e: 'remove', id }) },
  setTerrainBodyId(id) { log.push({ e: 'primary', id: id ?? null }) },
}
const flush = () => { const out = log; log = []; return out }
const heightFn = (x, z) => 20 * Math.sin(x / 90) + 10 * Math.cos(z / 70)
const enc = (p) => p.map((c) => (Number.isNaN(c) ? 'NaN' : c))
const settle = async () => {
  for (let i = 0; i < 4000 && !pendingTick; i++) await new Promise((r) => realSetTimeout(r, 0))
  if (!pendingTick) throw new Error('streamer tick was never scheduled')
}

const pathA = []
for (let t = 0; t < 40; t++) {
  const tick = [[30 * t, 0]]
  if (t === 5) tick.push([NaN, 0])
  if (t >= 10 && t < 25) tick.push([-400, 200])
  pathA.push(tick)
}
const pathB = []
for (let t = 0; t < 12; t++) pathB.push([[30 * t, 0], [-600, -600]])

const runScenario = async (name, options, path) => {
  let players = path[0]
  log = []
  nextId = 0
  const streamer = tp.createTerrainStreamer({ physics, getCenters: () => players, heightFn, intervalMs: INTERVAL, ...options })
  await streamer.start([0, 0])
  await streamer.cover([])
  const trace = [{ step: 'start', events: flush() }]
  for (let t = 0; t < path.length; t++) {
    players = path[t]
    await settle()
    const tick = pendingTick
    pendingTick = null
    tick()
    await streamer.cover([])
    trace.push({ step: t, events: flush() })
  }
  streamer.stop()
  trace.push({ step: 'stop', events: flush() })
  return {
    name,
    maxFields: options.maxFields ?? null,
    path: path.map((tick) => tick.map(enc)),
    trace,
  }
}

const scenarios = []
scenarios.push(await runScenario('default', {}, pathA))
scenarios.push(await runScenario('cap3', { maxFields: 3 }, pathB))
const pathC = []
for (let t = 0; t < 6; t++) pathC.push([[0, 0], [400, 0], [610, 0]])
scenarios.push(await runScenario('cluster', {}, pathC))

const statusAfter = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
if (statusAfter !== statusBefore) throw new Error('spoint working tree changed')
writeFileSync(outPath, JSON.stringify({ spoint: rev, extent: 510, resolution: 4, rebuildAt: 0.4, scenarios, spointStatusBefore: statusBefore, spointStatusAfter: statusAfter }, null, 1) + String.fromCharCode(10))
console.log(JSON.stringify(scenarios.map((s) => ({ name: s.name, steps: s.trace.length, events: s.trace.reduce((a, t) => a + t.events.length, 0) }))))
