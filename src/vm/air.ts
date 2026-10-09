// The air (SPEC §11): the free mana spread through the world's cells, as a gas. It presses from dense to thin, carries
// itself along (and its mana, and its momentum, with it), flows around what's solid, and is held still against the
// ground and the world's edge. Its pressure is its density in M times its gas constant, which is the square of its speed
// of sound (PHYSICS.airSound) times what a M of it weighs (RAW_MASS): the ideal gas law. That speed is fast enough beside
// the winds in it that it flows around things, as air does, rather than piling up against them. It
// weighs, so at rest it's thicker low down than high up: its own weight presses it down, and its pressure holds it up.
// That pressure, thicker below than above, is what holds up a parcel of mana lighter than the air around it.
//
// It's a finite-volume scheme on the grid. Between every two open neighbouring cells, each step, mana and momentum flow
// from the one upwind, and the pressure on the face between them pushes them apart, equally and oppositely. Against a
// solid cell or the world's edge, the face is closed: nothing flows through it, and what the air pushes on it is
// momentum given to the world from outside (`impulse.walls`).

import { PHYSICS } from './physics.ts'
import { total } from './parts.ts'
import { massOfParts, type World } from './world.ts'

/** The most of a cell's mana that may leave it through one face in one step. */
const MOST = 0.15
/** Slower than this (m/tick), air stops; and within this share of still air's density, stopped air is at rest. */
const STOP = 5e-4
const CALM = 5e-3

/** The mass of 1 M of raw mana: what the air weighs for each M of it, before anything's taken out or let into it. */
export const RAW_MASS = () => PHYSICS.manaMass.reduce((a, b) => a + b, 0) / 4

/** The air's gas constant: how hard it presses for each M a m³ of it, p = this × ρ. */
export const AIR_GAS = () => PHYSICS.airSound ** 2 * RAW_MASS()

/**
 * How high the air at rest thins by a factor of e, metres: its gas constant over what a M of it weighs, times g. Air at
 * rest is as thick, at height y, as e^(−y/H) times what it is at the ground.
 */
export function scaleHeight(): number {
  return AIR_GAS() / (PHYSICS.gravity * RAW_MASS())
}

/**
 * The air at rest, cell by cell: as much mana as the air has now, laid out as thick at each height as it would settle
 * (M in each cell). It's what the air's pressure and weight are measured from: air at rest, at rest's thickness,
 * presses and weighs nothing that the ground doesn't already hold up, and needs no work.
 */
function atRest(world: World, open: Uint8Array): Float64Array {
  const H = scaleHeight()
  const rows = new Float64Array(world.h)
  for (let y = 0; y < world.h; y++) rows[y] = Math.exp(-((y + 0.5) * world.cell) / H)
  let mana = 0
  let shape = 0
  for (let i = 0; i < world.size; i++)
    if (open[i]) {
      mana += total(world.air[i])
      shape += rows[Math.floor(i / world.w) % world.h]
    }
  const out = new Float64Array(world.size)
  if (shape <= 0) return out
  const S = mana / shape
  for (let i = 0; i < world.size; i++) if (open[i]) out[i] = S * rows[Math.floor(i / world.w) % world.h]
  return out
}

