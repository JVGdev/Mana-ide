// The bench: the same ball of fire, held or thrown in different ways, on the real machine, measured the same way. It's
// how to tell what an order is worth: what it costs to ingrain and to run, and what it does (SPEC §11, The bench).

import { join } from 'node:path'
import { assembleFile } from './load.ts'
import { fireball, hold, type Scene } from './scenes.ts'
import { adept, master, type CasterStats } from './vm/caster.ts'
import { orderLength } from './vm/sim.ts'
import type { Weave } from './vm/weave.ts'

export const BENCH_DIR = join(import.meta.dirname, '..', 'bench')

export type Kind = 'hold' | 'throw'
export type Variant = {
  id: string
  kind: Kind
  name: string
  /** Who holds it: the caster's hand, its own order, or nobody. */
  by: 'nobody' | 'hand' | 'order'
}

export const VARIANTS: Variant[] = [
  { id: 'HoldNothing', kind: 'hold', name: 'Nothing', by: 'nobody' },
  { id: 'HoldEvery', kind: 'hold', name: 'Hand: every particle', by: 'hand' },
  { id: 'HoldOther', kind: 'hold', name: 'Hand: every other', by: 'hand' },
  { id: 'HoldSurface', kind: 'hold', name: 'Hand: the surface', by: 'hand' },
  { id: 'HoldCling', kind: 'hold', name: 'Order: cling (feels the mana thin)', by: 'order' },
  { id: 'HoldCohere', kind: 'hold', name: "Order: cohere (and its neighbours' speed)", by: 'order' },
  { id: 'ThrowHand', kind: 'throw', name: 'Hand, not held', by: 'hand' },
  { id: 'ThrowHandHeld', kind: 'throw', name: 'Hand, held while in reach', by: 'hand' },
  { id: 'ThrowCling', kind: 'throw', name: 'Hand, held by cling (the Fireball)', by: 'order' },
  { id: 'ThrowCohere', kind: 'throw', name: 'Hand, held by cohere', by: 'order' },
  { id: 'ThrowHeading', kind: 'throw', name: 'Order: heading (an angle)', by: 'order' },
  { id: 'ThrowSteer', kind: 'throw', name: 'Order: steer (an angle and a speed)', by: 'order' },
  { id: 'ThrowSteerCling', kind: 'throw', name: 'Order: steer and cling', by: 'order' },
]

export const CASTERS = { adept, master } as const
export type CasterName = keyof typeof CASTERS

/** Layouts: the same throw with a little more or less mana gathered, so a result isn't one lucky arrangement. */
export const LAYOUTS = [0, 1, 2, 3, 4]
const AMOUNTS = [100, 110, 120, 130, 140]

/** How long a hold is kept, from when the ball is let go of. */
export const HOLD_TICKS = 40
/** The pillar's face, in metres (scenes.fireball). */
const PILLAR = 11

export type Result = {
  variant: string
  caster: CasterName
  dims: 2 | 3
  layout: number
  /** Ticks from the cast to letting go of it: pouring and ingraining. */
  ingrain: number
  /** Particles and mana when it was let go of. */
  particles: number
  mana: number
  /** At the end (a hold) or when its front reached the pillar (a throw; `arrived` false if it never did). */
  arrived: boolean
  /** Ticks from letting go to the end. */
  ticks: number
  /** Share of the mana it still holds that is still a ball: within `BALL` metres of its middle. And share still in it. */
  together: number
  kept: number
  /** Root-mean-square distance of its particles from its centre, metres. */
  spread: number
  /** M spent: poured by the caster, kicked by its orders, burned by its orders thinking. */
  push: number
  kick: number
  burn: number
  /** The caster's beats from letting go to the end, per tick. */
  handBeats: number
  ms: number
}

/** How far from its middle a particle can be and still be part of the ball, metres: twice the ball's radius. */
const BALL = 1

/** The mana of a weave that's still a ball: within BALL metres of its middle. */
function ball(w: Weave): number {
  const c = w.centre()
  if (!c) return 0
  let m = 0
  for (const p of w.particles) if (Math.hypot(p.pos[0] - c.pos[0], p.pos[1] - c.pos[1], p.pos[2] - c.pos[2]) < BALL) m += p.free.reduce((a, b) => a + b, 0)
  return m
}

function spread(w: Weave): number {
  const c = w.centre()
  if (!c) return 0
  let s = 0
  for (const p of w.particles) s += (p.pos[0] - c.pos[0]) ** 2 + (p.pos[1] - c.pos[1]) ** 2 + (p.pos[2] - c.pos[2]) ** 2
  return Math.sqrt(s / w.particles.length)
}

