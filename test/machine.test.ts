import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { assemble } from '../src/asm/assembler.ts'
import { resolver } from '../src/load.ts'
import { adept, type CasterStats } from '../src/vm/caster.ts'
import { total } from '../src/vm/parts.ts'
import { Sim, orderLength } from '../src/vm/sim.ts'
import { stampOf } from '../src/vm/weave.ts'
import { World, groundAmount, packed } from '../src/vm/world.ts'
import { PHYSICS } from '../src/vm/physics.ts'

function setup(src: string, tweak?: (s: CasterStats) => void) {
  const world = World.withGround(32, 24, 1, 8)
  const sim = new Sim(world)
  const stats = adept()
  tweak?.(stats)
  const caster = sim.addCaster('Mage', [2, 2.9, 0.125], stats)
  caster.will = { aim: [5, 2.9, 0.125], amount: 100, force: 0.5, maintain: false }
  const program = assemble(src, 'test.masm', resolver())
  const cast = sim.cast(caster, program)
  return { sim, caster, cast, world }
}

describe('the mind', () => {
  it('counts, loops and halts', () => {
    const { sim, cast } = setup(`
main:   LDI   n0, #0
        LDI   n1, #1
.loop:  ADD   n0, n1
        ADD   n1, #1
        CMP   n1, #10
        JLE   .loop
        HALT`)
    sim.runCasts()
    expect(cast.state).toBe('halted')
    expect(cast.frame.n[0]).toBe(55)
  })

  it('does the math shapes need', () => {
    const { sim, cast } = setup(`
        LDI   n0, #0.5236     ; 30°
        SIN   n0
        LDI   n1, #2
        SQRT  n1
        LDI   n2, #1
        LDI   n3, #1
        ATN2  n2, n3
        LDI   n4, #7
        DIV   n4, #0          ; dividing by nothing gives nothing
        LDI   n5, #-2.5
        FLOOR n5
        HALT`)
    sim.runCasts()
    const n = cast.frame.n
    expect(n[0]).toBeCloseTo(0.5, 4)
    expect(n[1]).toBeCloseTo(Math.SQRT2, 6)
    expect(n[2]).toBeCloseTo(Math.PI / 4, 6)
    expect(n[4]).toBe(0)
    expect(n[5]).toBe(-3)
  })

  it('calls routines, keeps a stack and remembers', () => {
    const { sim, cast } = setup(`
main:   LDI   n0, #21
        CALL  double
        PUSH  n0
        LDI   n0, #0
        POP   n1
        LDI   n2, #10
        ST    n1, [n2]
        LD    n3, [n2]
        HALT
double: ADD   n0, n0
        RET`)
    sim.runCasts()
    expect(cast.frame.n[1]).toBe(42)
    expect(cast.frame.n[3]).toBe(42)
  })

  it("can't think past the registers it has", () => {
    const { sim, cast } = setup('LDI n10, #1\nHALT', (s) => (s.mind.registers.genetics = 8))
    sim.runCasts()
    expect(cast.state).toBe('fault')
    expect(cast.fault).toMatch(/NO_ROOM: n10: this mind thinks with 8 registers/)
  })

  it('thinks faster with training, and with each casting (the Law of Conditioning)', () => {
    const src = `
main:   LDI   n0, #0
.l:     ADD   n0, #1
        CMP   n0, #400
        JLT   .l
        HALT`
    const ticks = (conditioning: number, training = 0) => {
      const { sim, cast, caster } = setup(src, (s) => (s.mind.speed.training = training))
      caster.conditioning.set(cast.name, conditioning)
      sim.runCasts()
      return cast.endedAt! - cast.startedAt
    }
    expect(ticks(0)).toBe(4) // 1200 beats at 300 a tick
    expect(ticks(9)).toBe(0) // ten times as many beats: it all fits in the first tick
    expect(ticks(0, 300)).toBe(2)
  })

  it('pays more for slow math, and counts where the beats went', () => {
    const { sim, cast } = setup(`
main:   LDI   n0, #1
        ADD   n0, #1
slow:   DIV   n0, #2
        SIN   n0
        HALT`)
    sim.runCasts()
    expect(cast.beats).toBe(1 + 1 + 4 + 8 + 1)
    expect(cast.profile.get(cast.program.labels.get('slow')!)).toEqual({ runs: 1, beats: 4 })
  })

  it('steps one instruction at a time, and the world moves when its beats for the tick run out', () => {
    const { sim, cast } = setup('LDI n0, #1\nLDI n1, #2\nLDI n2, #3\nHALT', (s) => (s.mind.speed.genetics = 2))
    expect(sim.stepInstruction(cast)).toBe(true)
    expect(cast.frame.n[0]).toBe(1)
    expect(sim.tick).toBe(0) // one more fits in this tick
    sim.stepInstruction(cast)
    expect(cast.frame.n[1]).toBe(2)
    expect(sim.tick).toBe(1) // that was the last: the tick ended
    sim.stepInstruction(cast)
    expect(cast.frame.n[2]).toBe(3)
    expect(sim.tick).toBe(1)
  })

  it('stops before a breakpoint, partway through a tick, and goes on from it', () => {
    const { sim, cast } = setup(`
main:   LDI   n0, #0
.loop:  ADD   n0, #1
here:   CMP   n0, #5
        JLT   main.loop
        HALT`)
    const here = cast.program.labels.get('here')!
    expect(sim.runUntil(cast, (a) => a === here, 10)).toBe(true)
    expect(sim.midTick).toBe(true)
    expect(cast.frame.n[0]).toBe(1)
    expect(sim.runUntil(cast, (a) => a === here, 10)).toBe(true)
    expect(cast.frame.n[0]).toBe(2)
    sim.step() // finish the tick
    expect(sim.midTick).toBe(false)
    expect(cast.state).toBe('halted')
    expect(cast.frame.n[0]).toBe(5)
  })

  it('counts a cast toward conditioning when it halts', () => {
    const { sim, cast, caster } = setup('HALT')
    sim.runCasts()
    expect(caster.conditioning.get(cast.name)).toBe(1)
  })

  it("won't run an order's instructions", () => {
    const { sim, cast } = setup('KICK n0:2\nHALT')
    sim.runCasts()
    expect(cast.fault).toMatch(/ORDER_ONLY/)
  })
})

