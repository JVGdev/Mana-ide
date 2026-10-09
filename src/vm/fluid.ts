// Mana as a fluid (SPEC §11): free mana is particles that press on their neighbours, drag the air they move through and
// are dragged by it, stop against matter, and shove bodies they run into.
//
// It's smoothed-particle hydrodynamics. Each particle feels its neighbours within PHYSICS.smoothing metres. Every force
// between two things in the world is equal and opposite (particle and particle, particle and air, air and air, particle
// and body), so the world's momentum only changes by what comes from outside, and `World.momentumError` checks it.

import { PHYSICS } from './physics.ts'
import { total } from './parts.ts'
import type { Body, Particle, Vec, World } from './world.ts'

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

type Kernels = { poly6: number; grad: number; spiky: number; lap: number }

function kernels(dims: 2 | 3): Kernels {
  const h = PHYSICS.smoothing
  return dims === 2
    ? { poly6: 4 / (Math.PI * h ** 8), grad: -24 / (Math.PI * h ** 8), spiky: -30 / (Math.PI * h ** 5), lap: 40 / (Math.PI * h ** 5) }
    : {
        poly6: 315 / (64 * Math.PI * h ** 9),
        grad: -945 / (32 * Math.PI * h ** 9),
        spiky: -45 / (Math.PI * h ** 6),
        lap: 45 / (Math.PI * h ** 6),
      }
}

/** The particle's mass: its free mana. The matter it holds is held up by that mana, and weighs nothing to it. */
const mass = (p: Particle) => total(p.free)

/** A blend of a per-part property by how much of each part a particle's mana is. */
function blend(p: Particle, by: number[]): number {
  const m = mass(p)
  if (m <= 0) return 0
  return (p.free[0] * by[0] + p.free[1] * by[1] + p.free[2] * by[2] + p.free[3] * by[3]) / m
}

const B = 1 << 12
const key = (x: number, y: number, z: number) => (x + 2048) + (y + 2048) * B + (z + 2048) * B * B

/** Every pair of particles closer than the smoothing length, once each. */
function pairs(world: World, ps: Particle[], f: (a: Particle, b: Particle, d: Vec, r: number) => void) {
  const h = PHYSICS.smoothing
  const grid = new Map<number, Particle[]>()
  const cellOf = (p: Particle) => [Math.floor(p.pos[0] / h), Math.floor(p.pos[1] / h), world.d === 1 ? 0 : Math.floor(p.pos[2] / h)]
  for (const p of ps) {
    const [x, y, z] = cellOf(p)
    const k = key(x, y, z)
    let list = grid.get(k)
    if (!list) grid.set(k, (list = []))
    list.push(p)
  }
  const zs = world.d === 1 ? [0] : [-1, 0, 1]
  for (const a of ps) {
    const [x, y, z] = cellOf(a)
    for (const dz of zs)
      for (let dy = -1; dy <= 1; dy++)
        for (let dx = -1; dx <= 1; dx++) {
          const list = grid.get(key(x + dx, y + dy, z + dz))
          if (!list) continue
          for (const b of list) {
            if (b.id <= a.id) continue
            const d: Vec = [a.pos[0] - b.pos[0], a.pos[1] - b.pos[1], world.d === 1 ? 0 : a.pos[2] - b.pos[2]]
            const r2 = d[0] * d[0] + d[1] * d[1] + d[2] * d[2]
            if (r2 >= h * h) continue
            f(a, b, d, Math.sqrt(r2))
          }
        }
  }
}

