// Worlds to cast the four spells in: shared by the tests and by mvm.

import { Sim } from './vm/sim.ts'
import { World, EARTH, type Vec } from './vm/world.ts'
import { master, type Caster } from './vm/caster.ts'

export type Scene = { sim: Sim; caster: Caster; dims: 2 | 3; ground: number }

/** Flat ground, 2 m deep, and a caster standing on it 2 m in. */
export function field(dims: 2 | 3 = 2): Scene {
  const ground = 8 // cells: 2 m
  const world = dims === 2 ? World.withGround(64, 32, 1, ground) : World.withGround(48, 24, 48, ground)
  const sim = new Sim(world)
  const z = dims === 2 ? world.cell / 2 : 6
  const caster = sim.addCaster('Mage', [2, ground * world.cell + 0.9, z])
  return { sim, caster, dims, ground }
}

/** A point on the surface of the ground, `x` metres along. */
export function onGround(s: Scene, x: number): Vec {
  const w = s.sim.world
  const cx = Math.floor(x / w.cell)
  return [(cx + 0.5) * w.cell, (s.ground - 0.5) * w.cell, s.caster.body.pos[2]]
}

export function stoneWall(dims: 2 | 3 = 2): Scene {
  const s = field(dims)
  // Raised by hand, 400 kg of earth: a master's spell.
  s.caster.stats = master()
  s.caster.will = { aim: onGround(s, 4), amount: 500, force: 0, maintain: false }
  if (dims === 3) {
    // A wall 4 m long is 256 cells of earth to lift, not 16: it takes a master to hold that much.
    s.caster.will.amount = 8000
    s.caster.stats.body.capacity.training = 12000
    s.caster.stats.mind.speed.training = 1200
  }
  return s
}

/** A pillar of earth 9 m away to throw at. */
export function fireball(dims: 2 | 3 = 2): Scene {
  const s = field(dims)
  const w = s.sim.world
  const z = Math.floor(s.caster.body.pos[2] / w.cell)
  w.fillBox([44, s.ground, dims === 2 ? 0 : z - 3], [47, 20, dims === 2 ? 0 : z + 3], EARTH)
  s.caster.will = { aim: [44 * w.cell, 3.1, s.caster.body.pos[2]], amount: 120, force: 0.5, maintain: false }
  return s
}

/** Someone 2.5 m away to blow back. */
export function gust(dims: 2 | 3 = 2): Scene & { target: { pos: Vec } } {
  const s = field(dims)
  const target = s.sim.world.addBody('Target', [4.5, s.caster.body.pos[1], s.caster.body.pos[2]])
  s.caster.will = { aim: [...target.pos], amount: 40, force: 0.6, maintain: true }
  return { ...s, target }
}

export function waterShield(dims: 2 | 3 = 2): Scene {
  const s = field(dims)
  s.caster.will = { aim: [4, 3, s.caster.body.pos[2]], amount: 200, force: 0, maintain: false }
  return s
}

/** A ball to hold in front of the caster for as long as they maintain it (the bench). */
export function hold(dims: 2 | 3 = 2): Scene {
  const s = field(dims)
  s.caster.will = { aim: [6, 3, s.caster.body.pos[2]], amount: 120, force: 0, maintain: true }
  return s
}

export const SCENES = { StoneWall: stoneWall, Fireball: fireball, Gust: gust, WaterShield: waterShield, Hold: hold } as const
