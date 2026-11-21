import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-field-paint.json')
const gitStatus = () => execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
const rev = execFileSync('git', ['-C', spoint, 'rev-parse', '--short', 'HEAD']).toString().trim()
const statusBefore = gitStatus()
const { createFieldLayers } = await import(pathToFileURL(join(spoint, 'src/terrain/FieldLayers.js')).href)
const fieldPaint = await import(pathToFileURL(join(spoint, 'src/terrain/fieldPaint.js')).href)
const { createFieldPaint, createHostFieldPaint, wrapHeightWithFieldPaint, naturalMaskAt, TERRAIN_FIELD_NAMES, FIELD_LAYER_NAMES, PAINTED_HEIGHT_FIELD, NATURAL_MASK_FIELD } = fieldPaint

const encode = (value) => JSON.parse(JSON.stringify(value, (_, item) => (typeof item === 'number' && !Number.isFinite(item) ? String(item) : item)))
const outcome = (fn) => {
  try {
    return { result: fn() }
  } catch (e) {
    return { error: { reason: e.reasonCode ?? null, message: e.message } }
  }
}
const byKey = (a, b) => (a.key < b.key ? -1 : a.key > b.key ? 1 : 0)
const sortedDoc = (doc) => Object.fromEntries(Object.entries(doc).map(([field, layers]) => [field, Object.fromEntries(Object.entries(layers).map(([layer, list]) => [layer, [...list].sort(byKey)]))]))
const runTrace = (make, ops, apply) => {
  const target = make()
  return ops.map((op) => ({ ...op, ...outcome(() => apply(target, op)) }))
}

