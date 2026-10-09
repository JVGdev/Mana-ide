// Mana as a fluid (SPEC §11): free mana is particles that press on their neighbours, drag the air they move through and
// are dragged by it, stop against matter, and shove bodies they run into. Matter held by mana gives a particle weight
// and inertia; water held in mana can't be squeezed, and holds together; earth held in mana is rock, its particles bound
// to each other.
//
// It's smoothed-particle hydrodynamics. Each particle feels its neighbours within PHYSICS.smoothing metres. Every force
// between two things in the world is equal and opposite (particle and particle, particle and air, air and air, particle
// and body), so the world's momentum only changes by what comes from outside (weight, pushes, the ground), and
// `World.momentumError` checks it.

import { PHYSICS } from './physics.ts'
import { total } from './parts.ts'
import { massOf, type Body, type Bond, type Particle, type Vec, type World } from './world.ts'

/** What the fluid needs to know from the machine. */
export type FluidHooks = {
  /** A body this particle passes through instead of striking: its weave's maker. */
  passes?: (p: Particle) => Body | undefined
  /** Particles held still: a weave in its caster's hand stays where it was laid until it's set loose. */
  still?: (p: Particle) => boolean
  /**
   * Matter held by mana in a cell, as the tick began, that isn't this particle's weave's own: it takes room, like matter
   * that isn't held. A weave's own matter never blocks it: it moves together.
   */
  othersCarried?: (p: Particle, cell: number) => number
}

type Kernels = { poly6: number; grad: number; spiky: number; lap: number; cohesion: number }

const kernelCache = new Map<string, Kernels>()

function kernels(dims: 2 | 3): Kernels {
  const h = PHYSICS.smoothing
  const key = `${dims}:${h}`
  let K = kernelCache.get(key)
  if (K) return K
  K =
    dims === 2
      ? { poly6: 4 / (Math.PI * h ** 8), grad: -24 / (Math.PI * h ** 8), spiky: -30 / (Math.PI * h ** 5), lap: 40 / (Math.PI * h ** 5), cohesion: 0 }
      : {
          poly6: 315 / (64 * Math.PI * h ** 9),
          grad: -945 / (32 * Math.PI * h ** 9),
          spiky: -45 / (Math.PI * h ** 6),
          lap: 45 / (Math.PI * h ** 6),
          cohesion: 0,
        }
  // Akinci's cohesion kernel (2013), scaled so it sums to one over its reach, in 2D or 3D.
  let sum = 0
  for (let i = 0; i < 1000; i++) {
    const r = ((i + 0.5) / 1000) * h
    sum += cohesionShape(r, h) * (dims === 2 ? 2 * Math.PI * r : 4 * Math.PI * r * r) * (h / 1000)
  }
  K.cohesion = 1 / sum
  kernelCache.set(key, K)
  return K
}

/** How neighbours pull at each distance: most at half the smoothing length, and pushing back when very close. */
function cohesionShape(r: number, h: number): number {
  if (r >= h || r <= 0) return 0
  const c = (h - r) ** 3 * r ** 3
  return r > h / 2 ? c : 2 * c - h ** 6 / 64
}

/** Free mana: what presses as a gas, and what the mana senses (DENS, GRAD, NVEL). */
const freeMass = (p: Particle) => total(p.free)
/** The mass of the matter it holds. */
function matterMass(p: Particle): number {
  const m = PHYSICS.matterMass
  return p.carried[0] * m[0] + p.carried[1] * m[1] + p.carried[2] * m[2] + p.carried[3] * m[3]
}

/** A blend of a per-part property by how much of each part a particle's free mana is. */
function blend(p: Particle, by: number[]): number {
  const m = freeMass(p)
  if (m <= 0) return 0
  return (p.free[0] * by[0] + p.free[1] * by[1] + p.free[2] * by[2] + p.free[3] * by[3]) / m
}

