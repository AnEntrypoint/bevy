import { writeFixture, spointImport } from './lib/spoint.mjs'

const GRID = 1024
const exactSplitFactor = (target) => {
  const bits = new BigUint64Array(1)
  const view = new Float64Array(bits.buffer)
  view[0] = target / 7.2
  for (let step = -64n; step <= 64n; step++) {
    const probe = new BigUint64Array([bits[0] + step])
    const candidate = new Float64Array(probe.buffer)[0]
    if (candidate * 7.2 === target) return candidate
  }
  throw new Error('no split factor makes the split distance exact')
}
const rounded = (v) => v.map((c) => Math.round(c * GRID) / GRID)
const unit = (d) => { const n = Math.hypot(...d); return d.map((c) => c / n) }
const cross = (a, b) => [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]

const DIRECTIONS = {
  faceCentre: [0, 0, 1],
  generic: [0.3, 0.5, 0.8],
  seam: [1, 0.1, 0.999],
  exactSeam: [1, 0, 1],
  cubeCorner: [1, 1, 1],
  pole: [0.001, 1, 0.002],
}
const ALTITUDES = (R) => ({
  orbit: 2 * R,
  high: 0.25 * R,
  low: 2000,
  ground: 1.7,
  belowGround: -30,
  deepBelow: -0.5 * R,
  hover: 30,
  mid: 0.1 * R,
  shelf: 0.06 * R,
})
const CAMERA_PLANS = [
  ['orbit', 'faceCentre'], ['orbit', 'cubeCorner'], ['mid', 'generic'], ['mid', 'seam'], ['high', 'generic'], ['high', 'seam'], ['low', 'faceCentre'],
  ['low', 'generic'], ['low', 'seam'], ['low', 'cubeCorner'], ['ground', 'faceCentre'], ['ground', 'generic'],
  ['ground', 'seam'], ['ground', 'exactSeam'], ['ground', 'cubeCorner'], ['ground', 'pole'], ['belowGround', 'generic'],
  ['belowGround', 'seam'], ['deepBelow', 'faceCentre'],
]
const DEEP_PLANS = CAMERA_PLANS.filter(([a]) => ['low', 'ground', 'belowGround'].includes(a))
const NEAR_FLOORS = (R) => [
  { leafM: 20, spanLeaves: 2, groundClearanceM: 1.7, innerSpanLeaves: 0 },
  { leafM: 20, spanLeaves: 3, groundClearanceM: null, innerSpanLeaves: 1 },
  { leafM: 5, spanLeaves: 0, groundClearanceM: -4, innerSpanLeaves: -1 },
  { leafM: 0, spanLeaves: 2, groundClearanceM: 1.7, innerSpanLeaves: 0 },
  { leafM: 100, spanLeaves: 5, groundClearanceM: 800, innerSpanLeaves: 4 },
  { leafM: 20, spanLeaves: 2, groundClearanceM: 80, innerSpanLeaves: 0 },
  { leafM: 20, spanLeaves: 2, groundClearanceM: 300, innerSpanLeaves: 0 },
  { leafM: (2 * R) / 2048, spanLeaves: 2, groundClearanceM: null, innerSpanLeaves: 0 },
]
const LOD_CONFIGS = [
  { name: 'default', maxLevel: 11, splitFactor: 0.25, viewportH: 1080, fovDegrees: 60, plans: CAMERA_PLANS },
  { name: 'preset', maxLevel: 13, splitFactor: 0.25, viewportH: 1440, fovDegrees: 75, plans: CAMERA_PLANS.filter((_, i) => i % 2 === 0) },
  { name: 'deep', maxLevel: 22, splitFactor: 0.25, viewportH: 1080, fovDegrees: 60, plans: DEEP_PLANS },
  { name: 'coarse', maxLevel: 11, splitFactor: 0.5, viewportH: 720, fovDegrees: 90, plans: DEEP_PLANS.slice(0, 3) },
  { name: 'wide', maxLevel: 11, splitFactor: 1, viewportH: 1080, fovDegrees: 60, plans: [['mid', 'generic'], ['shelf', 'generic']] },
  { name: 'tiny', maxLevel: 11, splitFactor: 0.05, viewportH: 720, fovDegrees: 90, plans: DEEP_PLANS.slice(0, 3) },
  { name: 'tie', maxLevel: 13, splitFactor: exactSplitFactor(2), viewportH: 1024, fovDegrees: 80, plans: [['hover', 'faceCentre'], ['hover', 'generic'], ['hover', 'cubeCorner']] },
]