const CELL = 0.5
const CHUNK = 4
const SAMPLES = [[0, 0], [0.25, 0.25], [0.5, 0.5], [-0.25, 0.25], [1.75, -0.5], [-2.5, 3], [1, -1], [-1.5, 2.5], [3.5, 4.5], [4, 4], [2, 2], [-3.75, -0.25], [5.25, -4.5]]
const HEIGHT = (x, z, yGuess) => 1.5 + 0.5 * x - 0.25 * z + 0.125 * yGuess
const STROKE_FN = {
  add: (s) => (p, inset, cur) => cur + s.amount * inset,
  set: (s) => (p, inset, cur) => cur + (s.target - cur) * inset,
  slope: (s) => (p, inset, cur) => cur + inset * (s.ax * p[0] + s.az * p[1]),
}
const STROKES = [
  { field: 'terrain:height', layer: 'PLAYER', center: [0.5, 0.5], radius: 2, kind: 'add', amount: 1 },
  { field: 'terrain:height', layer: 'CREATOR', center: [1, -1], place: [0.5, 0], radius: 1.5, kind: 'set', target: 2.5 },
  { field: 'terrain:height', layer: 'PLAYER', center: [-2.5, 3], place: [0, -0.5], radius: 3, kind: 'slope', ax: 0.25, az: -0.5 },
  { field: 'terrain:water', layer: 'PLAYER', center: [0, 0], radius: 1, kind: 'add', amount: -0.75 },
  { field: 'terrain:natural-mask', layer: 'CREATOR', center: [4, 4], radius: 2.5, kind: 'set', target: 0.6 },
  { field: 'terrain:natural-mask', layer: 'PLAYER', center: [3.5, 4.5], radius: 2, kind: 'add', amount: 0.2 },
  { field: 'terrain:height', layer: 'PLAYER', center: [0, 0], radius: 1, kind: 'add', amount: 0.3 },
]
const STROKE_REFUSALS = [
  { field: 'terrain:rock', layer: 'PLAYER', center: [0, 0], radius: 1, kind: 'add', amount: 1 },
  { field: 'terrain:height', layer: 'GHOST', center: [0, 0], radius: 1, kind: 'add', amount: 1 },
  { field: 'terrain:height', layer: 'PLAYER', center: [NaN, 0], radius: 1, kind: 'add', amount: 1 },
  { field: 'terrain:height', layer: 'PLAYER', center: [0, 0], place: [Infinity, 0], radius: 1, kind: 'add', amount: 1 },
  { field: 'terrain:height', layer: 'PLAYER', center: [0, 0], radius: 0, kind: 'add', amount: 1 },
  { field: 'terrain:height', layer: 'PLAYER', center: [0, 0], radius: NaN, kind: 'add', amount: 1 },
]
const applyStroke = (paint, s) => paint.stroke({ field: s.field, layer: s.layer, center: s.center, place: s.place, radius: s.radius, fn: STROKE_FN[s.kind](s) })
const READS = SAMPLES.flatMap(([x, z]) => TERRAIN_FIELD_NAMES.map((field) => ({ op: 'read', field, x, z })))
const CHUNK_QUERIES = [
  ['terrain:height', 'PLAYER', 0, 0],
  ['terrain:height', 'PLAYER', -1, 0],
  ['terrain:height', 'CREATOR', 0, 0],
  ['terrain:natural-mask', 'CREATOR', 1, 1],
  ['terrain:water', 'CREATOR', 5, 5],
  ['terrain:water', 'PLAYER', -2, -2],
].map(([field, layer, kx, kz]) => ({ op: 'chunkObject', field, layer, kx, kz }))
const WRAP_POINTS = [[0, 0, 0], [0.25, 0.25, 1], [-2.5, 3, 0.5], [1, -1, -2], [3.5, 4.5, 4], [-3.75, -0.25, 0]]
const PAINT_OPS = [
  ...STROKES.map((s) => ({ op: 'stroke', ...s })),
  ...STROKE_REFUSALS.map((s) => ({ op: 'stroke', ...s })),
  ...READS,
  ...CHUNK_QUERIES,
  ...WRAP_POINTS.map(([x, z, yGuess]) => ({ op: 'wrap', x, z, yGuess })),
  ...WRAP_POINTS.map(([x, z, yGuess]) => ({ op: 'wrapNull', x, z, yGuess })),
  ...SAMPLES.map(([x, z]) => ({ op: 'naturalMask', x, z })),
  { op: 'naturalMaskNull', x: 0, z: 0 },
  { op: 'toJSON' },
  { op: 'undo' },
  { op: 'undo' },
  ...READS,
  ...Array.from({ length: STROKES.length - 1 }, () => ({ op: 'undo' })),
  ...READS,
  ...CHUNK_QUERIES,
  { op: 'toJSON' },
]
const applyPaint = (paint, op) => {
  switch (op.op) {
    case 'stroke': return applyStroke(paint, op)
    case 'undo': return paint.undo()
    case 'read': return paint.read(op.field, op.x, op.z)
    case 'chunkObject': return paint.chunkObject(op.field, op.layer, op.kx, op.kz)
    case 'toJSON': return sortedDoc(paint.toJSON())
    case 'fromJSON': paint.fromJSON(op.input); return null
    case 'wrap': return wrapHeightWithFieldPaint(paint, HEIGHT)(op.x, op.z, op.yGuess)
    case 'wrapNull': return wrapHeightWithFieldPaint(null, HEIGHT)(op.x, op.z, op.yGuess)
    case 'naturalMask': return naturalMaskAt(paint, op.x, op.z)
    case 'naturalMaskNull': return naturalMaskAt(null, op.x, op.z)
  }
}
const makePaint = () => createFieldPaint({ cellSize: CELL, chunkCells: CHUNK })
const paintTrace = runTrace(makePaint, PAINT_OPS, applyPaint)
const savedDoc = paintTrace.find((step) => step.op === 'toJSON').result
const RESTORE_REFUSALS = [
  { op: 'fromJSON', input: null },
  { op: 'fromJSON', input: 3 },
  { op: 'fromJSON', input: { 'terrain:rock': {} } },
  { op: 'fromJSON', input: { 'terrain:height': { GHOST: [] } } },
  { op: 'fromJSON', input: { 'terrain:height': { PLAYER: 'x' } } },
  { op: 'fromJSON', input: { 'terrain:height': { PLAYER: [{ key: '0,0', cells: [1, 2] }] } } },
  { op: 'fromJSON', input: { 'terrain:height': { PLAYER: [{ key: '0,0', cells: [...Array(15).fill(0), 'NaN'] }] } } },
  { op: 'fromJSON', input: { 'terrain:height': { PLAYER: [{ key: 7, cells: Array(16).fill(0) }] } } },
]
const restoredOps = [{ op: 'fromJSON', input: savedDoc }, { op: 'toJSON' }, ...READS, { op: 'undo' }, ...RESTORE_REFUSALS, { op: 'toJSON' }]
const restoredTrace = runTrace(makePaint, restoredOps, applyPaint)

