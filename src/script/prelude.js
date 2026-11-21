globalThis.__handlers = globalThis.__handlers || [];
globalThis.__call_handler = (index, payload, meta, ctx) => globalThis.__handlers[index](ctx, payload, meta);
const __parse = (text) => (text === '' || text === undefined || text === null ? undefined : JSON.parse(text));
const __json = (value) => (value === undefined ? '' : JSON.stringify(value));
const __str = (value) => (typeof value === 'string' ? value : JSON.stringify(value));
const __num = (value, fallback = 0) => (typeof value === 'number' && Number.isFinite(value) ? value : fallback);
const __clamp = (value, low, high) => Math.min(high, Math.max(low, value));
const __deg = Math.PI / 180;
const __rad = 180 / Math.PI;
const __vec = (value, fallback = { x: 0, y: 0, z: 0 }) => {
  if (Array.isArray(value)) return { x: __num(value[0]), y: __num(value[1]), z: __num(value[2]) };
  if (value && typeof value === 'object') return { x: __num(value.x), y: __num(value.y), z: __num(value.z) };
  if (typeof value === 'number') return { x: 0, y: value, z: 0 };
  return fallback;
};
const __host = (op, payload) => {
  if (op !== 'apiCall') globalThis.__host('apiCall', __json({ op }));
  const text = globalThis.__host(op, __json(payload ?? {}));
  return text === '' ? undefined : JSON.parse(text);
};
const __sync = (target, path, key) => __set_json(path, key, __json(target));
function __wrap(path, key, value, parentSync) {
  if (value === null || value === undefined || typeof value !== 'object') return value;
  const target = value;
  const sync = parentSync ?? (() => __set_json(path, key, __json(target)));
  if (Array.isArray(target)) {
    return new Proxy(target, {
      get(t, prop) {
        if (prop === 'length') return t.length;
        if (typeof prop === 'string' && /^\d+$/.test(prop)) return __wrap(path, key, t[prop], sync);
        if (prop === 'push' || prop === 'pop' || prop === 'shift' || prop === 'unshift' || prop === 'splice' || prop === 'sort' || prop === 'reverse' || prop === 'fill') {
          return (...args) => { const result = t[prop](...args); sync(); return result; };
        }
        if (prop === Symbol.iterator) return function* () { for (const item of t) yield __wrap(path, key, item, sync); };
        return t[prop];
      },
      set(t, prop, next) { t[prop] = next; sync(); return true; },
      deleteProperty(t, prop) { delete t[prop]; sync(); return true; },
      has(t, prop) { return prop in t; },
      ownKeys(t) { return Reflect.ownKeys(t); },
      getOwnPropertyDescriptor(t, prop) { return { value: t[prop], writable: true, enumerable: true, configurable: true }; },
    });
  }
  return new Proxy(target, {
    get(t, prop) { if (prop in t) return __wrap(path, key, t[prop], sync); return undefined; },
    set(t, prop, next) { t[prop] = next; sync(); return true; },
    deleteProperty(t, prop) { delete t[prop]; sync(); return true; },
    has(t, prop) { return prop in t; },
    ownKeys(t) { return Reflect.ownKeys(t); },
    getOwnPropertyDescriptor(t, prop) { return { value: t[prop], writable: true, enumerable: true, configurable: true }; },
  });
}
function __local(initial, onChange) {
  const target = initial ?? {};
  return new Proxy(target, {
    get(t, prop) { if (prop in t) return typeof t[prop] === 'object' && t[prop] !== null ? __local(t[prop], () => onChange(target)) : t[prop]; return undefined; },
    set(t, prop, next) { t[prop] = next; onChange(target); return true; },
    deleteProperty(t, prop) { delete t[prop]; onChange(target); return true; },
    has(t, prop) { return prop in t; },
    ownKeys(t) { return Reflect.ownKeys(t); },
    getOwnPropertyDescriptor(t, prop) { return { value: t[prop], writable: true, enumerable: true, configurable: true }; },
  });
}


function __grid(place) {
  globalThis.__tiles = globalThis.__tiles || {};
  const store = (globalThis.__tiles[place] = globalThis.__tiles[place] || {});
  const row = (x) => {
    if (!store[x]) store[x] = {};
    const cells = store[x];
    return new Proxy(cells, {
      get(target, y) { return typeof y === 'string' && /^\d+$/.test(y) ? target[y] : target[y]; },
      set(target, y, value) { target[y] = value === 0 ? undefined : value; if (value === 0) delete target[y]; return true; },
      deleteProperty(target, y) { delete target[y]; return true; },
      has(target, y) { return y in target; },
      ownKeys(target) { return Reflect.ownKeys(target); },
      getOwnPropertyDescriptor(target, y) { return { value: target[y], writable: true, enumerable: true, configurable: true }; },
    });
  };
  return new Proxy(store, {
    get(_, x) { return typeof x === 'string' ? row(x) : undefined; },
    set(_, x, value) { if (value && typeof value === 'object') store[x] = value; else store[x] = {}; return true; },
    deleteProperty(_, x) { delete store[x]; return true; },
    has(_, x) { return x in store; },
    ownKeys() { return Reflect.ownKeys(store); },
    getOwnPropertyDescriptor(_, x) { return { value: store[x], writable: true, enumerable: true, configurable: true }; },
  });
}
function __nested(path, key) {
  const store = __parse(__get_json(path, key)) ?? {};
  const sync = () => __set_json(path, key, __json(store));
  const wrapNode = (node) => new Proxy(node, {
    get(target, prop) {
      if (typeof prop !== 'string' || prop === 'toJSON' || prop === 'then' || prop === 'constructor') return target[prop];
      if (target[prop] === undefined) target[prop] = {};
      const value = target[prop];
      if (value && typeof value === 'object') return wrapNode(value);
      return value;
    },
    set(target, prop, value) { target[prop] = value; sync(); return true; },
    deleteProperty(target, prop) { delete target[prop]; sync(); return true; },
    has(target, prop) { return prop in target; },
    ownKeys(target) { return Reflect.ownKeys(target); },
    getOwnPropertyDescriptor(target, prop) { return { value: target[prop], writable: true, enumerable: true, configurable: true }; },
  });
  return wrapNode(store);
}

function __keyed(path) {
  return new Proxy(Object.create(null), {
    get(_, key) {
      if (key === Symbol.toPrimitive) return () => '[object]';
      if (key === Symbol.iterator) return function* () { for (const name of __parse(__keys_json(path)) ?? []) yield __wrap(path, name, __parse(__get_json(path, name))); };
      if (key === 'then') return undefined;
      if (typeof key !== 'string') return undefined;
      if (key === 'length') return (__parse(__keys_json(path)) ?? []).length;
      const value = __parse(__get_json(path, key));
      return value === undefined ? undefined : __wrap(path, key, value);
    },
    set(_, key, value) { __set_json(path, key, __json(value)); return true; },
    deleteProperty(_, key) { __del(path, key); return true; },
    has(_, key) { return Boolean(__has(path, key)); },
    ownKeys() { return __parse(__keys_json(path)) ?? []; },
    getOwnPropertyDescriptor(_, key) { const value = __parse(__get_json(path, key)); return value === undefined ? undefined : { value, writable: true, enumerable: true, configurable: true }; },
  });
}

function __make(path) {
  if (path === 'world') {
    return new Proxy(Object.create(null), {
      get(_, key) {
        if (key === Symbol.toPrimitive) return () => '[object]';
        if (key === 'then') return undefined;
        if (typeof key !== 'string') return undefined;
        if (key === 'state') return __make('world:state');
        if (key === 'players' || key === 'places') return __parse(__get_json('world', key));
        if (key === 'ui') return __local(__parse(__get_json('world', key)) ?? {}, (value) => __set_json('world', key, __json(value)));
        if (key === 'camera') return __make('camera');
        return __wrap('world', key, __parse(__get_json('world', key)));
      },
      set(_, key, value) { __set_json('world', key, __json(value)); return true; },
      deleteProperty() { return true; },
      has(_, key) { return Boolean(__has('world', key)); },
      ownKeys() { return __parse(__keys_json('world')) ?? []; },
      getOwnPropertyDescriptor(_, key) { const value = __parse(__get_json('world', key)); return value === undefined ? undefined : { value, writable: true, enumerable: true, configurable: true }; },
    });
  }
  if (path === 'camera') {
    return new Proxy(Object.create(null), {
      get(_, key) {
        if (key === 'forward') return __host('cameraJson', {})?.forward ?? { x: 0, y: 0, z: -1 };
        if (key === 'fov') return __host('cameraJson', {})?.fov ?? 60;
        if (key === 'aspect') return __host('cameraJson', {})?.aspect ?? 1;
        if (key === 'insetRight') return __host('cameraJson', {})?.insetRight ?? 0;
        if (typeof key !== 'string') return undefined;
        return __parse(__get_json('world', 'camera'))?.[key];
      },
      set(_, key, value) { const camera = __parse(__get_json('world', 'camera')) ?? {}; camera[key] = value; __set_json('world', 'camera', __json(camera)); return true; },
      deleteProperty(_, key) { const camera = __parse(__get_json('world', 'camera')) ?? {}; delete camera[key]; __set_json('world', 'camera', __json(camera)); return true; },
      has(_, key) { return true; },
      ownKeys() { return Object.keys(__parse(__get_json('world', 'camera')) ?? {}); },
      getOwnPropertyDescriptor(_, key) { return { value: __parse(__get_json('world', 'camera'))?.[key], writable: true, enumerable: true, configurable: true }; },
    });
  }
  if (path === 'world:state') return __keyed(path);
  if (path.startsWith('place:')) {
    const rest = path.slice(6);
    const [name, suffix] = rest.split(':');
    if (suffix === 'state') return __keyed(path);
    if (rest.startsWith('fields/')) return __make('fields:' + rest.slice(7));
    return new Proxy(Object.create(null), {
      get(_, key) {
        if (key === Symbol.toPrimitive) return () => '[object]';
        if (key === 'then') return undefined;
        if (typeof key !== 'string') return undefined;
        if (key === 'state') return __make('place:' + rest + ':state');
        if (key === 'fields') return __make('fields:' + name);
        if (key === 'tiles') return __grid(name);
        if (key === 'objects') return (__parse(__get_json(path, 'objects')) ?? []).map((id) => __make('obj:' + id));
        if (key === 'players') return (__parse(__get_json(path, 'players')) ?? []).map((id) => __make('obj:' + id));
        return __wrap(path, key, __parse(__get_json(path, key)));
      },
      set(_, key, value) { __set_json(path, key, __json(value)); return true; },
      deleteProperty(_, key) { __del(path, key); return true; },
      has(_, key) { return Boolean(__has(path, key)); },
      ownKeys() { return __parse(__keys_json(path)) ?? []; },
      getOwnPropertyDescriptor(_, key) { const value = __parse(__get_json(path, key)); return value === undefined ? undefined : { value, writable: true, enumerable: true, configurable: true }; },
    });
  }
  if (path.startsWith('fields:')) {
    const place = path.slice(7);
    return new Proxy(Object.create(null), {
      get(_, name) {
        if (typeof name !== 'string') return undefined;
        const fields = __parse(__get_json('place:' + place, 'fields')) ?? {};
        if (!(name in fields)) return undefined;
        const field = fields[name];
        return {
          grid: field.grid,
          runtime: __local({}, () => {}),
          authored: __local({}, () => {}),
          changes: (_at) => ({ chunks: [], at: _at }),
        };
      },
      set() { return true; },
      deleteProperty() { return true; },
      has(_, name) { return true; },
      ownKeys() { return Object.keys(__parse(__get_json('place:' + place, 'fields')) ?? {}); },
      getOwnPropertyDescriptor(_, name) { return { value: this.get?.(_, name), writable: true, enumerable: true, configurable: true }; },
    });
  }
  if (path.startsWith('obj:')) {
    const id = path.slice(4);
    const special = (key) => {
      if (key === 'state') return __make('state:' + id);
      if (key === 'session') return __make('session:' + id);
      if (key === 'anim') return __make('anim:' + id);
      if (key === 'view') return __make('view:' + id);
      if (key === 'parent') { const parent = __parent(path); return parent == null ? null : __make('obj:' + parent); }
      if (key === 'children') return (__parse(__children_json(path)) ?? []).map((child) => __make('obj:' + child));
      if (key === 'target') { const target = __target(path); return target == null ? null : __make('obj:' + target); }
      if (key === 'look') return __wrap(path, 'look', __parse(__get_json(path, 'look')) ?? []);
      if (key === 'bones') return __nested(path, 'bones');
      if (key === 'material') return __nested(path, 'material');
      if (['clip', 'hideLocalPlayer', 'orientation', 'pixelSnap', 'pointerLock', 'projection', 'sensitivity', 'minPitch', 'maxPitch'].includes(key)) return __wrap(path, key, __parse(__get_json(path, key)));
      if (key === 'forces') {
        const current = __parse(__get_json(path, 'forces')) ?? {};
        return __local(current, (value) => __set_json(path, 'forces', __json(value)));
      }
      if (key === 'joints') {
        const current = __parse(__get_json(path, 'joints')) ?? {};
        return __local(current, (value) => __host('setJoint', { id, joints: value }));
      }
      if (key === 'ragdoll') {
        const current = __parse(__get_json(path, 'ragdoll'));
        return __local(current ?? {}, (value) => __host('setRagdoll', { id, value }));
      }
      return undefined;
    };
    return new Proxy(Object.create(null), {
      get(_, key) {
        if (key === Symbol.toPrimitive) return () => '[object]';
        if (key === Symbol.iterator) return undefined;
        if (key === 'then') return undefined;
        if (typeof key !== 'string') return undefined;
        if (key === 'id') return id;
        const direct = special(key);
        if (direct !== undefined) return direct;
        const value = __parse(__get_json(path, key));
        return value === undefined ? undefined : __wrap(path, key, value);
      },
      set(_, key, value) {
        if (key === 'state') { __replace_state(id, __json(value)); return true; }
        if (key === 'ragdoll') { __host('setRagdoll', { id, value }); return true; }
        if (key === 'joints') { __host('setJoint', { id, joints: value }); return true; }
        __set_json(path, key, __json(value));
        return true;
      },
      deleteProperty(_, key) { __del(path, key); return true; },
      has(_, key) { return Boolean(__has(path, key)) || key === 'id'; },
      ownKeys() { return __parse(__keys_json(path)) ?? []; },
      getOwnPropertyDescriptor(_, key) {
        const value = key === 'id' ? id : __parse(__get_json(path, key));
        if (value === undefined) return undefined;
        return { value, writable: true, enumerable: true, configurable: true };
      },
    });
  }
  if (path.startsWith('state:') || path.startsWith('session:') || path.startsWith('anim:') || path.startsWith('view:')) {
    return __keyed(path);
  }
  return __keyed(path);
}

