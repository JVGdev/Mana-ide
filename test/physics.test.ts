// The physics of mana and matter (SPEC §11): weight, the ground, water, rock, and what pushing costs.

import { describe, expect, it } from 'vitest'
import { assemble } from '../src/asm/assembler.ts'
import { resolver } from '../src/load.ts'
import { stepAir } from '../src/vm/air.ts'
import { mergeAndSplit, stepFluid } from '../src/vm/fluid.ts'
import { total } from '../src/vm/parts.ts'
import { PHYSICS } from '../src/vm/physics.ts'
import { Sim } from '../src/vm/sim.ts'
import { EARTH, WATER, World, massOf, type Particle, type Vec } from '../src/vm/world.ts'

const G = PHYSICS.gravity
const parts = (k: number, m: number) => {
  const p = [0, 0, 0, 0]
  p[k] = m
  return p as [number, number, number, number]
}

/** Open ground 2 m deep, no air mana: nothing drags. */
function ground() {
  const w = World.withGround(40, 24, 1, 8)
  for (const a of w.air) a.fill(0)
  return w
}

/** Particles of free mana of part `k`, each holding `held` of matter of part `k`, at `at`. */
function drop(w: World, at: Vec, k: number, n = 1, held = 0): Particle[] {
  const ps = w.pour(parts(k, PHYSICS.mote * n), at, 1)
  for (const p of ps) p.carried[k] = held
  return ps
}

const run = (w: World, ticks: number, each?: () => void) => {
  for (let t = 0; t < ticks; t++) {
    stepFluid(w)
    w.tick++
    expect(w.momentumError()).toBeLessThan(1e-6)
    each?.()
  }
}

describe('weight', () => {
  it('pulls water and earth down at 9.8 m/s², lifts fire a little, and leaves air be', () => {
    const w = ground()
    const water = drop(w, [1, 5, 0.125], WATER)[0]
    const earth = drop(w, [3, 5, 0.125], EARTH, 1, 2)[0]
    const fire = drop(w, [5, 5, 0.125], 0)[0]
    const air = drop(w, [7, 5, 0.125], 2)[0]
    run(w, 10)
    expect(water.vel[1]).toBeCloseTo(-10 * G, 6)
    expect(earth.vel[1]).toBeCloseTo(-10 * G, 6) // a heavier particle falls just as fast
    expect(fire.vel[1]).toBeGreaterThan(0)
    expect(air.vel[1]).toBe(0)
    expect(G * 900).toBeCloseTo(9.81, 6) // ticks are 1/30 s
  })

  it('stops what falls on the ground, which holds it up from then on', () => {
    const w = ground()
    const p = drop(w, [2, 3, 0.125], EARTH, 1, 5)[0]
    run(w, 60)
    expect(p.pos[1]).toBeGreaterThan(2 - 1e-9)
    expect(p.pos[1]).toBeLessThan(2.05)
    expect(Math.abs(p.vel[1])).toBeLessThan(1e-9)
  })

  it('gives matter held by mana its weight and inertia', () => {
    const w = ground()
    const p = drop(w, [2, 3, 0.125], EARTH, 1, 4)[0]
    expect(massOf(p)).toBeCloseTo(PHYSICS.mote + 4 * PHYSICS.matterMass[EARTH], 9)
  })

  it('keeps sliding things from sliding, by friction', () => {
    const w = ground()
    const p = drop(w, [2, 2.01, 0.125], EARTH, 1, 5)[0]
    p.vel[0] = 0.05
    w.impulse.push[0] += massOf(p) * 0.05
    run(w, 30)
    expect(p.vel[0]).toBe(0)
    expect(p.pos[0]).toBeLessThan(2.5)
  })
})

describe('water', () => {
  it('pulls together as it falls, lands and spreads into a puddle on the ground', () => {
    const w = ground()
    const ps = drop(w, [2, 3.2, 0.125], WATER, 12, 4)
    run(w, 120)
    for (const p of ps) {
      expect(p.pos[1]).toBeGreaterThan(1.99)
      expect(p.pos[1]).toBeLessThan(2.3)
      expect(Math.abs(p.vel[1])).toBeLessThan(0.01)
    }
  })

  it('holds together: neighbours close by pull each other in', () => {
    const w = World.withGround(40, 24, 1, 0)
    for (const a of w.air) a.fill(0)
    const saved = PHYSICS.fall[WATER]
    const savedM = PHYSICS.matterFall[WATER]
    PHYSICS.fall[WATER] = 0
    PHYSICS.matterFall[WATER] = 0
    try {
      const [a] = drop(w, [2, 3, 0.125], WATER, 1, 4)
      const [b] = drop(w, [2.18, 3, 0.125], WATER, 1, 4)
      const before = b.pos[0] - a.pos[0]
      run(w, 5)
      expect(b.pos[0] - a.pos[0]).toBeLessThan(before)
    } finally {
      PHYSICS.fall[WATER] = saved
      PHYSICS.matterFall[WATER] = savedM
    }
  })
})

