import { readFileSync } from 'node:fs'
import { createHash } from 'node:crypto'
import { join } from 'node:path'
import { writeParity, spointImport } from './lib/spoint.mjs'

const MAPSPINNER = 'packages/mapspinner/src/'
const TRANSMITTANCE_STRIDE = { x: 8, y: 4 }
const SCATTERING_STRIDE = { x: 4, y: 4, layer: 3 }
const TIME_OF_DAY_STEPS = 96
const KEYFRAME_ELEVATIONS_DEG = [-6, 0, 8, 30, 67]
const TRANSMITTANCE_PROBES = { r: [6360, 6380, 6430, 6500], mu: [-1, -0.5, -0.1, 0, 0.1, 0.5, 1] }
const INSCATTER_PROBES = { r: [6360, 6380, 6430, 6500], mu: [-0.5, -0.1, 0.1, 0.5, 1], muS: [-0.5, 0.1, 0.9] }

const readSource = (spoint, rel) => readFileSync(join(spoint, rel), 'utf8').replace(/\r/g, '')

function sliceBetween(text, startMarker, endMarker) {
  const start = text.indexOf(startMarker)
  if (start < 0) throw new Error('missing ' + startMarker)
  const end = text.indexOf(endMarker, start)
  if (end < 0) throw new Error('missing end after ' + startMarker)
  return text.slice(start, end + endMarker.length)
}

function evaluate(scope, expression) {
  const names = Object.keys(scope)
  return new Function(...names, 'return (' + expression + ')')(...names.map((n) => scope[n]))
}

function constLines(text, namePattern, scope) {
  const out = {}
  for (const line of text.split('\n')) {
    const m = line.match(new RegExp('^(?:export )?const (' + namePattern + ') = (.+)$'))
    if (!m) continue
    out[m[1]] = evaluate({ ...scope, ...out }, m[2])
  }
  return out
}

const float32Bytes = (data) => Buffer.from(data.buffer, data.byteOffset, data.byteLength)
const sha256 = (data) => createHash('sha256').update(float32Bytes(data)).digest('hex')

function strideIndices(count, stride) {
  const indices = []
  for (let i = 0; i < count; i += stride) indices.push(i)
  if (indices[indices.length - 1] !== count - 1) indices.push(count - 1)
  return indices
}

function sampleTransmittance({ data, width, height }) {
  const xs = strideIndices(width, TRANSMITTANCE_STRIDE.x)
  const ys = strideIndices(height, TRANSMITTANCE_STRIDE.y)
  const values = []
  for (const y of ys) for (const x of xs) for (let c = 0; c < 3; c++) values.push(data[(y * width + x) * 3 + c])
  return { xs, ys, values }
}

function sampleScattering({ data, width, height, layers }) {
  const xs = strideIndices(width, SCATTERING_STRIDE.x)
  const ys = strideIndices(height, SCATTERING_STRIDE.y)
  const layerIndices = strideIndices(layers, SCATTERING_STRIDE.layer)
  const values = []
  for (const l of layerIndices) for (const y of ys) for (const x of xs) for (let c = 0; c < 4; c++) values.push(data[((l * height + y) * width + x) * 4 + c])
  return { xs, ys, layers: layerIndices, values }
}

function transmittanceProbes(transmittance) {
  const cases = []
  for (const r of TRANSMITTANCE_PROBES.r) for (const mu of TRANSMITTANCE_PROBES.mu) {
    cases.push({ r, mu, transmittance: transmittance.transmittanceAt(r, mu), u: transmittance.uFromR(r), v: transmittance.vFromMu(mu, r) })
  }
  return cases
}

function inscatterProbes(scattering, transLut) {
  const cases = []
  for (const r of INSCATTER_PROBES.r) for (const mu of INSCATTER_PROBES.mu) for (const muS of INSCATTER_PROBES.muS) {
    const { inscatR, inscatM } = scattering.inscatterAt(r, mu, muS, transLut)
    cases.push({ r, mu, muS, inscatR, inscatM })
  }
  return cases
}

function muSLayers(scattering, layers) {
  const muS = Array.from({ length: layers }, (_, l) => scattering.muSFromLayer(l, layers))
  return { muS, layerFromMuS: muS.map((m) => scattering.layerFromMuS(m, layers)) }
}

function fakeLight() {
  return { hex: 0, intensity: 0, color: { setHex(h) { this.owner.hex = h } }, position: { set() {} } }
}

