// The Energy ledger (D20, SPEC §11): what the world holds as motion and as stored energy, so that where it goes can be
// counted. Mana is one ledger; Energy is the other, and each balances on its own.
//
// Units: kg·(m/tick)², the kilogram being the mass of 1 M of free mana. One is 900 J.

import { PHYSICS } from './physics.ts'
import { total } from './parts.ts'
import { storedInFluid, type FluidHooks } from './fluid.ts'
import { massOf, type World } from './world.ts'

export type Stored = {
  /** ½mv² of every particle, the air and every body. */
  motion: number
  /** Weight lifted: of particles and of the matter in the ground. */
  height: number
  /** Mana's gas pressed together, matter packed past full, and what coheres pulled apart. */
  gas: number
  packing: number
  cohesion: number
  /** The air pressed together. */
  air: number
}

export const JOULES = 900

/** Everything the world holds as Energy, by kind. */
export function stored(world: World, hooks: FluidHooks = {}): Stored {
  const g = PHYSICS.gravity
  let motion = 0
  let height = 0
  for (const p of world.particles) {
    const m = massOf(p)
    motion += 0.5 * m * (p.vel[0] ** 2 + p.vel[1] ** 2 + p.vel[2] ** 2)
    let w = 0
    for (let k = 0; k < 4; k++) w += p.free[k] * PHYSICS.fall[k] + p.carried[k] * PHYSICS.matterMass[k] * PHYSICS.matterFall[k]
    height += g * w * p.pos[1]
  }
  for (const b of world.bodies) motion += 0.5 * b.mass * (b.vel[0] ** 2 + b.vel[1] ** 2 + b.vel[2] ** 2)
  let air = 0
  const c2 = PHYSICS.airSound ** 2
  // Air stores energy pressed denser, or drawn thinner, than the rest of it is: c²(ln(ρ/ρ̄) + ρ̄/ρ − 1) for each M,
  // which is nothing at the air's own density. So mana let into the air, or gathered from it, as dense as it is
  // brings or takes no energy with it.
  const usual = usualAir(world)
  for (let i = 0; i < world.size; i++) {
    const M = total(world.air[i])
    if (M > 0) {
      const v = world.airVel
      motion += 0.5 * M * (v[i * 3] ** 2 + v[i * 3 + 1] ** 2 + v[i * 3 + 2] ** 2)
    }
    // An empty cell open to the air holds c²ρ̄: what the air around it would give, rushing in.
    if (!world.solidAt(i) && usual > 0) air += c2 * (M > 0 ? M * Math.log(M / usual) + usual - M : usual)
    const mt = world.matter[i]
    let w = 0
    for (let k = 0; k < 4; k++) w += mt[k] * PHYSICS.matterMass[k] * PHYSICS.matterFall[k]
    if (w) height += g * w * (world.coords(i)[1] + 0.5) * world.cell
  }
  return { motion, height, ...storedInFluid(world, hooks, usual > 0 ? usual / world.cell ** 3 : 1), air }
}

/** How much mana a cell of air holds, on average, among the cells open to it. */
export function usualAir(world: World): number {
  let sum = 0
  let cells = 0
  for (let i = 0; i < world.size; i++)
    if (!world.solidAt(i)) {
      sum += total(world.air[i])
      cells++
    }
  return cells ? sum / cells : 0
}

export const sum = (s: Stored) => s.motion + s.height + s.gas + s.packing + s.cohesion + s.air

export const heatOf = (world: World) => Object.values(world.heat).reduce((a, b) => a + b, 0)

/**
 * The ledger kept tick by tick: what it held when counting began, what casters and orders put in or took out, and
 * what the numbers got wrong, by step. Always: now + heat = start + outside + error.
 */
export type Ledger = { start: number; outside: number; error: Record<string, number> }
