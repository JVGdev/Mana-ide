// Ways to hold a ball of mana, and to throw one. Each is a routine for the caster's mind (mind.ts), paying for what it
// does in beats: the same routine a spell would run with PCNT, PPOS, PVEL and SHOV.

import { HOLD_GAIN, type Origin } from './sim.ts'
import type { Mind, Thought } from './mind.ts'

/** What the caster is after: keep the ball within `radius`, and if `speed`, get it moving at `speed` along `dir`. */
export type Goal = { radius: number; speed: number; dir: [number, number] }

/** Beats of arithmetic in one visit: the distance (with a square root), the direction (a division), the speeds, the loop. */
const VISIT_MATH = 22
/** Beats of arithmetic to tell whether a particle is near the surface: its squared distance, a compare, the loop. */
const SCAN_MATH = 7

/**
 * Visit particle i: sense it, and push it back toward the ball (and, throwing, forward). `set` gives every particle visited
 * just the radial speed wanted; `inward` only pushes one going the wrong way; `forward` doesn't hold at all, only throws.
 */
type Visit = 'set' | 'inward' | 'forward'

function* visit(h: Mind, i: number, goal: Goal, o: Origin, how: Visit): Thought {
  const s = yield* h.sense(i)
  yield* h.math(VISIT_MATH)
  if (!s) return
  const r = Math.hypot(s.x, s.y) || 1e-9
  const nx = s.x / r
  const ny = s.y / r
  const ur = s.vx * nx + s.vy * ny
  const want = -HOLD_GAIN * Math.max(0, r - goal.radius)
  let dr = want - ur
  if ((how === 'inward' && dr > 0) || how === 'forward') dr = 0
  let dx = dr * nx
  let dy = dr * ny
  if (goal.speed > 0) {
    const along = (o.vx + s.vx) * goal.dir[0] + (o.vy + s.vy) * goal.dir[1]
    const more = Math.max(0, goal.speed - along)
    dx += more * goal.dir[0]
    dy += more * goal.dir[1]
  }
  if (dx === 0 && dy === 0) return
  yield* h.push(i, dx, dy)
}

/** Every particle, one after another, round and round. */
function* everyOne(h: Mind, goal: Goal, how: Visit): Thought {
  for (;;) {
    const o = yield* h.origin()
    const n = yield* h.count()
    for (let i = 0; i < n; i++) yield* visit(h, i, goal, o, how)
  }
}

/** Half the particles each time round: the even ones, then the odd ones. */
function* everyOther(h: Mind, goal: Goal): Thought {
  for (let half = 0; ; half ^= 1) {
    const o = yield* h.origin()
    const n = yield* h.count()
    for (let i = half; i < n; i += 2) yield* visit(h, i, goal, o, 'inward')
  }
}

/** Find the particles near the surface (a cheap look at where each one is), then visit only those, a few times over. */
function* surface(h: Mind, goal: Goal, alternate: boolean): Thought {
  for (;;) {
    const n = yield* h.count()
    const outer: number[] = []
    for (let i = 0; i < n; i++) {
      const s = yield* h.where(i)
      yield* h.math(SCAN_MATH)
      if (s && s.x * s.x + s.y * s.y > (0.6 * goal.radius) ** 2) outer.push(i)
    }
    for (let round = 0; round < 4; round++) {
      const o = yield* h.origin()
      const start = alternate ? round & 1 : 0
      for (let k = start; k < outer.length; k += alternate ? 2 : 1) yield* visit(h, outer[k], goal, o, 'inward')
    }
  }
}

/** One full pass, then rest for a few ticks: the ball breathes between pushes. */
function* pulse(h: Mind, goal: Goal): Thought {
  for (;;) {
    const o = yield* h.origin()
    const n = yield* h.count()
    for (let i = 0; i < n; i++) yield* visit(h, i, goal, o, 'inward')
    for (let t = 0; t < 3; t++) yield* h.wait()
  }
}

/** Throwing: push only the back of the ball forward, and let the ball's own pressure pass the push on to the front. */
function* back(h: Mind, goal: Goal): Thought {
  for (;;) {
    const n = yield* h.count()
    const rear: number[] = []
    for (let i = 0; i < n; i++) {
      const s = yield* h.where(i)
      yield* h.math(SCAN_MATH)
      if (s && s.x * goal.dir[0] + s.y * goal.dir[1] < -0.3 * goal.radius) rear.push(i)
    }
    for (let round = 0; round < 3; round++) {
      const o = yield* h.origin()
      for (const i of rear) {
        const s = yield* h.sense(i)
        yield* h.math(10)
        if (!s) continue
        const along = (o.vx + s.vx) * goal.dir[0] + (o.vy + s.vy) * goal.dir[1]
        const more = goal.speed - along
        if (more > 0) yield* h.push(i, more * goal.dir[0], more * goal.dir[1])
      }
    }
  }
}

function* nothing(h: Mind): Thought {
  for (;;) yield* h.wait()
}

export type Strategy = {
  id: string
  name: string
  about: string
  routine: (h: Mind, goal: Goal) => Thought
  /** The ball carries the hold-yourself order instead of being held by its caster. */
  order?: boolean
  /** Only makes sense for a throw. */
  throwOnly?: boolean
}

export const STRATEGIES: Strategy[] = [
  { id: 'none', name: 'Nothing', about: 'Nobody holds it. Its own pressure takes it apart.', routine: nothing },
  {
    id: 'all',
    name: 'Every particle',
    about: 'Visit every particle in turn and set its speed toward the centre. Brute force.',
    routine: (h, g) => everyOne(h, g, 'set'),
  },
  {
    id: 'outward',
    name: 'Every particle, only if leaving',
    about: 'Visit every particle, but only push the ones moving out. Saves mana, not thought.',
    routine: (h, g) => everyOne(h, g, 'inward'),
  },
  {
    id: 'other',
    name: 'Every other particle',
    about: 'Half of them each time round, alternating. The neighbours pass the push on.',
    routine: everyOther,
  },
  {
    id: 'surface',
    name: 'The surface',
    about: 'A cheap look at where each particle is, then only the outer ones are pushed, four rounds before looking again.',
    routine: (h, g) => surface(h, g, false),
  },
  {
    id: 'surface-other',
    name: 'Every other on the surface',
    about: 'The surface, and only half of it each round.',
    routine: (h, g) => surface(h, g, true),
  },
  {
    id: 'pulse',
    name: 'Pulses',
    about: 'One full pass, then three ticks of rest. The ball breathes.',
    routine: pulse,
  },
  {
    id: 'order',
    name: 'Its own order',
    about:
      'Each particle carries an order to push itself back in, paid from its own mana: it shrinks as it holds. Thrown, the caster only pushes it forward.',
    routine: (h, g) => (g.speed > 0 ? everyOne(h, g, 'forward') : nothing(h)),
    order: true,
  },
  {
    id: 'back',
    name: 'Push the back',
    about: 'Throwing only: push the back of the ball forward and let its pressure carry the front.',
    routine: back,
    throwOnly: true,
  },
]

export const strategy = (id: string): Strategy => STRATEGIES.find((s) => s.id === id) ?? STRATEGIES[0]