function __quatMul(a, b) {
  return {
    x: a.w * b.x + a.x * b.w + a.y * b.z - a.z * b.y,
    y: a.w * b.y - a.x * b.z + a.y * b.w + a.z * b.x,
    z: a.w * b.z + a.x * b.y - a.y * b.x + a.z * b.w,
    w: a.w * b.w - a.x * b.x - a.y * b.y - a.z * b.z,
  };
}
function __quatAxis(axis, angle) {
  const half = angle * 0.5, s = Math.sin(half);
  return { x: axis.x * s, y: axis.y * s, z: axis.z * s, w: Math.cos(half) };
}
function __quatFromEuler(euler) {
  const yaw = (typeof euler === 'number' ? euler : euler?.yaw ?? 0) * __deg;
  const pitch = (typeof euler === 'number' ? 0 : euler?.pitch ?? 0) * __deg;
  const roll = (typeof euler === 'number' ? 0 : euler?.roll ?? 0) * __deg;
  const qy = __quatAxis({ x: 0, y: 1, z: 0 }, yaw);
  const qx = __quatAxis({ x: 1, y: 0, z: 0 }, pitch);
  const qz = __quatAxis({ x: 0, y: 0, z: 1 }, roll);
  return __quatMul(__quatMul(qz, qx), qy);
}
function __quatEuler(q) {
  const sinp = 2 * (q.w * q.x + q.y * q.z);
  const pitch = Math.abs(sinp) >= 1 ? Math.sign(sinp) * Math.PI / 2 : Math.asin(sinp);
  const yaw = Math.atan2(2 * (q.w * q.y - q.z * q.x), 1 - 2 * (q.y * q.y + q.x * q.x));
  const roll = Math.atan2(2 * (q.w * q.z - q.x * q.y), 1 - 2 * (q.z * q.z + q.x * q.x));
  return { yaw: yaw * __rad, pitch: pitch * __rad, roll: roll * __rad };
}
function __quatRotate(q, v) {
  const length = Math.hypot(q.x, q.y, q.z, q.w);
  if (!(length > 0)) throw new TypeError('[quat] quat-zero-length: cannot normalize a zero quaternion');
  const unit = { x: q.x / length, y: q.y / length, z: q.z / length, w: q.w / length };
  const p = { x: v.x, y: v.y, z: v.z, w: 0 };
  const conjugate = { x: -unit.x, y: -unit.y, z: -unit.z, w: unit.w };
  const out = __quatMul(__quatMul(unit, p), conjugate);
  return { x: out.x, y: out.y, z: out.z };
}
function __quatFromDirection(direction, axis = '-z') {
  const target = __vec(direction, { x: 0, y: 0, z: -1 });
  const length = Math.hypot(target.x, target.y, target.z) || 1;
  const t = { x: target.x / length, y: target.y / length, z: target.z / length };
  const bases = { x: { x: 1, y: 0, z: 0 }, y: { x: 0, y: 1, z: 0 }, z: { x: 0, y: 0, z: 1 } };
  const source = bases[axis.replace('-', '')] || bases.z;
  const sign = axis.startsWith('-') ? -1 : 1;
  const s = { x: source.x * sign, y: source.y * sign, z: source.z * sign };
  const dot = s.x * t.x + s.y * t.y + s.z * t.z;
  if (dot > 0.999999) return { x: 0, y: 0, z: 0, w: 1 };
  if (dot < -0.999999) {
    const axis2 = Math.abs(s.x) < 0.9 ? { x: 1, y: 0, z: 0 } : { x: 0, y: 1, z: 0 };
    const c = { x: s.y * axis2.z - s.z * axis2.y, y: s.z * axis2.x - s.x * axis2.z, z: s.x * axis2.y - s.y * axis2.x };
    const n = Math.hypot(c.x, c.y, c.z) || 1;
    return __quatAxis({ x: c.x / n, y: c.y / n, z: c.z / n }, Math.PI);
  }
  const cross = { x: s.y * t.z - s.z * t.y, y: s.z * t.x - s.x * t.z, z: s.x * t.y - s.y * t.x };
  const w = 1 + dot;
  const n = Math.hypot(cross.x, cross.y, cross.z, w) || 1;
  return { x: cross.x / n, y: cross.y / n, z: cross.z / n, w: w / n };
}
function __colorLerp(a, b, t) {
  const parse = (value) => {
    if (typeof value !== 'string') return null;
    const match = value.match(/oklch\(([^)]+)\)/);
    if (!match) return null;
    return match[1].trim().split(/\s+/).map(Number);
  };
  const pa = parse(a), pb = parse(b);
  if (!pa || !pb) return t < 0.5 ? a : b;
  const values = pa.map((value, index) => value + ((pb[index] ?? value) - value) * t);
  return 'oklch(' + values.join(' ') + ')';
}
globalThis.__builtins = {
  math: {
    clamp: (value, min, max) => __host('nativeMath', { op: 'clamp', a: value, b: min, c: max }),
    clamp01: (value) => Math.min(1, Math.max(0, value)),
    lerp: (a, b, t) => (typeof a === 'string' || typeof b === 'string' ? __colorLerp(a, b, t) : a + (b - a) * t),
    smoothstep: (edge0, edge1, x) => { const t = Math.min(1, Math.max(0, (x - edge0) / Math.max(1e-9, edge1 - edge0))); return t * t * (3 - 2 * t); },
    inverseLerp: (a, b, value) => (value - a) / Math.max(1e-9, b - a),
    remap: (inMin, inMax, outMin, outMax, value) => outMin + (outMax - outMin) * ((value - inMin) / Math.max(1e-9, inMax - inMin)),
    shortestArc: (fromDeg, toDeg) => __host('nativeMath', { op: 'shortestArc', a: fromDeg, b: toDeg }),
    stepAngle: (currentDeg, targetDeg, dt, opts = {}) => {
      const smoothing = opts.smoothing ?? 0.06;
      const delta = (() => { let d = (targetDeg - currentDeg) % 360; if (d > 180) d -= 360; if (d <= -180) d += 360; return d; })();
      let step = smoothing <= 0 ? delta : delta * Math.min(1, dt / smoothing);
      if (opts.maxDegPerSec != null) step = Math.max(-opts.maxDegPerSec * dt, Math.min(opts.maxDegPerSec * dt, step));
      return currentDeg + step;
    },
    degToRad: (deg) => __host('nativeMath', { op: 'degToRad', a: deg }),
    radToDeg: (rad) => __host('nativeMath', { op: 'radToDeg', a: rad }),
  },
  vec3: {
    add: (a, b) => __host('nativeVector', { op: 'add', a: __vec(a), b: __vec(b) }),
    sub: (a, b) => __host('nativeVector', { op: 'sub', a: __vec(a), b: __vec(b) }),
    scale: (v, scale) => __host('nativeVector', { op: 'scale', a: __vec(v), scale }),
    dot: (a, b) => { const x = __vec(a), y = __vec(b); return x.x * y.x + x.y * y.y + x.z * y.z; },
    length: (v) => { const x = __vec(v); return Math.hypot(x.x, x.y, x.z); },
    normalize: (v) => __host('nativeVector', { op: 'normalize', a: __vec(v) }),
    distance: (a, b) => { const x = __vec(a), y = __vec(b); return Math.hypot(x.x - y.x, x.y - y.y, x.z - y.z); },
    cross: (a, b) => { const x = __vec(a), y = __vec(b); return { x: x.y * y.z - x.z * y.y, y: x.z * y.x - x.x * y.z, z: x.x * y.y - x.y * y.x }; },
    directionFromYawPitch: (yaw = 0, pitch = 0) => {
      const y = yaw * __deg, p = pitch * __deg, cp = Math.cos(p);
      return { x: Math.sin(y) * cp, y: Math.sin(p), z: -Math.cos(y) * cp };
    },
    rotate: (rotation, v) => {
      const q = typeof rotation === 'number' ? __quatFromEuler({ yaw: rotation }) : (rotation && rotation.w !== undefined ? rotation : __quatFromEuler(rotation ?? {}));
      return __quatRotate(q, __vec(v));
    },
    rotationFromDirection: (direction, axis = '-z') => __quatFromDirection(direction, axis),
    rotationFromEuler: (euler) => __quatFromEuler(euler ?? {}),
    eulerFromRotation: (q) => __quatEuler(q),
  },
  easing: (() => {
    const c1 = 1.70158, c2 = c1 * 1.525, c3 = c1 + 1, c4 = (2 * Math.PI) / 3;
    const bounceOut = (t) => {
      const n1 = 7.5625, d1 = 2.75;
      if (t < 1 / d1) return n1 * t * t;
      if (t < 2 / d1) { t -= 1.5 / d1; return n1 * t * t + 0.75; }
      if (t < 2.5 / d1) { t -= 2.25 / d1; return n1 * t * t + 0.9375; }
      t -= 2.625 / d1; return n1 * t * t + 0.984375;
    };
    return {
      linear: (t) => t,
      easeOutBounce: bounceOut,
      easeInQuad: (t) => t * t,
      easeOutQuad: (t) => 1 - (1 - t) * (1 - t),
      easeInOutQuad: (t) => (t < 0.5 ? 2 * t * t : 1 - Math.pow(-2 * t + 2, 2) / 2),
      easeInCubic: (t) => t * t * t,
      easeOutCubic: (t) => 1 - Math.pow(1 - t, 3),
      easeInOutCubic: (t) => (t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2),
      easeInQuart: (t) => t * t * t * t,
      easeOutQuart: (t) => 1 - Math.pow(1 - t, 4),
      easeInOutQuart: (t) => (t < 0.5 ? 8 * t * t * t * t : 1 - Math.pow(-2 * t + 2, 4) / 2),
      easeInQuint: (t) => Math.pow(t, 5),
      easeOutQuint: (t) => 1 - Math.pow(1 - t, 5),
      easeInOutQuint: (t) => (t < 0.5 ? 16 * Math.pow(t, 5) : 1 - Math.pow(-2 * t + 2, 5) / 2),
      easeInSine: (t) => 1 - Math.cos((t * Math.PI) / 2),
      easeOutSine: (t) => Math.sin((t * Math.PI) / 2),
      easeInOutSine: (t) => -(Math.cos(Math.PI * t) - 1) / 2,
      easeInExpo: (t) => (t === 0 ? 0 : Math.pow(2, 10 * t - 10)),
      easeOutExpo: (t) => (t === 1 ? 1 : 1 - Math.pow(2, -10 * t)),
      easeInOutExpo: (t) => (t === 0 ? 0 : t === 1 ? 1 : t < 0.5 ? Math.pow(2, 20 * t - 10) / 2 : (2 - Math.pow(2, -20 * t + 10)) / 2),
      easeInCirc: (t) => 1 - Math.sqrt(1 - Math.pow(t, 2)),
      easeOutCirc: (t) => Math.sqrt(1 - Math.pow(t - 1, 2)),
      easeInOutCirc: (t) => (t < 0.5 ? (1 - Math.sqrt(1 - Math.pow(2 * t, 2))) / 2 : (Math.sqrt(1 - Math.pow(-2 * t + 2, 2)) + 1) / 2),
    };
  })(),
};