describe('the body', () => {
  it('loses mana to a lack of affinity when it filters, and only then', () => {
    const { sim, caster } = setup(
      `
        .use  Elements
        GATH  m0, #100
        CIRC  m0
        FILT  m1, m0, #EARTH
        HALT`,
      (s) => {
        s.body.affinity[3].genetics = 0.4
        s.body.drain.genetics = 0
      },
    )
    sim.step()
    const [m0, m1] = caster.regs
    expect(total(m1.parts)).toBeCloseTo(10, 9) // 25 of earth × 40%
    expect(m1.parts[3]).toBeCloseTo(10, 9)
    expect(m0.parts[3]).toBe(0)
    expect(total(m0.parts)).toBeCloseTo(75, 9) // the residue: the other three parts
    expect(total(caster.flow)).toBeCloseTo(60 + 15, 9) // baseline + what slipped
  })

  it('lets go of what it no longer holds: unheld mana joins the flow', () => {
    const { sim, caster } = setup('GATH m0, #50\nGATH m1, #50\nCIRC m1\nHALT', (s) => {
      s.body.focus.genetics = 3
      s.body.drain.genetics = 0
    })
    sim.step() // tick 0: m0 was never held
    expect(total(caster.regs[0].parts)).toBe(0)
    expect(total(caster.regs[1].parts)).toBeCloseTo(50, 9)
    sim.run(3) // held through tick 3, let go at its end
    expect(total(caster.regs[1].parts)).toBe(0)
    expect(total(caster.flow)).toBeCloseTo(160, 9)
  })

  it('drains its flow back to the air, down to its baseline', () => {
    const { sim, caster } = setup('GATH m0, #100\nHALT')
    sim.step()
    expect(total(caster.flow)).toBeCloseTo(160 - 15, 9)
    sim.run(20)
    expect(total(caster.flow)).toBeCloseTo(60, 9)
  })

  it('is harmed past its capacity', () => {
    const { sim, caster } = setup('GATH m0, #300\nCIRC m0\nHALT', (s) => (s.body.capacity.genetics = 200))
    sim.run(3)
    expect(caster.harm).toBeGreaterThan(0)
    expect(caster.condition.body).toBeLessThan(1)
    expect(sim.events.some((e) => e.kind === 'overcharge')).toBe(true)
  })

  it('works only the streams it has', () => {
    const { sim, cast } = setup('GATH m2, #10\nHALT', (s) => (s.body.streams.genetics = 2))
    sim.runCasts()
    expect(cast.fault).toMatch(/NO_STREAM/)
  })

  it('answers to four names only (the Law of the Four)', () => {
    const { sim, cast } = setup('GATH m0, #10\nFILT m1, m0, #4\nHALT')
    sim.runCasts()
    expect(cast.fault).toMatch(/NO_NAME/)
  })

  it('never makes or loses mana (the Law of Conservation)', () => {
    const { sim } = setup(`
        .use  Elements
        IN    n0:2, HAND
        IN    n3:5, AIM
        GATH  m0, #200
        CIRC  m0
        FILT  m1, m0, #AIR
        FILT  m2, m0, #FIRE
        LDI   n6, #20
        SEND  m1, n6, n0:2, n3:5
        VENT  m2
        HALT`)
    const before = sim.ledger().total
    for (let t = 0; t < 30; t++) {
      sim.step()
      expect(sim.ledger().total).toBeCloseTo(before, 6)
    }
  })
})

