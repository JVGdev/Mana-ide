import { afterEach, describe, expect, it } from 'vitest'
import { spell } from '../src/load.ts'
import { fireball, gust, onGround, stoneWall, waterShield, type Scene } from '../src/scenes.ts'
import { PHYSICS } from '../src/vm/physics.ts'
import { total } from '../src/vm/parts.ts'
import { EARTH, WATER, massOf } from '../src/vm/world.ts'

const burn = PHYSICS.orderBurn
afterEach(() => {
  PHYSICS.orderBurn = burn
})

/** Runs, checking every tick that no mana was made or lost, and that momentum only came from outside. */
function run(s: Scene, ticks: number, each?: (tick: number) => void) {
  const before = s.sim.ledger().total
  for (let t = 0; t < ticks; t++) {
    s.sim.step()
    expect(s.sim.ledger().total).toBeCloseTo(before, 4)
    expect(s.sim.world.momentumError()).toBeLessThan(1e-6)
    each?.(t)
  }
}

/** Runs until the spell has ended, and `after` more ticks. */
function finish(s: Scene, cast: { state: string }, after = 0, max = 200) {
  for (let t = 0; t < max && cast.state === 'running'; t++) run(s, 1)
  run(s, after)
}

const carried = (s: Scene, part: number) => {
  let v = 0
  for (const w of s.sim.weaves.values()) for (const p of w.particles) v += p.carried[part]
  return v
}

describe('Stone Wall', () => {
  it('lifts the ground out of a trench and sets it down in front of it, where it stands on its own (2D)', () => {
    const s = stoneWall(2)
    const cast = s.sim.cast(s.caster, spell('StoneWall'))
    const w = s.sim.world
    let massAtRelease = 0
    run(s, 170, () => {
      const weave = s.sim.weaves.get(cast.result ?? 1)
      if (weave && !weave.inHand && !massAtRelease) massAtRelease = weave.particles.reduce((m, p) => m + massOf(p), 0)
    })
    expect(cast.state).toBe('halted')
    const weave = s.sim.weaves.get(cast.result!)!
    expect(weave.particles.every((p) => p.order)).toBe(true) // every particle was ingrained before it moved
    expect(weave.regs[2]).toBe(2) // standing
    // It stands on the ground in front of the trench, 2 m high, its rock whole.
    const ys = weave.particles.map((p) => p.pos[1])
    expect(Math.min(...ys)).toBeGreaterThan(s.ground * w.cell - 0.05)
    expect(Math.max(...ys)).toBeGreaterThan(s.ground * w.cell + 1.9)
    for (const p of weave.particles) expect([13, 14, 15, 16]).toContain(w.coords(w.cellOf(p.pos))[0])
    const foot = weave.particles.filter((p) => p.pos[1] < s.ground * w.cell + 0.25).map((p) => p.pos[0])
    expect(Math.min(...foot)).toBeGreaterThan(3.45) // its foot on the ground in front of the trench
    expect(Math.max(...foot)).toBeLessThan(4.05)
    expect(w.bonds.length).toBeGreaterThan(4000)
    expect(weave.particles.every((p) => Math.abs(p.vel[1]) < 1e-3)).toBe(true) // still: the ground holds it up
    expect(carried(s, EARTH)).toBeGreaterThan(700)
    // Lifting it cost what lifting costs: its weight times the height, at pushEnergy for each M.
    const lift = (massAtRelease * PHYSICS.gravity * 2.05) / PHYSICS.pushEnergy
    expect(s.sim.spent.kick).toBeGreaterThan(lift * 0.85)
    expect(s.sim.spent.kick).toBeLessThan(lift * 1.25)
    // Where it came from is a trench, and what it dropped on the way up lies at the bottom of it.
    for (let y = s.ground - 4; y < s.ground; y++) expect(w.matter[w.index(16, y, 0)][EARTH]).toBeLessThan(PHYSICS.solid)
    expect(w.matter[w.index(13, s.ground - 1, 0)][EARTH]).toBe(100)
  })

  it('is weaker for a caster with little earth in them', () => {
    const strength = (affinity: number) => {
      const s = stoneWall(2)
      s.caster.stats.body.affinity[EARTH].genetics = affinity
      s.sim.cast(s.caster, spell('StoneWall'))
      run(s, 170)
      return carried(s, EARTH)
    }
    expect(strength(0.15)).toBeLessThan(strength(0.6) * 0.5)
  })

  it('fails where there is no earth, before gathering anything', () => {
    const s = stoneWall(2)
    s.caster.will.aim = [6, 4, s.caster.body.pos[2]] // the air
    const cast = s.sim.cast(s.caster, spell('StoneWall'))
    run(s, 2)
    expect(cast.state).toBe('failed')
    expect(cast.code).toBe(1)
    expect(s.caster.held()).toBeCloseTo(60, 6)
  })

  it('crumbles as its order burns its mana away, holding up less and less', () => {
    const s = stoneWall(2)
    s.sim.cast(s.caster, spell('StoneWall'))
    run(s, 170)
    const standing = carried(s, EARTH)
    const bonds = s.sim.world.bonds.length
    PHYSICS.orderBurn = 0.002 // a costlier order: the same wall, standing for less long
    run(s, 120)
    expect(carried(s, EARTH)).toBeLessThan(standing * 0.3)
    expect(s.sim.world.bonds.length).toBeLessThan(bonds * 0.3) // its rock comes apart as it lets go
    const w = s.sim.world
    let fallen = 0
    for (let i = 0; i < w.size; i++) fallen += w.matter[i][EARTH]
    expect(fallen).toBeGreaterThan(64 * 8 * 100 - standing * 0.5) // what it let go of is ground again
  })
})

