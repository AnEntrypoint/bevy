import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-chart-anchor.json')
const rev = execFileSync('git', ['-C', spoint, 'rev-parse', '--short', 'HEAD']).toString().trim()
const statusBefore = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
const spointModule = (rel) => import(pathToFileURL(join(spoint, rel)).href)
const { createPlanetFrame } = await spointModule('src/terrain/PlanetFrame.js')
const { CHART_ANCHORS_PER_FACE, CHART_REANCHOR_ANGLE_DEG, createChartAnchorLattice, chartAnchorKeyOfDir, chartAnchorDirOfKey, chartAnchorForDir, chartAnchorAngleDeg, chartNeedsReanchor, chartNeedsReanchorForCell, reanchorChartFor, snapshotChart } = await spointModule('src/shared/chartAnchor.js')

const RADIUS = 60
const ANCHOR_DIR = [0, 1, 0]
const OFFSET_Y = 0
const POINTS = [[0, 0, 0], [1, 2, 3], [-4, 0.5, 6]]
const VECTORS = [[1, 0, 0], [0, 1, 0], [0.3, -0.7, 2]]
const DIRS = [[0, 1, 0], [1, 0, 0], [0, 0, -1], [0.9, 0.3, 0.2], [-0.6, 0.2, 0.77], [0.5, 0.5, 0.7], [3, -4, 5], [-0.01, -1, 0.002], [0, 0, 0]]
const heightAt = (d) => 0.5 + 0.25 * d[0] - 0.1 * d[2]
const unit = (v) => { const l = Math.hypot(v[0], v[1], v[2]); return v.map((c) => c / l) }
const newFrame = (anchorDir) => createPlanetFrame({ sampler: { radius: RADIUS, hashVersion: 1, heightAt }, anchorDir, offsetY: OFFSET_Y, reliefScale: 1 })

const lattice = createChartAnchorLattice(RADIUS, CHART_ANCHORS_PER_FACE)
const latticeCases = [
  { radius: RADIUS, anchorsPerFace: CHART_ANCHORS_PER_FACE },
  { radius: RADIUS, anchorsPerFace: 1 },
  { radius: 6371, anchorsPerFace: 8 },
  { radius: 0, anchorsPerFace: CHART_ANCHORS_PER_FACE },
  { radius: -3, anchorsPerFace: 0.5 },
  { radius: RADIUS, anchorsPerFace: 0.5 },
].map(({ radius, anchorsPerFace }) => {
  try {
    const l = createChartAnchorLattice(radius, anchorsPerFace)
    return { radius, anchorsPerFace, ok: true, chunkM: l.chunkM, chunksPerFace: l.chunksPerFace }
  } catch (e) {
    return { radius, anchorsPerFace, ok: false, name: e.name, reason: e.reason }
  }
})
const keyProbes = DIRS.map((dir) => ({ dir, key: chartAnchorKeyOfDir(lattice, dir), centre: chartAnchorForDir(lattice, dir) }))
const frame = newFrame(ANCHOR_DIR)
const angleProbes = DIRS.map((dir) => ({ dir, angleDeg: chartAnchorAngleDeg(frame, dir), needs: chartNeedsReanchor(frame, dir), needsCell: chartNeedsReanchorForCell(lattice, frame, dir) }))

const STEPS = [
  { name: 'stay-near-anchor', dir: [0, 1, 0.01], cellTrigger: false, thresholdDeg: CHART_REANCHOR_ANGLE_DEG, expect: false },
  { name: 'reanchor-far', dir: unit([0.9, 0.3, 0.2]), cellTrigger: false, thresholdDeg: CHART_REANCHOR_ANGLE_DEG, expect: true },
  { name: 'stay-after-reanchor', dir: unit([0.9, 0.3, 0.2]), cellTrigger: false, thresholdDeg: CHART_REANCHOR_ANGLE_DEG, expect: false },
  { name: 'reanchor-cell-trigger', dir: unit([0.5, 0.5, 0.7]), cellTrigger: true, thresholdDeg: CHART_REANCHOR_ANGLE_DEG, expect: true },
  { name: 'reanchor-custom-threshold', dir: unit([-0.6, 0.2, 0.77]), cellTrigger: false, thresholdDeg: 5, expect: true },
  { name: 'stay-high-threshold', dir: unit([0.3, -0.2, 0.9]), cellTrigger: false, thresholdDeg: 179, expect: false },
]
const seq = newFrame(ANCHOR_DIR)
const initial = snapshotChart(seq)
const steps = STEPS.map((step) => {
  const r = reanchorChartFor({ frame: seq, lattice, dir: step.dir, thresholdDeg: step.thresholdDeg, cellTrigger: step.cellTrigger })
  if ((r !== null) !== step.expect) throw new Error(step.name + ': reanchor ' + (r === null ? 'null' : 'applied') + ' against expected ' + step.expect)
  return {
    name: step.name, dir: step.dir, cellTrigger: step.cellTrigger, thresholdDeg: step.thresholdDeg,
    result: r === null ? null : { from: r.from, to: r.to, anchorDir: r.anchorDir, angleBeforeDeg: r.angleBeforeDeg, angleAfterDeg: r.angleAfterDeg, tiltRad: r.transfer.tiltRad, transfer: { points: POINTS, pointResults: POINTS.map((p) => r.transfer.point(p)), vectors: VECTORS, vectorResults: VECTORS.map((v) => r.transfer.vec(v)) } },
    frameAfter: snapshotChart(seq),
  }
})

const statusAfter = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
if (statusAfter !== statusBefore) throw new Error('spoint working tree changed')
writeFileSync(outPath, JSON.stringify({
  spoint: rev, planetRadius: RADIUS, anchorDir: ANCHOR_DIR, offsetY: OFFSET_Y,
  constants: { anchorsPerFace: CHART_ANCHORS_PER_FACE, reanchorAngleDeg: CHART_REANCHOR_ANGLE_DEG },
  lattice: { chunkM: lattice.chunkM, cellM: lattice.cellM, cellsPerChunk: lattice.cellsPerChunk, chunksPerFace: lattice.chunksPerFace, cellsPerFace: lattice.cellsPerFace, cellAngle: lattice.cellAngle, chunkAngle: lattice.chunkAngle },
  latticeCases, keyProbes, angleProbes, initial, steps,
  spointStatusBefore: statusBefore, spointStatusAfter: statusAfter,
}, null, 1) + '\n')
console.log(JSON.stringify({ latticeCases: latticeCases.map((c) => [c.ok, c.reason ?? null]), steps: steps.map((s) => [s.name, s.result !== null]), keys: keyProbes.map((k) => k.key) }))
