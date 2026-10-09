// A weave: mana laid out in the world as a thing. A fireball, a wall, a shield. Its mana is particles (SPEC §11): the
// ones in its field, which its caster, or its order, keeps together.

import { PHYSICS } from './physics.ts'
import { total } from './parts.ts'
import type { Particle, Vec } from './world.ts'
import type { Caster } from './caster.ts'
import type { Program } from '../asm/assembler.ts'

export class Weave {
  /** Turn about the vertical, in radians: 0 faces +z. */
  yaw = 0
  particles: Particle[] = []
  regs = new Float64Array(PHYSICS.weaveRegisters)
  /** The routine its particles are ingrained with (INGR): an address in `program`. */
  order: number | null = null
  inHand = true
  manifestedAt = -1
  locks = { input: false, order: false }
  /** How far its field reaches around its centre, in metres. A particle further out leaves it. */
  field = PHYSICS.field

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
    const c = Math.cos(this.yaw)
    const s = Math.sin(this.yaw)
    return [c * v[0] + s * v[2], v[1], -s * v[0] + c * v[2]]
  }

  /** A direction in the world, turned into the weave's frame. */
  toFrame(v: Vec): Vec {
    const c = Math.cos(this.yaw)
    const s = Math.sin(this.yaw)
    return [c * v[0] - s * v[2], v[1], s * v[0] + c * v[2]]
  }

  /** A point in the weave's frame, in the world. */
  worldPos(off: Vec): Vec {
    const r = this.toWorld(off)
    return [this.origin[0] + r[0], this.origin[1] + r[1], this.origin[2] + r[2]]
  }

  /** The middle of its mana, and how it moves on average. */
  centre(): { pos: Vec; vel: Vec } | null {
    let m = 0
    const pos: Vec = [0, 0, 0]
    const vel: Vec = [0, 0, 0]
    for (const p of this.particles) {
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

  /** Free mana in it. */
  mana(): number {
    return this.particles.reduce((s, p) => s + total(p.free), 0)
  }
}
