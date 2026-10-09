import { describe, expect, it } from 'vitest'
import { assemble } from '../src/asm/assembler.ts'
import { resolver } from '../src/load.ts'
import { adept, type CasterStats } from '../src/vm/caster.ts'
import { total } from '../src/vm/parts.ts'
import { Sim } from '../src/vm/sim.ts'
import { World } from '../src/vm/world.ts'

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

  it('counts a cast toward conditioning when it halts', () => {
    const { sim, cast, caster } = setup('HALT')
    sim.runCasts()
    expect(caster.conditioning.get(cast.name)).toBe(1)
  })

  it("won't run an order's instructions", () => {
    const { sim, cast } = setup('MOVE n0:2\nHALT')
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
    const { sim, cast } = setup('IN n0:2, AIM\nWEAV n3, n0:2\nLOCK n3, SHAPE\nHALT')
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

  it('fray when an order thinks too long', () => {
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
        MANI  n3
        HALT
spin:   JMP   spin`)
    const before = sim.ledger().total
    sim.run(3)
    expect(sim.events.find((e) => e.kind === 'fray')?.detail).toMatch(/FRAYED/)
    expect(sim.weaves.size).toBe(0)
    expect(sim.ledger().total).toBeCloseTo(before, 6)
  })

  it('can be made to free matter, though nobody teaches it', () => {
    // An earth weave in the ground binds earth; LOOS turns some of it back into free mana.
    const { sim, world } = setup(`
        .use  Elements
        LDI   n0, #3.125
        LDI   n1, #1.875
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
        ORDR  n3, loosen
        MANI  n3
        HALT
loosen: LOOS  #10
        RET`)
    const before = sim.ledger()
    sim.step()
    const weave = [...sim.weaves.values()][0]
    const free = total(weave.cells[0].free)
    sim.step()
    expect(total(weave.cells[0].free)).toBeGreaterThan(free + 9)
    const after = sim.ledger()
    expect(after.total).toBeCloseTo(before.total, 6)
    expect(after.condensed).toBeLessThan(before.condensed - 9)
    expect(world.matter.length).toBeGreaterThan(0)
  })
})