/** One tick of the air: it flows, presses, falls under its weight, and evens out its speed with its neighbours. */
export function stepAir(world: World) {
  const n = world.size
  const dx = world.cell
  const A = dx * dx
  const V = dx * dx * dx
  const axes = world.d === 1 ? 2 : 3
  const { open, next, prev } = layout(world)
  const v = world.airVel
  const air = world.air
  const walls = world.impulse.walls
  const k2 = AIR_GAS()
  const g = PHYSICS.gravity
  const raw = RAW_MASS()
  // Pressure and weight are measured from the air at rest (atRest): the pressure that holds it up there, and the weight
  // it holds up, cancel, and the ground takes the rest. So air at rest, as thick as rest would have it, and as heavy,
  // needs no work, and is skipped. Only what's different from rest pushes or falls.
  const rest = atRest(world, open)
  const quiet = (i: number) => {
    const a = air[i]
    const m = a[0] + a[1] + a[2] + a[3]
    const r = rest[i]
    return v[i * 3] === 0 && v[i * 3 + 1] === 0 && v[i * 3 + 2] === 0 && Math.abs(m - r) <= CALM * r && Math.abs(massOfParts(a) - r * raw) <= CALM * r * raw
  }
  // Air that has all but stopped stops: what little motion it had goes into the ground, as the air's thickness would
  // have taken it there anyway.
  for (let i = 0; i < n; i++) {
    if (!open[i]) continue
    const s2 = v[i * 3] ** 2 + v[i * 3 + 1] ** 2 + v[i * 3 + 2] ** 2
    if (s2 === 0 || s2 > STOP * STOP) continue
    const m = massOfParts(air[i])
    world.warm('the air', 0.5 * m * s2)
    for (let k = 0; k < 3; k++) {
      walls[k] -= m * v[i * 3 + k]
      v[i * 3 + k] = 0
    }
  }

  // Steps enough that nothing crosses more than a share of a cell in one: the fastest wind or pressure wave sets it.
  let active: number[] = []
  for (let i = 0; i < n; i++) if (open[i] && !quiet(i)) active.push(i)
  if (!active.length) return
  let fastest = 0
  for (const i of active) fastest = Math.max(fastest, Math.hypot(v[i * 3], v[i * 3 + 1], v[i * 3 + 2]) + PHYSICS.airSound)
  const steps = Math.max(1, Math.ceil(fastest / (0.4 * dx)))
  const dt = 1 / steps

  const M = new Float64Array(n)
  const P = new Float64Array(n * 3)
  const sink = new Float64Array(n)
  const press = new Float64Array(n)
  const flow = new Float64Array(n * 4)
  const moved = new Float64Array(n * 3)
  const u = new Float64Array(n * 3)
  const busy = new Uint8Array(n)
  for (let s = 0; s < steps; s++) {
    if (s > 0) {
      active = []
      for (let i = 0; i < n; i++) if (open[i] && !quiet(i)) active.push(i)
      if (!active.length) break
    }
    // The active cells, and their neighbours: everything a face of an active cell touches.
    busy.fill(0)
    for (const i of active) {
      busy[i] = 1
      for (let ax = 0; ax < axes; ax++) {
        if (next[i * 3 + ax] >= 0) busy[next[i * 3 + ax]] = 1
        if (prev[i * 3 + ax] >= 0) busy[prev[i * 3 + ax]] = 1
      }
    }
    const touched: number[] = []
    for (let i = 0; i < n; i++) {
      if (!busy[i]) continue
      touched.push(i)
      const a = air[i]
      const m = a[0] + a[1] + a[2] + a[3]
      const mass = massOfParts(a)
      M[i] = mass
      P[i * 3] = mass * v[i * 3]
      P[i * 3 + 1] = mass * v[i * 3 + 1]
      P[i * 3 + 2] = mass * v[i * 3 + 2]
      press[i] = (k2 * (m - rest[i])) / V
      // How much heavier (or lighter) it is than the air at rest there, which its pressure holds up.
      sink[i] = g * (mass - rest[i] * raw)
      for (let k = 0; k < 4; k++) flow[i * 4 + k] = 0
      moved[i * 3] = moved[i * 3 + 1] = moved[i * 3 + 2] = 0
    }
    // Each face an active cell has, once: (i, j, axis), j = −1 for a closed face on i's + side, and i = −1 for one on
    // j's − side.
    const faces = (visit: (i: number, j: number, ax: number) => void) => {
      for (const i of active)
        for (let ax = 0; ax < axes; ax++) {
          visit(i, next[i * 3 + ax], ax)
          const k = prev[i * 3 + ax]
          // The face on i's − side, unless its other cell is active and does it.
          if (k < 0 || quiet(k)) visit(k, i, ax)
        }
    }
    // First the pressure on every face pushes the air on either side of it away from the other: a closed face, the
    // world's edge or something solid, pushes back with the air's own pressure.
    faces((i, j, ax) => {
      if (i >= 0 && j >= 0) {
        const f = ((press[i] + press[j]) / 2) * A * dt
        P[i * 3 + ax] -= f
        P[j * 3 + ax] += f
      } else if (j < 0) {
        const f = press[i] * A * dt
        P[i * 3 + ax] -= f
        walls[ax] -= f
      } else {
        const f = press[j] * A * dt
        P[j * 3 + ax] += f
        walls[ax] += f
      }
    })
    // And it falls by what it weighs beyond the air at rest there, or rises by what it weighs less.
    for (const i of touched) {
      P[i * 3 + 1] -= sink[i] * dt
      world.impulse.gravity[1] -= sink[i] * dt
    }
    for (const i of touched)
      for (let k = 0; k < 3; k++) u[i * 3 + k] = M[i] > 0 ? P[i * 3 + k] / M[i] : 0
    // Then the air, at the speed it now has, carries its mana and momentum across each open face, from the cell upwind.
    faces((i, j, ax) => {
      if (i < 0 || j < 0) return
      const speed = (u[i * 3 + ax] + u[j * 3 + ax]) / 2
      if (speed === 0) return
      const from = speed > 0 ? i : j
      const to = speed > 0 ? j : i
      if (M[from] <= 0) return
      const share = Math.min(MOST, (Math.abs(speed) * dt) / dx)
      const parts = air[from]
      for (let k = 0; k < 4; k++) {
        const q = parts[k] * share
        flow[from * 4 + k] -= q
        flow[to * 4 + k] += q
      }
      for (let k = 0; k < 3; k++) {
        const q = P[from * 3 + k] * share
        moved[from * 3 + k] -= q
        moved[to * 3 + k] += q
      }
    })
    for (const i of touched) {
      const parts = air[i]
      for (let k = 0; k < 4; k++) parts[k] = Math.max(0, parts[k] + flow[i * 4 + k])
      const m = massOfParts(parts)
      for (let k = 0; k < 3; k++) {
        const p = P[i * 3 + k] + moved[i * 3 + k]
        if (m > 1e-12) v[i * 3 + k] = p / m
        else {
          // A cell emptied keeps no momentum: what it had went with its mana.
          v[i * 3 + k] = 0
          walls[k] -= p
        }
      }
    }
  }
  viscosity(world, open, next, prev, axes)
}

