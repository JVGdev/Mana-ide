// The world without minds (PLAN step R3): scenarios from the physics and Energy tests, and a free ball of fire in the
// air, stepped tick by tick. engine/mana/tests/parity_world.rs runs the same ones.

import { mkdirSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { stepAir } from '../../src/vm/air.ts'
import { stored } from '../../src/vm/energy.ts'
import { mergeAndSplit, stepFluid } from '../../src/vm/fluid.ts'
import { PHYSICS } from '../../src/vm/physics.ts'
import { EARTH, WATER, World, type Particle, type Vec } from '../../src/vm/world.ts'
import { state } from './state.ts'

const out = join(import.meta.dirname, '..', 'target', 'parity')
mkdirSync(out, { recursive: true })

const parts = (k: number, m: number) => {
  const p = [0, 0, 0, 0]
  p[k] = m
  return p as [number, number, number, number]
}
function ground(air = false) {
  const w = World.withGround(40, 24, 1, 8)
  if (!air) for (const a of w.air) a.fill(0)
  return w
}
function drop(w: World, at: Vec, k: number, n = 1, held = 0): Particle[] {
  const ps = w.pour(parts(k, PHYSICS.mote * n), at, 1)
  for (const p of ps) p.carried[k] = held
  return ps
}
function column(w: World) {
  const ps: Particle[] = []
  for (let y = 0; y < 16; y++) for (let x = 0; x < 3; x++) ps.push(...drop(w, [2 + x * 0.06, 2.01 + y * 0.06, 0.125], EARTH, 1, 3))
  for (const p of ps) p.pos = [2 + ((p.id - ps[0].id) % 3) * 0.06, 2.01 + Math.floor((p.id - ps[0].id) / 3) * 0.06, 0.125]
  for (let i = 0; i < ps.length; i++)
    for (let j = i + 1; j < ps.length; j++) {
      const d = Math.hypot(ps[i].pos[0] - ps[j].pos[0], ps[i].pos[1] - ps[j].pos[1])
      if (d < PHYSICS.bondRange) w.bonds.push({ a: ps[i], b: ps[j], rest: d })
    }
}
const blow = (w: World, x0: number, x1: number, y0: number, y1: number, u: number) => {
  for (let y = y0; y < y1; y++)
    for (let x = x0; x < x1; x++) {
      const i = w.index(x, y, 0)
      w.airVel[i * 3] = u
      w.impulse.outside[0] += w.airMass(i) * u
    }
}

type Scenario = { name: string; world: () => World; ticks: number; step: (w: World) => void; tune?: Partial<typeof PHYSICS> }
const fluid = (w: World) => {
  stepFluid(w)
  w.tick++
}
const everything = (w: World) => {
  stepFluid(w)
  stepAir(w)
  mergeAndSplit(w)
  w.tick++
}
const scenarios: Scenario[] = [
  { name: 'weight', ticks: 10, step: fluid, world: () => {
    const w = ground()
    drop(w, [1, 5, 0.125], WATER); drop(w, [3, 5, 0.125], EARTH, 1, 2); drop(w, [5, 5, 0.125], 0); drop(w, [7, 5, 0.125], 2)
    return w
  } },
  { name: 'buoyancy', ticks: 10, step: fluid, world: () => {
    const w = ground(true)
    drop(w, [1, 4, 0.125], 0); drop(w, [3, 4, 0.125], 2); drop(w, [5, 4, 0.125], WATER); drop(w, [7, 4, 0.125], EARTH); drop(w, [9, 4, 0.125], EARTH, 1, 4)
    return w
  } },
  { name: 'landing', ticks: 60, step: fluid, world: () => {
    const w = ground()
    drop(w, [2, 3, 0.125], EARTH, 1, 5)
    return w
  } },
  { name: 'friction', ticks: 30, step: fluid, world: () => {
    const w = ground()
    const p = drop(w, [2, 2.01, 0.125], EARTH, 1, 5)[0]
    p.vel[0] = 0.05
    return w
  } },
  { name: 'puddle', ticks: 120, step: fluid, world: () => {
    const w = ground()
    drop(w, [2, 3.2, 0.125], WATER, 12, 4)
    return w
  } },
  { name: 'cohesion', ticks: 5, step: fluid, tune: { gravity: 0 }, world: () => {
    const w = World.withGround(40, 24, 1, 0)
    for (const a of w.air) a.fill(0)
    drop(w, [2, 3, 0.125], WATER, 1, 4); drop(w, [2.18, 3, 0.125], WATER, 1, 4)
    return w
  } },
  { name: 'column', ticks: 60, step: fluid, world: () => {
    const w = ground()
    column(w)
    return w
  } },
  { name: 'cracks', ticks: 30, step: fluid, tune: { bondStrength: 0.001 }, world: () => {
    const w = ground()
    column(w)
    return w
  } },
  { name: 'merging', ticks: 6, step: (w) => { mergeAndSplit(w); w.tick++ }, world: () => {
    const w = ground()
    const [a] = drop(w, [2, 3, 0.125], WATER, 1, 2)
    const [b] = drop(w, [2.05, 3, 0.125], WATER, 1, 1)
    a.pos = [2, 3, 0.125]; b.pos = [2.05, 3, 0.125]; a.vel = [0.001, 0, 0]; b.vel = [0.003, 0, 0]
    const [p] = drop(w, [6, 4, 0.125], WATER, 1)
    p.free[WATER] = 1
    return w
  } },
  { name: 'wind', ticks: 15, step: (w) => stepAir(w), world: () => {
    const w = World.withGround(48, 24, 1, 8)
    blow(w, 4, 8, 12, 16, 0.3)
    return w
  } },
  { name: 'pillar', ticks: 20, step: (w) => stepAir(w), world: () => {
    const w = World.withGround(48, 24, 1, 8)
    w.fillBox([24, 8, 0], [26, 13, 0], EARTH)
    blow(w, 8, 20, 8, 20, 0.2)
    return w
  } },
  { name: 'hole', ticks: 60, step: (w) => stepAir(w), world: () => {
    const w = World.withGround(48, 24, 1, 8)
    for (let y = 12; y < 16; y++) for (let x = 20; x < 24; x++) w.air[w.index(x, y, 0)].fill(0)
    return w
  } },
  { name: 'drag', ticks: 10, step: fluid, world: () => {
    const w = ground(true)
    const [p] = w.pour([0, 0, PHYSICS.mote, 0], [3, 4, 0.125], 1)
    p.vel = [0.2, 0, 0]
    return w
  } },
  { name: 'fireball', ticks: 80, step: everything, world: () => {
    const w = World.withGround(64, 32, 1, 8)
    w.fillBox([44, 8, 0], [47, 20, 0], EARTH)
    w.pour([18, 0, 0, 0], [3.1, 3.1, 0.125], 0, [0.3, 0.02, 0])
    w.pour([0, 6, 0, 2], [5, 3, 0.125], 0, [0, 0, 0])
    w.addBody('Target', [6, 2.9, 0.125])
    return w
  } },
  { name: 'fireball3d', ticks: 25, step: everything, world: () => {
    const w = World.withGround(24, 16, 24, 8)
    w.pour([18, 0, 0, 0], [2.1, 3.1, 3.1], 0, [0.2, 0.01, 0.05])
    w.addBody('Target', [4, 2.9, 3])
    return w
  } },
]

const saved = { ...PHYSICS }
for (const s of scenarios) {
  Object.assign(PHYSICS, saved, s.tune ?? {})
  const w = s.world()
  const ticks = [{ ...state(w), stored: stored(w) }]
  for (let t = 0; t < s.ticks; t++) {
    s.step(w)
    ticks.push({ ...state(w), stored: stored(w) })
  }
  writeFileSync(join(out, `world-${s.name}.json`), JSON.stringify(ticks))
}
Object.assign(PHYSICS, saved)
console.log(`${scenarios.length} world scenarios`)