describe('weaves', () => {
  it('must be set loose before they are locked', () => {
    const { sim, cast } = setup('IN n0:2, AIM\nWEAV n3, n0:2\nLOCK n3, INPUT\nHALT')
    sim.runCasts()
    expect(cast.fault).toMatch(/NOT_LOOSE/)
  })

  it('take nothing more once their input is locked', () => {
    const { sim, cast } = setup(`
        IN    n0:2, AIM
        WEAV  n3, n0:2
        MANI  n3
        LOCK  n3, INPUT
        GATH  m0, #10
        LDI   n4, #0
        LDI   n5, #0
        LDI   n6, #0
        LDI   n7, #5
        EMIT  m0, n7, n3, n4:6
        HALT`)
    sim.runCasts()
    expect(cast.fault).toMatch(/LOCKED/)
  })

  it('fall back into the body when a spell ends with them in hand', () => {
    const { sim, caster } = setup(
      `
        IN    n0:2, AIM
        WEAV  n3, n0:2
        GATH  m0, #40
        LDI   n4, #0
        LDI   n5, #0
        LDI   n6, #0
        LDI   n7, #40
        EMIT  m0, n7, n3, n4:6
        HALT`,
      (s) => (s.body.drain.genetics = 0),
    )
    sim.step()
    expect(sim.weaves.size).toBe(0)
    expect(total(caster.flow)).toBeCloseTo(100, 6)
  })

  it('fray where an order thinks too long: that particle lets go, and the rest of the weave goes on', () => {
    const { sim } = setup(`
        IN    n0:2, AIM
        WEAV  n3, n0:2
        GATH  m0, #40
        LDI   n4, #0
        LDI   n5, #0
        LDI   n6, #0
        LDI   n7, #40
        EMIT  m0, n7, n3, n4:6
        ORDR  n3, spin
        INGR  n3, #0
        MANI  n3
        HALT
spin:   JMP   spin`)
    const before = sim.ledger().total
    sim.run(3)
    expect(sim.events.find((e) => e.kind === 'fray')?.detail).toMatch(/FRAYED/)
    const weave = [...sim.weaves.values()][0]
    expect(weave.particles.length).toBeGreaterThan(0) // the rest are still its caster's, in reach
    expect(weave.particles.every((p) => !p.order)).toBe(true)
    expect(weave.particles.length).toBe(sim.world.particles.length - sim.world.particles.filter((p) => !p.weave).length)
    expect(sim.ledger().total).toBeCloseTo(before, 6)
  })

  it('frees matter when told to condense less than nothing (the flaw)', () => {
    // An earth weave in the ground, 0.6 m down, binds earth. Nothing checks CNDS's sign, so a negative amount runs it backwards.
    const src = (amount: number) => `
        .use  Elements
        LDI   n0, #3.125
        LDI   n1, #1.375
        LDI   n2, #0.125
        WEAV  n3, n0:2
        GATH  m0, #40
        CIRC  m0
        FILT  m1, m0, #EARTH
        LDI   n4, #0
        LDI   n5, #0
        LDI   n6, #0
        MEAS  n7, m1
        EMIT  m1, n7, n3, n4:6
        ORDR  n3, unmake
        PCNT  n8, n3
        LDI   n9, #0
again:  INGR  n3, n9
        ADD   n9, #1
        CMP   n9, n8
        JLT   again
        MANI  n3
        HALT
unmake: CNDS  #${amount}
        RET`
    const freed = (amount: number) => {
      const { sim, world } = setup(src(amount))
      // The ground it's poured into is rock, as full as earth gets: nothing it holds can be pushed into it.
      for (const m of world.matter) if (m[3] > 0) m[3] = packed(3)
      const before = sim.ledger()
      sim.run(2) // it's set loose, and its order runs, in the first tick
      const weave = [...sim.weaves.values()][0]
      const after = sim.ledger()
      expect(after.total).toBeCloseTo(before.total, 6)
      return { free: weave.mana() - 6, condensed: before.condensed - after.condensed } // 40 raw: 6 M of earth
    }
    const flaw = freed(-10)
    expect(flaw.free).toBeGreaterThan(9)
    expect(flaw.condensed).toBeGreaterThan(9)
    // A full cell has no room, so condensing the same amount the right way round makes nothing.
    expect(freed(10).condensed).toBeCloseTo(0, 6)
  })
})

