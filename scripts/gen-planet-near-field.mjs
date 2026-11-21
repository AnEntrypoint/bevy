import { writeFixture, spointImport } from './lib/spoint.mjs'

const latticeOptions = [
  { spacingM: 2, extentM: 100 },
  { spacingM: 1, extentM: 400, faceHalfSpanM: 500 },
  { spacingM: 1.5, extentM: 300, gridSteps: 12, tileBudget: 16, faceHalfSpanM: 200 },
  { spacingM: 1, extentM: 60, gridSteps: 2, tileBudget: 1 },
  { spacingM: 0.5, extentM: 1000, gridSteps: 9, tileBudget: 64, faceHalfSpanM: 300 },
  { spacingM: 3, extentM: 20 },
  { spacingM: 1, extentM: 200, gridSteps: 1, tileBudget: 0 },
  { spacingM: 1, extentM: 200, gridSteps: 4.5, tileBudget: 2.5, faceHalfSpanM: -1 },
  { spacingM: 2, extentM: 500, tileBudget: 4, faceHalfSpanM: 250 },
  { spacingM: 0, extentM: 100 },
  { spacingM: 1, extentM: -5 },
  { extentM: 100 },
  { spacingM: 1 },
  { spacingM: Infinity, extentM: 100 },
  { spacingM: 1, extentM: 100, gridSteps: 3, tileBudget: 9, faceHalfSpanM: 40 },
]

const reachGpfs = [undefined, 0.6, 1, 0.1, 5, 1e-9, 1000, 0, -2, NaN]

const gridLeaves = (half, size, originX = -half, originY = -half) => {
  const leaves = []
  for (let y = originY; y < half; y += size) for (let x = originX; x < half; x += size) leaves.push([x, y, size])
  return leaves
}

const mixedLeaves = (half, size) => [
  ...gridLeaves(half, size * 4).filter((_, i) => i % 3 !== 0),
  ...gridLeaves(half, size).filter((_, i) => i % 5 === 0),
  [half / 2, -half, size * 8],
]

const leafSets = (spacing) => {
  const tile = 9 * spacing
  return [
    ['uniform tile', gridLeaves(tile * 6, tile)],
    ['uniform half tile', gridLeaves(tile * 3, tile / 2)],
    ['coarse four tile', gridLeaves(tile * 8, tile * 4)],
    ['mixed', mixedLeaves(tile * 8, tile)],
    ['single huge', [[-tile * 10, -tile * 10, tile * 20]]],
    ['single small', [[0, 0, tile / 4]]],
    ['empty', []],
  ]
}

const cameraFractions = [[0, 0], [0.37, -0.81], [3.2, 2.9], [-4.4, 5.1], [100, 100], [-100, 0], [0, -100]]

const nullIfNonFinite = (value) => (typeof value === 'number' && !Number.isFinite(value) ? null : value)
const encode = (value) => (value === undefined ? null : JSON.parse(JSON.stringify(value, (key, v) => nullIfNonFinite(v))))

await writeFixture('spoint-planet-near-field', async ({ spoint, rev }) => {
  const { createNearFieldLattice } = await spointImport(spoint, 'src/terrain/NearFieldLattice.js')
  const created = latticeOptions.map((options) => {
    const jsOptions = Object.fromEntries(Object.entries(options).filter(([, v]) => v !== undefined))
    try { return { options, jsOptions, lattice: createNearFieldLattice(jsOptions) } } catch (error) {
      if (!(error instanceof RangeError)) throw error
      return { options, jsOptions, lattice: null }
    }
  })
  const creations = created.map(({ options, lattice }) => ({
    options: encode(options),
    lattice: lattice && {
      gridSteps: lattice.gridSteps, tileBudget: lattice.tileBudget, tileM: lattice.tileM, maxLeafM: lattice.maxLeafM,
      requestedHalfExtentM: lattice.requestedHalfExtentM, floorSpanLeaves: lattice.floorSpanLeaves,
      reach: reachGpfs.map((gpf) => ({ gpf: encode(gpf), result: encode(lattice.floorReachLeaves(gpf)) })),
    },
  }))
  const gpfs = [undefined, 1, 0.6, 0.05, 3, 1000, NaN, -1]
  const budgets = [undefined, 1, 16, 64, 1000, 0, 7]
  const overrides = [undefined, 2, 9, 12, 1, 5.5]
  const plans = []
  const sets = {}
  created.forEach(({ jsOptions, lattice }, creation) => {
    if (!lattice) return
    const spacing = jsOptions.spacingM
    const face = jsOptions.faceHalfSpanM > 0 ? jsOptions.faceHalfSpanM : null
    const cameras = [
      ...cameraFractions.map(([fx, fy]) => [fx * lattice.tileM, fy * lattice.tileM]),
      ...(face ? [[face, face], [-face, -face], [face * 3, 0], [-face, face]] : []),
      null, [NaN, 0], [0, Infinity],
    ]
    leafSets(spacing).forEach(([setName, leafTriples], pick) => {
      const set = creation + ' ' + setName
      sets[set] = leafTriples
      const leaves = leafTriples.map(([ox, oy, l]) => ({ ox, oy, l }))
      cameras.forEach((camera, c) => {
        const budget = budgets[(pick + c) % budgets.length]
        const gpf = gpfs[(pick * 3 + c) % gpfs.length]
        const gridSteps = overrides[(pick + c * 2) % overrides.length]
        const plan = lattice.plan(leaves, camera, gridSteps, budget, gpf)
        const out = { ...plan, drop: plan.drop.map((q) => leaves.indexOf(q)) }
        plans.push({ creation, set, camera: encode(camera), gridSteps: encode(gridSteps), budget: encode(budget), gpf: encode(gpf), plan: encode(out) })
      })
    })
  })
  return { source: 'src/terrain/NearFieldLattice.js', rev, leafSets: sets, creations, plans }
})