/** A blend of a property of free mana and one of matter, by how much of the particle's mass each part is. */
function blendAll(p: Particle, free: number[], matter: number[]): number {
  const mm = PHYSICS.matterMass
  let s = 0
  let m = 0
  for (let k = 0; k < 4; k++) {
    s += p.free[k] * free[k] + p.carried[k] * mm[k] * matter[k]
    m += p.free[k] + p.carried[k] * mm[k]
  }
  return m > 0 ? s / m : 0
}

/** How dense a particle's matter would be, packed full: a full cell's mass over its volume. */
function fullDensity(p: Particle): number {
  const held = total(p.carried)
  if (held <= 0) return 0
  return (PHYSICS.cellMatter * (matterMass(p) / held)) / PHYSICS.cell ** 3
}

/** Neighbours this step: pairs of indices into the particles, closer than the smoothing length, and how far apart. */
type Pairs = { n: number; a: Int32Array; b: Int32Array; d: Float64Array; r: Float64Array }

/** Every pair of particles closer than the smoothing length, once each. */
function findPairs(world: World, ps: Particle[]): Pairs {
  const h = PHYSICS.smoothing
  const flat = world.d === 1
  const grid = new Map<number, number[]>()
  const cx = new Int32Array(ps.length)
  const cy = new Int32Array(ps.length)
  const cz = new Int32Array(ps.length)
  const key = (x: number, y: number, z: number) => x + 2048 + (y + 2048) * 4096 + (z + 2048) * 16777216
  for (let i = 0; i < ps.length; i++) {
    const p = ps[i].pos
    cx[i] = Math.floor(p[0] / h)
    cy[i] = Math.floor(p[1] / h)
    cz[i] = flat ? 0 : Math.floor(p[2] / h)
    const k = key(cx[i], cy[i], cz[i])
    const list = grid.get(k)
    if (list) list.push(i)
    else grid.set(k, [i])
  }
  let cap = Math.max(64, ps.length * 16)
  let A = new Int32Array(cap)
  let Bi = new Int32Array(cap)
  let D = new Float64Array(cap * 3)
  let R = new Float64Array(cap)
  let n = 0
  const h2 = h * h
  const zr = flat ? 0 : 1
  for (let i = 0; i < ps.length; i++) {
    const pa = ps[i].pos
    for (let dz = -zr; dz <= zr; dz++)
      for (let dy = -1; dy <= 1; dy++)
        for (let dx = -1; dx <= 1; dx++) {
          const list = grid.get(key(cx[i] + dx, cy[i] + dy, cz[i] + dz))
          if (!list) continue
          for (const j of list) {
            if (j <= i) continue
            const pb = ps[j].pos
            const x = pa[0] - pb[0]
            const y = pa[1] - pb[1]
            const z = flat ? 0 : pa[2] - pb[2]
            const r2 = x * x + y * y + z * z
            if (r2 >= h2) continue
            if (n === cap) {
              cap *= 2
              const grow = <T extends Int32Array | Float64Array>(old: T, size: number): T => {
                const next = new (old.constructor as new (n: number) => T)(size)
                next.set(old)
                return next
              }
              A = grow(A, cap)
              Bi = grow(Bi, cap)
              D = grow(D, cap * 3)
              R = grow(R, cap)
            }
            A[n] = i
            Bi[n] = j
            D[n * 3] = x
            D[n * 3 + 1] = y
            D[n * 3 + 2] = z
            R[n] = Math.sqrt(r2)
            n++
          }
        }
  }
  return { n, a: A, b: Bi, d: D, r: R }
}

/**
 * What each particle feels: how dense the mana around it is, which way it thickens, and how its neighbours move. And
 * how dense the matter held around it is.
 */