/** A spell that gathers fire and pours it into a weave at a point ahead of the caster, before the code given. */
const pour = (rest: string, gather = 120) => `
        .use  Elements
        GATH  m0, #${gather}
        CIRC  m0
        FILT  m1, m0, #FIRE
        LDI   n0, #4
        LDI   n1, #4
        LDI   n2, #0.125
        WEAV  n3, n0:2
        LDI   n4, #0
        LDI   n5, #0
        LDI   n6, #0
        MEAS  n7, m1
        EMIT  m1, n7, n3, n4:6
${rest}`

describe('mana as a fluid', () => {
  // These look at single particles: none of them merge here (test/physics.test.ts has merging).
  const range = PHYSICS.mergeRange
  beforeEach(() => (PHYSICS.mergeRange = 0))
  afterEach(() => (PHYSICS.mergeRange = range))

  it('pours into particles, held still in the hand, and spreads by its own pressure once let go', () => {
    const { sim } = setup(pour('        TICK\n        TICK\n        TICK\n        TICK\n        MANI  n3\n        HALT'))
    sim.step()
    const weave = [...sim.weaves.values()][0]
    expect(weave.particles.length).toBe(Math.ceil(18 / PHYSICS.mote)) // 120 raw, a quarter fire, 60% kept
    const spread = () => {
      const c = weave.centre()!.pos
      return Math.max(...weave.particles.map((p) => Math.hypot(p.pos[0] - c[0], p.pos[1] - c[1])))
    }
    const laid = spread()
    sim.run(2)
    expect(spread()).toBeCloseTo(laid, 9) // in hand: still
    sim.run(10)
    expect(spread()).toBeGreaterThan(laid * 4)
    expect(sim.world.momentumError()).toBeLessThan(1e-9)
  })

  it('pushes a particle off the body, through its reach, as hard as the mind can transform mana into Energy', () => {
    // One particle: a quarter of 1⅔ M is fire, and 60% of that is 0.25 M.
    const src = pour(
      `        LDI   n8, #0.5
        LDI   n9, #0
        LDI   n10, #0
        LDI   n11, #0
        MEAS  n20, m0
        SHOV  m0, n3, n11, n8:10
        MEAS  n21, m0
        MANI  n3
        HALT`,
      5 / 3,
    )
    const { sim, cast, caster } = setup(src)
    sim.step()
    const weave = [...sim.weaves.values()][0]
    expect(weave.particles.length).toBe(1)
    const p = weave.particles[0]
    expect(p.vel[0]).toBeCloseTo(0.5, 1) // all it asked for (less what the air took)
    // It cost the kinetic energy it added to the particle and to the body it pushed off, and that mana went loose into
    // the air, still mana. The body was pushed back, and its feet on the ground held it.
    const m = 0.25 * PHYSICS.manaMass[0] // 0.25 M of fire; 1⅔ is a float32
    const energy = 0.5 * m * 0.5 ** 2 * (1 + m / caster.body.mass)
    expect(sim.transformed).toBeCloseTo(energy, 5)
    expect(cast.frame.n[20] - cast.frame.n[21]).toBeCloseTo(energy / PHYSICS.pushEnergy, 5)
    expect(sim.world.impulse.walls[0]).toBeCloseTo(m * 0.5, 2) // what the body took, its feet gave the ground
    expect(caster.body.vel[0]).toBe(0)
    // A weaker mind pushes less hard.
    const weak = setup(src, (s) => (s.mind.power.genetics = energy / 4))
    weak.sim.step()
    expect([...weak.sim.weaves.values()][0].particles[0].vel[0]).toBeLessThan(0.3)
    // Out of reach, nothing happens: it can't even be poured there.
    const far = setup(src, (s) => (s.body.reach.genetics = 0.5))
    far.sim.step()
    expect(far.sim.world.particles.length).toBe(0)
    expect(far.cast.frame.n[20]).toBe(far.cast.frame.n[21])
  })

  it('ingrains an order particle by particle, at a beat for every instruction it could run', () => {
    const { sim, cast } = setup(
      pour(`        ORDR  n3, order
        INGR  n3, #0
        INGR  n3, #1
        MANI  n3
        HALT
order:  CALL  sub
        CMP   n0, #0
        JEQ   .out
        NOP
.out:   RET
sub:    NOP
        RET`),
    )
    sim.runCasts()
    expect(orderLength(cast.program, cast.program.labels.get('order')!)).toBe(7)
    const weave = [...sim.weaves.values()][0]
    expect(weave.particles.filter((p) => p.order).length).toBe(2)
    const ingr = [...cast.program.lines].filter(([, l]) => l.text.trim().startsWith('INGR')).map(([a]) => a)
    for (const a of ingr) expect(cast.profile.get(a)!.beats).toBe(4 + 7)
  })

  it('runs an order in every ingrained particle, which burns its own mana to think and pays to push itself', () => {
    const { sim } = setup(
      pour(`        ORDR  n3, kick
        INGR  n3, #0
        MANI  n3
        HALT
kick:   LDI   n0, #0
        LDI   n1, #0.05
        LDI   n2, #0
        KICK  n0:2
        RET`),
    )
    sim.step()
    const weave = [...sim.weaves.values()][0]
    const p = weave.particles[0]
    const before = total(p.free)
    const vy = p.vel[1]
    sim.traceOrders = true
    sim.step()
    const trace = sim.traces.get(weave.id)!
    expect(trace.length).toBe(1) // only the ingrained one
    expect(trace[0].burned).toBeCloseTo(trace[0].beats * PHYSICS.orderBurn, 12)
    // It kicks first, and pays the kinetic energy the kick adds (and a little more, for the air it pushes off); its
    // thinking is paid for after.
    const m = before * PHYSICS.manaMass[0] // fire
    const kickCost = (m * vy * 0.05 + 0.5 * m * 0.05 ** 2) / PHYSICS.pushEnergy
    expect(total(p.free)).toBeCloseTo(before - kickCost - trace[0].burned, 4)
    expect(sim.world.momentumError()).toBeLessThan(1e-9)
  })

  it('lets a particle feel its neighbours: how dense, which way it thickens, how they move', () => {
    const { sim } = setup(
      pour(`        ORDR  n3, feel
        INGR  n3, #0
        MANI  n3
        HALT
feel:   DENS  n5
        GRAD  n6:8
        NVEL  n9:11
        IN    n12:14, VEL
        RET`),
    )
    sim.run(2)
    sim.traceOrders = true
    sim.step()
    const n = [...sim.traces.values()][0][0].n
    expect(n[5]).toBeGreaterThan(0)
    expect(Math.hypot(n[6], n[7], n[8])).toBeGreaterThan(0)
  })

  it('loses particles with no order that stray out of its caster\'s reach, and they settle into the air when they slow', () => {
    // Poured 2.3 m from a caster who reaches 2.4 m: once let go, what spreads further out is no longer theirs.
    const { sim } = setup(pour('        MANI  n3\n        HALT'), (s) => (s.body.reach.genetics = 2.4))
    sim.run(15)
    const weave = [...sim.weaves.values()][0]
    const loose = sim.world.particles.filter((p) => !p.weave)
    expect(weave?.particles.length ?? 0).toBeLessThan(72)
    expect(loose.length + (weave?.particles.length ?? 0)).toBeLessThanOrEqual(72)
    const before = sim.ledger().total
    sim.run(60)
    expect(sim.ledger().total).toBeCloseTo(before, 6)
    expect(sim.world.particles.filter((p) => !p.weave).length).toBeLessThan(loose.length)
  })

  it('passes a weave\'s registers from particle to particle by touch, and not to what nothing touches', () => {
    // Two particles poured together and one far off, each ingrained with an order that does nothing.
    const { sim } = setup(
      `        .use  Elements
        GATH  m0, #40
        CIRC  m0
        FILT  m1, m0, #FIRE
        LDI   n0, #3
        LDI   n1, #4
        LDI   n2, #0.125
        WEAV  n3, n0:2
        LDI   n4, #0
        LDI   n5, #0
        LDI   n6, #0
        LDI   n7, #0.5
        EMIT  m1, n7, n3, n4:6
        LDI   n4, #2
        LDI   n7, #0.25
        EMIT  m1, n7, n3, n4:6
        ORDR  n3, idle
        INGR  n3, #0
        INGR  n3, #1
        INGR  n3, #2
        MANI  n3
        HALT
idle:   RET`,
    )
    sim.step()
    const [a, b, far] = [...sim.weaves.values()][0].particles
    // One of the pair writes w3, as its order would (PUTW): the newest write.
    a.regs[3] = 5
    a.stamp[3] = stampOf(sim.tick, a.id)
    sim.step()
    expect(b.regs[3]).toBe(5) // touching it: it hears
    expect(far.regs[3]).toBe(0) // 2 m off: it doesn't
  })

  it('strains the mind for every joule it transforms, and harms it past its capacity', () => {
    // A strong push, again and again, by a mind of little capacity.
    const src = pour(
      `        MANI  n3
again:  CIRC  m0
        LDI   n8, #0.3
        LDI   n9, #0
        LDI   n10, #0
        PCNT  n5, n3
        LDI   n11, #0
each:   SHOV  m0, n3, n11, n8:10
        NEG   n8
        ADD   n11, #1
        CMP   n11, n5
        JLT   each
        TICK
        JMP   again`,
    )
    const { sim, caster } = setup(src, (s) => {
      s.mind.capacity.genetics = 0.001
      s.mind.recovery.genetics = 0
    })
    sim.run(20)
    expect(sim.transformed).toBeGreaterThan(0.001)
    expect(caster.strain).toBeCloseTo(sim.transformed, 6) // nothing eases it
    expect(caster.madness).toBeGreaterThan(0)
    expect(caster.condition.mind).toBeLessThan(1)
    expect(sim.events.some((e) => e.kind === 'overstrain')).toBe(true)
  })

  it('senses nothing out of reach', () => {
    const { sim, cast } = setup(`        .use  Elements
        LDI   n0, #2.125
        LDI   n1, #1.875
        LDI   n2, #0.125
        PROB  n4, n0:2, #EARTH
        LDI   n0, #8.125
        PROB  n5, n0:2, #EARTH
        HALT`)
    sim.runCasts()
    expect(cast.frame.n[4]).toBe(groundAmount(3)) // the ground under its feet
    expect(cast.frame.n[5]).toBe(0) // 6 m away: it can't tell
  })

  it("won't lock a shape: a shape is held by pushing", () => {
    expect(() => setup('IN n0:2, AIM\nWEAV n3, n0:2\nMANI n3\nLOCK n3, SHAPE\nHALT')).toThrow(/held by pushing/)
  })
})