describe('rock', () => {
  function column(w: World) {
    // A column of earth particles 1 m tall, bound like rock, standing on the ground.
    const ps: Particle[] = []
    for (let y = 0; y < 16; y++)
      for (let x = 0; x < 3; x++) ps.push(...drop(w, [2 + x * 0.06, 2.01 + y * 0.06, 0.125], EARTH, 1, 3))
    for (const p of ps) p.pos = [2 + ((p.id - ps[0].id) % 3) * 0.06, 2.01 + Math.floor((p.id - ps[0].id) / 3) * 0.06, 0.125]
    for (let i = 0; i < ps.length; i++)
      for (let j = i + 1; j < ps.length; j++) {
        const d = Math.hypot(ps[i].pos[0] - ps[j].pos[0], ps[i].pos[1] - ps[j].pos[1])
        if (d < PHYSICS.bondRange) w.bonds.push({ a: ps[i], b: ps[j], rest: d })
      }
    return ps
  }

  it('keeps its shape standing on the ground, under its own weight', () => {
    const w = ground()
    const ps = column(w)
    const top = Math.max(...ps.map((p) => p.pos[1]))
    const bonds = w.bonds.length
    run(w, 60)
    expect(Math.max(...ps.map((p) => p.pos[1]))).toBeGreaterThan(top - 0.03)
    expect(w.bonds.length).toBe(bonds)
  })

  it('breaks when it has to hold more than it can', () => {
    const w = ground()
    const ps = column(w)
    const bonds = w.bonds.length
    const saved = PHYSICS.bondStrength
    PHYSICS.bondStrength = 0.001
    try {
      run(w, 30)
    } finally {
      PHYSICS.bondStrength = saved
    }
    expect(w.bonds.length).toBeLessThan(bonds)
    void ps
  })
})

describe('what pushing costs', () => {
  /** A particle poured at the hand, let go of, and kicked by its own order: `kicks` is how, tick by tick. */
  function kicked(src: string) {
    const world = World.withGround(32, 24, 1, 8)
    const sim = new Sim(world)
    const caster = sim.addCaster('Mage', [2, 2.9, 0.125])
    caster.will = { aim: [5, 2.9, 0.125], amount: 100, force: 0.5, maintain: false }
    const program = assemble(
      `       GATH  m0, #100
        LDI   n0, #3
        LDI   n1, #4
        LDI   n2, #0.125
        WEAV  n3, n0:2
        LDI   n4, #0
        LDI   n5, #0
        LDI   n6, #0
        LDI   n7, #0.25
        EMIT  m0, n7, n3, n4:6
        ORDR  n3, kick
        INGR  n3, #0
        MANI  n3
        HALT
kick:   ${src}`,
      'test.masm',
      resolver(),
    )
    sim.cast(caster, program)
    sim.run(3)
    return { sim, p: world.particles.find((p) => p.weave)! }
  }

  it('costs the kinetic energy a push adds', () => {
    // Up 0.05 m/tick every tick: it pays for what it gains against its weight, and for its speed.
    const { sim, p } = kicked(`LDI n5, #0
        LDI   n6, #0.05
        LDI   n7, #0
        KICK  n5:7
        RET`)
    const spent = sim.spent.kick
    expect(spent).toBeGreaterThan(0)
    expect(spent).toBeLessThan((0.5 * massOf(p) * 0.1 ** 2 + massOf(p) * G * 1) / PHYSICS.pushEnergy + 0.01)
  })

  it('costs nothing to slow down, or to hold something up against its weight', () => {
    // Every tick, it kicks itself back to standing still: it only ever takes speed away.
    const { sim, p } = kicked(`IN n5:7, VEL
        NEG   n5
        NEG   n6
        NEG   n7
        KICK  n5:7
        RET`)
    sim.run(20)
    expect(sim.spent.kick).toBe(0)
    expect(Math.abs(p.vel[1])).toBeLessThanOrEqual(G + 1e-9) // it holds itself up, a tick's fall at most
  })

  it('costs more the faster it already goes the same way', () => {
    // Forward 0.05 m/tick, once, from standing and from 0.3 m/tick.
    const cost = (v: number) => {
      const { sim, p } = kicked(`CMP n4, #4
        JNE   .done
        LDI   n5, #0.05
        LDI   n6, #0
        LDI   n7, #0
        KICK  n5:7
.done:  RET`)
      p.vel = [v, p.vel[1], 0]
      sim.world.impulse.push[0] += massOf(p) * v
      const before = sim.spent.kick
      sim.run(2)
      return sim.spent.kick - before
    }
    const still = cost(0)
    const moving = cost(0.3)
    expect(still).toBeGreaterThan(0)
    expect(moving).toBeGreaterThan(still * 5)
  })
})

