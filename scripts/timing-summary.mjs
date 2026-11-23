import { readdirSync, readFileSync } from 'node:fs'

const dir = new URL('../target/witness/timing-history/', import.meta.url)
const files = readdirSync(dir).filter((name) => name.endsWith('.json')).sort().slice(-3)
const runs = files.map((name) => JSON.parse(readFileSync(new URL(name, dir), 'utf8')))
const median = (values) => [...values].sort((a, b) => a - b)[Math.floor(values.length / 2)]
const range = (values) => `${Math.min(...values).toFixed(2)}..${Math.max(...values).toFixed(2)}`
const line = (path, key) => runs.map((run) => run[path][key])
for (const path of ['flight', 'ground']) {
  const [p99, over, budget] = [line(path, 'p99Ms'), line(path, 'overBudget'), line(path, 'budgetMs')]
  console.log(`TIMING-SUMMARY ${path} runs=${runs.length} p99 median ${median(p99).toFixed(2)} range ${range(p99)} over median ${median(over)} range ${range(over)} budget median ${median(budget).toFixed(2)} range ${range(budget)}`)
}
console.log(`TIMING-SUMMARY verdicts ${runs.map((run) => run.verdict).join(' ')}`)