export function feel(world: World, ps: Particle[] = world.particles, pairs = findPairs(world, ps)) {
  const h = PHYSICS.smoothing
  const K = kernels(world.d === 1 ? 2 : 3)
  // In 2D the world is one cell deep: density per square metre, over that depth, is density per cubic metre.
  const depth = world.d === 1 ? world.cell : 1
  const n = ps.length
  const fm = new Float64Array(n)
  const mm = new Float64Array(n)
  const weight = new Float64Array(n)
  const self = K.poly6 * h ** 6
  for (let i = 0; i < n; i++) {
    const p = ps[i]
    fm[i] = freeMass(p)
    mm[i] = matterMass(p)
    p.rho = fm[i] * self
    p.rhoM = mm[i] * self
    p.grad = [0, 0, 0]
    p.nvel = [0, 0, 0]
  }
  const { a: A, b: Bi, d: D, r: R } = pairs
  for (let e = 0; e < pairs.n; e++) {
    const i = A[e]
    const j = Bi[e]
    const a = ps[i]
    const b = ps[j]
    const r = R[e]
    const q = h * h - r * r
    const w = K.poly6 * q * q * q
    const ma = fm[i]
    const mb = fm[j]
    a.rho += mb * w
    b.rho += ma * w
    a.rhoM += mm[j] * w
    b.rhoM += mm[i] * w
    const g = K.grad * q * q
    for (let k = 0; k < 3; k++) {
      const dk = D[e * 3 + k]
      a.grad[k] += mb * g * dk
      b.grad[k] -= ma * g * dk
      a.nvel[k] += mb * w * b.vel[k]
      b.nvel[k] += ma * w * a.vel[k]
    }
    weight[i] += mb * w
    weight[j] += ma * w
  }
  for (let i = 0; i < n; i++) {
    const p = ps[i]
    const w = weight[i]
    p.rho /= depth
    p.rhoM /= depth
    for (let k = 0; k < 3; k++) {
      p.grad[k] /= depth
      p.nvel[k] = w > 0 ? p.nvel[k] / w : p.vel[k]
    }
  }
}

/** One tick of the fluid: weight, pressure, holding together, rock, the air, and what the particles run into. */
export function stepFluid(world: World, hooks: FluidHooks = {}) {
  const ps = hooks.still ? world.particles.filter((p) => !hooks.still!(p)) : world.particles
  if (!ps.length) return
  const moving = new Set(ps)
  // Lowest first: the ground's support passes up through the rock within each pass.
  const low = (b: Bond) => Math.min(b.a.pos[1], b.b.pos[1])
  const bonds = world.bonds.filter((b) => moving.has(b.a) && moving.has(b.b)).sort((x, y) => low(x) - low(y))
  const dt = 1 / PHYSICS.substeps
  const blocked = blocker(world, hooks)
  for (let s = 0; s < PHYSICS.substeps; s++) {
    for (const p of ps) p.mass = massOf(p)
    const pairs = findPairs(world, ps)
    feel(world, ps, pairs)
    forces(world, ps, pairs, dt)
    hold(world, ps, bonds, dt, blocked)
    drag(world, ps, dt)
    move(world, ps, dt, hooks, blocked)
  }
  feel(world, ps)
  // Bonds stretched or squeezed too far have broken: the rock cracks there.
  const broken = new Set(bonds.filter((b) => b.rest < 0))
  if (broken.size) world.bonds = world.bonds.filter((b) => !broken.has(b))
}

