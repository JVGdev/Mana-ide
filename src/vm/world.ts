// The world: a grid of cells, each with free air mana and condensed matter. A 2D world is one cell deep.

import { PHYSICS } from './physics.ts'
import { add, dominant, share, take, total, zero, type Parts } from './parts.ts'
import type { Program } from '../asm/assembler.ts'
import { scaleHeight } from './air.ts'

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
  /**
   * Its own copy of its weave's registers (w0–w7), and when each was last written: the newest write wins, as copies pass
   * from particle to particle by contact (SPEC §5, Weaves). A stamp is the tick it was written, with the writer to break
   * ties.
   */
  regs: Float64Array
  stamp: Float64Array
  /** Which way its weave's frame faces, as it was told: the turn about the vertical its order reads and kicks in. */
  yaw: number
  /** The tick it was last pushed, by its caster or its order. */
  pushedAt: number
  /** The tick it last felt something stop it or strike it: matter, the ground, a body. TUCH reads it. */
  touchedAt: number
  /** How dense the mana around it is (M/m³), as its pressure works it out (the spiky kernel). */
  rho: number
  /** How dense it feels the mana around it is, which way it thickens, and how its neighbours move: what DENS, GRAD and NVEL read. */
  felt: number
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
 * A particle's mass: its free mana, and the matter it holds, each part by its weight a M (PHYSICS.manaMass): mass is
 * mana, free or condensed (D40). Pushing it, the air dragging it, and its weight all go by this.
 */
export function massOf(p: Particle): number {
  return massOfParts(p.free) + massOfParts(p.carried)
}

/** The mass of some mana, free or condensed, by part (PHYSICS.manaMass). */
export function massOfParts(p: Parts): number {
  const m = PHYSICS.manaMass
  return p[0] * m[0] + p[1] * m[1] + p[2] * m[2] + p[3] * m[3]
}

/** How many M of a part's matter a cell holds, packed full: its density over its weight a M. */
export function packed(k: number): number {
  return (PHYSICS.density[k] * PHYSICS.cell ** 3) / PHYSICS.manaMass[k]
}

/** The share of a cell's room some matter takes: each part's amount over what a full cell of it holds. */
export function fillOf(m: Parts): number {
  return m[0] / packed(0) + m[1] / packed(1) + m[2] / packed(2) + m[3] / packed(3)
}

/** How many M of a part a cell of the world's ground holds: full. */
export function groundAmount(k: number): number {
  return packed(k)
}

/**
 * Momentum given to the world from outside it: weight, and the ground and walls, which hold up and stop what's against
 * them, take what's pushed off them, and take the motion of matter let go of, which stops dead in the ground. Pushes and
 * kicks are inside it: what's pushed, and what it's pushed off (a caster's body, the air, the ground), take equal and
 * opposite shares. `outside` is what's given by hand, from beyond the world: a test setting something moving.
 */
