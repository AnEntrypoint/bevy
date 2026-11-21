import { writeParity, spointImport } from './lib/spoint.mjs'

await writeParity(async ({ spoint, rev }) => {
  const { PhysicsWorld } = await spointImport(spoint, 'src/physics/World.js')
  const { verticalVelocity } = await spointImport(spoint, 'src/shared/characterStep.js')
  const { createPlanetFrame } = await spointImport(spoint, 'src/terrain/PlanetFrame.js')
  const { createChartAnchorLattice, reanchorChartFor, snapshotChart, createChartTransfer } = await spointImport(spoint, 'src/shared/chartAnchor.js')

  const GRAVITY = -9.81
  const DT = 1 / 60
  const RADIUS = 0.4
  const HALF_HEIGHT = 0.9
  const IDENTITY = [0, 0, 0, 1]
  const PLANET_RADIUS = 60
  const ANCHOR_DIR = [0, 1, 0]
  const REANCHOR_DIR = [0.9, 0.3, 0.2]
  const HANDOFF_SOURCE = { anchorDir: [0, 1, 0], offsetY: 0.25 }
  const HANDOFF_DEST = { anchorDir: [0.6, 0.8, 0], offsetY: 0.3 }
  const POINTS = [[0, 0, 0], [1, 2, 3], [-4, 0.5, 6]]
  const VECTORS = [[1, 0, 0], [0, 1, 0], [0.3, -0.7, 2]]
  const round = (v) => v.map((c) => Number(c.toFixed(9)))
  const unit = (v) => { const l = Math.hypot(v[0], v[1], v[2]); return v.map((c) => c / l) }

  const sampler = { radius: PLANET_RADIUS, hashVersion: 1, heightAt: (d) => 0.5 + 0.25 * d[0] - 0.1 * d[2] }
  const frame = createPlanetFrame({ sampler, anchorDir: ANCHOR_DIR, offsetY: 0, reliefScale: 1 })
  const lattice = createChartAnchorLattice(PLANET_RADIUS)
  const reanchor = reanchorChartFor({ frame, lattice, dir: unit(REANCHOR_DIR) })
  if (!reanchor) throw new Error('chart re-anchor did not trigger')
  const { from, to, transfer } = reanchor
  const TILT_RAD = transfer.tiltRad
  const probesOf = (t) => ({
    points: POINTS,
    pointResults: POINTS.map((p) => round(t.point(p))),
    vectors: VECTORS,
    vectorResults: VECTORS.map((v) => round(t.vec(v))),
  })
  const probes = probesOf(transfer)
  const handoffFrom = snapshotChart(createPlanetFrame({ sampler, anchorDir: HANDOFF_SOURCE.anchorDir, offsetY: HANDOFF_SOURCE.offsetY, reliefScale: 1 }))
  const handoffTo = snapshotChart(createPlanetFrame({ sampler, anchorDir: HANDOFF_DEST.anchorDir, offsetY: HANDOFF_DEST.offsetY, reliefScale: 1 }))
  const handoffTransfer = createChartTransfer(handoffFrom, handoffTo)
  const handoff = { from: handoffFrom, to: handoffTo, tiltRad: handoffTransfer.tiltRad, probes: probesOf(handoffTransfer) }

  const w = new PhysicsWorld({ gravity: [0, GRAVITY, 0] })
  await w.init()
  w.addStaticBox([50, 0.5, 50], [0, -0.5, 0], IDENTITY)
  w.addStaticBox([0.5, 2, 20], [5, 2, 0], IDENTITY)
  w.addStaticBox([1.5, 0.15, 6], [-3, 0.15, 0], IDENTITY)

  const FLOOR_VELOCITY = transfer.m.slice(3, 6).map((c) => 0.0005 * c)
  const TILT_BOUND_MARGIN = 1.1
  const bounds = (v) => {
    const h = Math.hypot(v[0], v[2])
    return [TILT_RAD * h + 1e-3, TILT_RAD * h * TILT_BOUND_MARGIN + 1e-3]
  }
  const horizontalAt = (t) => [Math.cos(t), 0, Math.sin(t)]
  const gap = (t) => { const v = transfer.vec(horizontalAt(t)); const [lo, hi] = bounds(v); return v[1] - (lo + hi) / 2 }
  let thetaLo = null, thetaHi = null
  for (let k = 0; k < 720 && thetaLo === null; k++) { const t0 = (k / 720) * 2 * Math.PI, t1 = ((k + 1) / 720) * 2 * Math.PI; if (Math.sign(gap(t0)) !== Math.sign(gap(t1))) { thetaLo = t0; thetaHi = t1 } }
  if (thetaLo === null) throw new Error('no tilt-bound velocity between the margins')
  for (let i = 0; i < 200; i++) { const mid = (thetaLo + thetaHi) / 2; if (Math.sign(gap(mid)) === Math.sign(gap(thetaLo))) thetaLo = mid; else thetaHi = mid }
  const TILT_BOUND_VELOCITY = horizontalAt((thetaLo + thetaHi) / 2)
  const cast = [
    { name: 'grounded-floor', start: [0, 2.3, 0], settle: 40, velocity: FLOOR_VELOCITY },
    { name: 'grounded-tilt-bound', start: [-10, 2.3, 0], settle: 40, velocity: TILT_BOUND_VELOCITY },
    { name: 'airborne', start: [1, 3, 0], settle: 5, velocity: [1, 2, 0.5] },
  ]
  const states = new Map()
  for (const c of cast) {
    const id = w.addPlayerCharacter(RADIUS, HALF_HEIGHT, [...c.start], 80)
    states.set(id, { name: c.name, id, p: [...c.start], v: [0, 0, 0], onGround: false, velocity: c.velocity })
    for (let i = 0; i < c.settle; i++) {
      const st = states.get(id)
      const vy = verticalVelocity(st.v[1], st.onGround, GRAVITY, DT)
      w.setCharacterVelocity(id, [0, vy, 0])
      w.updateCharacter(id, DT)
      w.readCharacterPosition(id, st.p)
      w.readCharacterVelocity(id, st.v)
      st.onGround = w.getCharacterGroundState(id)
    }
    w.setCharacterVelocity(id, c.velocity)
  }

  const mgr = w._charMgr
  const snapshot = (id) => {
    const st = states.get(id)
    const p = [0, 0, 0], v = [0, 0, 0]
    w.readCharacterPosition(id, p)
    w.readCharacterVelocity(id, v)
    const g = mgr._groundNormals.get(id)
    const normal = g && g[3] ? round([g[0], g[1], g[2]]) : null
    return { p: round(p), v: round(v), onGround: w.getCharacterGroundState(id), normal, walls: mgr._walls.get(id)?.[0] ?? 0, name: st.name }
  }
  const before = [...states.keys()].map(snapshot)
  const counts = mgr.applyChartTransfer(transfer)
  const after = [...states.keys()].map(snapshot)

  const cases = before.map((b, i) => ({ name: b.name, before: { p: b.p, v: b.v, onGround: b.onGround, normal: b.normal, walls: b.walls }, after: { p: after[i].p, v: after[i].v, onGround: after[i].onGround, normal: after[i].normal, walls: after[i].walls } }))

  return {
    'spoint-chart-transfer.json': {
      spoint: rev,
      planetRadius: PLANET_RADIUS,
      from,
      to,
      tiltRad: TILT_RAD,
      probes,
      handoff,
      counts: { characters: counts.characters, groundedUpwardClamped: counts.groundedUpwardClamped },
      cases,
    },
  }
})