/** What each particle feels: how dense the mana around it is, which way it thickens, and how its neighbours move. */
export function feel(world: World, ps: Particle[] = world.particles) {
  const h = PHYSICS.smoothing
  const K = kernels(world.d === 1 ? 2 : 3)
  // In 2D the world is one cell deep: density per square metre, over that depth, is density per cubic metre.
  const depth = world.d === 1 ? world.cell : 1
  const weight = new Map<Particle, number>()
  for (const p of ps) {
    p.rho = mass(p) * K.poly6 * h ** 6
    p.grad = [0, 0, 0]
    p.nvel = [0, 0, 0]
    weight.set(p, 0)
  }
  pairs(world, ps, (a, b, d, r) => {
    const q = h * h - r * r
    const w = K.poly6 * q ** 3
    const ma = mass(a)
    const mb = mass(b)
    a.rho += mb * w
    b.rho += ma * w
    const g = K.grad * q * q
    for (let k = 0; k < 3; k++) {
      a.grad[k] += mb * g * d[k]
      b.grad[k] -= ma * g * d[k]
      a.nvel[k] += mb * w * b.vel[k]
      b.nvel[k] += ma * w * a.vel[k]
    }
    weight.set(a, weight.get(a)! + mb * w)
    weight.set(b, weight.get(b)! + ma * w)
  })
  for (const p of ps) {
    const w = weight.get(p)!
    p.rho /= depth
    for (let k = 0; k < 3; k++) {
      p.grad[k] /= depth
      p.nvel[k] = w > 0 ? p.nvel[k] / w : p.vel[k]
    }
  }
}

/** One tick of the fluid: pressure, viscosity, rising, the air, and what the particles run into. */
export function stepFluid(world: World, hooks: FluidHooks = {}) {
  const ps = hooks.still ? world.particles.filter((p) => !hooks.still!(p)) : world.particles
  if (!ps.length) return
  const dt = 1 / PHYSICS.substeps
  for (let s = 0; s < PHYSICS.substeps; s++) {
    feel(world, ps)
    forces(world, ps, dt)
    drag(world, ps, dt)
    move(world, ps, dt, hooks)
  }
  feel(world, ps)
}

function forces(world: World, ps: Particle[], dt: number) {
  const K = kernels(world.d === 1 ? 2 : 3)
  const h = PHYSICS.smoothing
  const depth = world.d === 1 ? world.cell : 1
  for (const p of ps) {
    p.acc = [0, blend(p, PHYSICS.rise), 0]
    world.impulse.rise[1] += mass(p) * p.acc[1] * dt
  }
  pairs(world, ps, (a, b, d, r) => {
    if (r < 1e-9) return
    const ma = mass(a)
    const mb = mass(b)
    if (ma <= 0 || mb <= 0) return
    // Pressure: equal and opposite. With pressure = k·ρ, each side's term is k/ρ.
    const pa = blend(a, PHYSICS.stiffness) / (a.rho * depth)
    const pb = blend(b, PHYSICS.stiffness) / (b.rho * depth)
    const f = -(pa + pb) * K.spiky * (h - r) ** 2 // force per (m_a·m_b), along a − b
    const visc = (((blend(a, PHYSICS.viscosity) + blend(b, PHYSICS.viscosity)) / 2) * K.lap * (h - r) * 2) / ((a.rho + b.rho) * depth)
    for (let k = 0; k < 3; k++) {
      const fk = (f * d[k]) / r + visc * (b.vel[k] - a.vel[k])
      a.acc[k] += fk * mb
      b.acc[k] -= fk * ma
    }
  })
  for (const p of ps) for (let k = 0; k < 3; k++) p.vel[k] += p.acc[k] * dt
}

/** A particle and the air it's in pull each other's speeds together: what one loses, the other gains. */
function drag(world: World, ps: Particle[], dt: number) {
  const v = world.airVel
  for (const p of ps) {
    const m = mass(p)
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
function move(world: World, ps: Particle[], dt: number, hooks: FluidHooks) {
  const blocked = (p: Particle, from: number, to: number) => {
    if (to < 0) return true
    if (to === from) return false
    const load = total(p.carried)
    if (load <= PHYSICS.epsilon) return world.solidAt(to)
    return PHYSICS.cellMatter - total(world.matter[to]) - (hooks.othersCarried?.(p, to) ?? 0) < load
  }
  for (const p of ps) {
    const m = mass(p)
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
      if (blocked(p, from, to)) {
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
