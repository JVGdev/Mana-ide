// The world: a grid of cells, each with free air mana and condensed matter. A 2D world is one cell deep.

import { PHYSICS } from './physics.ts'
import { add, dominant, share, take, total, zero, type Parts } from './parts.ts'

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

/** Mana let out loose: sent, or left behind by a weave that came apart. */
export type Loose = { pos: Vec; vel: Vec; parts: Parts }

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
  loose: Loose[] = []
  bodies: Body[] = []
  tick = 0
  private nextBody = 1

  constructor(
    readonly w: number,
    readonly h: number,
    readonly d = 1,
  ) {
    this.size = w * h * d
    this.air = Array.from({ length: this.size }, () => zero())
    this.matter = Array.from({ length: this.size }, () => zero())
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

  addBody(name: string, pos: Vec, mass = 1, half: Vec = [0.25, 0.9, 0.25]): Body {
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

  // Each tick

  /** Air mana evens out between neighbouring cells. */
  diffuseAir() {
    const rate = PHYSICS.airDiffusion / 2
    for (let i = 0; i < this.size; i++) {
      const [x, y, z] = this.coords(i)
      for (const j of [this.index(x + 1, y, z), this.index(x, y + 1, z), this.index(x, y, z + 1)]) {
        if (j < 0) continue
        const a = this.air[i]
        const b = this.air[j]
        for (let k = 0; k < 4; k++) {
          const f = (a[k] - b[k]) * rate
          a[k] -= f
          b[k] += f
        }
      }
    }
  }

  /** Loose mana flies, slows, pushes bodies, and settles back into the air. */
  moveLoose() {
    const still: Loose[] = []
    for (const p of this.loose) {
      const next: Vec = [p.pos[0] + p.vel[0], p.pos[1] + p.vel[1], p.pos[2] + p.vel[2]]
      const j = this.cellOf(next)
      if (j < 0 || this.solidAt(j)) {
        this.settle(p)
        continue
      }
      p.pos = next
      const body = this.bodyAt(p.pos)
      if (body) {
        const push = (p.parts[AIR] * PHYSICS.push) / body.mass
        body.vel[0] += p.vel[0] * push
        body.vel[2] += p.vel[2] * push
      }
      p.vel = [p.vel[0] * PHYSICS.looseDrag, p.vel[1] * PHYSICS.looseDrag, p.vel[2] * PHYSICS.looseDrag]
      if (Math.hypot(...p.vel) < PHYSICS.looseRest) this.settle(p)
      else still.push(p)
    }
    this.loose = still
  }

  settle(p: Loose) {
    add(this.air[this.clampedCellOf(p.pos)], p.parts)
    p.parts = zero()
  }

  /** Bodies pushed by wind slide along the ground until friction stops them. */
  moveBodies() {
    for (const b of this.bodies) {
      if (Math.abs(b.vel[0]) + Math.abs(b.vel[2]) < 1e-6) continue
      const next: Vec = [b.pos[0] + b.vel[0], b.pos[1], b.pos[2] + b.vel[2]]
      const i = this.cellOf(next)
      if (i >= 0 && !this.solidAt(i)) b.pos = next
      else b.vel = [0, 0, 0]
      b.vel[0] *= PHYSICS.bodyFriction
      b.vel[2] *= PHYSICS.bodyFriction
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

  /** All the mana in the world that isn't in a caster or a weave. */
  mana(): { air: number; matter: number; loose: number } {
    let air = 0
    let matter = 0
    for (let i = 0; i < this.size; i++) {
      air += total(this.air[i])
      matter += total(this.matter[i])
    }
    return { air, matter, loose: this.loose.reduce((s, p) => s + total(p.parts), 0) }
  }
}
