import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-terrain-grid-sample.json')
const rev = execFileSync('git', ['-C', spoint, 'rev-parse', '--short', 'HEAD']).toString().trim()
const statusBefore = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
const { sampleTerrainGridChunked } = await import(pathToFileURL(join(spoint, 'src/terrain/HeightfieldStreamer.js')).href)

const N = 12
const spacing = 40
const cornerX = -120
const cornerZ = -120
const UNSLICED_MS = 1e9
const EVERY_ROW_MS = 0
const ABORT_AFTER_YIELD = 4
const SENTINEL = 7.5

const heightFn = (x, z) => (x < -100 && z < -100 ? NaN : x > 250 && z > 250 ? Infinity : 0.03 * x - 0.02 * z + 0.0004 * x * z)

const run = async (budgetMs, seed = null, abortAfterYield = null) => {
  let heightFnCalls = 0
  let yields = 0
  const counted = (x, z) => {
    heightFnCalls += 1
    return heightFn(x, z)
  }
  const isAborted = () => {
    yields += 1
    return yields === abortAfterYield
  }
  const g = await sampleTerrainGridChunked({ heightFn: counted, N, spacing, cornerX, cornerZ, budgetMs, isAborted, seed })
  return { g, heightFnCalls, yields }
}

const record = (name, budgetMs, result, seed = null, abortAfterYield = null) => ({
  name,
  budgetMs,
  abortAfterYield,
  yields: result.yields,
  heightFnCalls: result.heightFnCalls,
  aborted: result.g === null,
  seed: seed ? { filled: [...seed.filled], samples: [...seed.samples] } : null,
  reused: result.g ? result.g.reused : null,
  solved: result.g ? result.g.solved : null,
  samples: result.g ? [...result.g.samples] : null,
})

const unslicedRun = await run(UNSLICED_MS)
const filled = Array.from({ length: N * N }, (_, k) => (((k % N) + Math.floor(k / N)) % 3 === 0 ? 1 : 0))
const seed = { filled, samples: Float32Array.from(unslicedRun.g.samples, (v, k) => (filled[k] ? SENTINEL : v)) }
const seededRun = await run(UNSLICED_MS, seed)
const slicedRun = await run(EVERY_ROW_MS)
const abortedRun = await run(EVERY_ROW_MS, null, ABORT_AFTER_YIELD)

const reusedCount = filled.reduce((a, b) => a + b, 0)
if (seededRun.g.reused !== reusedCount || seededRun.g.solved !== N * N - reusedCount) throw new Error('seed round trip counts differ')
if (seededRun.g.samples.some((v, k) => filled[k] && v !== SENTINEL)) throw new Error('seeded cells were recomputed')
if (slicedRun.g.samples.some((v, k) => v !== unslicedRun.g.samples[k])) throw new Error('time slicing changed the grid')
if (abortedRun.g !== null) throw new Error('abort did not return null')

const scenarios = [
  record('unsliced', UNSLICED_MS, unslicedRun),
  record('sliced', EVERY_ROW_MS, slicedRun),
  record('seeded', UNSLICED_MS, seededRun, seed),
  record('aborted', EVERY_ROW_MS, abortedRun, null, ABORT_AFTER_YIELD),
]
const statusAfter = execFileSync('git', ['-C', spoint, 'status', '--porcelain']).toString()
if (statusAfter !== statusBefore) throw new Error('spoint working tree changed')
const fixture = {
  spoint: rev,
  geometry: { N, spacing, cornerX, cornerZ },
  heightFn: 'x < -100 && z < -100 gives NaN; x > 250 && z > 250 gives Infinity; otherwise 0.03 * x - 0.02 * z + 0.0004 * x * z',
  notes: 'Time slicing is pinned by the budget. budgetMs 1e9 never elapses, so the run never yields (yields 0). budgetMs 0 makes performance.now() minus the slice start at least 0 after every row, so the run yields once per row (yields N) and isAborted is consulted each time. sampleMs is wall-clock time and is omitted. The grid is identical under both slicings. abortAfterYield 4 returns null after four rows. Seeded run: cells where (column + row) % 3 is 0 are marked filled and set to 7.5 in the seed, so reused counts them and solved takes the rest.',
  scenarios,
  spointStatusBefore: statusBefore,
  spointStatusAfter: statusAfter,
}
writeFileSync(outPath, JSON.stringify(fixture, null, 1) + String.fromCharCode(10))
console.log(JSON.stringify(scenarios.map((s) => ({ name: s.name, yields: s.yields, heightFnCalls: s.heightFnCalls, aborted: s.aborted, reused: s.reused, solved: s.solved }))))
