// The world: a grid of cells, each with free air mana and condensed matter. A 2D world is one cell deep.

import { PHYSICS } from './physics.ts'
import { add, dominant, share, take, total, zero, type Parts } from './parts.ts'
import type { Program } from '../asm/assembler.ts'

export type Vec = [number, number, number]

/** A body in the world: a caster, or something to push around. Its position is its centre. */
export type Body = {
  id: number
  name: string
  pos: Vec
  vel: Vec
  mass: number
  /** Half its size in each axis, in metres. */
  half: Vec
}

/** An order ingrained in a particle: the routine at `addr` of the spell that ingrained it. */
export type Ingrained = { program: Program; addr: number }

/**
 * A particle of free mana: one simulated particle stands for a crowd of real ones (SPEC §11). It's in a weave, or loose
 * (weave 0): sent, spent, or let go.
 */
export type Particle = {
  id: number
  pos: Vec
  vel: Vec
  free: Parts
  /** Matter it holds bound, or condensed itself. Bound matter moves with its mana and is held up by it. */
  carried: Parts
  weave: number
  order: Ingrained | null
  /** How much more pushes can change its velocity this tick, m/tick. */
  dvLeft: number
  /** The tick it was last pushed, by its caster or its order. */
  pushedAt: number
  /** How dense the mana around it is (M/m³), which way it thickens, and how its neighbours move: what it can feel. */
  rho: number
  grad: Vec
  nvel: Vec
  /** How dense the matter held around it is: its mass per m³. */
  rhoM: number
  /** The force on it this step. */
  acc: Vec
  /** Its mass, as of this step (massOf). */
  mass: number
}

/** Two particles of rock held together (PHYSICS.bondRange): a spring `rest` metres long. */
export type Bond = { a: Particle; b: Particle; rest: number }

/**
 * A particle's mass: its free mana, and the matter it holds (PHYSICS.matterMass). Pushing it, the air dragging it,
 * and its weight all go by this.
 */
export function massOf(p: Particle): number {
  const m = PHYSICS.matterMass
  return p.free[0] + p.free[1] + p.free[2] + p.free[3] + p.carried[0] * m[0] + p.carried[1] * m[1] + p.carried[2] * m[2] + p.carried[3] * m[3]
}

/**
 * Momentum given to the world from outside it: pushes, orders' kicks, weight, the ground and walls, what gathering takes
 * out of the air, and matter changing its mass: condensing (1 M of matter weighs less than 1 M of free mana), and matter
 * let go of, which stops dead in the ground.
 */
export type Impulses = Record<'push' | 'kick' | 'gravity' | 'walls' | 'gather' | 'matter', Vec>

export const FIRE = 0
export const WATER = 1
export const AIR = 2
export const EARTH = 3

export class World {
  readonly cell = PHYSICS.cell
  readonly size: number
  /** Free mana floating in each cell. */
  readonly air: Parts[]
  /** Condensed mana in each cell that no weave holds. */
  readonly matter: Parts[]
  /** How the air in each cell moves: x, y, z, cell by cell. */
  readonly airVel: Float64Array
  particles: Particle[] = []
  bodies: Body[] = []
  tick = 0
  impulse: Impulses = { push: [0, 0, 0], kick: [0, 0, 0], gravity: [0, 0, 0], walls: [0, 0, 0], gather: [0, 0, 0], matter: [0, 0, 0] }
  /** Rock: pairs of particles bound together. */
  bonds: Bond[] = []
  private nextBody = 1
  private nextParticle = 1

  constructor(
    readonly w: number,
    readonly h: number,
    readonly d = 1,
  ) {
    this.size = w * h * d
    this.air = Array.from({ length: this.size }, () => zero())
    this.matter = Array.from({ length: this.size }, () => zero())
    this.airVel = new Float64Array(this.size * 3)
  }

  index(x: number, y: number, z: number): number {
    if (x < 0 || y < 0 || z < 0 || x >= this.w || y >= this.h || z >= this.d) return -1
    return (z * this.h + y) * this.w + x
  }

