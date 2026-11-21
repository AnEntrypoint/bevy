import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const spoint = process.argv[2] ?? '/workspace/spoint'
const outPath = join(import.meta.dirname, '..', 'testdata', 'parity', 'spoint-hud-overlay.json')
const git = (args) => execFileSync('git', ['-C', spoint, ...args]).toString()
const rev = git(['rev-parse', '--short', 'HEAD']).trim()
const statusBefore = git(['status', '--porcelain'])

const shared = await import(pathToFileURL(join(spoint, 'apps/tps-game/shared.js')).href)
const app = await import(pathToFileURL(join(spoint, 'src/apps/AppGameplay.js')).href)
const { tpsGameClient } = await import(pathToFileURL(join(spoint, 'apps/tps-game/client-app.js')).href)
shared.bindCombat(app.COMBAT_API)
const cfg = shared.combatConfig()

const NOW = 100000
const PLAYER = 7
const realNow = Date.now
const TPS_FIELDS = ['lastShootTime', 'hitMarkerTime', 'headshotMarkerTime', 'killTime', 'streak', 'dmgDealtTime', 'lastDamageDealt', 'lastHitTime', 'lastDamageTaken', 'spawnShieldUntil']

const cases = [
  { name: 'idle', tps: {} },
  { name: 'hit-marker-rtt', tps: { hitMarkerTime: NOW - 100 }, rtt: 40 },
  { name: 'hit-marker-rtt-pad', tps: { hitMarkerTime: NOW - 170 }, rtt: 40 },
  { name: 'headshot-marker', tps: { headshotMarkerTime: NOW - 200 } },
  { name: 'kill-multi-rtt', tps: { killTime: NOW - 1000, streak: 4 }, rtt: 300 },
  { name: 'kill-triple-expired', tps: { killTime: NOW - 1600, streak: 3 } },
  { name: 'kill-confirm-cap', tps: { killTime: NOW - 2300, streak: 1 }, rtt: 2000 },
  { name: 'recoil-on', tps: { lastShootTime: NOW - 100 } },
  { name: 'dmg-dealt', tps: { dmgDealtTime: NOW - 200, lastDamageDealt: 25 } },
  { name: 'dmg-taken-low-hp', tps: { lastHitTime: NOW - 300, lastDamageTaken: 12 }, hp: 25 },
  { name: 'dmg-flash', tps: { lastHitTime: NOW - 100, lastDamageTaken: 12 }, hp: 100 },
  { name: 'shield-on', tps: { spawnShieldUntil: NOW + 1000 } },
  { name: 'low-hp-vignette', tps: {}, hp: 10 },
]

const declarationsOf = (text) => Object.fromEntries(
  (text ?? '').split(';').map((d) => d.trim()).filter((d) => d.length > 0).map((d) => {
    const at = d.indexOf(':')
    return [d.slice(0, at).trim(), d.slice(at + 1).trim()]
  })
)

const makeStyle = (text) => {
  const style = Object.assign({}, declarationsOf(text))
  Object.defineProperty(style, 'cssText', {
    enumerable: false,
    configurable: true,
    get() { return '' },
    set(value) { Object.assign(style, declarationsOf(value)) },
  })
  return style
}

class FakeNode {
  constructor(tag, attrs) {
    this.tag = tag
    this.id = attrs.id ?? ''
    this.classes = (attrs.class ?? '').split(' ').filter((c) => c.length > 0)
    this.style = makeStyle(attrs.style)
    this.children = []
    this.textContent = ''
  }
  set innerHTML(markup) { this.children = parseMarkup(markup) }
  appendChild(child) { this.children.push(child); return child }
  get descendants() { return this.children.flatMap((c) => [c, ...c.descendants]) }
  querySelectorAll(selector) {
    const match = selector.startsWith('#')
      ? (n) => n.id === selector.slice(1)
      : (n) => n.classes.includes(selector.slice(1))
    return this.descendants.filter(match)
  }
  querySelector(selector) { return this.querySelectorAll(selector)[0] ?? null }
}

const attrsOf = (text) => Object.fromEntries([...text.matchAll(/([a-zA-Z-]+)="([^"]*)"/g)].map((m) => [m[1], m[2]]))

const parseMarkup = (markup) => {
  const top = []
  const open = [top]
  for (const m of markup.matchAll(/<(\/?)([a-z]+)([^>]*)>/g)) {
    if (m[1] === '/') { open.pop(); continue }
    const node = new FakeNode(m[2], attrsOf(m[3]))
    open.at(-1).push(node)
    open.push(node.children)
  }
  return top
}

const makeDocument = () => {
  const body = new FakeNode('body', {})
  return {
    body,
    getElementById: (id) => body.querySelector('#' + id),
    createElement: (tag) => new FakeNode(tag, {}),
  }
}

const parseColor = (text) => {
  const hex = /^#([0-9a-f]{6})$/i.exec(text)
  if (hex) return [0, 2, 4].map((i) => parseInt(hex[1].slice(i, i + 2), 16)).concat([1])
  const rgba = /^rgba?\(([^)]*)\)$/.exec(text)
  if (rgba) {
    const parts = rgba[1].split(',').map(Number)
    return parts.length === 4 ? parts : parts.concat([1])
  }
  return null
}

const lastColorOf = (text) => {
  const found = [...text.matchAll(/rgba?\([^)]*\)/g)]
  return found.length > 0 ? parseColor(found.at(-1)[0]) : null
}