globalThis.__builtins.noise = (() => {
  const mix32 = (value) => {
    value = Math.imul(value ^ (value >>> 16), 0x45d9f3b);
    value = Math.imul(value ^ (value >>> 16), 0x45d9f3b);
    return (value ^ (value >>> 16)) >>> 0;
  };
  const hashString = (value) => {
    if (typeof value === 'number') return value >>> 0;
    let hash = 2166136261;
    const text = String(value);
    for (let i = 0; i < text.length; i++) { hash ^= text.charCodeAt(i); hash = Math.imul(hash, 16777619); }
    return hash >>> 0;
  };
  const random2 = (x, z, seed) => mix32(Math.imul(x, 374761393) + Math.imul(z, 668265263) + hashString(seed)) / 4294967296;
  const fade = (t) => t * t * t * (t * (t * 6 - 15) + 10);
  const value2 = (x, z, seed) => {
    const xi = Math.floor(x), zi = Math.floor(z);
    const tx = fade(x - xi), tz = fade(z - zi);
    const a = random2(xi, zi, seed), b = random2(xi + 1, zi, seed), c = random2(xi, zi + 1, seed), d = random2(xi + 1, zi + 1, seed);
    return (a * (1 - tx) + b * tx) * (1 - tz) + (c * (1 - tx) + d * tx) * tz;
  };
  const value3 = (x, y, z, seed) => {
    const xi = Math.floor(x), yi = Math.floor(y), zi = Math.floor(z);
    const tx = fade(x - xi), ty = fade(y - yi), tz = fade(z - zi);
    const sample = (dx, dy, dz) => random2(Math.imul(xi + dx, 73856093) + Math.imul(zi + dz, 19349663), yi + dy, seed);
    const x00 = sample(0, 0, 0) * (1 - tx) + sample(1, 0, 0) * tx;
    const x10 = sample(0, 1, 0) * (1 - tx) + sample(1, 1, 0) * tx;
    const x01 = sample(0, 0, 1) * (1 - tx) + sample(1, 0, 1) * tx;
    const x11 = sample(0, 1, 1) * (1 - tx) + sample(1, 1, 1) * tx;
    return (x00 * (1 - ty) + x10 * ty) * (1 - tz) + (x01 * (1 - ty) + x11 * ty) * tz;
  };
  const GRADIENTS2 = [[1, 1], [-1, 1], [1, -1], [-1, -1], [1, 0], [-1, 0], [0, 1], [0, -1]];
  const gradientPermutations = new Map();
  const gradientPermutation = (seed) => {
    const key = seed >>> 0;
    if (gradientPermutations.has(key)) return gradientPermutations.get(key);
    let state = key;
    const rng = () => {
      state |= 0; state = (state + 0x6d2b79f5) | 0;
      let t = Math.imul(state ^ (state >>> 15), 1 | state);
      t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
      return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
    rng(); rng();
    const perm = new Uint8Array(256);
    for (let i = 0; i < 256; i++) perm[i] = i;
    for (let i = 255; i > 0; i--) { const j = Math.floor(rng() * (i + 1)); const tmp = perm[i]; perm[i] = perm[j]; perm[j] = tmp; }
    const doubled = new Uint8Array(512);
    for (let i = 0; i < 512; i++) doubled[i] = perm[i & 255];
    gradientPermutations.set(key, doubled);
    return doubled;
  };
  const gradientNoise2D = (perm, x, y) => {
    const x0 = Math.floor(x), y0 = Math.floor(y);
    const xf = x - x0, yf = y - y0;
    const u = fade(xf), v = fade(yf);
    const dot = (ix, iy, dx, dy) => { const g = GRADIENTS2[perm[(perm[ix & 255] + iy) & 255] & 7]; return g[0] * dx + g[1] * dy; };
    const n00 = dot(x0, y0, xf, yf), n10 = dot(x0 + 1, y0, xf - 1, yf), n01 = dot(x0, y0 + 1, xf, yf - 1), n11 = dot(x0 + 1, y0 + 1, xf - 1, yf - 1);
    const nx0 = n00 + u * (n10 - n00), nx1 = n01 + u * (n11 - n01);
    return (nx0 + v * (nx1 - nx0)) * Math.SQRT2;
  };
  const gradientFbm2 = (opts) => {
    const perm = gradientPermutation(opts.seed ?? 1337);
    const frequency = opts.frequency ?? 1, octaves = Math.max(1, opts.octaves ?? 5), gain = opts.gain ?? 0.5, lacunarity = opts.lacunarity ?? 2;
    const x = __num(opts.x) * frequency, z = __num(opts.z) * frequency;
    let amplitude = 1, scale = 1, sum = 0, norm = 0;
    for (let o = 0; o < octaves; o++) { sum += gradientNoise2D(perm, x * scale, z * scale) * amplitude; norm += amplitude; amplitude *= gain; scale *= lacunarity; }
    return norm > 0 ? sum / norm : 0;
  };
  const fbm = (opts, mode) => {

    const x = __num(opts.x) * (opts.frequency ?? 1);
    const z = __num(opts.z) * (opts.frequency ?? 1);
    let amplitude = 1, sum = 0, norm = 0;
    const octaves = opts.octaves ?? 4, gain = opts.gain ?? 0.5, lacunarity = opts.lacunarity ?? 2;
    for (let i = 0; i < octaves; i++) {
      let value = value2(x * Math.pow(lacunarity, i), z * Math.pow(lacunarity, i), (opts.seed ?? 0) + i);
      if (mode === 'ridged') value = 1 - Math.abs(value * 2 - 1);
      if (mode === 'billowed') value = Math.abs(value * 2 - 1);
      sum += value * amplitude; norm += amplitude; amplitude *= gain;
    }
    const ridgeGain = mode === 'ridged' ? (opts.ridgeGain ?? 2) : 1;
    return Math.pow(sum / Math.max(1e-9, norm), ridgeGain) * (opts.amplitude ?? 1);
  };
  return {
    simplex2: (opts) => (value2(__num(opts.x) * (opts.frequency ?? 1), __num(opts.z) * (opts.frequency ?? 1), opts.seed ?? 0) * 2 - 1) * (opts.amplitude ?? 1),
    simplex3: (opts) => (value3(__num(opts.x) * (opts.frequency ?? 1), __num(opts.y) * (opts.frequency ?? 1), __num(opts.z) * (opts.frequency ?? 1), opts.seed ?? 0) * 2 - 1) * (opts.amplitude ?? 1),
    fbm2: (opts) => gradientFbm2(opts),
    ridged: (opts) => fbm(opts, 'ridged'),
    billowed2: (opts) => fbm(opts, 'billowed'),
    gradient2: (opts) => {
      const delta = opts.delta ?? 0.001;
      const sample = (x, z) => value2(x * (opts.frequency ?? 1), z * (opts.frequency ?? 1), opts.seed ?? 0);
      const dx = (sample(opts.x + delta, opts.z) - sample(opts.x - delta, opts.z)) / (2 * delta);
      const dz = (sample(opts.x, opts.z + delta) - sample(opts.x, opts.z - delta)) / (2 * delta);
      return { x: dx, z: dz, magnitude: Math.hypot(dx, dz) };
    },
    domainWarp2: (opts) => {
      const warp = opts.warpAmp ?? 1, wf = opts.warpFrequency ?? 1;
      const wx = value2(opts.x * wf, opts.z * wf, (opts.seed ?? 0) + 17) * 2 - 1;
      const wz = value2(opts.x * wf, opts.z * wf, (opts.seed ?? 0) + 31) * 2 - 1;
      return fbm({ ...opts, x: opts.x + wx * warp, z: opts.z + wz * warp }, 'fbm');
    },
    voronoi2: (opts) => {
      const x = opts.x * (opts.frequency ?? 1), z = opts.z * (opts.frequency ?? 1);
      const xi = Math.floor(x), zi = Math.floor(z);
      let f1 = Infinity, f2 = Infinity, cell = { x: 0, z: 0 };
      for (let dz = -1; dz <= 1; dz++) for (let dx = -1; dx <= 1; dx++) {
        const cx = xi + dx, cz = zi + dz;
        const px = cx + random2(cx, cz, opts.seed ?? 0), pz = cz + random2(cz, cx, (opts.seed ?? 0) + 7);
        const distance = Math.hypot(px - x, pz - z);
        if (distance < f1) { f2 = f1; f1 = distance; cell = { x: cx, z: cz }; } else if (distance < f2) f2 = distance;
      }
      return { f1, f2, cell };
    },
    white2: (opts) => random2(Math.floor(opts.x), Math.floor(opts.z), opts.seed ?? 0),
    makeSeededRng: (seed) => {
      let state = hashString(seed) || 1;
      const next = () => { state |= 0; state = (state + 0x6d2b79f5) | 0; let t = Math.imul(state ^ (state >>> 15), 1 | state); t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t; return ((t ^ (t >>> 14)) >>> 0) / 4294967296; };
      return { next, double: next, int32: () => (next() * 4294967296) | 0, uint32: () => (next() * 4294967296) >>> 0, range: (min, max) => min + next() * (max - min) };
    },
  };
})();
globalThis.__builtins.path = {
  createPath: (points, opts = {}) => {
    const pts = points.map((point) => Array.isArray(point) ? [__num(point[0]), __num(point[2] ?? point[1])] : [__num(point.x), __num(point.z)]);
    const closed = opts.closed ?? false;
    const segments = [];
    for (let i = 0; i < pts.length - 1; i++) segments.push([pts[i], pts[i + 1]]);
    if (closed && pts.length > 1) segments.push([pts[pts.length - 1], pts[0]]);
    const lengths = segments.map(([a, b]) => Math.hypot(b[0] - a[0], b[1] - a[1]));
    const total = lengths.reduce((sum, value) => sum + value, 0);
    return {
      points: pts,
      closed,
      length: total,
      distance: (x, z, within = Infinity) => {
        let best = Infinity;
        for (const [a, b] of segments) {
          const abx = b[0] - a[0], abz = b[1] - a[1];
          const t = __clamp(((x - a[0]) * abx + (z - a[1]) * abz) / Math.max(1e-9, abx * abx + abz * abz), 0, 1);
          best = Math.min(best, Math.hypot(x - (a[0] + abx * t), z - (a[1] + abz * t)));
        }
        return best <= within ? best : Infinity;
      },
      nearest: (x, z, within = Infinity) => {
        let best = null, walked = 0;
        for (let i = 0; i < segments.length; i++) {
          const [a, b] = segments[i];
          const abx = b[0] - a[0], abz = b[1] - a[1];
          const t = __clamp(((x - a[0]) * abx + (z - a[1]) * abz) / Math.max(1e-9, abx * abx + abz * abz), 0, 1);
          const px = a[0] + abx * t, pz = a[1] + abz * t;
          const distance = Math.hypot(x - px, z - pz);
          if (distance <= within && (!best || distance < best.distance)) {
            const length = lengths[i] || 1;
            best = { x: px, y: 0, z: pz, distance, arcLength: walked + length * t, t, segmentIndex: i, tx: abx / length, tz: abz / length };
          }
          walked += lengths[i];
        }
        return best;
      },
      at: (arcLength) => {
        let distance = closed ? ((arcLength % total) + total) % total : __clamp(arcLength, 0, total);
        let walked = 0;
        for (let i = 0; i < segments.length; i++) {
          const length = lengths[i] || 1;
          if (walked + length >= distance || i === segments.length - 1) {
            const [a, b] = segments[i];
            const t = __clamp((distance - walked) / length, 0, 1);
            return { x: a[0] + (b[0] - a[0]) * t, y: 0, z: a[1] + (b[1] - a[1]) * t, tx: (b[0] - a[0]) / length, tz: (b[1] - a[1]) / length };
          }
          walked += length;
        }
        return { x: pts[0][0], y: 0, z: pts[0][1], tx: 1, tz: 0 };
      },
    };
  },
};

function __physicsWorld(gravity = { x: 0, y: -9.81, z: 0 }) {
  const native = __host('physicsWorldCreate', { gravity: __vec(gravity) }) ?? {};
  const worldId = native.__nativeWorld ?? native.id;
  const bodies = [];
  const world = {
    __isWorld: true,
    __nativeWorld: worldId,
    id: worldId,
    gravity: __vec(gravity),
    bodies,
    add: (spec = {}) => {
      const body = __host('physicsWorldAdd', { world: worldId, spec }) ?? {};
      const handle = {
        __nativeBody: body.__nativeBody,
        position: body.position ?? __vec(spec.position),
        rotation: body.rotation ?? (spec.rotation ?? { x: 0, y: 0, z: 0, w: 1 }),
        velocity: body.velocity ?? __vec(spec.velocity),
        angularVelocity: body.angularVelocity ?? __vec(spec.angularVelocity),
        sleeping: false,
        passThrough: [],
        body: spec.body ?? 'dynamic',
        collider: spec.collider ?? 'box',
        size: __vec(spec.size, { x: 1, y: 1, z: 1 }),
        radius: spec.radius ?? 0.5,
        height: spec.height ?? 1,
      };
      bodies.push(handle);
      return handle;
    },
    remove: (body) => { const index = bodies.indexOf(body); if (index >= 0) bodies.splice(index, 1); },
    step: (dt = 1 / 60) => {
      __host('physicsWorldStep', { world: worldId, dt });
    },
    save: (into, subset) => {
      const bytes = __host('physicsWorldSave', { world: worldId, bodies: subset ?? undefined });
      const array = Uint8Array.from(((Array.isArray(bytes) ? bytes : [])).map((value) => __num(value)));
      if (into instanceof Uint8Array) { into.set(array.subarray(0, into.length)); return array.length; }
      return array;
    },
    restore: (bytes, subset) => {
      const values = bytes instanceof Uint8Array ? Array.from(bytes) : (Array.isArray(bytes) ? bytes : []);
      const ok = __host('physicsWorldRestore', { world: worldId, bytes: values, bodies: subset ?? undefined });
      return ok !== false;
    },
    raycast: (originValue, directionValue, opts = {}) => {
      const origin = __vec(originValue), direction = __vec(directionValue);
      const length = Math.hypot(direction.x, direction.y, direction.z) || 1;
      const dir = { x: direction.x / length, y: direction.y / length, z: direction.z / length };
      const distance = typeof opts === 'number' ? opts : (opts.distance ?? 200);
      let best = null;
      for (const body of bodies) {
        if (body === opts.body || (opts.ignoreEntities ?? []).includes(body)) continue;
        const half = body.collider === 'sphere' ? { x: body.radius, y: body.radius, z: body.radius } : { x: body.size.x / 2, y: body.size.y / 2, z: body.size.z / 2 };
        let tmin = 0, tmax = distance;
        const axes = ['x', 'y', 'z'];
        let hitAxis = 'y', sign = 1;
        for (const axis of axes) {
          const min = body.position[axis] - half[axis], max = body.position[axis] + half[axis];
          if (Math.abs(dir[axis]) < 1e-9) { if (origin[axis] < min || origin[axis] > max) { tmin = Infinity; break; } continue; }
          let t1 = (min - origin[axis]) / dir[axis], t2 = (max - origin[axis]) / dir[axis];
          let s = 1;
          if (t1 > t2) { const tmp = t1; t1 = t2; t2 = tmp; s = -1; }
          if (t1 > tmin) { tmin = t1; hitAxis = axis; sign = s; }
          tmax = Math.min(tmax, t2);
          if (tmin > tmax) { tmin = Infinity; break; }
        }
        if (tmin !== Infinity && tmin <= distance && (!best || tmin < best.distance)) {
          best = { id: 'physics-body', body, distance: tmin, toi: tmin, point: { x: origin.x + dir.x * tmin, y: origin.y + dir.y * tmin, z: origin.z + dir.z * tmin }, normal: { x: hitAxis === 'x' ? sign : 0, y: hitAxis === 'y' ? sign : 0, z: hitAxis === 'z' ? sign : 0 } };
        }
      }
      return best;
    },
    overlapPoint: (pointValue) => {
      const point = __vec(pointValue);
      return { hits: bodies.filter((body) => {
        const half = body.collider === 'sphere' ? { x: body.radius, y: body.radius, z: body.radius } : { x: body.size.x / 2, y: body.size.y / 2, z: body.size.z / 2 };
        return Math.abs(point.x - body.position.x) <= half.x && Math.abs(point.y - body.position.y) <= half.y && Math.abs(point.z - body.position.z) <= half.z;
      }) };
    },
  };
  return world;
}
globalThis.__builtins.physics = {
  raycast: (ctx, origin, direction, opts = {}) => {
    if (ctx && ctx.__isWorld) return ctx.raycast(origin, direction, opts);
    const options = typeof opts === 'number' ? { distance: opts } : opts;
    return __host('raycast', { origin: __vec(origin), direction: __vec(direction), ...options });
  },
  overlapPoint: (ctx, point, opts = {}) => {
    if (ctx && ctx.__isWorld) return ctx.overlapPoint(point, opts);
    return __host('overlapPoint', { point: __vec(point), ...opts });
  },
  overlapSegment: (ctx, a, b, opts = {}) => {
    if (ctx && ctx.__isWorld) {
      const hit = ctx.raycast(a, __vec(b) - __vec(a), { distance: Math.hypot(__vec(b).x - __vec(a).x, __vec(b).y - __vec(a).y, __vec(b).z - __vec(a).z) });
      return { hits: hit ? [hit] : [] };
    }
    return __host('overlapSegment', { a: __vec(a), b: __vec(b), ...opts });
  },
  traverseCheck: (ctx, from, to, opts = {}) => ctx && ctx.__isWorld ? { traversable: true, reason: 'ok' } : __host('traverseCheck', { from: __vec(from), to: __vec(to), ...opts }),
  jumpCheck: (ctx, from, to, opts = {}) => ctx && ctx.__isWorld ? { ok: true, reachable: true } : __host('jumpCheck', { from: __vec(from), to: __vec(to), ...opts }),
  getInputRay: (ctx, input) => {
    if (ctx && ctx.__isWorld) return ctx.inputRay ?? null;
    if (input && input.pointerRay) return input.pointerRay;
    return __host('inputRay', {});
  },
  createWorld: (opts = {}) => {
    const world = __physicsWorld(opts.gravity);
    world.then = (resolve) => resolve(world);
    return world;
  },
};
globalThis.__tweens = [];
globalThis.__builtins.tween = {
  animate: (ctx, target, keyframes, opts = {}) => {
    const id = typeof target === 'string' ? target : target?.id;
    if (!id) return;
    const duration = opts.duration ?? 1, delay = opts.delay ?? 0, easing = globalThis.__builtins.easing[opts.easing] ?? ((t) => t);
    const start = { now: __host('now', {}) };
    const begin = performance.now() + delay * 1000;
    const initial = {};
    for (const path of Object.keys(keyframes)) initial[path] = __read_path(__make('obj:' + id), path);
    globalThis.__tweens.push({ id, target: id, keyframes, opts, begin, duration, easing, initial, direction: opts.direction ?? 'normal', loop: opts.loop ?? false });
  },
};
function __read_path(object, path) {
  let value = object;
  for (const part of path.split('.')) { if (value == null) return undefined; value = value[part]; }
  return value;
}
function __write_path(object, path, value) {
  const parts = path.split('.');
  let target = object;
  for (let i = 0; i < parts.length - 1; i++) { if (target[parts[i]] == null) target[parts[i]] = {}; target = target[parts[i]]; }
  target[parts[parts.length - 1]] = value;
}
function __key_value(value, initial, t) {
  if (Array.isArray(value)) return value.map((entry) => __key_value(entry, initial, t));
  if (value && typeof value === 'object' && !Array.isArray(value)) {
    if ('value' in value) return __key_value(value.value, initial, t);
    const out = {};
    for (const key of Object.keys(value)) out[key] = __key_value(value[key], initial?.[key], t);
    return out;
  }
  if (typeof value === 'string' && value.startsWith('+=')) return __num(initial) + __num(Number(value.slice(2))) * t;
  if (typeof value === 'string' && value.startsWith('-=')) return __num(initial) - __num(Number(value.slice(2))) * t;
  if (typeof value === 'number' && typeof initial === 'number') return initial + (value - initial) * t;
  return value;
}
globalThis.__tick = (dt) => {
  const now = performance.now();
  for (let i = globalThis.__tweens.length - 1; i >= 0; i--) {
    const tween = globalThis.__tweens[i];
    if (now < tween.begin) continue;
    let t = tween.duration <= 0 ? 1 : (now - tween.begin) / (tween.duration * 1000);
    if (t >= 1 && !tween.loop) { t = 1; globalThis.__tweens.splice(i, 1); }
    else if (tween.loop) t = t % 1;
    if (tween.direction === 'reverse') t = 1 - t;
    else if (tween.direction === 'alternate') t = t < 0.5 ? t * 2 : 2 - t * 2;
    const eased = tween.easing(__clamp(t, 0, 1));
    const object = __make('obj:' + tween.target);
    for (const [path, value] of Object.entries(tween.keyframes)) {
      __write_path(object, path, __key_value(value, tween.initial[path], eased));
    }
  }
  const ctx = globalThis.__active_ctx;
  if (ctx) {
    for (const effect of globalThis.__effects ?? []) effect.step(dt);
  }
  const jobs = globalThis.__jobs ?? {};
  for (const [id, job] of Object.entries(jobs)) {
    if (!job.pending) continue;
    job.pending = false;
    if (job.callback) job.callback({ id, ok: true, data: null });
    delete jobs[id];
  }
};
globalThis.__jobs = globalThis.__jobs || {};
function __ballisticElevation(speed, g, distance, rise) {
  if (distance === 0) return null;
  const v2 = speed * speed;
  const root = v2 * v2 - g * (g * distance * distance + 2 * rise * v2);
  if (root < 0) return null;
  return Math.atan((v2 - Math.sqrt(root)) / (g * distance));
}
globalThis.__builtins.ballistics = {
  solveShot: (fromValue, toValue, speed, g, lead) => {
    const origin = __vec(fromValue), target = __vec(toValue);
    const velocity = lead === null || lead === undefined ? null : __vec(lead);
    const staticSolve = (aim) => {
      const dx = aim.x - origin.x, dz = aim.z - origin.z;
      const distance = Math.hypot(dx, dz);
      const theta = __ballisticElevation(speed, g, distance, aim.y - origin.y);
      if (theta === null) return null;
      const horizontal = speed * Math.cos(theta);
      if (!(horizontal > 0)) return null;
      return { vel: { x: (dx / distance) * horizontal, y: speed * Math.sin(theta), z: (dz / distance) * horizontal }, time: distance / horizontal };
    };
    let aim = { ...target }, firedAt = aim, solution = null;
    for (let i = 0; i < 4; i++) {
      firedAt = aim;
      solution = staticSolve(firedAt);
      if (solution === null) return null;
      if (velocity === null) break;
      aim = { x: target.x + velocity.x * solution.time, y: target.y + velocity.y * solution.time, z: target.z + velocity.z * solution.time };
    }
    return { ...solution, aim: firedAt };
  },
  elevation: __ballisticElevation,
};

globalThis.__builtins.spline = {
  splineLength: (ctx, id) => __host('splineLength', { id }) ?? 0,
  sampleSpline: (ctx, id, at, opts = {}) => __host('sampleSpline', { id, at, ...opts }),
  projectToSpline: (ctx, id, point, opts = {}) => __host('projectToSpline', { id, point: __vec(point), ...opts }),
};
globalThis.__builtins.nav = {
  findPath: (ctx, fromValue, toValue, opts = {}) => {
    const from = __vec(fromValue), to = __vec(toValue);
    const distance = Math.hypot(to.x - from.x, to.z - from.z);
    const steps = Math.max(1, Math.ceil(distance / Math.max(0.25, opts.budget ?? 1)));
    const points = [];
    let blocked = false;
    for (let i = 0; i <= steps; i++) {
      const t = i / steps;
      const x = from.x + (to.x - from.x) * t, z = from.z + (to.z - from.z) * t;
      const ground = __host('raycast', { origin: { x, y: (from.y + to.y) * 0.5 + 3, z }, direction: { x: 0, y: -1, z: 0 }, distance: 60, ignoreEntities: opts.ignore ?? [] });
      const y = ground?.point?.y ?? (from.y + (to.y - from.y) * t);
      points.push({ x, y, z });
      const waist = __host('raycast', { origin: { x, y: y + 0.6, z }, direction: { x: to.x - from.x, y: 0, z: to.z - from.z }, distance: 4, ignoreEntities: opts.ignore ?? [] });
      if (waist) blocked = true;
    }
    if (blocked) {
      for (const side of [-1, 1]) {
        const dx = to.z - from.z, dz = -(to.x - from.x);
        const length = Math.hypot(dx, dz) || 1;
        const offsetX = (dx / length) * 2 * side, offsetZ = (dz / length) * 2 * side;
        const detour = { x: (from.x + to.x) / 2 + offsetX, z: (from.z + to.z) / 2 + offsetZ };
        const clear = !__host('raycast', { origin: { x: detour.x, y: from.y + 0.6, z: detour.z }, direction: { x: to.x - from.x, y: 0, z: to.z - from.z }, distance: 2, ignoreEntities: opts.ignore ?? [] });
        if (clear) { points.splice(1, points.length - 2, { x: detour.x, y: from.y, z: detour.z }); blocked = false; break; }
      }
    }
    return { points, ok: !blocked, partial: blocked ? true : undefined };
  },
};
globalThis.__builtins['room-routing'] = {
  firstOpenRoom: (ctx) => {
    const rooms = ctx?.rooms ?? [];
    const requested = ctx?.requestedRoomId;
    const open = rooms.find((room) => room.id === requested && room.open && (room.maxPlayers == null || room.players < room.maxPlayers));
    if (open) return open.id;
    const available = rooms.find((room) => room.open && (room.maxPlayers == null || room.players < room.maxPlayers));
    return available ? available.id : globalThis.__builtins['room-routing'].nextRoomName(rooms);
  },
  mostFullWithSpace: (ctx) => {
    const rooms = (ctx?.rooms ?? []).filter((room) => room.open && (room.maxPlayers == null || room.players < room.maxPlayers));
    rooms.sort((a, b) => (b.players ?? 0) - (a.players ?? 0));
    return rooms.length ? rooms[0].id : null;
  },
  nextRoomName: (rooms, exclude = []) => {
    const used = new Set((rooms ?? []).map((room) => room.id).concat(exclude));
    for (let i = 1; ; i++) if (!used.has('room-' + i)) return 'room-' + i;
  },
  roomFor: (key) => 'room-' + String(key).replace(/[^a-zA-Z0-9_-]/g, '-'),
};
globalThis.__builtins.room_routing = globalThis.__builtins['room-routing'];
globalThis.__builtins.terrain = {
  shore: (height, level, opts = {}) => {
    const apron = opts.apron ?? 6, berm = opts.berm ?? 0.3, grade = opts.grade ?? 0.06;
    const rejoin = opts.rejoin ?? 24, shelf = opts.shelf ?? 2.2, shelfReach = opts.shelfReach ?? 6;
    const drop = opts.drop ?? 20, scale = opts.scale ?? 1;
    const h = (height - level) * scale;
    if (h >= berm) return level + berm + (h - berm) * 1.0 + grade * Math.max(0, h - berm) * (h > rejoin ? 1 : h / rejoin);
    if (h >= 0) return level + h * (apron / Math.max(0.001, apron));
    const depth = -h;
    if (depth <= shelf) return level - depth;
    const reach = Math.min(1, (depth - shelf) / Math.max(0.001, shelfReach));
    return level - shelf - reach * drop;
  },
};
globalThis.__builtins.voxels = (() => {
  class VoxelPart {
    constructor() { this.cells = new Map(); this.__nativePart = (__host('voxelPart', {}) ?? {}).__nativePart; }
    key(x, y, z) { return x + ',' + y + ',' + z; }
    set(data) { const { x = 0, y = 0, z = 0, material = 'default' } = data ?? {}; this.cells.set(this.key(Math.floor(x), Math.floor(y), Math.floor(z)), material); __host('voxelSet', { part: this.__nativePart, x: Math.floor(x), y: Math.floor(y), z: Math.floor(z), material }); return this; }
    native() { return __host('voxelJson', { part: this.__nativePart }) ?? { version: 1, bounds: null, palette: [], voxels: [] }; }
    fill({ from = [0, 0, 0], to = [0, 0, 0], material = 'default' } = {}) {
      for (let x = Math.min(from[0], to[0]); x <= Math.max(from[0], to[0]); x++)
        for (let y = Math.min(from[1], to[1]); y <= Math.max(from[1], to[1]); y++)
          for (let z = Math.min(from[2], to[2]); z <= Math.max(from[2], to[2]); z++) this.cells.set(this.key(x, y, z), material);
      return this;
    }
    hollowBox({ from = [0, 0, 0], to = [0, 0, 0], material = 'default' } = {}) {
      for (let x = Math.min(from[0], to[0]); x <= Math.max(from[0], to[0]); x++)
        for (let y = Math.min(from[1], to[1]); y <= Math.max(from[1], to[1]); y++)
          for (let z = Math.min(from[2], to[2]); z <= Math.max(from[2], to[2]); z++) {
            const edge = [x, y, z].some((value, index) => value === [from, to][0][index] || value === [from, to][1][index]);
            if (edge) this.cells.set(this.key(x, y, z), material);
          }
      return this;
    }
    sphere({ center = [0, 0, 0], radius = 1, material = 'default', filled = true } = {}) {
      const r = Math.ceil(radius);
      for (let x = -r; x <= r; x++) for (let y = -r; y <= r; y++) for (let z = -r; z <= r; z++) {
        const d = Math.hypot(x, y, z);
        if (d <= radius && (filled || d > radius - 1)) this.cells.set(this.key(center[0] + x, center[1] + y, center[2] + z), material);
      }
      return this;
    }
    cylinder({ base = [0, 0, 0], radius = 1, height = 1, axis = 'y', material = 'default', filled = true } = {}) {
      for (let h = 0; h < height; h++) for (let a = -radius; a <= radius; a++) for (let b = -radius; b <= radius; b++) {
        if (Math.hypot(a, b) > radius) continue;
        if (!filled && Math.hypot(a, b) < radius - 1) continue;
        const point = axis === 'x' ? [base[0] + h, base[1] + a, base[2] + b] : axis === 'z' ? [base[0] + a, base[1] + b, base[2] + h] : [base[0] + a, base[1] + h, base[2] + b];
        this.cells.set(this.key(point[0], point[1], point[2]), material);
      }
      return this;
    }
    cone({ base = [0, 0, 0], radius = 1, height = 1, axis = 'y', material = 'default', filled = true } = {}) {
      for (let h = 0; h < height; h++) {
        const r = radius * (1 - h / Math.max(1, height));
        for (let a = -Math.ceil(r); a <= Math.ceil(r); a++) for (let b = -Math.ceil(r); b <= Math.ceil(r); b++) {
          if (Math.hypot(a, b) > r) continue;
          if (!filled && Math.hypot(a, b) < r - 1) continue;
          const point = axis === 'x' ? [base[0] + h, base[1] + a, base[2] + b] : axis === 'z' ? [base[0] + a, base[1] + b, base[2] + h] : [base[0] + a, base[1] + h, base[2] + b];
          this.cells.set(this.key(point[0], point[1], point[2]), material);
        }
      }
      return this;
    }
    pyramid({ base = [0, 0, 0], size = 1, height = 1, material = 'default', filled = true } = {}) {
      for (let h = 0; h < height; h++) {
        const half = Math.max(0, (size / 2) * (1 - h / Math.max(1, height)));
        for (let a = -Math.ceil(half); a <= Math.ceil(half); a++) for (let b = -Math.ceil(half); b <= Math.ceil(half); b++)
          if (filled || Math.abs(a) === Math.ceil(half) || Math.abs(b) === Math.ceil(half)) this.cells.set(this.key(base[0] + a, base[1] + h, base[2] + b), material);
      }
      return this;
    }
    torus({ center = [0, 0, 0], majorRadius = 2, minorRadius = 0.5, axis = 'y', material = 'default', filled = true } = {}) {
      const r = Math.ceil(majorRadius + minorRadius);
      for (let x = -r; x <= r; x++) for (let y = -r; y <= r; y++) for (let z = -r; z <= r; z++) {
        const ring = Math.hypot(x, z) - majorRadius;
        const d = Math.hypot(ring, y);
        if (d <= minorRadius && (filled || d > minorRadius - 1)) this.cells.set(this.key(center[0] + x, center[1] + y, center[2] + z), material);
      }
      return this;
    }
    line({ from = [0, 0, 0], to = [0, 0, 0], material = 'default', thickness = 1 } = {}) {
      const steps = Math.max(1, Math.ceil(Math.hypot(to[0] - from[0], to[1] - from[1], to[2] - from[2])));
      for (let i = 0; i <= steps; i++) {
        const t = i / steps, x = Math.round(from[0] + (to[0] - from[0]) * t), y = Math.round(from[1] + (to[1] - from[1]) * t), z = Math.round(from[2] + (to[2] - from[2]) * t);
        const r = Math.max(0, Math.floor(thickness / 2));
        for (let a = -r; a <= r; a++) for (let b = -r; b <= r; b++) for (let c = -r; c <= r; c++) this.cells.set(this.key(x + a, y + b, z + c), material);
      }
      return this;
    }
    add(source, { at = [0, 0, 0] } = {}) {
      const cells = source instanceof VoxelPart ? source.cells : new Map((source?.voxels ?? []).map(([x, y, z, index]) => [x + ',' + y + ',' + z, source.palette?.[index] ?? 'default']));
      for (const [key, material] of cells) { const [x, y, z] = key.split(',').map(Number); this.cells.set(this.key(x + at[0], y + at[1], z + at[2]), material); }
      return this;
    }
    bounds() {
      if (!this.cells.size) return null;
      const values = [...this.cells.keys()].map((key) => key.split(',').map(Number));
      const min = [0, 1, 2].map((axis) => Math.min(...values.map((value) => value[axis])));
      const max = [0, 1, 2].map((axis) => Math.max(...values.map((value) => value[axis])));
      return [min, max];
    }
    serialize() {
      const palette = [...new Set(this.cells.values())];
      const voxels = [...this.cells.entries()].map(([key, material]) => { const [x, y, z] = key.split(',').map(Number); return [x, y, z, palette.indexOf(material)]; });
      return { version: 1, bounds: this.bounds(), palette, voxels };
    }
  }
  return {
    part: () => new VoxelPart(),
    material: (id) => ({ kind: 'material', id }),
    checker: (a, b) => ({ kind: 'checker', a, b }),
    random: (ids, weights) => ({ kind: 'random', ids, weights }),
    noise: (ids, opts) => ({ kind: 'noise', ids, opts }),
  };
})();

globalThis.__builtins.vibe = (() => {
  const chain = (pattern) => {
    const methods = ['s', 'gain', 'pan', 'lpf', 'hpf', 'fast', 'slow', 'every', 'swing', 'humanize', 'rate', 'prob', 'room', 'reverbBus', 'delaytime', 'delayfeedback', 'delayxfeed', 'delaylpf', 'drive', 'crush', 'grain', 'attack', 'decay', 'sustain', 'release', 'channel', 'scale', 'when'];
    for (const name of methods) {
      pattern[name] = (...args) => { pattern.ops.push({ name, args }); return pattern; };
    }
    pattern.stack = (...others) => __stack(pattern, ...others);
    return pattern;
  };
  const make = (events = []) => chain({ __pattern: true, events, ops: [] });
  const __s = (name) => make([{ sample: String(name) }]);
  const __note = (value) => make([{ note: value }]);
  const __stack = (...patterns) => make(patterns.flatMap((pattern) => pattern?.events ?? []));
  const __seq = (...patterns) => make(patterns.flatMap((pattern) => pattern?.events ?? []));
  const __silence = () => make([]);
  const __mini = (strings, ...values) => {
    const text = strings.reduce((out, part, index) => out + part + (values[index] ?? ''), '');
    return make(text.split(/\s+/).filter(Boolean).map((token) => ({ sample: token })));
  };
  return {
    note: __note,
    s: __s,
    stack: __stack,
    seq: __seq,
    silence: __silence,
    mini: __mini,
    arrange: (...items) => __stack(...items.filter((item) => item && item.__pattern)),
    scale: (key, degrees) => (degrees ?? []).map((degree) => ({ key, degree })),
    progression: (romans, key) => (romans ?? []).map((roman) => ({ roman, key })),
    arp: (chords, mode = 'up') => {
      const values = Array.isArray(chords) ? chords.flat() : [];
      if (mode === 'down') return values.slice().reverse();
      if (mode === 'updown') return values.concat(values.slice(1, -1).reverse());
      if (mode === 'random') return values.slice().sort(() => Math.random() - 0.5);
      return values;
    },
    chord: (name, octave = 3) => {
      const table = { major: [0, 4, 7], minor: [0, 3, 7], maj7: [0, 4, 7, 11], min7: [0, 3, 7, 10], dom7: [0, 4, 7, 10], sus2: [0, 2, 7], sus4: [0, 5, 7], dim: [0, 3, 6], aug: [0, 4, 8] };
      return (table[name] ?? table.major).map((step) => octave * 12 + step);
    },
    clamp01: (value) => __host('nativeMath', { op: 'clamp01', a: value }),
    lerp: (a, b, t) => a + (b - a) * t,
    smoothstep: (e0, e1, x) => { const t = __clamp((x - e0) / Math.max(1e-9, e1 - e0), 0, 1); return t * t * (3 - 2 * t); },
    above: (threshold, x, fade = 0.05) => __clamp((x - threshold) / Math.max(1e-6, fade), 0, 1),
    below: (threshold, x, fade = 0.05) => __clamp((threshold - x) / Math.max(1e-6, fade), 0, 1),
    between: (lo, hi, x, fade = 0.05) => Math.min(__clamp((x - lo) / fade, 0, 1), __clamp((hi - x) / fade, 0, 1)),
    mapRange: (x, inMin, inMax, outMin, outMax) => outMin + ((x - inMin) / Math.max(1e-9, inMax - inMin)) * (outMax - outMin),
  };
})();
globalThis.__builtins.fx = (() => {
  const field = (kind, value, args = {}) => {
    const node = { __field: true, kind, value, args, ops: [] };
    const numeric = ['add', 'sub', 'mul', 'div', 'min', 'max', 'mix', 'pow', 'mod', 'step', 'smoothstep', 'dot', 'atan2', 'to', 'clamp01', 'saturate', 'neg', 'abs', 'sin', 'cos', 'fract', 'floor', 'sqrt', 'length', 'normalize', 'cross', 'yaw'];
    for (const name of numeric) node[name] = (...params) => field('op', name, { left: node, params });
    return node;
  };
  const vec = (x, y, z) => field('vec', { x: __num(x), y: __num(y), z: __num(z) });
  const pop = (name, def) => ({ name, def, __pop: true });
  const emit = (kind, payload = {}) => {
    if (kind !== 'fx') return '';
    const ctx = globalThis.__active_ctx;
    if (!ctx) return '';
    const id = ctx.jayzee({ fx: payload, state: { fxScript: payload.script ?? '' } });
    globalThis.__fx_registry = globalThis.__fx_registry || {};
    globalThis.__fx_registry[id] = { payload, started: Date.now() };
    return id;
  };
  return {
    emit,
    pop,
    rgb: (r, g, b) => vec(r, g, b),
    vec3: vec,
    range: (min, max) => field('range', { min, max }),
    curve: (...points) => field('curve', points),
    noise: (input) => field('noise', input),
    curl: (freq) => field('curl', freq),
    cone: (deg) => field('cone', deg),
    sphereDir: () => field('sphereDir', 0),
    time: () => field('time', 0),
    floorHeight: () => field('floorHeight', 0),
    attr: (name) => field('attr', name),
    count: (popName) => field('count', popName),
    entity: (id) => field('entity', id),
    wind: () => vec(0, 0, 0),
    field: (def) => field('field', def),
    when: (source, value) => ({ __when: true, source, value }),
    disc: (opts) => ({ kind: 'disc', ...opts }),
    sphere: (opts) => ({ kind: 'sphere', ...opts }),
    box: (opts) => ({ kind: 'box', ...opts }),
    arc: (opts) => ({ kind: 'arc', ...opts }),
    line: (opts) => ({ kind: 'line', ...opts }),
    point: (opts) => ({ kind: 'point', ...opts }),
    path: (points, opts) => ({ kind: 'path', points, ...opts }),
    sprite: (opts) => ({ kind: 'sprite', ...opts }),
    mesh: (opts) => ({ kind: 'mesh', ...opts }),
    ribbon: (opts) => ({ kind: 'ribbon', ...opts }),
    light: (opts) => ({ kind: 'light', ...opts }),
    flicker: (hz, amount = 0.15) => field('op', 'flicker', { params: [hz, amount] }),
    firelight: (slowHz = 1.2, fastHz = 2) => field('op', 'firelight', { params: [slowHz, fastHz] }),
    spin: (maxRps = 1) => field('op', 'spin', { params: [maxRps] }),
    envelope: (...keys) => field('op', 'envelope', { params: keys }),
    gravity: () => field('op', 'gravity', {}),
    up: (speed = 1) => field('op', 'up', { params: [speed] }),
    buoyancy: (k) => field('op', 'buoyancy', { params: [k] }),
    drag: (k) => field('op', 'drag', { params: [k] }),
    blast: (speed) => field('op', 'blast', { params: [speed] }),
    radial: (speed) => field('op', 'radial', { params: [speed] }),
    attract: (strength) => field('op', 'attract', { params: [strength] }),
    vortex: (strength, axis = [0, 1, 0]) => field('op', 'vortex', { params: [strength, axis] }),
    hdr: (r, g, b) => vec(r, g, b),
    force: (f) => field('op', 'force', { params: [f] }),
    orbit: (opts) => field('op', 'orbit', { params: [opts] }),
    seek: (target, speed) => field('op', 'seek', { params: [target, speed] }),
    along: (opts) => field('op', 'along', { params: [opts] }),
    parent: (name) => field('parent', name),
  };
})();
globalThis.__builtins.three = (() => {
  const materials = ['MeshStandardNodeMaterial', 'MeshPhysicalNodeMaterial', 'MeshSSSNodeMaterial', 'MeshBasicNodeMaterial', 'MeshPhongNodeMaterial', 'MeshLambertNodeMaterial', 'MeshToonNodeMaterial', 'MeshMatcapNodeMaterial', 'LineBasicNodeMaterial', 'Line2NodeMaterial', 'SpriteNodeMaterial', 'PointsNodeMaterial'];
  const out = {};
  for (const name of materials) {
    out[name] = class {
      constructor(opts = {}) { Object.assign(this, opts); this.colorNode = opts.colorNode; this.opacityNode = opts.opacityNode; this.positionNode = opts.positionNode; this.sizeNode = opts.sizeNode; this.blending = opts.blending; }
      set color(value) { this.colorNode = value; }
      get color() { return this.colorNode; }
    };
  }
  const constants = ['FrontSide', 'BackSide', 'DoubleSide', 'NoBlending', 'NormalBlending', 'AdditiveBlending', 'SubtractiveBlending', 'MultiplyBlending', 'CustomBlending', 'AddEquation', 'SubtractEquation', 'ReverseSubtractEquation', 'MinEquation', 'MaxEquation', 'ZeroFactor', 'OneFactor', 'SrcColorFactor', 'OneMinusSrcColorFactor', 'SrcAlphaFactor', 'OneMinusSrcAlphaFactor', 'DstColorFactor', 'OneMinusDstColorFactor', 'DstAlphaFactor', 'OneMinusDstAlphaFactor', 'SrcAlphaSaturateFactor', 'Plane', 'RepeatWrapping', 'ClampToEdgeWrapping', 'MirroredRepeatWrapping', 'NearestFilter', 'LinearFilter', 'NearestMipmapNearestFilter', 'NearestMipmapLinearFilter', 'LinearMipmapNearestFilter', 'LinearMipmapLinearFilter', 'NoColorSpace', 'SRGBColorSpace', 'RGBAFormat', 'RGFormat', 'RedFormat', 'RGBAIntegerFormat', 'RGIntegerFormat', 'RedIntegerFormat', 'UnsignedByteType', 'HalfFloatType', 'FloatType', 'UnsignedIntType', 'NeverDepth', 'AlwaysDepth', 'LessDepth', 'LessEqualDepth', 'EqualDepth', 'GreaterEqualDepth', 'GreaterDepth', 'NotEqualDepth'];
  for (const name of constants) out[name] = name;
  for (const name of ['Color', 'Vector2', 'Vector3', 'Vector4', 'StorageTexture', 'Storage3DTexture', 'DataTexture', 'Data3DTexture']) out[name] = class { constructor(...args) { this.args = args; } };
  return out;
})();
globalThis.__builtins.tsl = (() => {
  const node = (value = 0) => {
    const target = { __node: true, value };
    return new Proxy(target, {
      get(object, key) {
        if (key in object) return object[key];
        if (key === Symbol.toPrimitive) return () => __num(object.value);
        if (typeof key !== 'string') return undefined;
        if (key === 'toVar' || key === 'assign') return () => node(value);
        if (key === 'compute') return () => node(value);
        return (...args) => node(value);
      },
    });
  };
  const n = (value) => node(value);
  const math = (name) => (...args) => node(args.find((value) => typeof value === 'number') ?? 0);
  const out = {
    float: n, vec2: n, vec3: n, vec4: n, color: n, int: n,
    mix: math('mix'), smoothstep: math('smoothstep'), step: math('step'), clamp: math('clamp'), min: math('min'), max: math('max'),
    abs: math('abs'), floor: math('floor'), ceil: math('ceil'), fract: math('fract'), sign: math('sign'), mod: math('mod'), negate: math('negate'),
    pow: math('pow'), sqrt: math('sqrt'), exp: math('exp'), log: math('log'), sin: math('sin'), cos: math('cos'), tan: math('tan'),
    atan: math('atan'), atan2: math('atan2'), length: math('length'), distance: math('distance'), dot: math('dot'), cross: math('cross'),
    normalize: math('normalize'), reflect: math('reflect'), refract: math('refract'), oneMinus: math('oneMinus'), saturate: math('saturate'),
    remap: math('remap'), remapClamp: math('remapClamp'),
    mx_noise_float: math('noise'), mx_fractal_noise_float: math('fractal'), mx_worley_noise_float: math('worley'), mx_noise_vec3: math('noise3'),
    hash: math('hash'), rand: math('rand'), oscSine: math('oscSine'), oscTriangle: math('oscTriangle'), oscSquare: math('oscSquare'), oscSawtooth: math('oscSawtooth'),
    uv: () => n(0), positionLocal: n(0), positionMorphed: n(0), positionWorld: n(0), positionView: n(0), normalLocal: n(0), normalWorld: n(0),
    cameraPosition: n(0), cameraViewMatrix: n(0), screenUV: n(0), screenSize: n(0), time: n(0), vertexColor: () => n(0), attribute: () => n(0), instanceIndex: n(0), vertexIndex: n(0), exp2: math('exp2'),
    frontFacing: n(0), sceneDepth: n(0), sceneDepthAt: () => n(0), depthFade: () => n(0), viewportSharedTexture: () => n(0),
    materialColor: n(0), materialOpacity: n(0), materialEmissive: n(0), audioLevel: n(0), audioSpectrum: () => n(0),
    billboarding: () => n(0), Fn: (fn) => (...args) => { try { return fn(args); } catch { return n(0); } },
    select: (cond, a, b) => n(a), If: (cond, a, b) => n(a), Loop: (fn) => { try { fn(0); } catch {} return n(0); },
    texture: n(0), textureSize: () => n(0), textureLoad: () => n(0), textureLevel: () => n(0), texture3D: () => n(0), convertToTexture: n(0),
    PI: Math.PI, PI2: Math.PI * 2, EPSILON: 1e-6, wgslFn: (text) => (...args) => n(0),
    instancedArray: (count = 1) => { const buffer = { element: (index) => n(0), setName: () => {}, compute: () => {} }; return buffer; },
    deltaTime: n(0), textureStore: () => {}, minFilter: 'LinearMipmapLinearFilter',
    workgroupArray: () => ({ element: () => n(0) }), workgroupBarrier: () => {},
    invocationLocalIndex: n(0), workgroupId: n(0), localId: n(0),
    atomicAdd: () => n(0), atomicSub: () => n(0), atomicMax: () => n(0), atomicMin: () => n(0), atomicAnd: () => n(0), atomicOr: () => n(0), atomicXor: () => n(0),
    atomicStore: () => {}, atomicLoad: () => n(0),
    objectWorldMatrix: () => n(0), objectPosition: () => n(0), objectScale: () => n(0), objectDirection: () => n(0),
  };
  return out;
})();
globalThis.__builtins.lighting = (() => {
  const atmosphere = () => globalThis.__active_ctx?.place?.atmosphere ?? {};
  const rotation = atmosphere()?.sun?.rotation ?? { pitch: 45, yaw: 135 };
  const yaw = __num(rotation.yaw, 135) * __deg, pitch = __num(rotation.pitch, 45) * __deg;
  const direction = { x: Math.sin(yaw) * Math.cos(pitch), y: Math.sin(pitch), z: -Math.cos(yaw) * Math.cos(pitch) };
  const intensity = __num(atmosphere()?.sun?.light?.intensity, 1);
  return {
    sunDirection: direction,
    sunRadiance: { x: intensity, y: intensity, z: intensity },
    ambientRadiance: { x: 0.2, y: 0.2, z: 0.2 },
    wet: 0,
    rain: 0,
    windDirection: { x: 1, y: 0, z: 0 },
    windSpeed: 0,
    windTravel: { x: 0, y: 0, z: 0 },
    fogColor: (toward) => atmosphere()?.fog?.color ?? 'oklch(0.9 0 0)',
    sunVisibility: (P) => 1,
    lightsAt: (P, opts) => ({ x: 0, y: 0, z: 0 }),
    lightCount: (P) => 0,
    lightRadiance: (P, i) => ({ x: 0, y: 0, z: 0 }),
    lightPosition: (P, i) => ({ x: 0, y: 0, z: 0 }),
  };
})();
globalThis.__builtins.water = (() => {
  const wave = (xz, t, tile, flow, flowSpeed, gust) => {
    const x = xz?.x ?? xz?.[0] ?? 0, z = xz?.z ?? xz?.[1] ?? 0;
    const speed = __num(flowSpeed, 1), scale = __num(tile, 1) || 1;
    return {
      x: Math.cos((x / scale + z / scale) * 2 + t * speed * 1.7) * 0.5,
      z: Math.sin((z / scale - x / scale) * 2 + t * speed * 1.3) * 0.5,
    };
  };
  return {
    waterRippleSlope: (xz, t, tile, flow, flowSpeed, gust) => wave(xz, t, tile, flow, flowSpeed, gust),
    waterFoam: (xz, t, flowVelocity, amount = 1, scale = 1, softness = 0.08) => __clamp(Math.sin((xz?.x ?? 0) * scale + t * 2) * 0.5 + 0.5, 0, 1) * amount,
    waterFoamField: (xz, t, flowVelocity, scale = 1) => __clamp(Math.sin((xz?.x ?? 0) + (xz?.z ?? 0) + t) * 0.5 + 0.5, 0, 1),
    waterFoamFieldLod: (xz, t, flowVelocity, scale, footprint) => __clamp(Math.sin((xz?.x ?? 0) + (xz?.z ?? 0) + t) * 0.5 + 0.5, 0, 1),
    waterFoamCoverage: (amount, field, softness = 0.08) => __clamp(__num(amount) * __num(field, 1), 0, 1),
    waterTransmittance: (deepTint, tintDistance, depth) => {
      const fade = Math.exp(-Math.abs(__num(depth)) / Math.max(0.001, __num(tintDistance, 1)));
      return { r: fade, g: fade, b: fade, a: fade };
    },
    waterMedium: (scene, transmission, scatter) => scene,
    waterUnderwaterDistortion: (uv, t, distance, visibility, strength) => ({
      x: Math.sin((uv?.x ?? 0) * 20 + t * 2) * strength * 0.01,
      y: Math.cos((uv?.y ?? 0) * 20 + t * 2) * strength * 0.01,
    }),
    waterRayBoxDistance: (origin, direction, lower, upper, sceneDistance) => {
      let tmin = 0, tmax = sceneDistance ?? Infinity;
      for (const axis of ['x', 'y', 'z']) {
        const d = direction?.[axis] ?? 0, o = origin?.[axis] ?? 0;
        if (Math.abs(d) < 1e-9) continue;
        const t1 = ((lower?.[axis] ?? -1e9) - o) / d, t2 = ((upper?.[axis] ?? 1e9) - o) / d;
        tmin = Math.max(tmin, Math.min(t1, t2));
        tmax = Math.min(tmax, Math.max(t1, t2));
      }
      return tmax >= tmin ? tmax : -1;
    },
    waterCaustics: (floorXZ, t) => ({ x: Math.sin((floorXZ?.x ?? 0) * 3 + t) * 0.5 + 0.5, y: Math.cos((floorXZ?.z ?? 0) * 3 + t) * 0.5 + 0.5, z: 0 }),
    waterCausticsAt: (floorPos, surfaceY, sunDirection, t) => ({ x: Math.sin((floorPos?.x ?? 0) + t) * 0.5 + 0.5, y: 1, z: 0 }),
    waterCausticsRefracted: (floorPos, surfaceY, sunDirection, t) => ({ x: Math.sin((floorPos?.x ?? 0) + t) * 0.5 + 0.5, y: 1, z: 0 }),
    waterFresnel: (nDotV, f0) => __num(f0, 0.02) + (1 - __num(f0, 0.02)) * Math.pow(1 - __clamp(nDotV, 0, 1), 5),
  };
})();
globalThis.__builtins.env = {
  envAt: (ctx, point) => ({
    wind: { x: 1, y: 0, z: 0 },
    rain: 0,
    light: 1,
    charge: 0,
    water: { depth: 0, flow: { x: 0, y: 0, z: 0 } },
    cloud: 0,
  }),
};
globalThis.__builtins.postfx = {
  grade: (color, opts = {}) => ({ __postfx: 'grade', color, opts }),
  vignette: (color, intensity = 0.2, opts = {}) => ({ __postfx: 'vignette', color, intensity, opts }),
  grain: (color, intensity = 0.04) => ({ __postfx: 'grain', color, intensity }),
  film: (color, intensity = 0.04) => ({ __postfx: 'film', color, intensity }),
  tint: (color, tint, amount = 1) => ({ __postfx: 'tint', color, tint, amount }),
  scanlines: (color, intensity = 0.2, count = 400) => ({ __postfx: 'scanlines', color, intensity, count }),
  dotScreen: (color, angle = 0, scale = 1) => ({ __postfx: 'dotScreen', color, angle, scale }),
  chromaticAberration: (color, strength = 0.01) => ({ __postfx: 'chromaticAberration', color, strength }),
  dof: (color, viewZ, focusDistance, focusRange, bokehScale) => ({ __postfx: 'dof', color, viewZ, focusDistance, focusRange, bokehScale }),
  pixelate: (scene, size = 4) => ({ __postfx: 'pixelate', scene, size }),
  blur: (scene, amount = 1) => ({ __postfx: 'blur', scene, amount }),
  bloom: (color) => ({ __postfx: 'bloom', color }),
};

globalThis.performance = globalThis.performance || { now: () => Date.now() };
globalThis.TextEncoder = globalThis.TextEncoder || class { encode(text) { const bytes = []; for (let i = 0; i < text.length; i++) { let code = text.charCodeAt(i); if (code < 0x80) bytes.push(code); else if (code < 0x800) { bytes.push(0xc0 | (code >> 6), 0x80 | (code & 63)); } else { bytes.push(0xe0 | (code >> 12), 0x80 | ((code >> 6) & 63), 0x80 | (code & 63)); } } return new Uint8Array(bytes); } };
globalThis.TextDecoder = globalThis.TextDecoder || class { decode(bytes) { let out = ''; for (let i = 0; i < bytes.length;) { const byte = bytes[i++]; if (byte < 0x80) out += String.fromCharCode(byte); else if (byte < 0xe0) out += String.fromCharCode(((byte & 31) << 6) | (bytes[i++] & 63)); else out += String.fromCharCode(((byte & 15) << 12) | ((bytes[i++] & 63) << 6) | (bytes[i++] & 63)); } return out; } };
globalThis.__builtins.vec = globalThis.__builtins.vec3;
globalThis.__builtins['block-shape'] = { block: (kind, opts = {}) => ({ kind, ...opts }), shape: (kind, opts = {}) => ({ kind, ...opts }) };
globalThis.__builtins.authoring = { define: (value) => value, tool: (value) => value };
globalThis.__builtins.globe = {
  configure: (options) => __host('globeConfigure', options ?? {}),
  info: () => __host('globeInfo', {}) ?? {},
  toCartesian: (latitude, longitude, altitude) => __host('globeToCartesian', { latitude, longitude, altitude }),
  toSpherical: (position) => __host('globeToSpherical', { position }),
  elevation: (latitude, longitude) => __host('globeElevation', { latitude, longitude }),
  terrainHeight: (latitude, longitude) => __host('globeElevation', { latitude, longitude }),
  chunks: (limit) => __host('globeChunks', { limit }),
  gravity: (position) => __host('globeGravity', { position }),
  orientation: (latitude, longitude, heading) => __host('globeOrientation', { latitude, longitude, heading }),
  navigation: (options) => __host('globeNavigation', options ?? {}),
  navigate: (options) => __host('globeNavigation', options ?? {}),
  wrap: (longitude) => __host('globeNavigation', { toLongitude: longitude })?.wrapped,
};
globalThis.__builtins.planet = globalThis.__builtins.globe;
globalThis.__builtins.spherical = globalThis.__builtins.globe;
globalThis.__globe_config = null;
globalThis.__fx_registry = globalThis.__fx_registry || {};
function __make_ctx(selfId, placeName) {
  const self = __make('obj:' + selfId);
  const place = __make('place:' + placeName);
  const world = __make('world');
  const session = __make('session:' + selfId);
  const params = __parse(__params_json(selfId)) ?? {};
  const viewFn = (offset) => ({ __view: true, offset });
  viewFn.camera = __local(Object.assign({}, __parse(__get_json('view:' + selfId, 'camera')) ?? {}), (value) => __set_json('view:' + selfId, 'camera', __json(value)));
  const playerEntries = __parse(__players_json('world')) ?? [];
  const firstPlayer = playerEntries[0];
  const playerId = typeof firstPlayer === 'string' ? firstPlayer : firstPlayer?.id;
  const player = playerId ? __make('obj:' + playerId) : null;
  const music = __local({ volume: 1, playing: false }, () => {});
  const geometry = { triangles: 0, quads: 0, vertices: 0, materials: 0 };
  const materialState = {};
  const ctx = {
    self,
    place,
    world,
    session,
    player,
    state: self.state,
    view: viewFn,
    params,
    lod: 2,
    seed: 0,
    day: 0,
    field: (name, x, z) => __host('readField', { name, x, z }) ?? 0,
    quality: 'high',
    device: { class: 'desktop', coarse: false, maxTextures: 16, quality: 'high' },
    geometry,
    files: {
      read: (path) => __host('fileRead', { path }),
      write: (path, text) => __host('fileWrite', { path, text }),
      delete: (path) => __host('fileDelete', { path }),
      list: () => __host('fileList', {}) ?? [],
    },
    now: () => __host('now', {}) ?? 0,
    dtMs: __num(globalThis.__dt_ms?.(), 0),
    gapMs: 0,
    constants: {},
    portal: null,
    globe: {
      configure: (options) => __host('globeConfigure', options ?? {}),
      info: () => __host('globeInfo', {}) ?? {},
      toCartesian: (latitude, longitude, altitude) => __host('globeToCartesian', { latitude, longitude, altitude }),
      toSpherical: (position) => __host('globeToSpherical', { position }),
      elevation: (latitude, longitude) => __host('globeElevation', { latitude, longitude }),
      terrainHeight: (latitude, longitude) => __host('globeElevation', { latitude, longitude }),
      chunks: (limit) => __host('globeChunks', { limit }),
      gravity: (position) => __host('globeGravity', { position }),
      orientation: (latitude, longitude, heading) => __host('globeOrientation', { latitude, longitude, heading }),
      navigation: (options) => __host('globeNavigation', options ?? {}),
      navigate: (options) => __host('globeNavigation', options ?? {}),
      wrap: (longitude) => __host('globeNavigation', { toLongitude: longitude })?.wrapped,
    },
    random: () => __num(__host('nativeRandom', {}), __random()),
    log: (...args) => __log('info', args.map((value) => (typeof value === 'string' ? value : __json(value))).join(' ')),
    warn: (...args) => __log('warn', args.map((value) => (typeof value === 'string' ? value : __json(value))).join(' ')),
    error: (...args) => __log('error', args.map((value) => (typeof value === 'string' ? value : __json(value))).join(' ')),
    getObject: (value) => {
      const id = typeof value === 'string' ? value : value?.id;
      return id && __exists(id) ? __make('obj:' + id) : null;
    },
    jayzee: (...args) => {
      const [first, second] = args;
      if (typeof first === 'string') {
        const id = first;
        const value = __local(second ?? {}, (target) => __set_json('obj:' + id, '', __json(target)));
        return __jayzee(id, __json(second ?? {}));
      }
      return __jayzee('', __json(first ?? {}));
    },
    spawn: (first, second) => (typeof first === 'string' ? __jayzee(first, __json(second ?? {})) : __jayzee('', __json(first ?? {}))),
    uvProjection: null,
    __feelRaw: null,
    __glass: 0,
    __safeDestroy: false,
    __soundGun: null,
    __soundQ: null,
    __soundRaw: null,
    destroy: (value, options) => {
      if (Array.isArray(value)) value.forEach((id) => __destroy(__json(id)));
      else __destroy(value === undefined ? __json(selfId) : __json(typeof value === 'string' ? value : value?.id));
    },
    query: (options) => {
      const opts = typeof options === 'string' ? { tags: [options] } : (options ?? {});
      const result = __parse(__query_json(__json(opts))) ?? [];
      if (opts.select === 'ids') return result;
      return result.map((id) => __make('obj:' + id));
    },
    after: (seconds, hook, payload) => __after(seconds, hook, __json(payload ?? null)),
    cancel: (id) => __cancel(String(id)),
    getTimers: (owner) => __parse(__timers_json(owner ?? '')) ?? [],
    sleep: (seconds) => __sleep(seconds ?? -1),
    emit: (name, payload, options) => {
      const result = __emit(String(name), __json(payload ?? null), __json(options ?? {}));
      return result === '' ? undefined : result;
    },
    on: (name, handler, options) => {
      const index = globalThis.__handlers.length;
      globalThis.__handlers.push(handler);
      const id = __on_id(String(name), index, __json(options ?? {}));
      return () => __off(id);
    },
    changed: (glob) => __parse(__changed_json(String(glob))) ?? [],
    getWaterLevelAt: (x, z, target) => __host('waterLevel', { x, z, place: target }),
    readField: (name, x, z, target) => __host('readField', { name, x, z, place: target }) ?? 0,
    getTerrainNormal: (x, z, target) => __host('terrainNormal', { name: 'terrain', x, z, place: target }),
    getTerrainHeight: (x, z, target) => __host('terrainHeight', { x, z, place: target }),
    sampleSpline: (id, t) => __host('sampleSpline', { id, t }),
    splineLength: (id) => __host('splineLength', { id }),
    projectToSpline: (id, point) => __host('projectToSpline', { id, position: __vec(point) }),
    spawnPlayer: (options) => __host('spawnPlayer', options ?? {}),
    setInput: (options) => __host('setInput', options ?? {}),
    setAnim: (channel, value) => __host('setAnim', { id: selfId, channel, value }),
    ensureScatter: (options) => __host('featureProbe', { name: 'ensureScatter', options }) ?? { instances: 0 },
    getPixelProvenance: (x, y) => __host('featureProbe', { name: 'getPixelProvenance', x, y }) ?? { x: x ?? 0, y: y ?? 0, source: 'local-shim' },
    enterPlace: (entity, options) => {
      if (typeof entity === 'object' && entity !== null) {
        options = entity;
        entity = undefined;
      }
      const target = options?.place ?? options?.name ?? options;
      return __host('enterPlace', { id: entity ? (typeof entity === 'string' ? entity : entity.id) : selfId, place: target });
    },
    cross: (entity, target) => {
      const id = typeof entity === 'string' ? entity : entity?.id ?? selfId;
      return __cross(id, target);
    },
    attach: (entity, parent, offset) => {
      const id = typeof entity === 'string' ? entity : entity?.id;
      const parentId = parent == null ? null : (typeof parent === 'string' ? parent : parent.id);
      __attach(id ?? '', __json(parentId), __json(offset ?? null));
    },
    commit: (message, fn) => {
      if (typeof message === 'function') { fn = message; message = ''; }
      try { if (fn) fn(); } catch (error) { __log('error', String(error)); }
      return __host('commit', { message: message ?? '' });
    },
    resolveControls: (surface) => ({ surface: surface ?? {}, diagnostics: [] }),
    worldToScreen: (position) => __host('worldToScreen', { position: __vec(position) }),
    getObjectBoneNames: (id) => ({ names: [] }),
    getObjectMorphTargetNames: (id) => ({ names: [] }),
    getObjectSocket: () => null,
    getVibeAudio: (id) => {
      const native = __host('vibeSnapshot', { id }) ?? __host('featureProbe', { name: 'getVibeAudio', id });
      const object = __make('obj:' + id);
      const audio = object?.audio;
      if (!audio) return null;
      return { playing: !audio.paused, bpm: audio.bpm ?? 120, bar: native?.tick ?? 0, barPos: 0, beat: 0, beatPos: 0, cycle: 0, cyclePos: 0, voices: [], triggers: [] };
    },
    stopFx: (id) => { if (globalThis.__fx_registry) globalThis.__fx_registry[id] = null; },
    pulseFx: (id, name, opts) => {
      const entry = globalThis.__fx_registry?.[id];
      if (entry) { entry.pulses = entry.pulses || []; entry.pulses.push({ name, at: Date.now(), opts }); }
    },
    getFxState: (id) => __host('fxState', { id }) ?? (() => {
      const entry = globalThis.__fx_registry?.[id];
      return entry ? { alive: 0, rate: 0, lifetime: null, expectedAlive: 0, pulses: entry.pulses ?? [] } : null;
    })(),
    music,
    notifyDm: (message, options) => { __host('dmNotify', { message: String(message), options: options ?? null }); __log('info', String(message)); },
    notifyDmOnce: (key, message, options) => {
      if (globalThis.__dm_once_keys?.has?.(key)) return;
      (globalThis.__dm_once_keys ??= new Set()).add(key);
      __host('dmNotify', { key, message: String(message ?? key), options: options ?? null });
    },
    getRoomId: () => '',
    presence: async (userIds) => (userIds ?? []).map((value) => ({ userId: typeof value === 'string' ? value : value.userId, online: true, place: placeName })),
    job: (jobId, args, callback, options) => {
      const record = __host('jobRecord', { job: jobId, args: args ?? {} }) ?? {};
      const id = record.id ?? ('job-' + jobId + '-' + Date.now());
      if (typeof callback === 'string') {
        __after(0, callback, __json(record));
      } else {
        globalThis.__jobs[id] = { pending: true, callback: typeof callback === 'function' ? callback : null, options, record };
      }
      return id;
    },
    awaitJob: async (requestId) => ({ id: requestId, ok: true, data: null }),
    proposePurchase: (buyer, itemId) => 'purchase-' + itemId,
    sql: {
      run: async (query, params) => [],
      exec: async (query, params) => [],
      query: async (query, params) => [],
    },
    trade: (mine, opts) => __host('tradeReceipt', { mine, with: opts?.with ?? null, for: opts?.for ?? null }) ?? { accepted: false },
    setBpm: (value) => { ctx.__bpm = value; },
    setCps: (value) => { ctx.__bpm = value * 60; },
    setSwing: (value) => { ctx.__swing = value; },
    param: (name, fallback) => params[name] ?? fallback,
    albedo: () => ctx,
    color: () => ctx,
    roughness: () => ctx,
    metalness: () => ctx,
    emissive: () => ctx,
    smooth: () => ctx,
    flat: () => ctx,
    quad: (...args) => { geometry.quads += 1; geometry.vertices += 4; geometry.triangles += 2; },
    tri: (...args) => { geometry.triangles += 1; geometry.vertices += 3; },
    vertex: (...args) => { geometry.vertices += 1; },
    normal: () => {},
    uv: () => {},
    index: () => {},
    material: (value) => { geometry.materials += 1; return value; },
    scene: { __node: true },
    depth: { __texture: true },
    data: { __texture: true },
    additive: { __texture: true },
    viewZ: 0,
    viewZAt: () => 0,
    normalAt: () => ({ x: 0, y: 1, z: 0 }),
    worldPosition: () => ({ x: 0, y: 0, z: 0 }),
    skyVisibility: () => 1,
    previousUv: () => ({ x: 0, y: 0 }),
    velocity: { x: 0, y: 0 },
    target: () => ({ __target: true }),
    pass: (target, fn) => ({ __texture: true }),
    cube: { __texture: true },
    cover: { __texture: true },
  };
  ctx.self.material = materialState;
  globalThis.__active_ctx = ctx;
  return ctx;
}
globalThis.__make_ctx = __make_ctx;
globalThis.__make = __make;
const __place_ctxs = {};
function __place_ctx(placeName) {
  const ctx = (__place_ctxs[placeName] ??= __make_ctx('', placeName));
  ctx.dtMs = __num(globalThis.__dt_ms?.(), 0);
  return ctx;
}
globalThis.__place_ctx = __place_ctx;
globalThis.__invoke0 = (fn, a) => { try { fn(a); return ''; } catch (error) { return String(error && error.stack ? String(error) + '\n' + error.stack : error); } };
globalThis.__invoke1 = (fn, a, b) => { try { fn(a, b); return ''; } catch (error) { return String(error && error.stack ? String(error) + '\n' + error.stack : error); } };
globalThis.__invoke2 = (fn, a, b, c) => { try { fn(a, b, c); return ''; } catch (error) { return String(error && error.stack ? String(error) + '\n' + error.stack : error); } };
globalThis.__call1 = (fn, a, b) => { try { return fn(a, b); } catch (error) { return '__error:' + String(error && error.stack ? error.stack : error); } };
globalThis.__builtins.postfx.ctx = globalThis.__builtins.postfx;
