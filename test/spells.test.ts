import { afterEach, describe, expect, it } from 'vitest'
import { spell } from '../src/load.ts'
import { fireball, gust, onGround, stoneWall, waterShield, type Scene } from '../src/scenes.ts'
import { PHYSICS } from '../src/vm/physics.ts'
import { total } from '../src/vm/parts.ts'
import { EARTH, WATER } from '../src/vm/world.ts'

const leak = PHYSICS.leak
afterEach(() => {
  PHYSICS.leak = leak
})

/** Runs, checking every tick that no mana was made or lost. */
function run(s: Scene, ticks: number, each?: (tick: number) => void) {
  const before = s.sim.ledger().total
  for (let t = 0; t < ticks; t++) {
    s.sim.step()
    expect(s.sim.ledger().total).toBeCloseTo(before, 4)
    each?.(t)
  }
}

const carried = (s: Scene, part: number) => {
  let v = 0
  for (const w of s.sim.weaves.values()) for (const c of w.cells) v += c.carried[part]
  return v
}

describe('Stone Wall', () => {
  it('lifts the ground into a wall and leaves a trench (2D)', () => {
    const s = stoneWall(2)
    const cast = s.sim.cast(s.caster, spell('StoneWall'))
    run(s, 12)
    expect(cast.state).toBe('halted')
    const weave = s.sim.weaves.get(cast.result!)!
    expect(weave.locks.shape).toBe(true)
    const w = s.sim.world
    // Every cell of the wall stands above the ground now, holding its earth.
    for (const c of weave.cells) {
      const [x, y] = w.coords(w.cellOf(weave.worldPos(c)))
      expect([24, 25]).toContain(x)
      expect(y).toBeGreaterThanOrEqual(s.ground)
      expect(c.carried[EARTH]).toBeGreaterThan(50)
    }
    // Where it came from is a trench.
    for (let y = 1; y < s.ground; y++) expect(w.matter[w.index(24, y, 0)][EARTH]).toBeLessThan(PHYSICS.solid)
    // The ground beside it holds.
    expect(w.matter[w.index(23, s.ground - 1, 0)][EARTH]).toBe(100)
  })

  it('stands in 3D too: 4 m long', () => {
    const s = stoneWall(3)
    const cast = s.sim.cast(s.caster, spell('StoneWall'))
    run(s, 30)
    expect(cast.state).toBe('halted')
    const weave = s.sim.weaves.get(cast.result!)!
    const w = s.sim.world
    const zs = new Set(weave.cells.map((c) => w.coords(w.cellOf(weave.worldPos(c)))[2]))
    expect(zs.size).toBe(16) // 4 m of cells across the caster's line of sight
    expect(carried(s, EARTH)).toBeGreaterThan(16 * 2 * 8 * 50)
  })

  it('is weaker for a caster with little earth in them', () => {
    const strength = (affinity: number) => {
      const s = stoneWall(2)
      s.caster.stats.body.affinity[EARTH].genetics = affinity
      s.sim.cast(s.caster, spell('StoneWall'))
      run(s, 12)
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

  it('crumbles back into its trench as its mana leaks', () => {
    PHYSICS.leak = 0.05
    const s = stoneWall(2)
    s.sim.cast(s.caster, spell('StoneWall'))
    run(s, 12)
    const standing = carried(s, EARTH)
    run(s, 80)
    expect(carried(s, EARTH)).toBeLessThan(standing * 0.1)
    const w = s.sim.world
    let fallen = 0
    for (let x = 20; x < 30; x++) for (let y = 0; y < s.ground + 8; y++) fallen += w.matter[w.index(x, y, 0)][EARTH]
    expect(fallen - 8 * 8 * 100).toBeGreaterThan(standing * 0.9) // the columns either side were already ground
    // Nothing is left standing: no earth hangs in the air above the ground's top.
    for (let y = s.ground + 1; y < s.ground + 8; y++) expect(w.matter[w.index(24, y, 0)][EARTH]).toBeLessThan(PHYSICS.solid)
  })
})

describe('Fireball', () => {
  for (const dims of [2, 3] as const) {
    it(`flies, bursts against the pillar, and is gone (${dims}D)`, () => {
      const s = fireball(dims)
      const cast = s.sim.cast(s.caster, spell('Fireball'))
      let burstAt: number | undefined
      let reached = 0
      run(s, 60, (t) => {
        const weave = s.sim.weaves.get(cast.result ?? -1)
        if (!weave) return
        reached = Math.max(reached, weave.origin[0])
        if (burstAt === undefined && weave.regs[0] === 1) burstAt = t
      })
      expect(cast.state).toBe('halted')
      expect(burstAt).toBeDefined()
      expect(reached).toBeGreaterThan(9.5) // the pillar's face is at 11 m
      const gone = s.sim.events.find((e) => e.kind === 'dissolve')
      expect(gone?.detail).toBe('its order let it go')
      expect(s.sim.weaves.size).toBe(0)
    })
  }

  it('is a ball: a disc of cells in 2D, a sphere in 3D, with no holes', () => {
    for (const dims of [2, 3] as const) {
      const s = fireball(dims)
      const cast = s.sim.cast(s.caster, spell('Fireball'))
      while (cast.state === 'running') s.sim.step()
      const weave = s.sim.weaves.get(cast.result!)!
      const cells = new Set(weave.cells.map((c) => c.off.map((v) => Math.round(v / 0.25)).join()))
      const r = 2 // 0.5 m, in cells
      for (let x = -r; x <= r; x++)
        for (let y = -r; y <= r; y++)
          for (let z = dims === 2 ? 0 : -r; z <= (dims === 2 ? 0 : r); z++) {
            const d = Math.hypot(x, y, z)
            if (d <= r) expect(cells.has([x, y, z].join())).toBe(true) // every cell inside
            if (d > r + 0.5) expect(cells.has([x, y, z].join())).toBe(false) // nothing past the rim
          }
      const fire = weave.cells.reduce((sum, c) => sum + total(c.free), 0)
      expect(fire).toBeCloseTo(120 * 0.25 * 0.6, 1) // all the fire it was given, less a tick's leak
    }
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
    run(s, 4)
    const weave = s.sim.weaves.get(cast.result!)!
    expect(weave.locks.shape && weave.locks.input).toBe(true)
    expect(carried(s, WATER)).toBeGreaterThan(5)
    s.caster.body.pos[0] += 1
    run(s, 3)
    expect(weave.origin[0]).toBeCloseTo(s.caster.body.pos[0], 6)
  })

  it('sags and falls in a splash as its mana leaks', () => {
    PHYSICS.leak = 0.3
    const s = waterShield(2)
    s.sim.cast(s.caster, spell('WaterShield'))
    run(s, 4)
    const made = carried(s, WATER)
    run(s, 40)
    expect(carried(s, WATER)).toBeLessThan(made * 0.05)
    const w = s.sim.world
    let water = 0
    for (let i = 0; i < w.size; i++) water += w.matter[i][WATER]
    expect(water).toBeGreaterThan(made * 0.9)
  })

  it('casts in 3D: a shell around the caster', () => {
    const s = waterShield(3)
    const cast = s.sim.cast(s.caster, spell('WaterShield'))
    run(s, 50) // about 450 points: an adept's mind takes a while
    expect(cast.state).toBe('halted')
    const weave = s.sim.weaves.get(cast.result!)!
    const w = s.sim.world
    const zs = new Set(weave.cells.map((c) => w.coords(w.cellOf(weave.worldPos(c)))[2]))
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
