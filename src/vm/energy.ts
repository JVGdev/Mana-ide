// The Energy ledger (D20, SPEC §11): what the world holds as motion and as stored energy, so that where it goes can be
// counted. Mana is one ledger; Energy is the other, and each balances on its own.
//
// Units: kg·(m/tick)², the kilogram being the mass of 1 M of free mana. One is 900 J.

import { PHYSICS } from './physics.ts'
import { total } from './parts.ts'
import { buoyed, parcels, storedInFluid } from './fluid.ts'
import { RAW_MASS, scaleHeight } from './air.ts'
import { massOf, massOfParts, type World } from './world.ts'

export type Stored = {
  /** ½mv² of every particle, the air and every body. */
  motion: number
  /** Weight lifted: of particles, the air, and the matter in the ground. */
  height: number
  /** Mana's gas pressed together, matter packed past full, and what coheres pulled apart. */
  gas: number
  packing: number
  cohesion: number
  /** The air pressed together. */
  air: number
  /**
   * Mana in casters' bodies, worth what the same mana is worth in the air at rest (airLevel): so drawing it in, or
   * letting it out, as thick as the air is, brings or takes no energy.
   */
  bodies: number
}

export const JOULES = 900

/** Everything the world holds as Energy, by kind. `inBodies` is the free mana in casters' bodies (M). */
export function stored(world: World, inBodies = 0): Stored {
  const g = PHYSICS.gravity
  let motion = 0
  let height = 0
  // A particle's height is worth its weight, less what the air holds up of it. The air holds it up from the height where
  // the air at rest is as thick as it is on average: there, a parcel of mana is worth what the same mana is worth as
  // air (at rest, mana in the air is worth the same at any height, its weight and its pressure trading off), so mana
  // settling out of a parcel into the air brings no energy with it.
  const inCell = parcels(world, world.particles)
  const raw = RAW_MASS()
  const level = airLevel(world)
  for (const p of world.particles) {
    const m = massOf(p)
    motion += 0.5 * m * (p.vel[0] ** 2 + p.vel[1] ** 2 + p.vel[2] ** 2)
    const c = world.cellOf(p.pos)
    height += g * (m * p.pos[1] - total(p.free) * raw * buoyed(world, c, inCell.get(c) ?? 0) * (p.pos[1] - level))
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
    const y = (world.coords(i)[1] + 0.5) * world.cell
    if (M > 0) {
      const v = world.airVel
      const mass = massOfParts(world.air[i])
      motion += 0.5 * mass * (v[i * 3] ** 2 + v[i * 3 + 1] ** 2 + v[i * 3 + 2] ** 2)
      height += g * mass * y
    }
    // An empty cell open to the air holds c²ρ̄: what the air around it would give, rushing in.
    if (!world.solidAt(i) && usual > 0) air += c2 * (M > 0 ? M * Math.log(M / usual) + usual - M : usual)
    const mt = world.matter[i]
    let w = 0
    for (let k = 0; k < 4; k++) w += mt[k] * PHYSICS.matterMass[k]
    if (w) height += g * w * y
  }
  return { motion, height, ...storedInFluid(world, usual > 0 ? usual / world.cell ** 3 : 1), air, bodies: g * raw * level * inBodies }
}

/** The height where the air at rest is as thick as the air is on average, metres. */
export function airLevel(world: World): number {
  const H = scaleHeight()
  let mana = 0
  let shape = 0
  let cells = 0
  for (let i = 0; i < world.size; i++)
    if (!world.solidAt(i)) {
      mana += total(world.air[i])
      shape += Math.exp(-((world.coords(i)[1] + 0.5) * world.cell) / H)
      cells++
    }
  if (mana <= 0 || !cells) return 0
  // At rest, M(y) = S·e^(−y/H) with S = mana / shape; it's mana/cells at y = H·ln(S·cells/mana).
  return H * Math.log(cells / shape)
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

export const sum = (s: Stored) => s.motion + s.height + s.gas + s.packing + s.cohesion + s.air + s.bodies

export const heatOf = (world: World) => Object.values(world.heat).reduce((a, b) => a + b, 0)

/**
 * The ledger kept tick by tick: what it held when counting began, what casters and orders put in or took out, and
 * what the numbers got wrong, by step. Always: now + heat = start + outside + error. Of what came from outside, `minds`
 * is what minds transformed out of mana (D32), counted push by push, and `bodies` the rest: what bodies did to the
 * world's mana by gathering it, pouring it and letting it out.
 */
export type Ledger = { start: number; outside: number; minds: number; bodies: number; error: Record<string, number> }