const px = (v) => (/^-?\d+(\.\d+)?$/.test(v) ? v + 'px' : v)

const transformOf = (text) => {
  if (text === undefined) return null
  const translate = /translate\(([^,]+),([^)]+)\)/.exec(text)
  const rotate = /rotate\(([^)]+)deg\)/.exec(text)
  const scale = /scale\(([^)]+)\)/.exec(text)
  return {
    translate: translate ? [translate[1], translate[2]] : null,
    rotate: rotate ? Number(rotate[1]) : 0,
    scale: scale ? Number(scale[1]) : null,
  }
}

const gradientOf = (background) => {
  if (!background || !background.startsWith('radial-gradient')) return null
  const shape = /radial-gradient\((\w+)/.exec(background)[1]
  const stops = [...background.matchAll(/(rgba?\([^)]*\)) (\d+(?:\.\d+)?%)/g)].map((m) => ({ color: parseColor(m[1]), at: m[2] }))
  return { shape, stops }
}

const geometryOf = (style) => {
  const inset = style.inset === '0'
  return {
    left: px(style.left ?? (inset ? '0' : 'auto')),
    top: px(style.top ?? (inset ? '0' : 'auto')),
    width: px(style.width ?? (inset ? '100%' : 'auto')),
    height: px(style.height ?? (inset ? '100%' : 'auto')),
    transform: transformOf(style.transform),
    fontSize: style['font-size'] ? parseFloat(style['font-size']) : null,
    color: style.color ? parseColor(style.color) : null,
    background: style.background && !style.background.startsWith('radial-gradient') ? parseColor(style.background) : null,
    gradient: gradientOf(style.background),
    boxShadow: style['box-shadow'] ?? null,
    boxShadowColor: style['box-shadow'] ? lastColorOf(style['box-shadow']) : null,
  }
}

const walk = (node, parentId) => node.children.flatMap((child) => [{ node: child, parentId }, ...walk(child, child.id || parentId)])

const elementRecord = ({ node, parentId }) => ({
  key: node.id || (node.classes.includes('hm') ? 'hm' : parentId + '/bar'),
  parent: parentId,
  ...geometryOf(node.style),
})

const makeEngine = (c) => {
  const hp = c.hp ?? 100
  return {
    THREE: { PointLight: class { constructor() { this.color = { setHex() {} }; this.position = { set() {} }; this.intensity = 0; this.distance = 0 } } },
    scene: { add() {} },
    combat: app.COMBAT_API,
    playerId: PLAYER,
    players: { setAiming() {}, getAnimator: () => null, setExpression() {} },
    cam: { punch() {} },
    client: {
      getRTT: () => c.rtt ?? 0,
      getLocalState: () => null,
      state: { players: [{ id: PLAYER, health: hp, position: [0, 1.3, 0], velocity: [0, 0, 0], onGround: true }] },
    },
  }
}

const readDom = (overlay) => {
  const byId = (id) => overlay.querySelector('#' + id)
  return {
    crosshairTransform: byId('tps-cross').style.transform ?? null,
    hitOpacity: byId('tps-hit').style.opacity ?? null,
    hitMarkBackgrounds: overlay.querySelectorAll('.hm').map((n) => n.style.background ?? null),
    killText: byId('tps-kill').textContent ?? null,
    killOpacity: byId('tps-kill').style.opacity ?? null,
    dmgDealtText: byId('tps-dmg-dealt').textContent ?? null,
    dmgDealtOpacity: byId('tps-dmg-dealt').style.opacity ?? null,
    dmgTakenText: byId('tps-dmg-taken').textContent ?? null,
    dmgTakenOpacity: byId('tps-dmg-taken').style.opacity ?? null,
    shieldOpacity: byId('tps-shield').style.opacity ?? null,
    vignetteOpacity: byId('tps-vignette').style.opacity ?? null,
  }
}

const runCase = (c) => {
  globalThis.document = makeDocument()
  Date.now = () => NOW
  const engine = makeEngine(c)
  tpsGameClient.setup(engine)
  const tps = engine._tps
  const overlay = tps.overlay
  const elements = walk(overlay, overlay.id).map(elementRecord)
  Object.assign(tps, c.tps)
  tpsGameClient.onFrame(1 / 60, engine)
  const inputs = {
    now: NOW,
    rtt: c.rtt ?? 0,
    hp: c.hp ?? 100,
    tps: Object.fromEntries(TPS_FIELDS.map((k) => [k, c.tps[k] ?? 0])),
  }
  return { elements, inputs, outputs: readDom(overlay) }
}

let elements = null
const results = cases.map((c) => {
  const run = runCase(c)
  const json = JSON.stringify(run.elements)
  if (elements === null) elements = run.elements
  else if (JSON.stringify(elements) !== json) throw new Error('overlay markup differs in case ' + c.name)
  return { name: c.name, inputs: run.inputs, outputs: run.outputs }
})
Date.now = realNow

const statusAfter = git(['status', '--porcelain'])
if (statusAfter !== statusBefore) throw new Error('spoint working tree changed')
writeFileSync(outPath, JSON.stringify({
  spoint: rev,
  combat: { spawnInvulnMs: cfg.spawnInvulnMs },
  elements,
  cases: results,
  spointStatusBefore: statusBefore,
  spointStatusAfter: statusAfter,
}, null, 2) + '\n')
console.log('cases ' + results.length + ' elements ' + elements.length + ' ' + outPath)