describe('Fireball', () => {
  for (const dims of [2, 3] as const) {
    it(`holds together in flight, bursts against the pillar, and is gone (${dims}D)`, () => {
      const s = fireball(dims)
      const cast = s.sim.cast(s.caster, spell('Fireball'))
      let burstAt: number | undefined
      let reached = 0
      let released = 0
      let strayed = 0
      run(s, 70, (t) => {
        const weave = s.sim.weaves.get(cast.result ?? -1) ?? [...s.sim.weaves.values()][0]
        if (!weave) return
        if (!weave.inHand && !released) released = weave.mana()
        reached = Math.max(reached, weave.origin[0])
        if (burstAt === undefined && weave.regs[0] === 1) {
          burstAt = t
          strayed = s.sim.strayed
        }
      })
      expect(cast.state).toBe('halted')
      expect(burstAt).toBeDefined()
      expect(reached).toBeGreaterThan(10) // the pillar's face is at 11 m
      expect(strayed).toBeLessThan(released * 0.15) // held by its own order all the way there
      const gone = s.sim.events.find((e) => e.kind === 'dissolve')
      expect(gone?.detail).toBe('its order let it go')
      expect(s.sim.weaves.size).toBe(0)
    })
  }

  it('is poured into one point and held in the hand while its order is ingrained', () => {
    const s = fireball(2)
    const cast = s.sim.cast(s.caster, spell('Fireball'))
    s.sim.step()
    const weave = [...s.sim.weaves.values()][0]
    expect(weave.inHand).toBe(true)
    expect(weave.particles.length).toBe(72) // 120 M gathered, a quarter of it fire, 60% kept: 18 M in 0.25 M particles
    const at = weave.particles.map((p) => [...p.pos])
    s.sim.run(3)
    expect(weave.particles.map((p) => [...p.pos])).toEqual(at) // the hand holds it still
    while (weave.inHand) s.sim.step()
    expect(weave.particles.every((p) => p.order)).toBe(true)
    expect(cast.state).toBe('running') // still to throw
  })

  it("can show any particle's order, instruction by instruction", () => {
    const s = fireball(2)
    const cast = s.sim.cast(s.caster, spell('Fireball'))
    while (s.sim.weaves.get(1)?.inHand !== false) s.sim.step()
    s.sim.step()
    expect(s.sim.traces.size).toBe(0) // only when asked: it costs time
    s.sim.traceOrders = true
    s.sim.step()
    const weave = s.sim.weaves.get(1)!
    const traces = s.sim.traces.get(weave.id)!
    expect(traces.length).toBeGreaterThan(10)
    expect(new Set(traces.map((t) => t.id)).size).toBe(traces.length) // one for each particle that ran its order
    const t = traces[3]
    expect(t.steps[0].addr).toBe(cast.program.labels.get('Fireball.order'))
    expect(Array.from(t.steps[0].n.slice(0, 3))).toEqual(t.off) // it starts knowing where it is from the centre
    expect(t.outcome).toBe('done')
    expect(t.beats).toBeGreaterThan(5)
    expect(t.beats).toBeLessThanOrEqual(64)
    expect(t.burned).toBeCloseTo(t.beats * PHYSICS.orderBurn, 9) // thinking burns its mana
  })
})

