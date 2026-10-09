// The machine (PLAN step R4): the four spells cast in their scenes, tick by tick, with everything a caster, a cast and a
// weave hold. engine/mana/tests/parity_machine.rs casts the same ones.

import { mkdirSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { spell } from '../../src/load.ts'
import { SCENES } from '../../src/scenes.ts'
import { PHYSICS } from '../../src/vm/physics.ts'
import type { Sim } from '../../src/vm/sim.ts'
import { state } from './state.ts'

const out = join(import.meta.dirname, '..', 'target', 'parity')
mkdirSync(out, { recursive: true })

export function machine(sim: Sim, full = true) {
  const parts = (p: number[]) => [...p]
  return {
    ...state(sim.world, full),
    casters: sim.casters.map((c) => [
      c.name, sim.world.bodies.indexOf(c.body), ...c.flow, ...c.regs.flatMap((r) => [...parts(r.parts), r.holdUntil]),
      c.harm, c.strain, c.madness, c.powerLeft, c.condition.body, c.condition.mind,
    ]),
    casts: sim.casts.map((c) => [
      c.state, c.frame.pc, ...c.frame.n, c.frame.flags, c.left, c.beats, c.result ?? -1, c.code ?? -1, c.fault ?? '',
      c.endedAt ?? -1, [...c.frame.stack], [...c.frame.calls], c.yielded,
    ]),
    weaves: [...sim.weaves.values()].map((w) => [
      w.id, w.particles.map((p) => p.id), ...w.origin, w.yaw, w.inHand, ...w.regs, ...w.stamp, w.order ?? -1, w.manifestedAt,
      w.lastLeft, w.locks.input, w.locks.order,
    ]),
    events: sim.events.map((e) => [e.tick, e.kind, e.caster ?? '', e.weave ?? -1, e.detail ?? '']),
    spent: [sim.spent.push, sim.spent.kick, sim.spent.burn, sim.transformed, sim.strayed],
    ledger: sim.ledger(),
    energy: sim.trackEnergy ? [sim.energy.start, sim.energy.outside, sim.energy.minds, sim.energy.bodies, Object.entries(sim.energy.error)] : [],
    traces: sim.traceOrders
      ? [...sim.traces].map(([id, ts]) => [id, ts.map((t) => [
          t.tick, t.weave, t.particle, t.id, ...t.off, [...t.w], t.steps.map((s) => [s.addr, ...s.n]), [...t.n], t.outcome, t.beats, t.burned, ...t.kick, t.cnds,
        ])])
      : [],
  }
}

type Scenario = { name: string; scene: keyof typeof SCENES; dims: 2 | 3; ticks: number; energy?: boolean; trace?: boolean; full?: boolean; then?: { ticks: number; tune: Partial<typeof PHYSICS> } }
const scenarios: Scenario[] = [
  { name: 'fireball', scene: 'Fireball', dims: 2, ticks: 70 },
  { name: 'fireball-energy', scene: 'Fireball', dims: 2, ticks: 70, energy: true, trace: true },
  { name: 'fireball3d', scene: 'Fireball', dims: 3, ticks: 45 },
  { name: 'gust', scene: 'Gust', dims: 2, ticks: 40, energy: true },
  { name: 'watershield', scene: 'WaterShield', dims: 2, ticks: 90, energy: true },
  { name: 'watershield3d', scene: 'WaterShield', dims: 3, ticks: 30 },
  { name: 'stonewall', scene: 'StoneWall', dims: 2, ticks: 230, full: false, then: { ticks: 60, tune: { orderBurn: 0.002 } } },
  { name: 'stonewall-energy', scene: 'StoneWall', dims: 2, ticks: 120, full: false, energy: true },
]

const saved = { ...PHYSICS }
const only = process.env.MANA_PARITY_ONLY
for (const s of scenarios) {
  if (only && !s.name.startsWith(only)) continue
  Object.assign(PHYSICS, saved)
  const scene = SCENES[s.scene](s.dims)
  const sim = scene.sim
  if (s.trace) sim.traceOrders = true
  if (s.energy) sim.keepEnergy()
  sim.cast(scene.caster, spell(s.scene))
  const full = process.env.MANA_PARITY_FULL ? true : s.full ?? true
  const ticks = [machine(sim, full)]
  for (let t = 0; t < s.ticks; t++) {
    sim.step()
    ticks.push(machine(sim, full))
  }
  if (s.then) {
    Object.assign(PHYSICS, s.then.tune)
    for (let t = 0; t < s.then.ticks; t++) {
      sim.step()
      ticks.push(machine(sim, full))
    }
  }
  const final = sim.casts.map((c) => [...c.profile].map(([a, p]) => [a, p.runs, p.beats]))
  writeFileSync(join(out, `machine-${s.name}.json`), JSON.stringify({ ticks, profile: final }))
}
Object.assign(PHYSICS, saved)
console.log(`${scenarios.length} machine scenarios`)