function timeOfDaySamples(createTimeOfDay) {
  const sun = fakeLight(); sun.color.owner = sun
  const ambient = fakeLight(); ambient.color.owner = ambient
  const studio = { intensity: 1 }
  const tod = createTimeOfDay(sun, ambient, { studio })
  const sample = (fraction) => {
    const out = tod.setFraction(fraction)
    return { fraction, elevationDeg: out.elevDeg, dir: [...out.dir], sunColor: out.sunColor, sunIntensity: out.sunIntensity, ambientColor: out.ambientColor, ambientIntensity: out.ambientIntensity, fillScale: studio.intensity }
  }
  const regular = Array.from({ length: TIME_OF_DAY_STEPS }, (_, k) => sample(k / TIME_OF_DAY_STEPS))
  const keyframeFractions = KEYFRAME_ELEVATIONS_DEG.map((deg) => 0.25 + Math.asin(deg / (90 - 23)) / (2 * Math.PI))
  return { regular, atKeyframeElevations: keyframeFractions.map(sample) }
}

function timeOfDayDefaults(createTimeOfDay) {
  const sun = fakeLight(); sun.color.owner = sun
  const ambient = fakeLight(); ambient.color.owner = ambient
  const tod = createTimeOfDay(sun, ambient, {})
  return { startFraction: tod.getFraction(), dayLengthSec: tod.getDayLengthSec() }
}

function keyframes(THREE, todSource) {
  const text = sliceBetween(todSource, 'const TERRAIN_AMBIENT_FLOOR_ALBEDO', '\n]\n')
  const { KEYFRAMES } = new Function('THREE', text + '\nreturn { KEYFRAMES }')(THREE)
  const linear = (hex) => { const c = new THREE.Color(hex); return [c.r, c.g, c.b] }
  return KEYFRAMES.map((k) => ({ ...k, sunLinear: linear(k.sunColor), ambientLinear: linear(k.ambientColor) }))
}

function sceneLights(sceneSource) {
  const text = sliceBetween(sceneSource, 'export function setupLights', '\n}\n').replace('export function', 'function')
  const log = []
  const object = (kind, args) => {
    const o = { kind, args, calls: [], props: {} }
    o.position = { set: (...a) => o.calls.push(['position.set', a]) }
    o.shadow = { mapSize: { set: (...a) => o.calls.push(['shadow.mapSize.set', a]) }, camera: {} }
    o.target = {}
    o.updateMatrix = () => {}
    log.push(o)
    return new Proxy(o, { set(t, k, v) { if (k === 'castShadow' || k === 'matrixAutoUpdate') t.props[k] = v; else t[k] = v; return true } })
  }
  const THREE = {
    AmbientLight: function (...a) { return object('AmbientLight', a) },
    DirectionalLight: function (...a) { return object('DirectionalLight', a) },
    PointLight: function (...a) { return object('PointLight', a) },
  }
  new Function('THREE', 'scene', text + '\nreturn setupLights(scene)')(THREE, { add: () => {} })
  const shadowOf = (o) => ({ bias: o.shadow.bias, normalBias: o.shadow.normalBias, radius: o.shadow.radius, near: o.shadow.camera.near, far: o.shadow.camera.far })
  return log.map((o) => ({ kind: o.kind, args: o.args, calls: o.calls, props: o.props, shadow: o.kind === 'DirectionalLight' ? shadowOf(o) : null }))
}

function sceneBackdrop(sceneSource) {
  const background = sceneSource.match(/scene\.background = new THREE\.Color\((0x[0-9a-fA-F]+)\)/)[1]
  const fog = sceneSource.match(/scene\.fog = new THREE\.FogExp2\((0x[0-9a-fA-F]+), ([0-9.]+)\)/)
  return { background: Number(background), fogColor: Number(fog[1]), fogDensity: Number(fog[2]) }
}

function displayReferred(source) {
  const scalar = constLines(source, 'DISPLAY_CLAMP|TONE_MAPPING_[A-Z]+|ACES_INPUT_ROWS|ACES_OUTPUT_ROWS', {})
  const invertRows3 = evaluate({}, sliceBetween(source, 'function invertRows3', '\n}\n').replace('function invertRows3', 'function'))
  return {
    constants: scalar,
    acesInputInverseRows: invertRows3(scalar.ACES_INPUT_ROWS),
    acesOutputInverseRows: invertRows3(scalar.ACES_OUTPUT_ROWS),
    invertAcesFilmicSource: sliceBetween(source, 'function invertAcesFilmic', '\n}\n'),
    sceneLinearToDisplayReferredSource: sliceBetween(source, 'export function sceneLinearToDisplayReferred', '\n}\n'),
    displayReferredToSceneLinearSource: sliceBetween(source, 'export function displayReferredToSceneLinear', '\n}\n'),
  }
}

