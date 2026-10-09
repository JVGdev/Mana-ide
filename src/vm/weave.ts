// A weave: mana laid out in the world as a thing. A fireball, a wall, a shield. Its mana is particles (SPEC §11): those
// that carry its order, and, until they're given one, those its caster can still reach.

import { PHYSICS } from './physics.ts'
import { total } from './parts.ts'
import type { Particle, Vec } from './world.ts'
import type { Caster } from './caster.ts'
import type { Program } from '../asm/assembler.ts'

export class Weave {
  /** Turn about the vertical, in radians: 0 faces +z. */
  yaw = 0
  particles: Particle[] = []
  /**
   * What its caster has written into its registers (WSET), and when: the mana they pour into it (EMIT) carries this. The
   * registers themselves live in its particles, each with its own copy (Particle.regs).
   */
  regs = new Float64Array(PHYSICS.weaveRegisters)
  stamp = new Float64Array(PHYSICS.weaveRegisters)
  /** The routine its particles are ingrained with (INGR): an address in `program`. */
  order: number | null = null
  inHand = true
  manifestedAt = -1
  locks = { input: false, order: false }
  /** Why its last particles left it: how it ended, once it has. */
  lastLeft = ''

  constructor(
    readonly id: number,
    readonly maker: Caster,
    /** The spell it was woven by: its order is code there. */
    readonly program: Program,
    /**
     * Where positions in it are measured from. In hand, it's where it was begun, and EMIT lays mana out around it. Set
     * loose, it's its centre: the middle of its mana, which moves as its mana does.
     */
    public origin: Vec,
  ) {}

  /** A direction in the weave's frame, turned into the world's. */
  toWorld(v: Vec): Vec {
    return turnToWorld(this.yaw, v)
  }

  /** A direction in the world, turned into the weave's frame. */
  toFrame(v: Vec): Vec {
    return turnToFrame(this.yaw, v)
  }

  /**
   * Register k as its mana has it: the newest copy among its particles (for whoever watches; nothing in the world reads
   * it this way). With no particles, what its caster wrote.
   */
  reg(k: number): number {
    let best = this.stamp[k]
    let v = this.regs[k]
    for (const p of this.particles)
      if (p.stamp[k] > best) {
        best = p.stamp[k]
        v = p.regs[k]
      }
    return v
  }

  /** A point in the weave's frame, in the world. */
  worldPos(off: Vec): Vec {
    const r = this.toWorld(off)
    return [this.origin[0] + r[0], this.origin[1] + r[1], this.origin[2] + r[2]]
  }

  /** The middle of its mana, and how it moves on average. */
  centre(): { pos: Vec; vel: Vec } | null {
    return centreOf(this.particles)
  }

  /** Free mana in it. */
  mana(): number {
    return this.particles.reduce((s, p) => s + total(p.free), 0)
  }
}

/** A direction in a frame turned `yaw` about the vertical, in the world. */
export function turnToWorld(yaw: number, v: Vec): Vec {
  const c = Math.cos(yaw)
  const s = Math.sin(yaw)
  return [c * v[0] + s * v[2], v[1], -s * v[0] + c * v[2]]
}

/** A direction in the world, in a frame turned `yaw` about the vertical. */
export function turnToFrame(yaw: number, v: Vec): Vec {
  const c = Math.cos(yaw)
  const s = Math.sin(yaw)
  return [c * v[0] - s * v[2], v[1], s * v[0] + c * v[2]]
}

/**
 * A register write's stamp: the tick it was written, and who wrote it (a particle's id; 0, its caster) to break ties
 * the same way every time. The newest wins; in the same tick, the caster, then the oldest particle.
 */
export function stampOf(tick: number, writer: number): number {
  return tick * 1048576 + (1048575 - (writer % 1048576))
}

/** The middle of some particles' mana, and how it moves on average. */
export function centreOf(ps: Particle[]): { pos: Vec; vel: Vec } | null {
  let m = 0
  const pos: Vec = [0, 0, 0]
  const vel: Vec = [0, 0, 0]
  for (const p of ps) {
    const mass = total(p.free)
    m += mass
    for (let k = 0; k < 3; k++) {
      pos[k] += mass * p.pos[k]
      vel[k] += mass * p.vel[k]
    }
  }
  if (m <= 0) return null
  return { pos: pos.map((v) => v / m) as Vec, vel: vel.map((v) => v / m) as Vec }
}
