import { createHash } from 'node:crypto'
import { createRequire } from 'node:module'
import { readFile } from 'node:fs/promises'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { join } from 'node:path'
import { writeFixture, spointImport } from './lib/spoint.mjs'

const spointRoot = process.argv[2] ?? '/workspace/spoint'
const sharp = createRequire(import.meta.url)(join(spointRoot, 'node_modules/sharp'))
const STRIDE = 8
const OFFSET = [3, 5]
const LAYERS = ['grass', 'rock', 'sand', 'snow']
const NORMAL_FILES = ['grass-normals.jpg', 'ground-normals.jpg', 'sand-normals.jpg', 'snow-normals.jpg']

globalThis.fetch = async (url) => {
  const bytes = await readFile(fileURLToPath(url))
  return { ok: true, status: 200, blob: async () => ({ bytes }) }
}
globalThis.createImageBitmap = async ({ bytes }) => {
  const { data, info } = await sharp(bytes).ensureAlpha().raw().toBuffer({ resolveWithObject: true })
  return { width: info.width, height: info.height, data, close() {} }
}
globalThis.OffscreenCanvas = class {
  constructor(width, height) { this.width = width; this.height = height }
  getContext() {
    let bitmap
    return {
      drawImage: (source, x, y, w, h) => {
        if (source.width !== w || source.height !== h || x !== 0 || y !== 0) throw new Error('drawImage resample is not shimmed')
        bitmap = source
      },
      getImageData: () => ({ data: new Uint8ClampedArray(bitmap.data) }),
    }
  }
}

const FILES = LAYERS.flatMap((name, m) => [name + '-color.jpg', name + '-displacement.jpg', NORMAL_FILES[m]])
const RAW_STRIDE = 16
const rawSampleHex = (data, size) => {
  const out = []
  for (let y = OFFSET[1]; y < size; y += RAW_STRIDE) for (let x = OFFSET[0]; x < size; x += RAW_STRIDE) {
    const o = (y * size + x) * 4
    out.push(Buffer.from(data.subarray(o, o + 4)).toString('hex'))
  }
  return out.join('')
}

const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex')
const sampleHex = (all, layer, size) => {
  const out = []
  for (let y = OFFSET[1]; y < size; y += STRIDE) for (let x = OFFSET[0]; x < size; x += STRIDE) {
    const o = ((layer * size + y) * size + x) * 4
    out.push(Buffer.from(all.subarray(o, o + 4)).toString('hex'))
  }
  return out.join('')
}

await writeFixture('spoint-planet-layers', async ({ spoint, rev }) => {
  const { decodeSurfaceTextureSet, decodeImageToPixels } = await spointImport(spoint, 'packages/mapspinner/src/surface-texture-decode.js')
  const textures = pathToFileURL(join(spoint, 'packages/mapspinner/textures') + '/').href
  const set = await decodeSurfaceTextureSet(textures)
  const layerBytes = set.sz * set.sz * 4
  const layers = LAYERS.map((name, m) => {
    const albedo = set.albAll.subarray(m * layerBytes, (m + 1) * layerBytes)
    const normal = set.nrmAll.subarray(m * layerBytes, (m + 1) * layerBytes)
    return { name, albedoSha256: sha256(albedo), normalSha256: sha256(normal), albedoSample: sampleHex(set.albAll, m, set.sz), normalSample: sampleHex(set.nrmAll, m, set.sz) }
  })
  const rawDecodes = []
  for (const file of FILES) {
    const { data, width } = await decodeImageToPixels(new URL(file, textures).href, set.sz, set.sz)
    rawDecodes.push({ file, sample: rawSampleHex(data, width) })
  }
  return {
    spoint: rev,
    modules: ['packages/mapspinner/src/surface-texture-decode.js'],
    decoder: 'sharp ' + sharp.versions.sharp + ' (libvips, mozjpeg/libjpeg-turbo, fancy upsampling) standing in for the browser createImageBitmap and OffscreenCanvas of the worker; the shims only return decoded RGBA, drawImage at the native 1024x1024 is the identity',
    size: set.sz,
    sampleRule: 'per layer, pixels (x, y) with x = 3 mod 8 and y = 5 mod 8 in row-major order, each as 4 bytes RGBA in lowercase hex; 128 x 128 pixels per layer',
    rawSampleRule: 'per source jpg, the decodeImageToPixels RGBA at pixels x = 3 mod 16, y = 5 mod 16 in row-major order, before any deshade or packing; 64 x 64 pixels per file',
    rawStride: RAW_STRIDE,
    rawDecodes,
    stride: STRIDE,
    offset: OFFSET,
    rockMean: set.rockMean,
    meanL: set.meanL,
    layers,
  }
})