function qualityExposure(source) {
  const presets = evaluate({}, sliceBetween(source, 'const PRESETS = {', '\n}\n').replace('const PRESETS = ', ''))
  return Object.fromEntries(Object.entries(presets).map(([name, p]) => [name, { toneMappingExposure: p.toneMappingExposure, shadowsEnabled: p.shadowsEnabled }]))
}

function skyConstants(skySource, transmittance) {
  const imported = { ATM_BOTTOM: transmittance.ATM_BOTTOM, ATM_TOP: transmittance.ATM_TOP, ATM_RAYLEIGH: transmittance.ATM_RAYLEIGH, ATM_MIE_EXT: transmittance.ATM_MIE_EXT, ATM_RAYLEIGH_H: transmittance.ATM_RAYLEIGH_H, ATM_MIE_H: transmittance.ATM_MIE_H }
  return constLines(skySource, '(?:ATM|SKY|HALO)_[A-Z0-9_]+|MIE_K|RAYLEIGH_PHASE_K', imported)
}

await writeParity(async ({ spoint, rev }) => {
  const transmittance = await spointImport(spoint, MAPSPINNER + 'atmosphere-transmittance-lut.js')
  const scattering = await spointImport(spoint, MAPSPINNER + 'atmosphere-scattering-lut.js')
  const { createTimeOfDay } = await spointImport(spoint, 'client/core/TimeOfDay.js')
  const THREE = await spointImport(spoint, 'node_modules/three/build/three.module.js')
  const transLut = transmittance.bakeTransmittanceLUT()
  const scatLut = scattering.bakeScatteringLUT(undefined, undefined, undefined, undefined, transLut)
  const todSource = readSource(spoint, 'client/core/TimeOfDay.js')
  const sceneSource = readSource(spoint, 'client/core/SceneSetup.js')
  const fixture = {
    spointRev: rev,
    nodeVersion: process.version,
    rule: 'sample indices per axis are 0, stride, 2*stride, ... below the axis size, plus size-1 when not already present; entries are stored x fastest, then y, then layer, channels innermost; values are the float32 table entries widened to double',
    constants: {
      bottom: transmittance.ATM_BOTTOM,
      top: transmittance.ATM_TOP,
      rayleighScaleHeight: transmittance.ATM_RAYLEIGH_H,
      rayleigh: transmittance.ATM_RAYLEIGH,
      mieScaleHeight: transmittance.ATM_MIE_H,
      mieExtinction: transmittance.ATM_MIE_EXT,
      transmittanceWidth: transmittance.LUT_WIDTH,
      transmittanceHeight: transmittance.LUT_HEIGHT,
      scatteringWidth: scattering.SCAT_LUT_WIDTH,
      scatteringHeight: scattering.SCAT_LUT_HEIGHT,
      scatteringLayers: scattering.SCAT_LUT_LAYERS,
      sky: skyConstants(readSource(spoint, MAPSPINNER + 'tsl/sky-tsl.js'), transmittance),
    },
    transmittanceTable: {
      width: transLut.width, height: transLut.height, channels: 3, stride: TRANSMITTANCE_STRIDE, entries: transLut.data.length,
      sha256Float32Le: sha256(transLut.data), sample: sampleTransmittance(transLut),
    },
    scatteringTable: {
      width: scatLut.width, height: scatLut.height, layers: scatLut.layers, channels: 4, stride: SCATTERING_STRIDE, entries: scatLut.data.length,
      sha256Float32Le: sha256(scatLut.data), sample: sampleScattering(scatLut),
    },
    transmittanceProbes: { steps: 500, cases: transmittanceProbes(transmittance) },
    inscatterProbes: { steps: 64, cases: inscatterProbes(scattering, transLut) },
    muSLayers: muSLayers(scattering, scatLut.layers),
    keyframes: keyframes(THREE, todSource),
    timeOfDay: { ...timeOfDayDefaults(createTimeOfDay), tiltDeg: 23, samples: timeOfDaySamples(createTimeOfDay) },
    sceneSetup: { backdrop: sceneBackdrop(sceneSource), lights: sceneLights(sceneSource) },
    displayReferred: displayReferred(readSource(spoint, MAPSPINNER + 'tsl/display-referred-tsl.js')),
    qualityPresets: qualityExposure(readSource(spoint, 'client/core/QualityPresets.js')),
  }
  return { 'spoint-planet-atmosphere.json': fixture }
}, { indent: 1 })
