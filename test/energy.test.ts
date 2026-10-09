// The Energy ledger (D20): where the world's energy comes from and where it goes.

import { describe, expect, it } from 'vitest'
import { spell } from '../src/load.ts'
import { SCENES } from '../src/scenes.ts'
import { stepFluid } from '../src/vm/fluid.ts'
import { stored, sum, heatOf } from '../src/vm/energy.ts'
import { PHYSICS } from '../src/vm/physics.ts'
import { EARTH, World, massOf } from '../src/vm/world.ts'

/** Open ground 2 m deep, with no air. */
function ground() {
  const w = World.withGround(40, 24, 1, 8)
  for (const a of w.air) a.fill(0)
  return w
}

describe('the Energy ledger', () => {
  it('turns a fall into motion, and the landing into heat, to the last joule', () => {
    const w = ground()
    const [p] = w.pour([0, 0, 0, PHYSICS.mote], [2, 3, 0.125], 1)
    p.carried[EARTH] = 4
    const before = sum(stored(w))
    for (let t = 0; t < 40; t++) {
      stepFluid(w)
      // While it falls, what it loses in height it gains in speed.
      const now = sum(stored(w)) + heatOf(w)
      expect(Math.abs(now - before)).toBeLessThan(0.01 * massOf(p) * PHYSICS.gravity) // a centimetre of fall, stepping
    }
    expect(p.vel[1]).toBe(0) // landed
    // It fell about a metre: its weight times that went into the ground as heat.
    expect(heatOf(w)).toBeCloseTo(massOf(p) * PHYSICS.gravity * (3 - p.pos[1]), 3)
    expect((w.heat['striking'] ?? 0) + (w.heat['the ground'] ?? 0)).toBeCloseTo(heatOf(w), 9)
  })

  it('loses to the air what the air slows', () => {
    const w = World.withGround(40, 24, 1, 8)
    const [p] = w.pour([0, 0, PHYSICS.mote, 0], [3, 4, 0.125], 1)
    p.vel = [0.2, 0, 0]
    const before = sum(stored(w))
    for (let t = 0; t < 10; t++) stepFluid(w)
    expect(p.vel[0]).toBeLessThan(0.2)
    expect(w.heat['the air dragging']).toBeGreaterThan(0)
    // What it lost went into the air's motion and into heat.
    expect(sum(stored(w)) + heatOf(w)).toBeCloseTo(before, 6)
  })

  for (const [name, ticks, close] of [
    ['Fireball', 70, 0.05],
    ['Gust', 30, 0.05],
    ['WaterShield', 80, 0.05],
    ['StoneWall', 160, 0.25],
  ] as const)
    it(`balances through a ${name}: what's held and turned to heat is what was put in, near enough`, () => {
      const s = SCENES[name](2)
      s.sim.keepEnergy()
      s.sim.cast(s.caster, spell(name))
      s.sim.run(ticks)
      const e = s.sim.energyNow()
      expect(e.total + e.heat).toBeCloseTo(e.start + e.outside + e.errorTotal, 6) // the ledger's own sums
      expect(e.outside).toBeGreaterThan(0) // the caster put energy in
      expect(e.heat).toBeGreaterThan(0)
      // What the numbers get wrong is small beside what moved through.
      expect(Math.abs(e.errorTotal)).toBeLessThan(close * (e.heat + Math.abs(e.outside)))
    })
})