describe('merging and splitting', () => {
  it('merges particles at rest beside each other, keeping their mana, matter and momentum', () => {
    const w = ground()
    const [a] = drop(w, [2, 3, 0.125], WATER, 1, 2)
    const [b] = drop(w, [2.05, 3, 0.125], WATER, 1, 1)
    a.pos = [2, 3, 0.125]
    b.pos = [2.05, 3, 0.125]
    a.vel = [0.001, 0, 0]
    b.vel = [0.003, 0, 0]
    w.impulse.push[0] += massOf(a) * 0.001 + massOf(b) * 0.003
    const changes = mergeAndSplit(w)
    expect(changes).toEqual([{ kind: 'merge', into: a, from: b, weave: 1 }])
    expect(w.particles).toEqual([a])
    expect(a.free[WATER]).toBeCloseTo(2 * PHYSICS.mote, 12)
    expect(a.carried[WATER]).toBe(3)
    expect(w.momentumError()).toBeLessThan(1e-12)
    expect(a.pos[0]).toBeGreaterThan(2)
    expect(a.pos[0]).toBeLessThan(2.05)
  })

  it("doesn't merge particles moving apart, or ones that would be too big", () => {
    const w = ground()
    const [a] = drop(w, [2, 3, 0.125], WATER)
    const [b] = drop(w, [2.05, 3, 0.125], WATER)
    a.pos = [2, 3, 0.125]
    b.pos = [2.05, 3, 0.125]
    b.vel = [0.02, 0, 0]
    expect(mergeAndSplit(w).filter((c) => c.kind === 'merge')).toEqual([])
    b.vel = [0, 0, 0]
    a.free[WATER] = PHYSICS.maxMote
    expect(mergeAndSplit(w).filter((c) => c.kind === 'merge')).toEqual([])
  })

  it('splits a big particle that has spread thin, and both halves keep its weave and order', () => {
    const w = ground()
    const [p] = drop(w, [2, 4, 0.125], WATER, 1)
    p.free[WATER] = 1
    p.order = { program: { bytes: new Uint8Array(), labels: new Map(), lines: new Map() } as never, addr: 7 }
    const changes = mergeAndSplit(w)
    expect(changes.map((c) => c.kind)).toEqual(['split'])
    expect(w.particles.length).toBe(2)
    for (const q of w.particles) {
      expect(q.free[WATER]).toBeCloseTo(0.5, 12)
      expect(q.weave).toBe(1)
      expect(q.order).toBe(p.order)
    }
    mergeAndSplit(w)
    expect(w.particles.length).toBe(4) // alone, they split again, down to motes
  })

  it('leaves rock and what is in a hand whole', () => {
    const w = ground()
    const [a] = drop(w, [2, 3, 0.125], EARTH, 1, 2)
    const [b] = drop(w, [2.05, 3, 0.125], EARTH, 1, 2)
    a.pos = [2, 3, 0.125]
    b.pos = [2.05, 3, 0.125]
    w.bonds.push({ a, b, rest: 0.05 })
    expect(mergeAndSplit(w)).toEqual([])
    w.bonds = []
    expect(mergeAndSplit(w, { still: () => true })).toEqual([])
  })
})

