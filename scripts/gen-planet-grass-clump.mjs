import { writeParity, spointImport } from './lib/spoint.mjs'

const CANVAS_SIZE = 256
const PROBE_STEP = 2
const EDGE_MARGIN_PX = 2
const GEO_CASES = [[2, 1], [1, 1], [3, 3]]

const recordingCanvas = () => {
  const calls = []
  const ctx = {
    set fillStyle(v) { calls.push({ op: 'fillStyle', v }) },
    fillRect() { calls.push({ op: 'rect' }) },
    beginPath() { calls.push({ op: 'begin' }) },
    moveTo(x, y) { calls.push({ op: 'm', x, y }) },
    lineTo(x, y) { calls.push({ op: 'l', x, y }) },
    closePath() { calls.push({ op: 'close' }) },
    fill() { calls.push({ op: 'fill' }) },
  }
  globalThis.document = { createElement: () => ({ width: 0, height: 0, getContext: () => ctx }) }
  return calls
}

const filledPolygons = (calls) => {
  const polys = []
  let cur = null
  let grey = 0
  for (const c of calls) {
    if (c.op === 'fillStyle' && /^rgb/.test(c.v)) grey = Number(c.v.match(/\d+/)[0])
    if (c.op === 'begin') cur = { grey, pts: [] }
    if (c.op === 'm' || c.op === 'l') cur.pts.push([c.x, c.y])
    if (c.op === 'fill') polys.push(cur)
  }
  return polys
}

const inside = (p, x, y) => {
  let c = false
  for (let i = 0, j = p.length - 1; i < p.length; j = i++) {
    const [xi, yi] = p[i], [xj, yj] = p[j]
    if ((yi > y) !== (yj > y) && x < ((xj - xi) * (y - yi)) / (yj - yi) + xi) c = !c
  }
  return c
}

const edgeDistance = (p, x, y) => {
  let best = Infinity
  for (let i = 0, j = p.length - 1; i < p.length; j = i++) {
    const [ax, ay] = p[j], [bx, by] = p[i]
    const dx = bx - ax, dy = by - ay
    const t = Math.max(0, Math.min(1, ((x - ax) * dx + (y - ay) * dy) / (dx * dx + dy * dy || 1)))
    best = Math.min(best, Math.hypot(x - ax - t * dx, y - ay - t * dy))
  }
  return best
}

await writeParity(async ({ spoint, rev }) => {
  const calls = recordingCanvas()
  const { makeClumpAlphaTexture, makeClumpGeo } = await spointImport(spoint, 'client/core/GrassClump.js')
  makeClumpAlphaTexture()
  const polys = filledPolygons(calls)
  const probes = []
  for (let py = 0; py < CANVAS_SIZE; py += PROBE_STEP) for (let px = 0; px < CANVAS_SIZE; px += PROBE_STEP) {
    const x = px + 0.5, y = py + 0.5
    if (polys.some((p) => edgeDistance(p.pts, x, y) < EDGE_MARGIN_PX)) continue
    let value = 0
    for (const p of polys) if (inside(p.pts, x, y)) value = p.grey
    if (value > 0 || px < 64 || (px % 10 === 0 && py % 14 === 0)) probes.push([px, py, value])
  }
  const geo = Object.fromEntries(GEO_CASES.map(([blades, crossings]) => {
    const g = makeClumpGeo(blades, crossings)
    return [blades + '_' + crossings, { position: Array.from(g.attributes.position.array), normal: Array.from(g.attributes.normal.array), uv: Array.from(g.attributes.uv.array), index: Array.from(g.index.array) }]
  }))
  return {
    'spoint-planet-grass-clump.json': {
      rev,
      rule: 'makeClumpAlphaTexture draws filled polygons on a recording canvas; each probe pixel centre further than the edge margin from every polygon edge takes the grey of the last polygon containing it; makeClumpGeo geometry is recorded whole',
      polygons: polys.length,
      probes,
      geo,
    },
  }
})