/** Which cells are open to the air, and each one's open neighbour on each side (−1 where the face is closed). */
function layout(world: World) {
  const n = world.size
  const open = new Uint8Array(n)
  for (let i = 0; i < n; i++) open[i] = world.solidAt(i) ? 0 : 1
  const next = new Int32Array(n * 3).fill(-1)
  const prev = new Int32Array(n * 3).fill(-1)
  const { w, h, d } = world
  for (let z = 0; z < d; z++)
    for (let y = 0; y < h; y++)
      for (let x = 0; x < w; x++) {
        const i = x + w * (y + h * z)
        if (!open[i]) continue
        if (x + 1 < w && open[i + 1]) {
          next[i * 3] = i + 1
          prev[(i + 1) * 3] = i
        }
        if (y + 1 < h && open[i + w]) {
          next[i * 3 + 1] = i + w
          prev[(i + w) * 3 + 1] = i
        }
        if (z + 1 < d && open[i + w * h]) {
          next[i * 3 + 2] = i + w * h
          prev[(i + w * h) * 3 + 2] = i
        }
      }
  return { open, next, prev }
}

/**
 * The air carries its speed to its neighbours (it's thick), and the ground, walls and the world's edge hold still the air
 * against them.
 */
function viscosity(world: World, open: Uint8Array, next: Int32Array, prev: Int32Array, axes: number) {
  const v = world.airVel
  const k = PHYSICS.airViscosity
  const walls = world.impulse.walls
  let heat = 0
  // A share k of a speed taken away takes 1 − (1 − k)² of the energy in it.
  const lost = 1 - (1 - k) * (1 - k)
  const still = (a: number, M: number) => {
    heat += 0.5 * M * (v[a * 3] ** 2 + v[a * 3 + 1] ** 2 + v[a * 3 + 2] ** 2) * lost
    for (let i = 0; i < 3; i++) {
      const j = k * M * v[a * 3 + i]
      v[a * 3 + i] -= j / M
      walls[i] -= j
    }
  }
  const moving = (i: number) => v[i * 3] !== 0 || v[i * 3 + 1] !== 0 || v[i * 3 + 2] !== 0
  for (let a = 0; a < world.size; a++) {
    if (!open[a]) continue
    const Ma = world.airMass(a)
    if (Ma <= 0) continue
    // Still air beside still air: nothing to even out.
    let near = moving(a)
    for (let ax = 0; ax < axes && !near; ax++) {
      const b = next[a * 3 + ax]
      if (b >= 0 && moving(b)) near = true
    }
    if (!near) continue
    for (let ax = 0; ax < axes; ax++) {
      const b = next[a * 3 + ax]
      const Mb = b < 0 ? 0 : world.airMass(b)
      if (Mb <= 0) {
        still(a, Ma)
        continue
      }
      const mu = (Ma * Mb) / (Ma + Mb)
      let rel2 = 0
      for (let c = 0; c < 3; c++) {
        const rel = v[a * 3 + c] - v[b * 3 + c]
        rel2 += rel * rel
        const j = k * mu * rel
        v[a * 3 + c] -= j / Ma
        v[b * 3 + c] += j / Mb
      }
      heat += 0.5 * mu * rel2 * lost
    }
    // The faces below, behind and to the side, which the loop above doesn't reach from this cell.
    for (let ax = 0; ax < axes; ax++) {
      const b = prev[a * 3 + ax]
      if (b < 0 || total(world.air[b]) <= 0) still(a, Ma)
    }
  }
  world.warm('the air', heat)
}