function forces(world: World, ps: Particle[], pairs: Pairs, dt: number) {
  const K = kernels(world.d === 1 ? 2 : 3)
  const h = PHYSICS.smoothing
  const depth = world.d === 1 ? world.cell : 1
  const g = PHYSICS.gravity
  const n = ps.length
  // What each particle brings, worked out once: its free and matter mass, how its free mana presses, how hard it
  // coheres, how thick it is, and how dense its matter would be packed full.
  const fm = new Float64Array(n)
  const mm = new Float64Array(n)
  const press = new Float64Array(n)
  const over = new Float64Array(n)
  const coh = new Float64Array(n)
  const visc = new Float64Array(n)
  const rho = new Float64Array(n)
  for (let i = 0; i < n; i++) {
    const p = ps[i]
    let w = 0
    for (let k = 0; k < 4; k++) w += p.free[k] * PHYSICS.fall[k] + p.carried[k] * PHYSICS.matterMass[k] * PHYSICS.matterFall[k]
    // Weight: free mana by part, and the matter it holds.
    p.acc = [0, -g * w, 0]
    world.impulse.gravity[1] -= g * w * dt
    fm[i] = freeMass(p)
    mm[i] = matterMass(p)
    // Free mana presses like a gas: pressure = k·ρ, so its term is k/ρ.
    press[i] = fm[i] > 0 ? blend(p, PHYSICS.stiffness) / (p.rho * depth) : 0
    // Matter can't be packed past full: past it, it presses back.
    over[i] = mm[i] > 0 ? (PHYSICS.matterStiffness * Math.max(0, p.rhoM - fullDensity(p))) / (p.rhoM * p.rhoM) : 0
    coh[i] = blendAll(p, PHYSICS.cohesion, PHYSICS.matterCohesion)
    visc[i] = blendAll(p, PHYSICS.viscosity, PHYSICS.viscosity)
    rho[i] = p.rho + p.rhoM
  }
  const { a: A, b: Bi, d: D, r: R } = pairs
  const h6 = h ** 6
  for (let e = 0; e < pairs.n; e++) {
    const r = R[e]
    if (r < 1e-9) continue
    const i = A[e]
    const j = Bi[e]
    const a = ps[i]
    const b = ps[j]
    const Ma = a.mass
    const Mb = b.mass
    if (Ma <= 0 || Mb <= 0) continue
    const hr = h - r
    let s = 0
    if (fm[i] > 0 && fm[j] > 0) s += -(press[i] + press[j]) * K.spiky * hr * hr * fm[i] * fm[j]
    if (over[i] + over[j] > 0 && mm[i] > 0 && mm[j] > 0) s += (-(over[i] + over[j]) * K.spiky * hr * hr * mm[i] * mm[j]) / depth
    if (coh[i] > 0 && coh[j] > 0) {
      // Akinci's cohesion: most pull at half the smoothing length, pushing back when very close.
      const c = hr * hr * hr * r * r * r
      const shape = r > h / 2 ? c : 2 * c - h6 / 64
      s += (-((coh[i] + coh[j]) / 2) * Ma * Mb * K.cohesion * shape) / depth
    }
    // Thickness: neighbours' motions even out.
    const v = (((visc[i] + visc[j]) / 2) * K.lap * hr * 2 * Ma * Mb) / ((rho[i] + rho[j]) * depth)
    for (let k = 0; k < 3; k++) {
      const fk = (s * D[e * 3 + k]) / r + v * (b.vel[k] - a.vel[k])
      a.acc[k] += fk
      b.acc[k] -= fk
    }
  }
  for (const p of ps) {
    const M = p.mass
    if (M <= 0) continue
    for (let k = 0; k < 3; k++) p.vel[k] += (p.acc[k] / M) * dt
  }
}

type Blocked = (p: Particle, from: number, to: number, down: boolean) => boolean

/**
 * Whether what's in a cell stops a particle moving into it from another. Free mana is stopped by solid matter. Mana
 * holding matter is stopped where there's no room for what it holds (matter blocks matter), and, coming down, by any
 * solid matter: loose earth holds up what lands on it.
 */
function blocker(world: World, hooks: FluidHooks): Blocked {
  return (p, from, to, down) => {
    if (to < 0) return true
    if (to === from) return false
    const load = total(p.carried)
    if (load <= PHYSICS.epsilon) return world.solidAt(to)
    if (down && world.solidAt(to)) return true
    return PHYSICS.cellMatter - total(world.matter[to]) - (hooks.othersCarried?.(p, to) ?? 0) < load
  }
}

