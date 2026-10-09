// A caster: the person the machine runs on. A body that holds mana, a mind that holds numbers.

import { total, zero, type Parts } from './parts.ts'
import type { Body, Vec } from './world.ts'

/** A stat is what they were born with, scaled by how they are now, plus what they've trained. */
export type Stat = { genetics: number; training: number }

export type CasterStats = {
  body: {
    capacity: Stat
    baseline: Stat
    drain: Stat
    focus: Stat
    streams: Stat
    /** Metres from the body that they can still push mana. */
    reach: Stat
    /** Fire, water, air, earth: 0–1. */
    affinity: [Stat, Stat, Stat, Stat]
  }
  mind: {
    speed: Stat
    registers: Stat
    memory: Stat
  }
}

const s = (genetics: number, training = 0): Stat => ({ genetics, training })

/** An ordinary trained mage. */
export function adept(): CasterStats {
  return {
    body: {
      capacity: s(600),
      baseline: s(60),
      drain: s(15),
      focus: s(6),
      streams: s(4),
      reach: s(4),
      affinity: [s(0.6), s(0.6), s(0.6), s(0.6)],
    },
    mind: { speed: s(300), registers: s(32), memory: s(256) },
  }
}

/** A child with the gift: a small body and a mind with 8 registers. It can't hold Shapes.ball as written. */
export function child(): CasterStats {
  return {
    body: {
      capacity: s(150),
      baseline: s(30),
      drain: s(8),
      focus: s(3),
      streams: s(2),
      reach: s(2.5),
      affinity: [s(0.3), s(0.3), s(0.3), s(0.3)],
    },
    mind: { speed: s(120), registers: s(8), memory: s(32) },
  }
}

/** A master: years of training on top of an adept's body and mind. */
export function master(): CasterStats {
  return {
    body: {
      capacity: s(600, 12000),
      baseline: s(60, 40),
      drain: s(15, 10),
      focus: s(6, 6),
      streams: s(4, 4),
      reach: s(3, 2),
      affinity: [s(0.6, 0.25), s(0.6, 0.25), s(0.6, 0.25), s(0.6, 0.25)],
    },
    mind: { speed: s(300, 1200), registers: s(32), memory: s(256) },
  }
}

/** What the caster wills: the live input a spell reads through its ports. */
export type Will = { aim: Vec; amount: number; force: number; maintain: boolean }

export type ManaRegister = { parts: Parts; holdUntil: number }

export class Caster {
  /** How they are right now, 0–1. Overcharge harm lowers the body's. */
  condition = { body: 1, mind: 1 }
  flow: Parts
  regs: ManaRegister[] = Array.from({ length: 8 }, () => ({ parts: zero(), holdUntil: -1 }))
  will: Will = { aim: [0, 0, 0], amount: 0, force: 0, maintain: false }
  /** How many times they've cast each spell, by name. */
  conditioning = new Map<string, number>()
  /** Harm taken from overcharge, in M past capacity, summed over ticks. */
  harm = 0

  constructor(
    readonly name: string,
    readonly body: Body,
    public stats: CasterStats = adept(),
  ) {
    this.flow = zero()
    this.flow[0] = this.flow[1] = this.flow[2] = this.flow[3] = this.baseline / 4
  }

  private bodyStat(st: Stat) {
    return st.genetics * this.condition.body + st.training
  }
  private mindStat(st: Stat) {
    return st.genetics * this.condition.mind + st.training
  }

  get capacity() {
    return this.bodyStat(this.stats.body.capacity)
  }
  get baseline() {
    return this.bodyStat(this.stats.body.baseline)
  }
  get drain() {
    return this.bodyStat(this.stats.body.drain)
  }
  get focus() {
    return Math.max(0, Math.floor(this.bodyStat(this.stats.body.focus)))
  }
  get streams() {
    return Math.max(0, Math.min(8, Math.floor(this.bodyStat(this.stats.body.streams))))
  }
  get reach() {
    return Math.max(0, this.bodyStat(this.stats.body.reach))
  }
  affinity(k: number) {
    return Math.max(0, Math.min(1, this.bodyStat(this.stats.body.affinity[k])))
  }
  get speed() {
    return Math.max(1, this.mindStat(this.stats.mind.speed))
  }
  get registers() {
    return Math.max(0, Math.min(32, Math.floor(this.mindStat(this.stats.mind.registers))))
  }
  get memory() {
    return Math.max(0, Math.min(256, Math.floor(this.mindStat(this.stats.mind.memory))))
  }

  /** The casting hand: a little in front of the body, towards where they aim. */
  get hand(): Vec {
    const p = this.body.pos
    const dx = this.will.aim[0] - p[0]
    const dz = this.will.aim[2] - p[2]
    const len = Math.hypot(dx, dz) || 1
    return [p[0] + (0.4 * dx) / len, p[1] + 0.2, p[2] + (0.4 * dz) / len]
  }

  /** Mana in the body: its flow and what its registers hold. Weaves in hand are counted by the machine. */
  held(): number {
    return total(this.flow) + this.regs.reduce((sum, r) => sum + total(r.parts), 0)
  }
}