  coords(i: number): Vec {
    const x = i % this.w
    const y = Math.floor(i / this.w) % this.h
    const z = Math.floor(i / (this.w * this.h))
    return [x, y, z]
  }

  /** The cell a point is in, or -1 outside the world. */
  cellOf(p: Vec): number {
    const c = this.cell
    return this.index(Math.floor(p[0] / c + 1e-7), Math.floor(p[1] / c + 1e-7), Math.floor(p[2] / c + 1e-7))
  }

  /** The centre of a cell, in metres. */
  centre(x: number, y: number, z = 0): Vec {
    return [(x + 0.5) * this.cell, (y + 0.5) * this.cell, (z + 0.5) * this.cell]
  }

  /** The nearest cell to a point, inside the world. */
  clampedCellOf(p: Vec): number {
    const c = this.cell
    const clamp = (v: number, n: number) => Math.max(0, Math.min(n - 1, Math.floor(v / c + 1e-7)))
    return this.index(clamp(p[0], this.w), clamp(p[1], this.h), clamp(p[2], this.d))
  }

  /** Unbound earth and water: what blocks and what counts as a touch. */
  solidAt(i: number): boolean {
    if (i < 0) return true
    const m = this.matter[i]
    return m[EARTH] + m[WATER] >= PHYSICS.solid
  }

  private earthAt(i: number): boolean {
    return i >= 0 && this.matter[i][EARTH] >= PHYSICS.solid
  }

  // Building worlds

  /** Fills every cell below `top` (in cells) with matter, and the open air above with air mana. */
  static withGround(w: number, h: number, d: number, top: number, part = EARTH): World {
    const world = new World(w, h, d)
    for (let i = 0; i < world.size; i++) {
      const [, y] = world.coords(i)
      if (y < top) world.matter[i][part] = PHYSICS.cellMatter
      else world.air[i].fill(PHYSICS.airMana / 4)
    }
    return world
  }

  /** Fills a box of cells (inclusive) with matter, where the air mana there was. For building a world, before it runs. */
  fillBox(from: Vec, to: Vec, part = EARTH, amount = PHYSICS.cellMatter) {
    for (let z = from[2]; z <= to[2]; z++)
      for (let y = from[1]; y <= to[1]; y++)
        for (let x = from[0]; x <= to[0]; x++) {
          const i = this.index(x, y, z)
          if (i < 0) continue
          this.matter[i][part] += amount
          this.air[i] = zero()
        }
  }

  /** A body's mass is in M, like mana's: a person is about 60. */
  addBody(name: string, pos: Vec, mass = 60, half: Vec = [0.25, 0.9, 0.25]): Body {
    const body: Body = { id: this.nextBody++, name, pos: [...pos], vel: [0, 0, 0], mass, half }
    this.bodies.push(body)
    return body
  }

  /** The body whose box holds a point, if any. */
  bodyAt(p: Vec, except?: Body): Body | undefined {
    const c = this.cell / 2
    return this.bodies.find(
      (b) =>
        b !== except &&
        Math.abs(p[0] - b.pos[0]) <= b.half[0] + c &&
        Math.abs(p[1] - b.pos[1]) <= b.half[1] + c &&
        (this.d === 1 || Math.abs(p[2] - b.pos[2]) <= b.half[2] + c),
    )
  }

  // Particles and air

