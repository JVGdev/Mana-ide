// The mana fluid sandbox (SPEC §11): its ledgers balance, and holding, throwing and letting go do what physics says.

import { describe, expect, it } from 'vitest'
import { DEFAULT_SETUP, Run, play } from '../sandbox/scenario.ts'
import { DEFAULTS } from '../sandbox/sim.ts'
import { STRATEGIES } from '../sandbox/strategies.ts'

describe('the sandbox', () => {
  it('keeps mana and momentum balanced, every tick, whatever holds the ball', () => {
    for (const s of STRATEGIES) {
      const run = new Run({ ...DEFAULT_SETUP, kind: 'throw', caster: 'master', strategy: s.id })
      for (let t = 0; t < 80; t++) {
        run.step()
        expect(run.manaError()).toBeLessThan(1e-6)
        expect(run.sim.momentumError()).toBeLessThan(1e-6)
      }
    }
  })

  it('lets an unheld ball come apart under its own pressure', () => {
    const r = play({ ...DEFAULT_SETUP, strategy: 'none' }, 40)
    expect(r.held).toBeLessThan(0.5)
    expect(r.spread).toBeGreaterThan(1.8)
  })

  it('holds a ball together when a master pushes it back in', () => {
    const r = play({ ...DEFAULT_SETUP, caster: 'master', strategy: 'all' }, 40)
    expect(r.held).toBe(1)
    expect(r.spread).toBeLessThan(1.3)
    expect(r.spent).toBeGreaterThan(0)
  })

  it('holds better with a bigger mind', () => {
    const adept = play({ ...DEFAULT_SETUP, caster: 'adept', strategy: 'all' }, 40)
    const master = play({ ...DEFAULT_SETUP, caster: 'master', strategy: 'all' }, 40)
    expect(master.spread).toBeLessThan(adept.spread)
  })

  it('lets an order hold its own ball, paying with the ball', () => {
    const run = new Run({ ...DEFAULT_SETUP, strategy: 'order' })
    for (let t = 0; t < 40; t++) run.step()
    expect(run.sim.spread()).toBeLessThan(1.2)
    expect(run.sim.selfSpent).toBeGreaterThan(0)
    expect(run.mind.spent).toBe(0)
  })

  it('costs the caster beats to ingrain an order, particle by particle', () => {
    const run = new Run({ ...DEFAULT_SETUP, caster: 'adept', strategy: 'order' })
    run.step()
    const ingrained = run.sim.particles.filter((p) => p.order).length
    expect(ingrained).toBeGreaterThan(0)
    expect(ingrained).toBeLessThan(run.sim.particles.length) // 300 beats don't go round 120 particles at 24 each
    while (run.sim.particles.some((p) => p.held && !p.order) && run.sim.tick < 30) run.step()
    expect(run.sim.tick).toBeGreaterThanOrEqual(9)
  })

  it('burns an order\'s own mana as it thinks, even when it never pushes', () => {
    // Nothing moves the ball, and it's laid out exactly within its radius, so the order never has to push.
    const quiet = { ...DEFAULTS, stiffness: 0, rise: 0 }
    const run = new Run({ ...DEFAULT_SETUP, caster: 'master', strategy: 'order', seed: 0, settings: quiet })
    for (let t = 0; t < 20; t++) run.step()
    expect(run.sim.impulse.order[0]).toBe(0)
    expect(run.sim.selfSpent).toBeGreaterThan(0)
  })

  it('drags the air along behind a thrown ball, and is slowed by it', () => {
    const run = new Run({ ...DEFAULT_SETUP, kind: 'throw', caster: 'master', strategy: 'all' })
    for (let t = 0; t < 30; t++) run.step()
    const behind = run.sim.cellAt(run.sim.origin.x - 1.5, run.sim.origin.y)
    expect(run.sim.airVx[behind]).toBeGreaterThan(0)
    const thin = play({ ...DEFAULT_SETUP, kind: 'throw', caster: 'master', strategy: 'all', settings: { ...DEFAULTS, airMana: 0 } }, 160)
    const thick = play({ ...DEFAULT_SETUP, kind: 'throw', caster: 'master', strategy: 'all', settings: { ...DEFAULTS, airMana: 120 } }, 160)
    expect(thick.hitAt).toBeGreaterThan(thin.hitAt)
  })

  it('throws a ball by pushing it for as long as it is in reach, and the ball flies on its momentum', () => {
    const run = new Run({ ...DEFAULT_SETUP, kind: 'throw', caster: 'master', strategy: 'all' })
    while (run.letGoAt < 0 && run.sim.tick < 100) run.step()
    expect(run.letGoAt).toBeGreaterThan(1) // speeding up takes ticks: a push is only so strong
    const speed = run.sim.origin.vx
    // It leaves the caster's reach before it's up to the speed they wanted: the reach limits the throw.
    expect(run.mind.inReach()).toBe(false)
    expect(speed).toBeGreaterThan(0.2)
    expect(speed).toBeLessThan(DEFAULT_SETUP.speed)
    for (let t = 0; t < 5; t++) run.step()
    expect(run.sim.origin.vx).toBeGreaterThan(0.85 * speed) // nobody pushes it now, and it keeps going
  })

  it('carries a ball held by its own order to the wall mostly together', () => {
    const r = play({ ...DEFAULT_SETUP, kind: 'throw', caster: 'master', strategy: 'order' }, 160)
    expect(r.hitAt).toBeGreaterThan(0)
    expect(r.held).toBeGreaterThan(0.7)
  })
})