/** How close above the ground a particle rests on it, metres. */
const CONTACT = 0.02

/**
 * What holds particles where they are. Rock keeps its shape: every bond is pushed back to its length, by equal and
 * opposite changes in its two particles' speeds (and a little more to close what it's off by). The ground holds up what
 * rests on it, and its friction holds it from sliding. A few passes over all of them, so that the ground's support
 * reaches up through the rock. A bond that would have to pull or push harder than it can breaks (marked by a negative
 * length), and so does one bent too far, or one whose particles no longer hold matter.
 */
function hold(world: World, ps: Particle[], bonds: Bond[], dt: number, blocked: Blocked) {
  const grounded: Particle[] = []
  for (const p of ps) {
    const from = world.cellOf(p.pos)
    const to = world.cellOf([p.pos[0], p.pos[1] - CONTACT, p.pos[2]])
    if (to !== from && blocked(p, from, to, true)) grounded.push(p)
  }
  if (!grounded.length && !bonds.length) return
  const fix = 0.2 / dt
  const strength = PHYSICS.bondStrength * dt * PHYSICS.bondIterations
  const walls = world.impulse.walls
  const iterations = bonds.length ? PHYSICS.bondIterations : 1
  for (let it = 0; it < iterations; it++) {
    for (const p of grounded) {
      if (p.vel[1] >= 0) continue
      // The ground takes what presses down, and its friction what slides, up to its share of that.
      const m = p.mass
      const press = -p.vel[1]
      walls[1] += m * press
      p.vel[1] = 0
      const slide = Math.hypot(p.vel[0], p.vel[2])
      if (slide <= 0) continue
      const stop = Math.min(1, (PHYSICS.friction * press) / slide)
      for (const k of [0, 2]) {
        walls[k] -= m * p.vel[k] * stop
        p.vel[k] -= p.vel[k] * stop
      }
    }
    for (const bond of bonds) {
      if (bond.rest < 0) continue
      const { a, b } = bond
      const dx = a.pos[0] - b.pos[0]
      const dy = a.pos[1] - b.pos[1]
      const dz = a.pos[2] - b.pos[2]
      const r = Math.sqrt(dx * dx + dy * dy + dz * dz)
      if (r < 1e-9) continue
      const off = r - bond.rest
      if (Math.abs(off) > PHYSICS.bondBreak * bond.rest || total(a.carried) <= PHYSICS.epsilon || total(b.carried) <= PHYSICS.epsilon) {
        bond.rest = -1
        continue
      }
      const Ma = a.mass
      const Mb = b.mass
      const mu = (Ma * Mb) / (Ma + Mb)
      const nx = dx / r
      const ny = dy / r
      const nz = dz / r
      const closing = (a.vel[0] - b.vel[0]) * nx + (a.vel[1] - b.vel[1]) * ny + (a.vel[2] - b.vel[2]) * nz
      const j = -mu * (closing + fix * off)
      if (Math.abs(j) > strength * mu) {
        bond.rest = -1
        continue
      }
      a.vel[0] += (j * nx) / Ma
      a.vel[1] += (j * ny) / Ma
      a.vel[2] += (j * nz) / Ma
      b.vel[0] -= (j * nx) / Mb
      b.vel[1] -= (j * ny) / Mb
      b.vel[2] -= (j * nz) / Mb
    }
  }
}

/** A particle and the air it's in pull each other's speeds together: what one loses, the other gains. */
function drag(world: World, ps: Particle[], dt: number) {
  const v = world.airVel
  for (const p of ps) {
    const m = p.mass
    const c = world.cellOf(p.pos)
    if (m <= 0 || c < 0) continue
    const M = total(world.air[c])
    if (M <= 0) continue
    const rate = (PHYSICS.airDrag * M) / PHYSICS.airMana
    const mu = (m * M) / (m + M)
    const k = mu * (1 - Math.exp(-rate * (1 + m / M) * dt))
    for (let i = 0; i < 3; i++) {
      const j = k * (p.vel[i] - v[c * 3 + i])
      p.vel[i] -= j / m
      v[c * 3 + i] += j / M
    }
  }
}