const authoredCells = (k) => Array.from({ length: CHUNK * CHUNK }, (_, i) => (i - 8 + k) * 0.125)
const LAYER_POINTS = [[0, 0], [0.5, 0], [-0.25, -0.25], [1.5, 1.5], [2, 2], [-2.5, -2.5], [-2, -2], [-1.5, 4.5], [6.5, -1.5], [7.75, -0.25]]
const readsAt = (points) => points.map(([x, z]) => ({ op: 'readField', x, z }))
const LAYER_OPS = [
  { op: 'span' },
  ...LAYER_POINTS.map(([x, z]) => ({ op: 'locate', x, z })),
  { op: 'setAuthored', kx: 0, kz: 0, cells: authoredCells(0) },
  { op: 'setAuthored', kx: -1, kz: 2, cells: authoredCells(3) },
  { op: 'setAuthored', kx: 0, kz: 0, cells: [1, 2] },
  ...readsAt(LAYER_POINTS),
  { op: 'paint', x: 0.5, z: 0.5, delta: 0.25 },
  { op: 'paint', x: 0.5, z: 0.5, delta: 0.125 },
  { op: 'paint', x: -0.25, z: -0.25, delta: 1 },
  { op: 'paint', x: 2.5, z: 2.5, delta: -0.5 },
  { op: 'paint', x: 1.5, z: 1.5, delta: 0.75 },
  ...readsAt(LAYER_POINTS),
  { op: 'changes', at: 0 },
  { op: 'changes', at: 3 },
  { op: 'paintedChunks' },
  { op: 'exportRuntime' },
  { op: 'clearRuntime', kx: 0, kz: 0 },
  { op: 'clearRuntime', kx: 0, kz: 0 },
  { op: 'clearRuntime', kx: 7, kz: 7 },
  ...readsAt(LAYER_POINTS),
  { op: 'importRuntime', list: [{ key: '3,-1', cells: Array.from({ length: 16 }, (_, i) => i * 0.5 - 2) }] },
  ...readsAt(LAYER_POINTS),
  { op: 'exportRuntime' },
  { op: 'paintedChunks' },
  { op: 'clearAllRuntime' },
  { op: 'changes', at: 0 },
  { op: 'exportRuntime' },
  ...readsAt(LAYER_POINTS),
  { op: 'importRuntime', list: 'x' },
  { op: 'importRuntime', list: [{ key: 7, cells: Array(16).fill(0) }] },
  { op: 'importRuntime', list: [{ key: '0,0', cells: [1, 2] }] },
  { op: 'importRuntime', list: [{ key: '0,0', cells: [...Array(15).fill(0), 'NaN'] }] },
]
const applyLayer = (layer, op) => {
  switch (op.op) {
    case 'span': return layer.span
    case 'locate': return layer.locate(op.x, op.z)
    case 'setAuthored': layer.setAuthored(op.kx, op.kz, Float32Array.from(op.cells)); return null
    case 'paint': return layer.paint(op.x, op.z, op.delta)
    case 'readField': return layer.readField(op.x, op.z)
    case 'changes': {
      const c = layer.changes(op.at)
      return { chunks: [...c.chunks].sort(), at: c.at }
    }
    case 'clearRuntime': return layer.clearRuntime(op.kx, op.kz)
    case 'clearAllRuntime': return layer.clearAllRuntime()
    case 'importRuntime': return layer.importRuntime(op.list)
    case 'paintedChunks': return [...layer.paintedChunks()].sort()
    case 'exportRuntime': return [...layer.exportRuntime()].sort(byKey)
  }
}
const layerTrace = runTrace(() => createFieldLayers({ cellSize: CELL, chunkCells: CHUNK }), LAYER_OPS, applyLayer)
const layerConstructErrors = [{ cellSize: 0, chunkCells: 4 }, { cellSize: 0.5, chunkCells: 2.5 }, { cellSize: 0.5, chunkCells: 0 }, { cellSize: NaN, chunkCells: 4 }].map((c) => ({ ...c, ...outcome(() => createFieldLayers(c)) }))
const paintConstructErrors = [{ cellSize: 0, chunkCells: 4 }, { cellSize: 0.5, chunkCells: 2.5 }].map((c) => ({ ...c, ...outcome(() => createFieldPaint(c)) }))

