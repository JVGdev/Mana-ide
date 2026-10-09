// The two experiments. Hold: a fireball at rest, held for a while. Throw: a fireball at the hand, held while it's pushed
// up to speed, let go, and flown into the wall.

import { DEFAULTS, Sandbox, WORLD, type Settings } from './sim.ts'
import { CASTERS, Mind, type CasterKind } from './mind.ts'
import { strategy, type Goal } from './strategies.ts'

export type Kind = 'hold' | 'throw'

export type Setup = {
  kind: Kind
  strategy: string
  caster: CasterKind
  /** M of fire mana in the ball. Fireball's test world casts 120. */
  amount: number
  /** Metres. */
  radius: number
  /** m/tick to throw it at. */
  speed: number
  /** Turns and jostles the ball's particles: the same seed, the same ball. */
  seed: number
  settings: Settings
}

export const DEFAULT_SETUP: Setup = {
  kind: 'hold',
  strategy: 'surface-other',
  caster: 'adept',
  amount: 120,
  radius: 0.5,
  speed: 0.4,
  seed: 1,
  settings: DEFAULTS,
}

export class Run {
  readonly sim: Sandbox
  readonly mind: Mind
  readonly goal: Goal
  /** The tick the caster let go, or -1. */
  letGoAt = -1
  /** The tick the ball's centre reached the wall, or -1. */
  hitAt = -1
  /** Share of the ball's mana still together when it was let go, and when it reached the wall. */
  heldAtLetGo = 0
  heldAtHit = 0
  readonly mana0: number

  constructor(readonly setup: Setup) {
    const s = strategy(setup.strategy)
    const at = setup.kind === 'hold' ? { x: 3.5, y: 2 } : { x: 1.6, y: 1.5 }
    this.sim = new Sandbox(setup.settings, { ...at, radius: setup.radius, amount: setup.amount, seed: setup.seed })
    const c = CASTERS[setup.caster]
    this.mind = new Mind(this.sim, c.beats, c.mana)
    this.goal = { radius: setup.radius, speed: setup.kind === 'throw' ? setup.speed : 0, dir: [1, 0] }
    this.mind.run((h) => s.routine(h, this.goal))
    this.mana0 = this.sim.mana()
    if (s.id === 'none') this.letGo()
  }

  /** One tick: the caster thinks, the world moves, and the caster lets go when the throw is done. */
  step() {
    const { sim, mind } = this
    mind.think()
    sim.step()
    if (this.setup.kind === 'throw') {
      const fast = sim.origin.vx >= 0.95 * this.goal.speed
      if (mind.holding && (fast || !mind.inReach())) this.letGo()
      if (this.hitAt < 0 && sim.origin.x >= WORLD.wall - this.setup.radius) {
        this.hitAt = sim.tick
        this.heldAtHit = sim.held()
      }
    }
  }

  letGo() {
    if (!this.mind.holding) return
    this.mind.let()
    this.letGoAt = this.sim.tick
    this.heldAtLetGo = this.sim.held()
  }

  /** How far the mana ledger has drifted from where it began. Zero, give or take rounding. */
  manaError(): number {
    return Math.abs(this.sim.mana() - this.mana0)
  }
}

export type Result = {
  strategy: string
  caster: CasterKind
  /** Share of the mana still in the weave at the end (hold), or on reaching the wall (throw). */
  held: number
  /** RMS spread of the weave against the ball as laid (1 = as laid). */
  spread: number
  spent: number
  beatsPerTick: number
  pushesPerTick: number
  letGoAt: number
  hitAt: number
  heldAtLetGo: number
  manaError: number
  momentumError: number
}

export function play(setup: Setup, ticks: number): Result {
  const run = new Run(setup)
  for (let t = 0; t < ticks; t++) {
    run.step()
    if (setup.kind === 'throw' && run.hitAt >= 0) break
  }
  const { sim, mind } = run
  const holdTicks = run.letGoAt >= 0 ? run.letGoAt : sim.tick
  return {
    strategy: setup.strategy,
    caster: setup.caster,
    held: setup.kind === 'throw' ? (run.hitAt >= 0 ? run.heldAtHit : 0) : sim.held(),
    spread: sim.spread(),
    spent: mind.spent + sim.selfSpent,
    beatsPerTick: mind.used / Math.max(1, holdTicks),
    pushesPerTick: mind.pushes / Math.max(1, holdTicks),
    letGoAt: run.letGoAt,
    hitAt: run.hitAt,
    heldAtLetGo: run.heldAtLetGo,
    manaError: run.manaError(),
    momentumError: sim.momentumError(),
  }
}