function aimGroundPointOf(source) {
  const text = source.match(/function aimGroundPoint[\s\S]*?\n}\n/)[0]
  return new Function(text + '; return aimGroundPoint')()
}

await writeFixture('spoint-planet-quadtree', async ({ spoint, rev }) => {
  const { Quadtree, localToDeformed } = await spointImport(spoint, 'packages/mapspinner/src/quadtree.js')
  const { PLANET_RADIUS_M } = await spointImport(spoint, 'packages/mapspinner/src/planet-radius.js')
  const { TERRAIN_DEFAULTS } = await spointImport(spoint, 'packages/mapspinner/src/terrain-defaults.js')
  const { resolveLodConfig, applyLodConfig, LOD_STEP, LOD_POP_ALTITUDE_MUL } = await spointImport(spoint, 'packages/mapspinner/src/lod-config.js')
  const { FACE_FRAME, worldToFaceLocal, pickFace } = await spointImport(spoint, 'packages/mapspinner/src/planet-orchestrator-cull.js')
  const { readFileSync } = await import('node:fs')
  const aimGroundPoint = aimGroundPointOf(readFileSync(spoint + '/packages/mapspinner/src/tsl/planet-tsl.js', 'utf8'))

  const referenceLeaves = (R, lodCfg, camera, nearFloor, forward) => {
    const qt = new Quadtree(R)
    applyLodConfig(qt, R, lodCfg.viewportH, resolveLodConfig({ splitFactor: lodCfg.splitFactor, maxLevel: lodCfg.maxLevel }, lodCfg.fovDegrees / 180 * Math.PI))
    const camDist = Math.hypot(...camera)
    const aim = forward ? aimGroundPoint(camera, forward, camDist, R) : null
    const nearFace = pickFace(camera)
    const keys = []
    const nodes = []
    for (let face = 0; face < 6; face++) {
      const lc = worldToFaceLocal(face, camera, R)
      const al = aim ? worldToFaceLocal(face, aim, R) : null
      if (nearFloor && face === nearFace) qt.setNearFieldLeafFloor(nearFloor.leafM, nearFloor.spanLeaves, nearFloor.groundClearanceM, nearFloor.innerSpanLeaves)
      else qt.setNearFieldLeafFloor(0, 1)
      const leaves = qt.updateQuadtree(lc[0], lc[1], lc[2], lc[0], lc[1], al ? al[0] : undefined, al ? al[1] : undefined, camDist - R, null)
      for (const q of leaves) {
        keys.push(face + ':' + q.level + ':' + q.tx + ':' + q.ty)
        nodes.push({ face, level: q.level, tx: q.tx, ty: q.ty, ox: q.ox, oy: q.oy, l: q.l })
      }
    }
    return { keys: keys.join(' '), aim, nodes, splitDist: qt.splitDist }
  }

  const referenceChildren = (R, n) => {
    const qt = new Quadtree(R)
    qt.computeSplitDist(1.8, 1024, Math.PI / 3)
    qt.setConfig(R, n.level + 1, 2.0)
    qt.updateQuadtree(n.ox + n.l / 2, n.oy + n.l / 2, R, n.ox + n.l / 2, n.oy + n.l / 2, undefined, undefined, 0, null)
    qt._n = 0
    qt._recurse(n.level, n.tx, n.ty, n.ox, n.oy, n.l)
    return qt._leaves.slice(0, qt._n).map((q) => ({ face: n.face, level: q.level, tx: q.tx, ty: q.ty, ox: q.ox, oy: q.oy, l: q.l }))
  }

  const nodeRecord = (R, n) => {
    const F = FACE_FRAME[n.face]
    const corner = ([u, v]) => {
      const p = localToDeformed(n.ox + u * n.l, n.oy + v * n.l, 0, R)
      return [0, 1, 2].map((k) => (p[0] * F.u[k] + p[1] * F.v[k] + p[2] * F.c[k]) / R)
    }
    const children = referenceChildren(R, n)
    const parentOrigin = { face: n.face, level: n.level - 1, tx: n.tx >> 1, ty: n.ty >> 1, ox: n.ox - (n.tx & 1) * n.l, oy: n.oy - (n.ty & 1) * n.l, l: 2 * n.l }
    const parentChildren = n.level > 0 ? referenceChildren(R, parentOrigin) : []
    if (n.level > 0 && !parentChildren.some((c) => c.tx === n.tx && c.ty === n.ty)) throw new Error('parent candidate does not list the node as a child')
    return {
      radius: R, face: n.face, level: n.level, x: n.tx, y: n.ty, size: n.l,
      corners: [[0, 0], [1, 0], [0, 1], [1, 1], [0.5, 0.5]].map((uv) => ({ uv, dir: corner(uv) })),
      children: children.map((c) => [c.level, c.tx, c.ty]),
      parent: n.level > 0 ? [parentOrigin.level, parentOrigin.tx, parentOrigin.ty] : null,
    }
  }

  const cases = []
  const nodeSeeds = new Map()
  for (const R of [63600, 6360000]) {
    for (let face = 0; face < 6; face++) nodeSeeds.set(R + ':' + face + ':0:0:0', { R, node: { face, level: 0, tx: 0, ty: 0, ox: -R, oy: -R, l: 2 * R } })
    const altitude = ALTITUDES(R)
    const addCase = (name, lodCfg, camera, nearFloor, forward) => {
      const result = referenceLeaves(R, lodCfg, camera, nearFloor, forward)
      cases.push({
        name, radius: R, maxLevel: lodCfg.maxLevel, splitFactor: lodCfg.splitFactor, viewportH: lodCfg.viewportH, fovDegrees: lodCfg.fovDegrees, splitDist: result.splitDist,
        camera, nearFloor, forward, aim: result.aim, leafCount: result.nodes.length, leaves: result.keys,
      })
      const stride = Math.max(1, Math.floor(result.nodes.length / 2))
      result.nodes.forEach((n, i) => { if (i % stride === 0) nodeSeeds.set(R + ':' + n.face + ':' + n.level + ':' + n.tx + ':' + n.ty, { R, node: n }) })
    }
    for (const lodCfg of LOD_CONFIGS) {
      for (const [alt, dirName] of lodCfg.plans) {
        const d = unit(DIRECTIONS[dirName])
        const camera = rounded(d.map((c) => c * (R + altitude[alt])))
        addCase(lodCfg.name + '/' + alt + '/' + dirName, lodCfg, camera, null, null)
      }
    }
    const downward = (camera, tiltRad) => {
      const up = unit(camera)
      const east = unit(cross([0, 1, 0], up))
      return rounded([0, 1, 2].map((k) => east[k] * Math.cos(tiltRad) - up[k] * Math.sin(tiltRad)))
    }
    const tilts = [['ground', 'generic', 0.1], ['ground', 'generic', -0.3], ['low', 'seam', 0.4], ['high', 'faceCentre', 0.5], ['orbit', 'generic', 0.2]]
    for (const lodCfg of [LOD_CONFIGS[0], LOD_CONFIGS[2]]) {
      for (const [alt, dirName, tilt] of tilts) {
        const camera = rounded(unit(DIRECTIONS[dirName]).map((c) => c * (R + altitude[alt])))
        const toward = rounded(camera.map((c, k) => (k === 0 ? -c * 0.3 : -c)))
        addCase(lodCfg.name + '/aim/' + alt + '/' + dirName + '/' + tilt, lodCfg, camera, null, tilt > 0.3 ? toward : downward(camera, tilt))
      }
    }
    for (const [lodCfg, dirNames] of [[LOD_CONFIGS[0], ['faceCentre', 'exactSeam']], [LOD_CONFIGS[2], ['seam']]]) {
      NEAR_FLOORS(R).forEach((nearFloor, i) => {
        for (const dirName of dirNames) {
          const camera = rounded(unit(DIRECTIONS[dirName]).map((c) => c * (R + 1.7)))
          addCase(lodCfg.name + '/floor' + i + '/' + dirName, lodCfg, camera, nearFloor, null)
        }
      })
    }
  }
  const nodes = [...nodeSeeds.values()].map(({ R, node }) => nodeRecord(R, node))
  return { source: 'packages/mapspinner/src/quadtree.js, lod-config.js, planet-orchestrator-cull.js, aimGroundPoint text from tsl/planet-tsl.js', spointRev: rev, defaults: { radius: PLANET_RADIUS_M, maxLevel: TERRAIN_DEFAULTS.maxLevel, splitFactor: TERRAIN_DEFAULTS.splitFactor, lodStep: LOD_STEP, popAltitudeMul: LOD_POP_ALTITUDE_MUL }, cases, nodes }
})