export type Impulses = Record<'gravity' | 'walls' | 'outside', Vec>

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
  impulse: Impulses = { gravity: [0, 0, 0], walls: [0, 0, 0], outside: [0, 0, 0] }
  /** Rock: pairs of particles bound together. */
  bonds: Bond[] = []
  /**
   * Energy that motion has turned into heat, by how: kg·(m/tick)², the kilogram being 1 M of free mana (one is 900 J).
   * Counted where it happens, exactly: two things that even out their speeds lose what the evening out takes (D29).
   */
  heat: Record<string, number> = {}

  /** Motion turned into heat. */
  warm(how: string, energy: number) {
    if (energy) this.heat[how] = (this.heat[how] ?? 0) + energy
  }
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

  /** The mass of the air in a cell. */
  airMass(i: number): number {
    return massOfParts(this.air[i])
  }

  /** Unbound earth and water: what blocks and what counts as a touch. */
  solidAt(i: number): boolean {
    if (i < 0) return true
    const m = this.matter[i]
    return m[EARTH] / packed(EARTH) + m[WATER] / packed(WATER) >= PHYSICS.solid
  }

  private earthAt(i: number): boolean {
    return i >= 0 && this.matter[i][EARTH] / packed(EARTH) >= PHYSICS.solid
  }

  /** The share of a cell's room its unbound matter takes. */
  fill(i: number): number {
    return fillOf(this.matter[i])
  }

  // Building worlds

  /**
   * Fills every cell below `top` (in cells) with matter, and the open air above with air mana, as thick as it settles
   * under its own weight: PHYSICS.airMana a cell at the ground, thinning as it goes up.
   */
  static withGround(w: number, h: number, d: number, top: number, part = EARTH): World {
    const world = new World(w, h, d)
    const H = scaleHeight()
    for (let i = 0; i < world.size; i++) {
      const [, y] = world.coords(i)
      if (y < top) world.matter[i][part] = groundAmount(part)
      else world.air[i].fill((PHYSICS.airMana / 4) * Math.exp(-((y + 0.5 - top) * world.cell) / H))
    }
    return world
  }

  /** Fills a box of cells (inclusive) with matter, where the air mana there was. For building a world, before it runs. */
  fillBox(from: Vec, to: Vec, part = EARTH, amount = groundAmount(part)) {
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
        regs: new Float64Array(PHYSICS.weaveRegisters),
        stamp: new Float64Array(PHYSICS.weaveRegisters),
        yaw: 0,
        pushedAt: -1,
        touchedAt: -1,
        rho: 0,
        felt: 0,
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

  /** A new particle like `p`, with a new id and nothing in it yet. */
  spawn(p: Particle): Particle {
    const q: Particle = {
      ...p,
      id: this.nextParticle++,
      pos: [...p.pos],
      vel: [...p.vel],
      free: zero(),
      carried: zero(),
      regs: p.regs.slice(),
      stamp: p.stamp.slice(),
      grad: [...p.grad],
      nvel: [...p.nvel],
      acc: [0, 0, 0],
    }
    this.particles.push(q)
    return q
  }

  /**
   * Mana let into the air of a cell, carrying momentum (px, py, pz) into it. It comes to the air's speed there: what that
   * evening out takes from their motion is heat.
   */
  addAir(i: number, parts: Parts, momentum: Vec = [0, 0, 0]) {
    if (i < 0) return
    const before = this.airMass(i)
    const m = massOfParts(parts)
    const after = before + m
    if (after <= 0) return
    let lost = 0
    for (let k = 0; k < 3; k++) {
      const v = this.airVel[i * 3 + k]
      const next = (before * v + momentum[k]) / after
      lost += 0.5 * before * v * v + (m > 0 ? (momentum[k] * momentum[k]) / (2 * m) : 0) - 0.5 * after * next * next
      this.airVel[i * 3 + k] = next
    }
    this.warm('mixing', lost)
    add(this.air[i], parts)
  }

  /** Mana drawn out of the air of a cell. Returns it, and the momentum it takes with it. */
  takeAir(i: number, f: number): { parts: Parts; momentum: Vec } {
    const parts = share(this.air[i], f)
    const m = massOfParts(parts)
    return { parts, momentum: [m * this.airVel[i * 3], m * this.airVel[i * 3 + 1], m * this.airVel[i * 3 + 2]] }
  }

  // Each tick

  /**
   * Bodies stand on the ground, which holds them up, and slide along it when they're pushed, until friction stops them:
   * the ground takes from their speed, each tick, as much as their weight pressing on it lets it (PHYSICS.friction × g).
   * What it takes is heat. A body that runs into something solid stops.
   */
  moveBodies() {
    const grip = PHYSICS.friction * PHYSICS.gravity
    let heat = 0
    for (const b of this.bodies) {
      const before: Vec = [...b.vel]
      const ke = 0.5 * b.mass * (b.vel[0] ** 2 + b.vel[2] ** 2)
      const speed = Math.hypot(b.vel[0], b.vel[2])
      if (speed > 0) {
        const next: Vec = [b.pos[0] + b.vel[0], b.pos[1], b.pos[2] + b.vel[2]]
        const i = this.cellOf(next)
        if (i >= 0 && !this.solidAt(i)) b.pos = next
        else b.vel = [0, 0, 0]
        const slow = Math.min(1, grip / speed)
        b.vel[0] -= b.vel[0] * slow
        b.vel[2] -= b.vel[2] * slow
      }
      heat += ke - 0.5 * b.mass * (b.vel[0] ** 2 + b.vel[2] ** 2)
      for (const k of [0, 2]) this.impulse.walls[k] += b.mass * (b.vel[k] - before[k])
      // It stands: nothing moves it up or down.
      this.impulse.walls[1] -= b.mass * b.vel[1]
      b.vel[1] = 0
    }
    this.warm('friction', heat)
  }

  /**
   * Matter follows its nature: earth falls and piles, water falls and spreads, air and flame rise.
   * `carried` is the share of each cell's room the matter weaves hold there takes: it takes room too.
   */
  settleMatter(carried: Float64Array) {
    const moved = new Uint8Array(this.size)
    const room = (j: number) => 1 - fillOf(this.matter[j]) - carried[j]
    // How much of a cell's room each M of a cell's matter takes.
    const each = (m: Parts) => (total(m) > 0 ? fillOf(m) / total(m) : 0)
    const flip = this.tick % 2 === 0 ? 1 : -1
    const sides: [number, number][] = this.d > 1 ? [[flip, 0], [-flip, 0], [0, flip], [0, -flip]] : [[flip, 0], [-flip, 0]]

    const tryMove = (i: number, j: number, amount: number): number => {
      if (j < 0) return 0
      const r = room(j)
      if (r <= PHYSICS.epsilon) return 0
      const moving = take(this.matter[i], Math.min(amount, r / each(this.matter[i])))
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
              const fj = j >= 0 ? fillOf(this.matter[j]) + carried[j] : 1
              const fi = fillOf(m)
              if (fj < fi) tryMove(i, j, (fi - fj) / 2 / each(m))
            }
          } else if (kind === FIRE) {
            // Flame spreads as any gas does: from where there's more of it to where there's less, evening out with its
            // neighbours by a share of the difference (Fick's law), and no further once they're even.
            for (const [sx, sz] of sides) {
              const j = this.index(x + sx, y, z + sz)
              if (j < 0 || room(j) <= 0) continue
              const diff = m[FIRE] - this.matter[j][FIRE]
              if (diff <= 0) continue
              const f = Math.min(room(j) / each(m), ((PHYSICS.flameSpread * diff) / (sides.length + 1)) * (total(m) / m[FIRE]))
              add(this.matter[j], take(m, f))
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
      const mass = this.airMass(i)
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
