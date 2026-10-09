// The caster in the sandbox: a mind with beats to think with, and a body with mana to push with. Holding a ball is a
// routine the mind runs, written as a generator: every sense, push and sum it does costs beats, and when a tick's beats
// run out it waits for the next tick, partway through whatever it was doing.

import type { Origin, Sandbox } from './sim.ts'

/** Beats each thing costs, as the machine counts them (SPEC §4): body and reach instructions are 4. */
export const COST = {
  /** `PCNT`: how many particles the weave holds. */
  count: 4,
  /** Reading the weave's centre and how it moves. */
  origin: 4,
  /** `PPOS`: where a particle is. */
  where: 4,
  /** `PPOS` and `PVEL`: where a particle is and how it moves. */
  sense: 8,
  /** `SHOV`. */
  push: 4,
}

export type CasterKind = 'child' | 'adept' | 'master'

/** Beats per tick (the mind's speed) and the mana they can spend: the same as the machine's casters. */
export const CASTERS: Record<CasterKind, { beats: number; mana: number }> = {
  child: { beats: 120, mana: 150 },
  adept: { beats: 300, mana: 600 },
  master: { beats: 1500, mana: 12600 },
}

export type Seen = { x: number; y: number; vx: number; vy: number }

export type Thought = Generator<void, void, void>

export class Mind {
  /** Beats left this tick. Below zero, the mind is still paying for what it started. */
  beats = 0
  /** Beats spent, all told. */
  used = 0
  usedThisTick = 0
  pushes = 0
  pushesThisTick = 0
  /** Mana spent on pushes. */
  spent = 0
  /** False once the caster has let go: the routine stops. */
  holding = true
  private thought: Thought | null = null

  constructor(
    readonly sim: Sandbox,
    readonly speed: number,
    /** Mana the body can spend. */
    public mana: number,
  ) {
    sim.outside += mana
  }

  run(routine: (mind: Mind) => Thought) {
    this.thought = routine(this)
  }

  /** The mind's part of a tick: it thinks until its beats run out. */
  think() {
    this.beats = Math.min(this.beats, 0) + this.speed
    this.usedThisTick = 0
    this.pushesThisTick = 0
    if (!this.holding || !this.thought) return
    if (this.thought.next().done) this.thought = null
  }

  let() {
    this.holding = false
    this.thought = null
  }

  // What a routine can do. Each is a generator: `yield*` it.

  /** Spend beats, waiting for the next tick when they run out. */
  *pay(n: number): Thought {
    this.beats -= n
    this.used += n
    this.usedThisTick += n
    while (this.beats < 0) yield
  }

  /** Arithmetic: n beats of it. */
  *math(n: number): Thought {
    yield* this.pay(n)
  }

  /** End this tick's thinking (`TICK`). */
  *wait(): Thought {
    this.beats = 0
    yield
  }

  *count(): Generator<void, number, void> {
    yield* this.pay(COST.count)
    return this.sim.weave.length
  }

  *origin(): Generator<void, Origin, void> {
    yield* this.pay(COST.origin)
    return { ...this.sim.origin }
  }

  /** Where particle i of the weave is, from the weave's centre. */
  *where(i: number): Generator<void, Seen | null, void> {
    yield* this.pay(COST.where)
    return this.seen(i)
  }

  /** Where particle i is and how it moves, from the weave's centre and relative to how the weave moves. */
  *sense(i: number): Generator<void, Seen | null, void> {
    yield* this.pay(COST.sense)
    return this.seen(i)
  }

  /** Push particle i, changing its velocity by (dvx, dvy). */
  *push(i: number, dvx: number, dvy: number): Thought {
    yield* this.pay(COST.push)
    const p = this.sim.weave[i]
    if (!p || !this.inReach()) return
    this.pushes++
    this.pushesThisTick++
    this.spent += this.sim.push(p, dvx, dvy, (m) => {
      const got = Math.min(m, this.mana)
      this.mana -= got
      this.sim.outside -= got
      return got
    })
  }

  /** The weave is close enough to the body to push. */
  inReach(): boolean {
    const o = this.sim.origin
    const c = this.sim.caster
    return Math.hypot(o.x - c.x, o.y - c.y) <= this.sim.settings.reach
  }

  private seen(i: number): Seen | null {
    const p = this.sim.weave[i]
    if (!p) return null
    const o = this.sim.origin
    return { x: p.x - o.x, y: p.y - o.y, vx: p.vx - o.vx, vy: p.vy - o.vy }
  }
}
