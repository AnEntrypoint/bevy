const lift = (o) => (o instanceof V ? o : new V([o]))
const settle = (value, kind) => (kind === 'u' ? value >>> 0 : kind === 'i' ? value | 0 : value)
const ops = {
  add: (a, b) => a + b,
  sub: (a, b) => a - b,
  div: (a, b) => a / b,
  mul: (a, b, kind) => (kind === 'f' ? a * b : Math.imul(a, b)),
  bitXor: (a, b) => a ^ b,
}

class V {
  constructor(components, kind = 'f') {
    this.c = components
    this.k = kind
  }

  get x() { return new V([this.c[0]], this.k) }
  get y() { return new V([this.c[1]], this.k) }

  zip(other, op) {
    const o = lift(other)
    const kind = this.k === 'f' || o.k === 'f' ? 'f' : this.k
    const width = Math.max(this.c.length, o.c.length)
    return new V(Array.from({ length: width }, (_, i) => settle(op(this.c[i % this.c.length], o.c[i % o.c.length], kind), kind)), kind)
  }

  add(o) { return this.zip(o, ops.add) }
  sub(o) { return this.zip(o, ops.sub) }
  mul(o) { return this.zip(o, ops.mul) }
  div(o) { return this.zip(o, ops.div) }
  bitXor(o) { return this.zip(o, ops.bitXor) }
  negate() { return new V(this.c.map((v) => -v), this.k) }
  greaterThanEqual(o) { return new V([this.c[0] >= lift(o).c[0]], 'b') }
  lessThan(o) { return new V([this.c[0] < lift(o).c[0]], 'b') }
  and(o) { return new V([this.c[0] && lift(o).c[0]], 'b') }
  toVar() { return new V([...this.c], this.k) }
  assign(o) { this.c = [...lift(o).c]; return this }
  addAssign(o) { this.c = this.add(o).c }
  mulAssign(o) { this.c = this.mul(o).c }
}

const each = (v, f) => new V(v.c.map(f), 'f')
export const float = (o) => new V([...lift(o).c], 'f')
export const int = (o) => new V(lift(o).c.map(Math.trunc), 'i')
export const uint = (o) => new V(lift(o).c.map((v) => v >>> 0), 'u')
export const ivec2 = (o) => new V(lift(o).c.map(Math.trunc), 'i')
export const uvec2 = (a, b) => new V(b === undefined ? lift(a).c.map((v) => v >>> 0) : [lift(a).c[0] >>> 0, lift(b).c[0] >>> 0], 'u')
export const vec2 = (a, b) => new V([lift(a).c[0], lift(b).c[0]], 'f')
export const abs = (v) => each(v, Math.abs)
export const sin = (v) => each(v, Math.sin)
export const cos = (v) => each(v, Math.cos)
export const floor = (v) => each(v, Math.floor)
export const pow = (v, e) => each(v, (c) => Math.pow(c, lift(e).c[0]))
export const mix = (a, b, t) => lift(a).add(lift(b).sub(a).mul(t))
export const Fn = (body) => ({ setLayout: () => (...args) => body(args) })
export const Loop = ({ start, end }, body) => {
  for (let i = start.c[0]; i < end.c[0]; i++) body({ i: new V([i], 'i') })
}
export const If = (condition, body) => {
  if (lift(condition).c[0]) body()
}