  /** New particles of free mana at a point, at most a mote each, spread over `spread` metres so their pressure can act. */
  pour(parts: Parts, at: Vec, weave: number, vel: Vec = [0, 0, 0], spread = PHYSICS.pour): Particle[] {
    const out: Particle[] = []
    const n = Math.ceil(total(parts) / PHYSICS.mote - 1e-9)
    if (n <= 0) return out
    for (let k = 0; k < n; k++) {
      const id = this.nextParticle++
      // A fixed scatter by id, so the same spell always lays out the same way.
      const u = frac(id * 0.7548776662) - 0.5
      const v = frac(id * 0.5698402910) - 0.5
      const w = frac(id * 0.3141592653) - 0.5
      // Within the cell the point is in: mana poured at a point is poured into that cell.
      const inCell = (x: number, d: number) => {
        const lo = Math.floor(x / this.cell + 1e-7) * this.cell
        return Math.min(lo + this.cell * 0.999, Math.max(lo + this.cell * 0.001, x + d))
      }
      const pos: Vec = [inCell(at[0], u * 2 * spread), inCell(at[1], v * 2 * spread), this.d === 1 ? at[2] : inCell(at[2], w * 2 * spread)]
      const p: Particle = {
        id,
        pos,
        vel: [...vel],
        free: parts.map((x) => x / n) as Parts,
        carried: zero(),
        weave,
        order: null,
        dvLeft: PHYSICS.pushRate,
        pushedAt: -1,
        rho: 0,
        grad: [0, 0, 0],
        nvel: [0, 0, 0],
        rhoM: 0,
        acc: [0, 0, 0],
        mass: 0,
      }
      this.particles.push(p)
      out.push(p)
    }
    parts.fill(0)
    return out
  }

  /** Mana let into the air of a cell, carrying momentum (px, py, pz) into it. */
  addAir(i: number, parts: Parts, momentum: Vec = [0, 0, 0]) {
    if (i < 0) return
    const before = total(this.air[i])
    const after = before + total(parts)
    if (after <= 0) return
    for (let k = 0; k < 3; k++) this.airVel[i * 3 + k] = (before * this.airVel[i * 3 + k] + momentum[k]) / after
    add(this.air[i], parts)
  }

  /** Mana drawn out of the air of a cell. What it takes leaves with its share of the air's momentum. */
  takeAir(i: number, f: number): Parts {
    const got = share(this.air[i], f)
    for (let k = 0; k < 3; k++) this.impulse.gather[k] -= total(got) * this.airVel[i * 3 + k]
    return got
  }

  // Each tick

  /** Air mana evens out between neighbouring cells, and what moves carries its momentum. */
  diffuseAir() {
    const rate = PHYSICS.airDiffusion / 2
    const v = this.airVel
    for (let i = 0; i < this.size; i++) {
      const [x, y, z] = this.coords(i)
      for (const j of [this.index(x + 1, y, z), this.index(x, y + 1, z), this.index(x, y, z + 1)]) {
        if (j < 0) continue
        const a = this.air[i]
        const b = this.air[j]
        const ta = total(a)
        const tb = total(b)
        let moved = 0
        for (let k = 0; k < 4; k++) {
          const f = (a[k] - b[k]) * rate
          a[k] -= f
          b[k] += f
          moved += f
        }
        if (Math.abs(moved) < 1e-12) continue
        // `moved` M went from i to j (or back, if negative), with the speed of where it came from.
        const from = moved > 0 ? i : j
        const na = ta - moved
        const nb = tb + moved
        for (let k = 0; k < 3; k++) {
          const p = moved * v[from * 3 + k]
          const pa = ta * v[i * 3 + k] - p
          const pb = tb * v[j * 3 + k] + p
          v[i * 3 + k] = na > 1e-12 ? pa / na : 0
          v[j * 3 + k] = nb > 1e-12 ? pb / nb : 0
        }
      }
    }
  }

  /** Bodies pushed by mana slide along the ground until friction stops them. */
  moveBodies() {
    for (const b of this.bodies) {
      if (Math.abs(b.vel[0]) + Math.abs(b.vel[2]) < 1e-6) {
        this.impulse.walls[0] -= b.mass * b.vel[0]
        this.impulse.walls[2] -= b.mass * b.vel[2]
        b.vel = [0, 0, 0]
        continue
      }
      const next: Vec = [b.pos[0] + b.vel[0], b.pos[1], b.pos[2] + b.vel[2]]
      const i = this.cellOf(next)
      const before: Vec = [...b.vel]
      if (i >= 0 && !this.solidAt(i)) b.pos = next
      else b.vel = [0, 0, 0]
      b.vel[0] *= PHYSICS.bodyFriction
      b.vel[2] *= PHYSICS.bodyFriction
      for (const k of [0, 2]) this.impulse.walls[k] += b.mass * (b.vel[k] - before[k])
    }
  }

