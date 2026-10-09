// The air (SPEC §11): the free mana spread through the world's cells, as a gas. It presses from dense to thin, carries
// itself along (and its mana, and its momentum, with it), flows around what's solid, and is held still against the
// ground and the world's edge. Its pressure is its density times the square of its speed of sound (PHYSICS.airSound):
// fast enough beside the winds in it that it flows around things, as air does, rather than piling up against them.
//
// It's a finite-volume scheme on the grid. Between every two open neighbouring cells, each step, mana and momentum flow
// from the one upwind, and the pressure on the face between them pushes them apart, equally and oppositely. Against a
// solid cell or the world's edge, the face is closed: nothing flows through it, and what the air pushes on it is
// momentum given to the world from outside (`impulse.walls`).

import { PHYSICS } from './physics.ts'
import { total } from './parts.ts'
import type { World } from './world.ts'

/** The most of a cell's mana that may leave it through one face in one step. */
const MOST = 0.15
/** Slower than this (m/tick), air stops; and within this share of still air's density, stopped air is at rest. */
const STOP = 5e-4
const CALM = 5e-3

/** One tick of the air: it flows, presses, and evens out its speed with its neighbours. */
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
  const k2 = PHYSICS.airSound ** 2
  // Pressure is measured from the air's own, all through the world: how dense it is on average. A pressure the same all
  // round pushes nothing anywhere: inside the air its pushes cancel, and on any closed surface (the ground, a pillar, the
  // world's edge) they add up to nothing. So air at rest, as dense as the rest of it, needs no work, and is skipped.
  let sum = 0
  let cells = 0
  for (let i = 0; i < n; i++)
    if (open[i]) {
      sum += total(air[i])
      cells++
    }
  const usual = cells ? sum / cells : 0
  const still = (k2 * usual) / V
  const quiet = (i: number) => {
    const a = air[i]
    const m = a[0] + a[1] + a[2] + a[3]
    return v[i * 3] === 0 && v[i * 3 + 1] === 0 && v[i * 3 + 2] === 0 && Math.abs(m - usual) <= CALM * usual
  }
  // Air that has all but stopped stops: what little motion it had goes into the ground, as the air's thickness would
  // have taken it there anyway.
  for (let i = 0; i < n; i++) {
    if (!open[i]) continue
    const s2 = v[i * 3] ** 2 + v[i * 3 + 1] ** 2 + v[i * 3 + 2] ** 2
    if (s2 === 0 || s2 > STOP * STOP) continue
    const m = total(air[i])
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
      M[i] = m
      P[i * 3] = m * v[i * 3]
      P[i * 3 + 1] = m * v[i * 3 + 1]
      P[i * 3 + 2] = m * v[i * 3 + 2]
      press[i] = m > 0 ? (k2 * m) / V - still : -still
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
      let m = 0
      for (let k = 0; k < 4; k++) {
        parts[k] = Math.max(0, parts[k] + flow[i * 4 + k])
        m += parts[k]
      }
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

/** How hard a cell's air presses for its density: pressure = this × density. */
function stiffness(world: World, i: number): number {
  return total(world.air[i]) > 0 ? PHYSICS.airSound ** 2 : 0
}

/**
 * The air carries its speed to its neighbours (it's thick), and the ground, walls and the world's edge hold still the air
 * against them.
 */
function viscosity(world: World, open: Uint8Array, next: Int32Array, prev: Int32Array, axes: number) {
  const v = world.airVel
  const k = PHYSICS.airViscosity
  const walls = world.impulse.walls
  const still = (a: number, M: number) => {
    for (let i = 0; i < 3; i++) {
      const j = k * M * v[a * 3 + i]
      v[a * 3 + i] -= j / M
      walls[i] -= j
    }
  }
  const moving = (i: number) => v[i * 3] !== 0 || v[i * 3 + 1] !== 0 || v[i * 3 + 2] !== 0
  for (let a = 0; a < world.size; a++) {
    if (!open[a]) continue
    const Ma = total(world.air[a])
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
      const Mb = b < 0 ? 0 : total(world.air[b])
      if (Mb <= 0) {
        still(a, Ma)
        continue
      }
      const mu = (Ma * Mb) / (Ma + Mb)
      for (let c = 0; c < 3; c++) {
        const j = k * mu * (v[a * 3 + c] - v[b * 3 + c])
        v[a * 3 + c] -= j / Ma
        v[b * 3 + c] += j / Mb
      }
    }
    // The faces below, behind and to the side, which the loop above doesn't reach from this cell.
    for (let ax = 0; ax < axes; ax++) {
      const b = prev[a * 3 + ax]
      if (b < 0 || total(world.air[b]) <= 0) still(a, Ma)
    }
  }
}
