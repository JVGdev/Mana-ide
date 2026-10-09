// The bench (src/bench.ts): every way of holding and throwing runs on the machine, keeps its ledgers, and repeats.

import { describe, expect, it } from 'vitest'
import { VARIANTS, runVariant } from '../src/bench.ts'

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
    })

  it('repeats exactly', () => {
    const a = runVariant(variant('ThrowCling'), 'adept', 2, 1)
    const b = runVariant(variant('ThrowCling'), 'adept', 2, 1)
    expect({ ...a, ms: 0 }).toEqual({ ...b, ms: 0 })
  })

  it('an order holds a still ball better than nothing does', () => {
    const nothing = runVariant(variant('HoldNothing'), 'adept', 2, 2)
    const cling = runVariant(variant('HoldCling'), 'adept', 2, 2)
    expect(nothing.spread).toBeGreaterThan(0.8)
    expect(cling.spread).toBeLessThan(0.5)
    expect(cling.burn).toBeGreaterThan(0) // and pays for it from the ball
  })

  it('an order told only an angle flies, but nothing holds it together', () => {
    const r = runVariant(variant('ThrowHeading'), 'adept', 2, 2)
    expect(r.ingrain).toBeGreaterThan(0)
    expect(r.together).toBeLessThan(0.5) // what's left of the ball when it gets there, if it does, is a scatter
  })

  it('an order that feels its neighbours holds a thrown ball together all the way', () => {
    const r = runVariant(variant('ThrowCohere'), 'master', 2, 2)
    expect(r.arrived).toBe(true)
    expect(r.together).toBeGreaterThan(0.85)
  })
})
