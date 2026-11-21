import { writeParity, spointImport } from './lib/spoint.mjs'

const RADIUS = 6360
const smooth = (d) => 40 + 25 * d[0] - 15 * d[1] + 10 * d[2] * d[2]

await writeParity(async ({ spoint, rev }) => {
  const pf = await spointImport(spoint, 'src/terrain/PlanetFrame.js')
  const sampler = { radius: RADIUS, hashVersion: 2, heightAt: smooth }

  const relief = [0.001, null, undefined, 0, 0.25].map((given) => {
    const frame = pf.createPlanetFrame({ sampler, anchorDir: [0, 1, 0], offsetY: 0, reliefScale: given })
    return { reliefScale: given === undefined ? 'absent' : given, value: frame.reliefScale }
  })

  const snapshot = (f) => ({
    chartEpoch: f.chartEpoch,
    anchorHeight: f.anchorHeight,
    anchorSurfaceWorld: [...f.anchorSurfaceWorld],
    localToDir: [[100, 50, null], [100, 50, 20]].map(([x, z, y]) => f.localToDir(x, z, y ?? undefined)),
    localToWorld: [[0, 0, 0], [100, 5, -50]].map(([x, y, z]) => f.localToWorld(x, y, z)),
    groundHeightLocal: [[0, 0], [100, 50]].map(([x, z]) => f.groundHeightLocal(x, z)),
  })
  const chartFrame = pf.createPlanetFrame({ sampler, anchorDir: [0.6, 0, 0.8], offsetY: 5, reliefScale: 0.001 })
  const before = snapshot(chartFrame)
  const returned = chartFrame.reanchor([0, 1, 0])
  const after = snapshot(chartFrame)
  const returnedAgain = chartFrame.reanchor([0.6, 0, 0.8])
  const back = snapshot(chartFrame)

  const memoFrame = pf.createPlanetFrame({ sampler, anchorDir: [0.6, 0, 0.8], offsetY: 5, reliefScale: 0.001 })
  pf.resetSurfaceSolveStats()
  pf.resetGroundMemoStats()
  const memoStep = (x, z) => {
    try {
      return { x, z, value: memoFrame.groundHeightLocal(x, z), error: null, stats: pf.groundMemoStats(), solve: pf.surfaceSolveStats() }
    } catch (e) {
      return { x, z, value: null, error: e.name, stats: pf.groundMemoStats(), solve: pf.surfaceSolveStats() }
    }
  }
  pf.resetGroundMemoStats()
  const memoSteps = [[0, 0], [0, 0], [0.1, 0], [1024, 0], [0, 0], [100, 50], [100, 50], [7000, 0], [7000, 0], [NaN, 0]].map(([x, z]) => memoStep(x, z))
  memoFrame.reanchor([0, 1, 0])
  memoSteps.push({ reanchor: [0, 1, 0], chartEpoch: memoFrame.chartEpoch, stats: pf.groundMemoStats(), solve: pf.surfaceSolveStats() })
  memoSteps.push(memoStep(0, 0))
  memoSteps.push(memoStep(100, 50))

  const gpuSource = (x, z, yGuess) => { if (x > 5000) throw new pf.SurfaceSolveError(x, z, 1e-4, 64, 0.5); return 0.5 + 0.01 * x - 0.02 * z }
  const attempt = (fn) => { try { return { value: fn(), error: null } } catch (e) { return { value: null, error: e.name } } }
  const sourceFrame = pf.createPlanetFrame({ sampler, anchorDir: [0.6, 0, 0.8], offsetY: 5, reliefScale: 0.001 })
  const snapSources = () => ({ names: sourceFrame.groundSources, source: sourceFrame.groundSource })
  const sources = {}
  sources.initial = { ...snapSources(), cpu: sourceFrame.groundHeightLocal(100, 50) }
  sourceFrame.registerGroundSource('gpu', gpuSource)
  sources.gpu = { ...snapSources(), values: [sourceFrame.groundHeightLocal(100, 0), sourceFrame.groundHeightLocal(0, 50)] }
  sources.gpuThrows = { ...snapSources(), ...attempt(() => sourceFrame.groundHeightLocal(6000, 0)) }
  sources.emptyName = { ...snapSources(), ...attempt(() => sourceFrame.registerGroundSource('', gpuSource)) }
  sources.nullishSource = { ...snapSources(), ...attempt(() => sourceFrame.registerGroundSource('nullish', (x, z, yGuess) => null)) }
  sources.groundSourceOfGpu = attempt(() => typeof sourceFrame.groundSourceOf('gpu'))
  sources.groundSourceOfMissing = attempt(() => sourceFrame.groundSourceOf('nope'))
  sourceFrame.groundHeightLocal = (x, z, yGuess) => 1 + 0.001 * x
  sources.anonymousA = { ...snapSources(), values: [sourceFrame.groundHeightLocal(100, 50)] }
  sourceFrame.groundHeightLocal = (x, z, yGuess) => 2
  sources.anonymousB = { ...snapSources() }
  sourceFrame.groundHeightLocal = sourceFrame.cpuGroundHeightLocal
  sources.cpuSetter = { ...snapSources(), values: [sourceFrame.groundHeightLocal(100, 50)] }

  const captured = []
  const realConsoleError = console.error
  console.error = (msg) => captured.push(String(msg))
  const cpuWarning = {}
  const warnCase = (label, divergent, sourceName) => {
    const f = pf.createPlanetFrame({ sampler, anchorDir: [0, 1, 0], offsetY: 0, reliefScale: 0.001 })
    f.cpuHeightDivergentFromGround = divergent
    if (sourceName) f.registerGroundSource(sourceName, gpuSource)
    captured.length = 0
    const values = [f.elevationAtDir([0.6, 0.3, 0.7]), f.elevationAtDir([0.6, 0.3, 0.7])]
    cpuWarning[label] = { messages: captured.length, text: captured[0] ?? null, values }
  }
  warnCase('divergentGpu', true, 'gpu')
  warnCase('plainGpu', false, 'gpu')
  warnCase('divergentCpu', true, null)
  console.error = realConsoleError

  return {
    'spoint-planet-frame-parts.json': { spoint: rev, relief, reanchor: { before, returned, after, returnedAgain, back }, memo: memoSteps, sources, cpuWarning },
  }
})