const HOST_CASES = [
  { name: 'absent', absent: true },
  { name: 'null terrain config', cfg: null },
  { name: 'empty terrain config', cfg: {} },
  { name: 'fieldPaint undefined', cfg: { fieldPaint: undefined } },
  { name: 'fieldPaint empty object', cfg: { fieldPaint: {} } },
  { name: 'fieldPaint array', cfg: { fieldPaint: [] } },
  { name: 'fieldPaint null sizes', cfg: { fieldPaint: { cellSize: null, chunkCells: null } } },
  { name: 'custom sizes', cfg: { fieldPaint: { cellSize: 0.5, chunkCells: 4 } } },
  { name: 'chunkCells only', cfg: { fieldPaint: { chunkCells: 1 } } },
  { name: 'fieldPaint null', cfg: { fieldPaint: null } },
  { name: 'fieldPaint number', cfg: { fieldPaint: 3 } },
  { name: 'cellSize zero', cfg: { fieldPaint: { cellSize: 0 } } },
  { name: 'cellSize string', cfg: { fieldPaint: { cellSize: '1' } } },
  { name: 'cellSize NaN', cfg: { fieldPaint: { cellSize: NaN } } },
  { name: 'cellSize Infinity', cfg: { fieldPaint: { cellSize: Infinity } } },
  { name: 'chunkCells zero', cfg: { fieldPaint: { chunkCells: 0 } } },
  { name: 'chunkCells fractional', cfg: { fieldPaint: { chunkCells: 2.5 } } },
  { name: 'chunkCells string', cfg: { fieldPaint: { chunkCells: '4' } } },
]
const hostCases = HOST_CASES.map((c) => ({
  name: c.name,
  ...(c.absent ? { absent: true } : { input: c.cfg }),
  ...outcome(() => {
    const o = (c.absent ? createHostFieldPaint() : createHostFieldPaint(c.cfg)).chunkObject(PAINTED_HEIGHT_FIELD, 'PLAYER', 0, 0)
    return { cellSize: o.cellSize, chunkCells: o.chunkCells }
  }),
}))

const out = {
  spoint: rev,
  cellSize: CELL,
  chunkCells: CHUNK,
  fieldNames: [...TERRAIN_FIELD_NAMES],
  layerNames: [...FIELD_LAYER_NAMES],
  paintedHeightField: PAINTED_HEIGHT_FIELD,
  naturalMaskField: NATURAL_MASK_FIELD,
  layerTrace,
  layerConstructErrors,
  paintTrace,
  restoredTrace,
  paintConstructErrors,
  hostCases,
}
if (gitStatus() !== statusBefore) throw new Error('spoint working tree changed')
writeFileSync(outPath, JSON.stringify(encode(out), null, 1) + '\n')
console.log(JSON.stringify({ rev, layerSteps: layerTrace.length, paintSteps: paintTrace.length, restoredSteps: restoredTrace.length, hostCases: hostCases.length }))