/** Casts one variant in one layout, and measures it. `each` sees every tick, for tests. */
export function runVariant(v: Variant, caster: CasterName, dims: 2 | 3, layout: number, each?: (s: Scene) => void): Result {
  const start = performance.now()
  const s = v.kind === 'hold' ? hold(dims) : fireball(dims)
  const stats: CasterStats = CASTERS[caster]()
  s.caster.stats = stats
  s.caster.will.amount = AMOUNTS[layout]
  const cast = s.sim.cast(s.caster, assembleFile(join(BENCH_DIR, `${v.id}.masm`)))
  const sim = s.sim
  const r: Result = {
    variant: v.id,
    caster,
    dims,
    layout,
    ingrain: 0,
    particles: 0,
    mana: 0,
    arrived: false,
    ticks: 0,
    together: 0,
    kept: 0,
    spread: 0,
    push: 0,
    kick: 0,
    burn: 0,
    handBeats: 0,
    ms: 0,
  }
  let weave: Weave | undefined
  let letGo = -1
  let beatsAtLetGo = 0
  const spentAtLetGo = { push: 0, kick: 0, burn: 0 }
  const measure = (w: Weave | undefined) => {
    r.ticks = sim.tick - letGo
    r.together = w && w.mana() > 0 ? ball(w) / w.mana() : 0
    r.kept = w ? w.mana() / r.mana : 0
    r.spread = w ? spread(w) : 0
    r.push = sim.spent.push - spentAtLetGo.push
    r.kick = sim.spent.kick - spentAtLetGo.kick
    r.burn = sim.spent.burn - spentAtLetGo.burn
    r.handBeats = (cast.beats - beatsAtLetGo) / Math.max(1, r.ticks)
  }
  for (let t = 0; t < 160; t++) {
    sim.step()
    each?.(s)
    weave ??= [...sim.weaves.values()][0]
    if (!weave) continue
    const live = sim.weaves.get(weave.id)
    if (letGo < 0) {
      if (weave.inHand) continue
      letGo = sim.tick - 1
      r.ingrain = letGo - cast.startedAt
      r.particles = weave.particles.length
      r.mana = weave.mana()
      beatsAtLetGo = cast.beats
      Object.assign(spentAtLetGo, sim.spent)
    }
    if (v.kind === 'hold') {
      if (sim.tick - letGo >= HOLD_TICKS) {
        s.caster.will.maintain = false
        measure(live)
        r.arrived = !!live
        break
      }
      continue
    }
    // A throw has arrived when the front of the ball reaches the pillar: not a stray particle ahead of it.
    const c = live?.centre()?.pos
    const front = live && c ? Math.max(0, ...live.particles.filter((p) => Math.hypot(p.pos[0] - c[0], p.pos[1] - c[1], p.pos[2] - c[2]) < BALL).map((p) => p.pos[0])) : 0
    if (front >= PILLAR - 0.05) {
      measure(live)
      r.arrived = true
      break
    }
    if (!live) break
  }
  if (letGo >= 0 && !r.arrived && v.kind === 'throw') measure(sim.weaves.get(weave!.id))
  r.ms = performance.now() - start
  return r
}

export type Summary = Omit<Result, 'layout' | 'arrived' | 'ms'> & { arrived: number; ms: number }

/** The mean over layouts; `arrived` is the share that arrived. */
export function summarise(rs: Result[]): Summary {
  const mean = (f: (r: Result) => number) => rs.reduce((s, r) => s + f(r), 0) / rs.length
  const r0 = rs[0]
  return {
    variant: r0.variant,
    caster: r0.caster,
    dims: r0.dims,
    ingrain: mean((r) => r.ingrain),
    particles: mean((r) => r.particles),
    mana: mean((r) => r.mana),
    arrived: mean((r) => (r.arrived ? 1 : 0)),
    ticks: mean((r) => r.ticks),
    together: mean((r) => r.together),
    kept: mean((r) => r.kept),
    spread: mean((r) => r.spread),
    push: mean((r) => r.push),
    kick: mean((r) => r.kick),
    burn: mean((r) => r.burn),
    handBeats: mean((r) => r.handBeats),
    ms: mean((r) => r.ms),
  }
}

/** The order a variant ingrains, and how long it is: what it costs to ingrain, per particle. */
export function orderSize(v: Variant): number {
  const program = assembleFile(join(BENCH_DIR, `${v.id}.masm`))
  const addr = program.labels.get(`${v.id}.order`)
  return addr === undefined ? 0 : orderLength(program, addr)
}
