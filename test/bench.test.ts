// The bench (src/bench.ts): every way of holding and throwing runs on the machine, keeps its ledgers, and repeats.

import { describe, expect, it } from 'vitest'
import { VARIANTS, runVariant } from '../src/bench.ts'
import { PHYSICS } from '../src/vm/physics.ts'

const variant = (id: string) => VARIANTS.find((v) => v.id === id)!

describe('the bench', () => {
  for (const v of VARIANTS)
    it(`${v.id}: keeps mana and momentum, every tick`, () => {
      let before: number | undefined
      const r = runVariant(v, 'adept', 2, 2, (s) => {
        before ??= s.sim.ledger().total
        expect(s.sim.ledger().total).toBeCloseTo(before, 4)
        expect(s.sim.world.momentumError()).toBeLessThan(1e-6)
      })
      expect(r.particles).toBeGreaterThan(0)
      expect(PHYSICS.orderKnowsCentre).toBe(true) // put back after a run where orders only feel
    })

  it('repeats exactly', () => {
    const a = runVariant(variant('ThrowPull'), 'adept', 2, 1)
    const b = runVariant(variant('ThrowPull'), 'adept', 2, 1)
    expect({ ...a, ms: 0 }).toEqual({ ...b, ms: 0 })
  })

  it('an order holds a still ball better than nothing does', () => {
    const nothing = runVariant(variant('HoldNothing'), 'adept', 2, 2)
    const pull = runVariant(variant('HoldPull'), 'adept', 2, 2)
    expect(nothing.spread).toBeGreaterThan(0.8)
    expect(pull.spread).toBeLessThan(0.5)
    expect(pull.burn).toBeGreaterThan(0) // and pays for it from the ball
  })

  it('an order told only an angle flies, but nothing holds it together', () => {
    const r = runVariant(variant('ThrowHeading'), 'adept', 2, 2)
    expect(r.ingrain).toBeGreaterThan(0)
    expect(r.arrived).toBe(false)
  })

  it('an order that only feels holds a thrown ball together all the way', () => {
    const r = runVariant(variant('ThrowFeel'), 'master', 2, 2)
    expect(r.arrived).toBe(true)
    expect(r.together).toBeGreaterThan(0.9)
  })
})
