// A weave: mana laid out in the world as a thing. A fireball, a wall, a shield.

import { PHYSICS } from './physics.ts'
import { zero, type Parts } from './parts.ts'
import type { Vec } from './world.ts'
import type { Caster } from './caster.ts'
import type { Program } from '../asm/assembler.ts'

export type WeaveCell = {
  /** Where it is from the weave's origin, in the weave's frame (x right, y up, z forward). */
  off: Vec
  /** Free mana. */
  free: Parts
  /** Matter it holds bound, or condensed itself. */
  carried: Parts
}

export class Weave {
  /** Turn about the vertical, in radians: 0 faces +z. */
  yaw = 0
  cells: WeaveCell[] = []
  regs = new Float64Array(PHYSICS.weaveRegisters)
  order: number | null = null
  inHand = true
  manifestedAt = -1
  locks = { shape: false, input: false, order: false }
  private byKey = new Map<string, WeaveCell>()

  constructor(
    readonly id: number,
    readonly maker: Caster,
    /** The spell it was woven by: its order is code there. */
    readonly program: Program,
    public origin: Vec,
  ) {}

  /** A direction in the weave's frame, turned into the world's. */
  toWorld(v: Vec): Vec {
    const c = Math.cos(this.yaw)
    const s = Math.sin(this.yaw)
    return [c * v[0] + s * v[2], v[1], -s * v[0] + c * v[2]]
  }

  worldPos(cell: WeaveCell): Vec {
    const r = this.toWorld(cell.off)
    return [this.origin[0] + r[0], this.origin[1] + r[1], this.origin[2] + r[2]]
  }

  /** The cell at a position in the weave's frame, snapped to the grid; made if there's none. */
  cellAt(off: Vec, size: number): WeaveCell {
    const snapped = off.map((v) => Math.round(v / size)) as Vec
    const key = snapped.join(',')
    let cell = this.byKey.get(key)
    if (!cell) {
      cell = { off: snapped.map((v) => v * size) as Vec, free: zero(), carried: zero() }
      this.byKey.set(key, cell)
      this.cells.push(cell)
    }
    return cell
  }
}