describe('the second flaw', () => {
  it("lets a big ordered particle take over someone else's mana that comes to rest against it", () => {
    const w = ground()
    const [a] = drop(w, [2, 3, 0.125], EARTH)
    const [b] = drop(w, [2.05, 3, 0.125], EARTH)
    a.pos = [2, 3, 0.125]
    b.pos = [2.05, 3, 0.125]
    a.free[EARTH] = 0.5
    b.weave = 2
    a.order = { program: { bytes: new Uint8Array(), labels: new Map(), lines: new Map() } as never, addr: 3 }
    const [c] = mergeAndSplit(w)
    expect(c).toMatchObject({ kind: 'merge', into: a, from: b, weave: 2 })
    expect(a.weave).toBe(1)
    expect(a.free[EARTH]).toBeCloseTo(0.75, 12) // weave 2's mana is weave 1's now, and runs its order
  })

  it("spreads an order through another caster's resting mana, and says so", () => {
    const world = World.withGround(32, 24, 1, 8)
    const sim = new Sim(world)
    const lay = (name: string, order: boolean) => {
      const caster = sim.addCaster(name, [name === 'A' ? 1 : 1.5, 2.9, 0.125])
      caster.will = { aim: [3, 2.9, 0.125], amount: 100, force: 0, maintain: false }
      const program = assemble(
        `       .use  Elements
        GATH  m0, #100
        FILT  m1, m0, #EARTH
        LDI   n0, #3
        LDI   n1, #2.1
        LDI   n2, #0.125
        WEAV  n3, n0:2
        LDI   n4, #0
        LDI   n5, #0
        LDI   n6, #0
        LDI   n7, #${order ? 0.75 : 0.25}
        EMIT  m1, n7, n3, n4:6
        HOLD  n3, #2
${order ? '        ORDR  n3, still\n        INGR  n3, #0\n        INGR  n3, #1\n        INGR  n3, #2\n' : ''}        MANI  n3
        HALT
still:  RET`,
        `${name}.masm`,
        resolver(),
      )
      sim.cast(caster, program)
      sim.step()
    }
    // A's three motes settle on the ground and merge into one particle of 0.75 M, ordered. Then B's one mote lands beside it.
    lay('A', true)
    sim.run(30)
    lay('B', false)
    const b = [...sim.weaves.values()].find((wv) => wv.maker.name === 'B')!
    const before = b.mana()
    sim.run(60)
    const taken = sim.events.filter((e) => e.kind === 'taken')
    expect(taken.length).toBeGreaterThan(0)
    expect(taken[0].detail).toMatch(/weave \d+'s mana, and gave it its order/)
    expect(b.mana()).toBeLessThan(before)
  })
})

describe('the air', () => {
  const airMana = (w: World, x0: number, x1: number, y0: number, y1: number) => {
    let m = 0
    for (let y = y0; y < y1; y++) for (let x = x0; x < x1; x++) m += total(w.air[w.index(x, y, 0)])
    return m
  }
  const airRun = (w: World, ticks: number, each?: () => void) => {
    let mana = 0
    for (const a of w.air) mana += total(a)
    for (let t = 0; t < ticks; t++) {
      stepAir(w)
      let now = 0
      for (const a of w.air) now += total(a)
      expect(now).toBeCloseTo(mana, 6)
      expect(w.momentumError()).toBeLessThan(1e-6)
      each?.()
    }
  }
  /** Air in cells x0–x1, y0–y1 set moving at `u` m/tick along x, its momentum given from outside. */
  const blow = (w: World, x0: number, x1: number, y0: number, y1: number, u: number) => {
    for (let y = y0; y < y1; y++)
      for (let x = x0; x < x1; x++) {
        const i = w.index(x, y, 0)
        w.airVel[i * 3] = u
        w.impulse.push[0] += total(w.air[i]) * u
      }
  }

  it('carries itself along: a puff of wind travels on, and takes its mana with it', () => {
    const w = World.withGround(48, 24, 1, 8)
    blow(w, 4, 8, 12, 16, 0.3)
    const centre = () => {
      let m = 0
      let x = 0
      for (let i = 0; i < w.size; i++) {
        const p = total(w.air[i]) * w.airVel[i * 3]
        m += p
        x += p * w.coords(i)[0]
      }
      return x / m
    }
    const start = centre()
    airRun(w, 15)
    expect(centre()).toBeGreaterThan(start + 3) // the moving air is cells further on
  })

  it('flows around what stands in its way', () => {
    const w = World.withGround(48, 24, 1, 8)
    w.fillBox([24, 8, 0], [26, 13, 0], EARTH) // a pillar 1.5 m high
    blow(w, 8, 20, 8, 20, 0.2)
    let up = 0
    airRun(w, 20, () => {
      for (let y = 8; y < 14; y++) up = Math.max(up, w.airVel[w.index(23, y, 0) * 3 + 1])
    })
    expect(up).toBeGreaterThan(0.02) // in front of the pillar, the wind turns up and over it
    for (let y = 8; y < 14; y++) expect(total(w.air[w.index(25, y, 0)])).toBe(0) // and none goes through it
  })

  it('fills back in where mana was taken out of it', () => {
    const w = World.withGround(48, 24, 1, 8)
    for (let y = 12; y < 16; y++) for (let x = 20; x < 24; x++) w.air[w.index(x, y, 0)].fill(0)
    const hole = () => airMana(w, 20, 24, 12, 16)
    airRun(w, 60)
    expect(hole()).toBeGreaterThan(16 * PHYSICS.airMana * 0.5)
  })

  it('presses from dense to thin', () => {
    const w = World.withGround(48, 24, 1, 8)
    const i = w.index(20, 14, 0)
    w.air[i].fill(PHYSICS.airMana / 2) // twice as dense as around it
    const around = () => airMana(w, 17, 24, 11, 18) - total(w.air[i])
    const before = around()
    airRun(w, 1)
    expect(total(w.air[i])).toBeLessThan(PHYSICS.airMana * 2)
    expect(around()).toBeGreaterThan(before) // what it pushed out is around it
  })
})