describe('Gust', () => {
  it('pushes someone back while it is maintained', () => {
    const s = gust(2)
    const x = s.target.pos[0]
    const cast = s.sim.cast(s.caster, spell('Gust'))
    run(s, 10)
    s.caster.will.maintain = false
    run(s, 15)
    expect(cast.state).toBe('halted')
    expect(s.target.pos[0] - x).toBeGreaterThan(0.25)
    expect(s.caster.harm).toBe(0)
  })

  it('sends a breath every tick it is kept up, and pays for its speed', () => {
    const s = gust(2)
    s.sim.cast(s.caster, spell('Gust'))
    const sent: number[] = []
    run(s, 6, () => sent.push(s.sim.world.particles.filter((p) => !p.weave).reduce((t, p) => t + p.free[2], 0)))
    expect(sent.at(-1)).toBeGreaterThan(sent[0] * 3)
    expect(s.sim.world.impulse.push[0]).toBeGreaterThan(0)
  })

  it('overcharges a caster who keeps it up too long', () => {
    const s = gust(2)
    s.sim.cast(s.caster, spell('Gust'))
    run(s, 40)
    expect(s.caster.harm).toBeGreaterThan(0)
    expect(s.caster.condition.body).toBeLessThan(1)
  })
})

describe('Water Shield', () => {
  it('makes water around the caster, and follows them', () => {
    const s = waterShield(2)
    const cast = s.sim.cast(s.caster, spell('WaterShield'))
    finish(s, cast, 2)
    const weave = s.sim.weaves.get(cast.result!)!
    expect(weave.locks.input).toBe(true)
    expect(carried(s, WATER)).toBeGreaterThan(5)
    s.caster.body.pos[0] += 1
    run(s, 25)
    expect(weave.origin[0]).toBeCloseTo(s.caster.body.pos[0], 1)
  })

  it('falls in a splash as its order burns its mana away', () => {
    const s = waterShield(2)
    const cast = s.sim.cast(s.caster, spell('WaterShield'))
    finish(s, cast, 2)
    const made = carried(s, WATER)
    expect(made).toBeGreaterThan(5)
    PHYSICS.orderBurn = 0.002 // a costlier order: the same shield, holding for less long
    run(s, 80)
    expect(carried(s, WATER)).toBeLessThan(made * 0.05)
    const w = s.sim.world
    let water = 0
    for (let i = 0; i < w.size; i++) water += w.matter[i][WATER]
    expect(water).toBeGreaterThan(made * 0.9)
  })

  it('casts in 3D: a shell around the caster', () => {
    const s = waterShield(3)
    const cast = s.sim.cast(s.caster, spell('WaterShield'))
    finish(s, cast, 2) // about 450 points, each with a sine and a cosine, then every particle ingrained: it takes a while
    expect(cast.state).toBe('halted')
    const weave = s.sim.weaves.get(cast.result!)!
    const w = s.sim.world
    const zs = new Set(weave.particles.map((p) => w.coords(w.cellOf(p.pos))[2]))
    expect(zs.size).toBeGreaterThan(5)
    expect(carried(s, WATER)).toBeGreaterThan(5)
  })
})

describe('scenes', () => {
  it('put the aim on the ground', () => {
    const s = stoneWall(2)
    const p = onGround(s, 6)
    expect(s.sim.world.matter[s.sim.world.cellOf(p)][EARTH]).toBe(100)
    expect(total(s.sim.world.matter[s.sim.world.cellOf([p[0], p[1] + 0.25, p[2]])])).toBe(0)
  })
})