  /**
   * Matter follows its nature: earth falls and piles, water falls and spreads, air and flame rise.
   * `carried` is the matter weaves hold in each cell: it takes room too.
   */
  settleMatter(carried: Float64Array) {
    const moved = new Uint8Array(this.size)
    const room = (j: number) => PHYSICS.cellMatter - total(this.matter[j]) - carried[j]
    const flip = this.tick % 2 === 0 ? 1 : -1
    const sides: [number, number][] = this.d > 1 ? [[flip, 0], [-flip, 0], [0, flip], [0, -flip]] : [[flip, 0], [-flip, 0]]

    const tryMove = (i: number, j: number, amount: number): number => {
      if (j < 0) return 0
      const r = room(j)
      if (r <= PHYSICS.epsilon) return 0
      const moving = take(this.matter[i], Math.min(amount, r))
      add(this.matter[j], moving)
      moved[j] = 1
      return total(moving)
    }

    for (let y = 0; y < this.h; y++)
      for (let z = 0; z < this.d; z++)
        for (let x = 0; x < this.w; x++) {
          const i = this.index(x, y, z)
          const m = this.matter[i]
          if (moved[i] || total(m) <= PHYSICS.epsilon) continue
          const kind = dominant(m)
          // Earth holds together: a solid cell with solid earth beside it stays, like ground over a trench.
          if (kind === EARTH && this.solidAt(i) && sides.some(([sx, sz]) => this.earthAt(this.index(x + sx, y, z + sz)))) continue
          const falls = kind === EARTH || kind === WATER
          const dy = falls ? -1 : 1
          const here = () => total(m)
          tryMove(i, this.index(x, y + dy, z), here())
          for (const [sx, sz] of sides) {
            if (here() <= PHYSICS.epsilon) break
            tryMove(i, this.index(x + sx, y + dy, z + sz), here())
          }
          if (here() <= PHYSICS.epsilon) continue
          // What couldn't fall or rise: water and air spread to the sides, flame thins into warmth.
          if (kind === WATER || kind === AIR) {
            for (const [sx, sz] of sides) {
              const j = this.index(x + sx, y, z + sz)
              if (j >= 0 && total(this.matter[j]) + carried[j] < here()) tryMove(i, j, (here() - total(this.matter[j])) / 2)
            }
          } else if (kind === FIRE) {
            for (const [sx, sz] of sides) {
              const j = this.index(x + sx, y, z + sz)
              if (j < 0 || room(j) <= 0) continue
              add(this.matter[j], share(m, PHYSICS.fireSpread / sides.length))
              moved[j] = 1
            }
          }
        }
  }

  /** All the mana in the world that isn't in a caster, by where it is. Particles in weaves are counted by the machine. */
  mana(): { air: number; matter: number; loose: number } {
    let air = 0
    let matter = 0
    for (let i = 0; i < this.size; i++) {
      air += total(this.air[i])
      matter += total(this.matter[i])
    }
    let loose = 0
    for (const p of this.particles) if (!p.weave) loose += total(p.free) + total(p.carried)
    return { air, matter, loose }
  }

  /** Momentum in the world (particles, air, bodies) less what came from outside: zero, when nothing is lost. */
  momentumError(): number {
    const m: Vec = [0, 0, 0]
    for (const p of this.particles) {
      const mass = massOf(p)
      for (let k = 0; k < 3; k++) m[k] += mass * p.vel[k]
    }
    for (let i = 0; i < this.size; i++) {
      const mass = total(this.air[i])
      if (mass) for (let k = 0; k < 3; k++) m[k] += mass * this.airVel[i * 3 + k]
    }
    for (const b of this.bodies) for (let k = 0; k < 3; k++) m[k] += b.mass * b.vel[k]
    for (const j of Object.values(this.impulse)) for (let k = 0; k < 3; k++) m[k] -= j[k]
    return Math.hypot(...m)
  }
}

function frac(x: number): number {
  return x - Math.floor(x)
}