/**
 * Each particle moves, one axis at a time, so it can't slip through a corner. It stops dead at the edge of the world, and
 * against matter: free mana against solid matter, and mana holding matter against a cell with no room for what it holds
 * (matter blocks matter). It strikes the bodies it runs into, sharing its speed with them.
 */
function move(world: World, ps: Particle[], dt: number, hooks: FluidHooks, blocked: Blocked) {
  for (const p of ps) {
    const m = p.mass
    for (let k = 0; k < 3; k++) {
      if (k === 2 && world.d === 1) {
        world.impulse.walls[2] -= m * p.vel[2]
        p.vel[2] = 0
        continue
      }
      if (!p.vel[k]) continue
      const next: Vec = [...p.pos]
      next[k] += p.vel[k] * dt
      const from = world.cellOf(p.pos)
      const to = world.cellOf(next)
      if (blocked(p, from, to, k === 1 && p.vel[1] < 0)) {
        world.impulse.walls[k] -= m * p.vel[k]
        p.vel[k] = 0
      } else p.pos = next
    }
    const body = world.bodyAt(p.pos, hooks.passes?.(p))
    if (body && m > 0) {
      // A body only slides along the ground: it takes the mana's push across, and the ground takes the rest.
      const mu = (m * body.mass) / (m + body.mass)
      for (const k of [0, 2]) {
        const j = mu * (p.vel[k] - body.vel[k])
        p.vel[k] -= j / m
        body.vel[k] += j / body.mass
      }
    }
  }
}

/**
 * The air carries its speed to its neighbours, and the ground, walls and the world's edge hold still the air against
 * them. It doesn't carry itself along yet, and isn't kept from piling up: a wake spreads where it was made.
 */
export function airFlow(world: World) {
  const v = world.airVel
  const k = PHYSICS.airViscosity
  const still = (a: number, M: number) => {
    for (let i = 0; i < 3; i++) {
      const j = k * M * v[a * 3 + i]
      v[a * 3 + i] -= j / M
      world.impulse.walls[i] -= j
    }
  }
  for (let a = 0; a < world.size; a++) {
    const Ma = total(world.air[a])
    if (Ma <= 0) continue
    const moving = v[a * 3] || v[a * 3 + 1] || v[a * 3 + 2]
    const [x, y, z] = world.coords(a)
    const around = [
      [x + 1, y, z],
      [x, y + 1, z],
      [x, y, z + 1],
    ]
    for (const [i, j, l] of around) {
      if (l === z + 1 && world.d === 1) continue
      const b = world.index(i, j, l)
      const Mb = b < 0 ? 0 : total(world.air[b])
      if (b < 0 || Mb <= 0 || world.solidAt(b)) {
        if (moving) still(a, Ma)
        continue
      }
      if (!moving && !(v[b * 3] || v[b * 3 + 1] || v[b * 3 + 2])) continue
      const mu = (Ma * Mb) / (Ma + Mb)
      for (let c = 0; c < 3; c++) {
        const jx = k * mu * (v[a * 3 + c] - v[b * 3 + c])
        v[a * 3 + c] -= jx / Ma
        v[b * 3 + c] += jx / Mb
      }
    }
    // The edges below, behind and to the side, which the loop above doesn't reach from this cell.
    for (const [i, j, l] of [
      [x - 1, y, z],
      [x, y - 1, z],
      [x, y, z - 1],
    ]) {
      if (l === z - 1 && world.d === 1) continue
      const b = world.index(i, j, l)
      if (moving && (b < 0 || total(world.air[b]) <= 0 || world.solidAt(b))) still(a, Ma)
    }
  }
}
