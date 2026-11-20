import { execFileSync } from 'node:child_process'
import { mkdirSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const outDir = join(import.meta.dirname, '..', 'testdata', 'parity')
const rev = execFileSync('git', ['-C', spoint, 'rev-parse', '--short', 'HEAD']).toString().trim()
const { createNoise2D, createFbm2D } = await import(pathToFileURL(join(spoint, 'src/procgen/NoiseTerrain.js')).href)

const points = [[0.3, 0.7], [-3.25, 4.5], [12.75, -8.125], [100.5, -0.25], [0, 0], [-0.001, 0.001]]
const cases = []
for (const seed of [1337, 7, 99999]) {
  for (const [x, z] of points) cases.push({ kind: 'noise2d', seed, x, z })
  for (const octaves of [1, 3, 5]) {
    for (const persistence of [0.5, 0.4]) {
      for (const lacunarity of [2, 2.1]) {
        for (const [x, z] of points) cases.push({ kind: 'fbm', seed, octaves, persistence, lacunarity, x, z })
      }
    }
  }
}
const run = (c) => {
  if (c.kind === 'noise2d') return createNoise2D(c.seed)(c.x, c.z)
  return createFbm2D(c.seed, { octaves: c.octaves, persistence: c.persistence, lacunarity: c.lacunarity })(c.x, c.z)
}
mkdirSync(outDir, { recursive: true })
writeFileSync(join(outDir, 'spoint-noise.json'), JSON.stringify({ source: `spoint ${rev}: src/procgen/NoiseTerrain.js createNoise2D and createFbm2D`, cases, expected: cases.map(run) }) + '\n')
console.log(`wrote ${cases.length} noise cases from spoint ${rev}`)
