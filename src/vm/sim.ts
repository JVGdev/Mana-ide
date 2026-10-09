// The machine running in the world: casters casting, weaves running their orders, the world moving. One tick at a time.

import { decode, type Instr } from '../asm/disassembler.ts'
import { PORT_BY_CODE } from '../asm/isa.ts'
import type { Program } from '../asm/assembler.ts'
import { PHYSICS } from './physics.ts'
import { add, take, total, zero, type Parts } from './parts.ts'
import { Caster, type CasterStats, type ManaRegister } from './caster.ts'
import { Weave, centreOf, stampOf, turnToFrame, turnToWorld } from './weave.ts'
import { EARTH, World, fillOf, massOf, massOfParts, packed, type Body, type Particle, type Vec } from './world.ts'
import { findPairs, mergeAndSplit, stepFluid, type Change, type FluidHooks } from './fluid.ts'
import { stepAir } from './air.ts'
import { heatOf, stored, sum, type Ledger } from './energy.ts'

export class Fault extends Error {
  constructor(
    readonly code: string,
    message: string,
  ) {
    super(`${code}: ${message}`)
  }
}

export type SimEvent = {
  tick: number
  kind:
    | 'cast'
    | 'halt'
    | 'fail'
    | 'fault'
    | 'weave'
    | 'manifest'
    | 'lock'
    | 'dissolve'
    | 'fray'
    | 'thin-air'
    | 'overcharge'
    | 'overstrain'
    | 'taken'
  caster?: string
  weave?: number
  detail?: string
}

/** A thinking frame: a mind casting a spell, or a weave's cell running its order. */
export type Frame = {
  n: Float64Array
  /** How many of `n` exist. */
  limit: number
  flags: number
  stack: number[]
  calls: number[]
  memory: Float64Array
  pc: number
}

export type Cast = {
  caster: Caster
  program: Program
  name: string
  frame: Frame
  state: 'running' | 'halted' | 'failed' | 'fault'
  code?: number
  fault?: string
  /** The weave in n0 when it halted, if any. */
  result?: number
  startedAt: number
  endedAt?: number
  /** Beats of thought spent so far. */
  beats: number
  /** Where they went: how many times each instruction ran, and its beats, by address. */
  profile: Map<number, { runs: number; beats: number }>
  /** Beats left in the tick being thought. */
  left: number
  /** Done thinking for this tick: out of beats, or it said `TICK`. */
  yielded: boolean
}

/** One instruction of a particle's order: where it is, and the particle's registers just before it ran. */
export type OrderStep = { addr: number; n: Float64Array }

/** One particle's order, run for one tick, instruction by instruction (when `traceOrders` is on). */
export type OrderTrace = {
  tick: number
  weave: number
  /** Which of its weave's particles, as PPOS counted them as the tick began. */
  particle: number
  /** The particle's id: particles merge and split, so the count can change before the next tick. */
  id: number
  /** Where it was from its weave's centre, in the weave's frame, as the tick began. */
  off: Vec
  /** Its copy of the weave's registers as the tick began: what GETW reads. */
  w: Float64Array
  steps: OrderStep[]
  /** Its registers when it was done. */
  n: Float64Array
  /** Done, let go (DISS), or the fault that frayed it. */
  outcome: string
  beats: number
  /** Its own mana its thinking burned. */
  burned: number
  /** How it pushed itself (KICK), in the weave's frame. */
  kick: Vec
  cnds: number
}

const decoded = new WeakMap<Uint8Array, Map<number, Instr>>()
const lengths = new WeakMap<Uint8Array, Map<number, number>>()

/**
 * How long an order is: every instruction it could ever run from `addr`, following its jumps and calls. Ingraining it
 * into a particle takes a beat for each.
 */
export function orderLength(program: Program, addr: number): number {
  let cache = lengths.get(program.bytes)
  if (!cache) lengths.set(program.bytes, (cache = new Map()))
  const known = cache.get(addr)
  if (known !== undefined) return known
  const seen = new Set<number>()
  const todo = [addr]
  while (todo.length) {
    const a = todo.pop()!
    if (seen.has(a) || a >= program.bytes.length) continue
    let instr: Instr
    try {
      instr = fetch(program, a)
    } catch {
      continue
    }
    seen.add(a)
    const name = instr.op.name
    const next = a + instr.size
    if (['RET', 'HALT', 'DISS', 'FAIL', 'TICK'].includes(name)) continue
    if (name === 'JMP') todo.push(instr.args[0])
    else if (name === 'CALL' || name.startsWith('J')) todo.push(instr.args[0], next)
    else todo.push(next)
  }
  cache.set(addr, seen.size)
  return seen.size
}

function fetch(program: Program, addr: number): Instr {
  let cache = decoded.get(program.bytes)
  if (!cache) decoded.set(program.bytes, (cache = new Map()))
  let instr = cache.get(addr)
  if (!instr) {
    if (addr >= program.bytes.length) throw new Fault('NO_ROOM', `ran off the end of the spell at ${addr}`)
    cache.set(addr, (instr = decode(program.bytes, addr)))
  }
  return instr
}

/** What a push goes off: a caster's body (through their reach), the air in a cell, or the ground. */
type Against = { kind: 'body'; body: Body } | { kind: 'air'; cell: number } | { kind: 'ground' }

export class Sim {
  casters: Caster[] = []
  weaves = new Map<number, Weave>()
  casts: Cast[] = []
  events: SimEvent[] = []
  private nextWeave = 1
  /** Partway through a tick: some minds may still be thinking, and the world hasn't moved yet. */
  midTick = false
  /** Record every particle's order, instruction by instruction, into `traces`. For the tester; it costs time. */
  traceOrders = false
  /** Last tick's orders, by weave id, one trace per particle that ran one (in the order they ran). */
  traces = new Map<number, OrderTrace[]>()
  /** Matter held by mana, per world cell, as of the start of this tick's orders. */
  private carried: Float64Array
  /** Beats the instruction being run costs beyond its own: ingraining a long order. */
  private extra = 0
  /**
   * Weaves that have come apart. Their casters can still name them: there's nothing in them to sense or push, and mana
   * emitted into one goes loose.
   */
  private gone = new Map<number, Weave>()
  /**
   * M spent so far: poured onto particles by casters (`push`, and what SEND spends) and by orders on themselves (`kick`),
   * and burned thinking. Poured mana isn't used up: it goes loose, still mana.
   */
  spent = { push: 0, kick: 0, burn: 0 }
  /** Energy minds (casters' and orders') have transformed out of mana into the world so far (D32). */
  transformed = 0
  /** M of free mana that has strayed out of its caster's reach before it was given an order, and left its weave. */
  strayed = 0
  /** What each caster can feel of each weave this tick: its particles within their reach (felt). */
  private feltCache = new WeakMap<Weave, { key: string; ps: Particle[] }>()
  /** Bumped whenever a weave's particles change outside the world's step, so that what's felt is felt again. */
  private version = 0
  /** Whether the Energy ledger is kept (keepEnergy). */
  trackEnergy = false
  energy: Ledger = { start: 0, outside: 0, minds: 0, bodies: 0, error: {} }

  constructor(readonly world: World) {
    this.carried = new Float64Array(world.size)
  }

  get tick() {
    return this.world.tick
  }

  addCaster(name: string, pos: Vec, stats?: CasterStats): Caster {
    const caster = new Caster(name, this.world.addBody(name, pos), stats)
    this.casters.push(caster)
    return caster
  }

  /** Starts a spell. It begins at its first instruction, or at `entry`. */
  cast(caster: Caster, program: Program, entry?: string): Cast {
    const pc = entry ? program.labels.get(entry) : 0
    if (pc === undefined) throw new Error(`no label ${entry}`)
    const name = entry ?? [...program.labels].find(([, a]) => a === 0)?.[0] ?? 'spell'
    const cast: Cast = {
      caster,
      program,
      name,
      frame: frame(caster.registers, caster.memory, pc),
      state: 'running',
      startedAt: this.tick,
      beats: 0,
      profile: new Map(),
      left: 0,
      yielded: true,
    }
    this.casts.push(cast)
    this.log({ kind: 'cast', caster: caster.name, detail: name })
    return cast
  }

  run(ticks: number) {
    for (let i = 0; i < ticks; i++) this.step()
  }

  /** Runs until every cast has ended, or `max` ticks. */
  runCasts(max = 200) {
    for (let i = 0; i < max && this.casts.some((c) => c.state === 'running'); i++) this.step()
  }

  step() {
    this.startThinking()
    this.endTick()
  }

  /**
   * Runs one instruction of `cast`'s mind. When that's its last for this tick, the tick ends: the other minds finish
   * thinking and the world moves. Returns false if the cast has nothing left to run.
   */
  stepInstruction(cast: Cast): boolean {
    for (let tries = 0; tries < 2 && cast.state === 'running'; tries++) {
      this.startThinking()
      const ran = this.think(cast, { max: 1 })
      if (!cast.yielded) this.think(cast, { max: 0 }) // will the next one fit in this tick?
      if (cast.yielded || cast.state !== 'running') this.endTick()
      if (ran) return true
    }
    return false
  }

  /**
   * Runs until `stop` is true just before one of `cast`'s instructions, and leaves the machine there, partway through a
   * tick. The instruction it's resuming from doesn't count. Returns true if it stopped, false if `ticks` ran out or
   * the cast ended.
   */
  runUntil(cast: Cast, stop: (addr: number) => boolean, ticks = 1): boolean {
    let from = this.midTick ? cast.frame.pc : -1
    const check = (addr: number) => {
      if (addr === from) {
        from = -1
        return false
      }
      from = -1
      return stop(addr)
    }
    for (let t = 0; t < ticks && cast.state === 'running'; t++) {
      this.startThinking()
      this.think(cast, { stop: check })
      if (!cast.yielded && cast.state === 'running') return true
      this.endTick()
    }
    return false
  }

  /** The start of a tick: every running mind gets its beats, less what it went over by last tick. */
  private startThinking() {
    if (this.midTick) return
    this.midTick = true
    for (const c of this.casters) c.powerLeft = c.power
    for (const cast of this.casts)
      if (cast.state === 'running') {
        cast.left = cast.caster.speed * (1 + (cast.caster.conditioning.get(cast.name) ?? 0)) + Math.min(0, cast.left)
        cast.yielded = cast.left <= 0
      }
  }

  /** The rest of a tick: minds still thinking finish, then bodies, weaves and the world. */
  private endTick() {
    const w = this.world
    const hooks = this.hooks
    // The Energy ledger, if it's being kept: each step either puts energy in from outside (minds and orders), turns it
    // into heat (counted where it happens, or measured for the steps that only lose it), or gets it wrong, which is
    // counted too.
    let e = this.trackEnergy ? this.measure() : 0
    const step = (kind: 'outside' | 'exact' | 'loses', name: string, run: () => void) => {
      if (!this.trackEnergy) return run()
      const heat = heatOf(w)
      const minds = this.transformed
      run()
      const now = this.measure()
      const appeared = now - e + (heatOf(w) - heat)
      if (kind === 'outside') {
        // What minds transformed out of mana is counted exactly, where it happened. The rest is what bodies did by
        // gathering, pouring and letting mana out.
        const transformed = this.transformed - minds
        this.energy.outside += appeared
        this.energy.minds += transformed
        this.energy.bodies += appeared - transformed
      } else if (kind === 'loses' && appeared < 0) w.warm(name, -appeared)
      else this.energy.error[name] = (this.energy.error[name] ?? 0) + appeared
      e = now
    }
    step('outside', 'casters and orders', () => {
      // 1. Minds think.
      for (const cast of this.casts) if (cast.state === 'running') this.think(cast)
      this.midTick = false
      // 2–4. Bodies: holds run down, flows drain, overcharge.
      for (const c of this.casters) this.breathe(c)
      // 5. Weaves take hold of what matter their mana can bind, and let go of what it can't.
      for (const weave of this.weaves.values()) this.hold(weave)
      // 6. Particles of weaves set loose run their orders.
      if (this.traceOrders) this.traces = new Map()
      this.relay()
      this.measureCarried()
      for (const weave of [...this.weaves.values()]) if (!weave.inHand) this.runOrders(weave)
    })
    // 7. The world moves: the mana, the air, bodies, matter.
    this.measureCarried()
    step('exact', 'the fluid', () => stepFluid(w, hooks))
    step('loses', 'the air flowing', () => stepAir(w))
    // Particles at rest together merge, and big ones spread thin split.
    step('exact', 'merging', () => this.regroup(mergeAndSplit(w, hooks)))
    step('loses', 'settling', () => {
      for (const weave of [...this.weaves.values()]) this.keep(weave)
      this.settleLoose()
      this.unbind()
      this.bond()
      w.moveBodies()
      this.measureCarried()
      w.settleMatter(this.carried)
    })
    w.tick++
  }

  /** What the fluid needs from the machine: what's in whose hand, and whose matter is whose. */
  private hooks: FluidHooks = {
    holder: (p) => {
      const wv = p.weave ? this.weaves.get(p.weave) : undefined
      return wv?.inHand ? wv.maker.body : undefined
    },
    othersCarried: (p, cell) => this.carried[cell] - (this.carriedBy.get(p.weave)?.get(cell) ?? 0),
  }

  /** Free mana in casters' bodies. */
  private inBodies(): number {
    return this.casters.reduce((s, c) => s + c.held(), 0)
  }

  /** All the Energy the world holds. */
  private measure(): number {
    return sum(stored(this.world, this.inBodies()))
  }

  /**
   * Starts keeping the Energy ledger: from now on every tick counts where energy comes from and goes (it costs time). The
   * world's heat counts from zero.
   */
  keepEnergy() {
    this.trackEnergy = true
    this.world.heat = {}
    this.energy = { start: this.measure(), outside: 0, minds: 0, bodies: 0, error: {} }
  }

  /** The ledger now: what the world holds, as heat and otherwise, and how far it is off. */
  energyNow() {
    const held = stored(this.world, this.inBodies())
    const heat = heatOf(this.world)
    const error = Object.values(this.energy.error).reduce((a, b) => a + b, 0)
    return { held, total: sum(held), heat, ...this.energy, errorTotal: error }
  }


  /** Every M of mana in the world, wherever it is. Conservation says `total` never changes. */
  ledger() {
    const world = this.world.mana()
    let weaves = 0
    let carried = 0
    for (const wv of this.weaves.values())
      for (const p of wv.particles) {
        weaves += total(p.free)
        carried += total(p.carried)
      }
    const casters = this.casters.reduce((s, c) => s + c.held(), 0)
    return {
      ...world,
      weaves,
      carried,
      casters,
      free: world.air + world.loose + weaves + casters,
      condensed: world.matter + carried,
      total: world.air + world.matter + world.loose + weaves + carried + casters,
    }
  }

  private log(e: Omit<SimEvent, 'tick'>) {
    this.events.push({ tick: this.tick, ...e })
  }

  // The mind

  /**
   * Thinks with what's left of this tick's beats, until they run out or it says `TICK`. It can stop sooner: after `max`
   * instructions, or before one `stop` picks. Returns how many it ran.
   */
  private think(cast: Cast, opts: { max?: number; stop?: (addr: number) => boolean } = {}): number {
    let ran = 0
    while (cast.state === 'running' && !cast.yielded) {
      let instr: Instr
      try {
        instr = fetch(cast.program, cast.frame.pc)
      } catch (e) {
        this.end(cast, 'fault', e instanceof Fault ? e.message : String(e))
        return ran
      }
      if (cast.left < instr.op.beats) {
        cast.yielded = true
        return ran
      }
      if (opts.max !== undefined && ran >= opts.max) return ran
      if (opts.stop?.(instr.addr)) return ran
      cast.left -= instr.op.beats
      ran++
      cast.beats += instr.op.beats
      const spent = cast.profile.get(instr.addr) ?? { runs: 0, beats: 0 }
      spent.runs++
      spent.beats += instr.op.beats
      cast.profile.set(instr.addr, spent)
      this.extra = 0
      try {
        if (this.execMind(cast, instr) === 'tick') cast.yielded = true
      } catch (e) {
        if (!(e instanceof Fault)) throw e
        this.end(cast, 'fault', e.message)
      }
      // A long order takes more beats to ingrain than there may be left this tick: the next tick pays the rest.
      if (this.extra) {
        cast.left -= this.extra
        cast.beats += this.extra
        spent.beats += this.extra
        if (cast.left <= 0) cast.yielded = true
      }
    }
    return ran
  }

  private end(cast: Cast, state: Cast['state'], detail?: string) {
    cast.state = state
    cast.endedAt = this.tick
    const caster = cast.caster
    if (state === 'fault') cast.fault = detail
    if (state === 'halted') {
      const r = cast.frame.n[0]
      if (this.weaves.get(r)?.maker === caster) cast.result = r
      caster.conditioning.set(cast.name, (caster.conditioning.get(cast.name) ?? 0) + 1)
    }
    this.log({ kind: state === 'halted' ? 'halt' : state === 'failed' ? 'fail' : 'fault', caster: caster.name, detail: detail ?? cast.name })
    // Weaves still in hand were never set loose: their mana falls back into the body's flow.
    for (const weave of [...this.weaves.values()])
      if (weave.maker === caster && weave.inHand) {
        for (const p of weave.particles) {
          add(caster.flow, take(p.free, Infinity))
          this.dropMatter(p)
        }
        this.removeEmpty()
        this.weaves.delete(weave.id)
        this.log({ kind: 'dissolve', caster: caster.name, weave: weave.id, detail: 'never set loose' })
      }
  }

  /** Runs one instruction of a casting mind. */
  private execMind(cast: Cast, instr: Instr): 'next' | 'tick' | 'end' {
    const f = cast.frame
    const caster = cast.caster
    const a = instr.args
    const op = instr.op
    if (op.order) throw new Fault('ORDER_ONLY', `${op.name} only works inside a weave's order`)

    const common = execCommon(f, instr)
    if (common === 'ret-empty' || common === 'halt') {
      this.end(cast, 'halted')
      return 'end'
    }
    if (common === 'tick') return 'tick'
    if (common !== 'unhandled') return 'next'

    const next = () => {
      f.pc = instr.addr + instr.size
      return 'next' as const
    }
    const m = (k: number): ManaRegister => {
      const r = a[k]
      if (r >= caster.streams) throw new Fault('NO_STREAM', `m${r}: ${caster.name} works ${caster.streams} streams`)
      return caster.regs[r]
    }
    const n = (k: number) => get(f, a[k])
    const s = (k: number) => source(f, instr, k)
    const vec = (k: number): Vec => [get(f, a[k]), get(f, a[k] + 1), get(f, a[k] + 2)]
    const weave = (k: number): Weave => {
      const wv = this.weaves.get(n(k)) ?? this.gone.get(n(k))
      if (!wv || wv.maker !== caster) throw new Fault('NOT_YOURS', `there's no weave ${n(k)} of ${caster.name}'s`)
      return wv
    }
    const into = (md: ManaRegister, from: ManaRegister, parts: Parts) => {
      md.holdUntil = total(md.parts) <= PHYSICS.epsilon ? from.holdUntil : Math.min(md.holdUntil, from.holdUntil)
      add(md.parts, parts)
    }

    switch (op.name) {
      case 'FAIL':
        cast.code = a[0]
        this.end(cast, 'failed', `code ${a[0]}`)
        return 'end'
      case 'IN': {
        const port = PORT_BY_CODE.get(a[1])!
        const values = this.port(caster, port.name)
        values.forEach((v, k) => set(f, a[0] + k, v))
        return next()
      }

      // Body
      case 'GATH': {
        const r = m(0)
        const want = Math.max(0, s(1))
        const got = this.gather(caster, want)
        if (total(r.parts) <= PHYSICS.epsilon) r.holdUntil = this.tick
        add(r.parts, got)
        if (total(got) < want - 1e-6) this.log({ kind: 'thin-air', caster: caster.name, detail: `${round(total(got))} of ${round(want)} M` })
        return next()
      }
      case 'CIRC':
        m(0).holdUntil = this.tick + caster.focus
        return next()
      case 'FILT': {
        const md = m(0)
        const ms = m(1)
        const k = s(2)
        if (!Number.isInteger(k) || k < 0 || k > 3) throw new Fault('NO_NAME', `mana answers to four names, 0–3, not ${k}`)
        const part = ms.parts[k]
        ms.parts[k] = 0
        const kept = part * caster.affinity(k)
        const out = zero()
        out[k] = kept
        into(md, ms, out)
        caster.flow[k] += part - kept
        return next()
      }
      case 'SPLT': {
        const md = m(0)
        const ms = m(1)
        into(md, ms, take(ms.parts, Math.max(0, s(2))))
        return next()
      }
      case 'JOIN': {
        const md = m(0)
        const ms = m(1)
        into(md, ms, take(ms.parts, Infinity))
        return next()
      }
      case 'MEAS':
        set(f, a[0], total(m(1).parts))
        return next()
      case 'PART': {
        const k = s(2)
        set(f, a[0], Number.isInteger(k) && k >= 0 && k < 4 ? m(1).parts[k] : 0)
        return next()
      }
      case 'VENT':
        this.world.addAir(this.world.clampedCellOf(caster.body.pos), take(m(0).parts, Infinity))
        return next()

      // Reach
      case 'PROB':
      case 'AIRM': {
        const i = this.world.cellOf(vec(1))
        const k = s(2)
        let v = 0
        if (i >= 0 && this.reaches(caster, vec(1)) && Number.isInteger(k) && k >= 0 && k < 4) {
          v = op.name === 'AIRM' ? this.world.air[i][k] : this.world.matter[i][k] + this.carriedPart(i, k)
        }
        set(f, a[0], v)
        return next()
      }
      case 'SEND': {
        // Mana thrown out of the body's reach at a speed, and the body pushed back by it. The mind transforms some of
        // what's thrown into the Energy the throw takes (D32): that share goes loose where it was let out, still mana.
        const r = m(0)
        const pos = this.inside(vec(2))
        if (!this.reaches(caster, pos)) return next() // out of reach: nothing is let out
        const want = Math.min(Math.max(0, n(1)), total(r.parts))
        const vel = vec(3)
        const body = caster.body
        const B = body.mass
        // Sending m kg at v costs ½m|v|², and pushes the body back by m·v across the ground: −m(v·u) + m²|v|²/2B, where
        // u is how the body moves. Out of what's let out, `sent` goes, and the rest pays for it: solve
        // sent + E(sent)/pushEnergy = want, and E(sent) ≤ the power left.
        const flat = vel[0] * vel[0] + vel[2] * vel[2]
        const per = total(r.parts) > 0 ? massOfParts(r.parts) / total(r.parts) : 1 // kilograms a M
        const a2 = (per * per * flat) / (2 * B)
        const a1 = per * (0.5 * (flat + vel[1] * vel[1]) - (vel[0] * body.vel[0] + vel[2] * body.vel[2]))
        const solve = (qa: number, qb: number, c: number) => (qa > 1e-15 ? (-qb + Math.sqrt(qb * qb + 4 * qa * c)) / (2 * qa) : c / qb)
        let sent = Math.max(0, solve(a2 / PHYSICS.pushEnergy, 1 + a1 / PHYSICS.pushEnergy, want))
        let energy = a1 * sent + a2 * sent * sent
        if (energy > caster.powerLeft) {
          sent = Math.max(0, solve(a2, a1, caster.powerLeft))
          energy = a1 * sent + a2 * sent * sent
        }
        const spent = Math.max(0, energy) / PHYSICS.pushEnergy
        const parts = take(r.parts, sent + spent)
        this.world.addAir(this.world.clampedCellOf(pos), take(parts, spent))
        const t = massOfParts(parts)
        if (t > PHYSICS.epsilon) {
          for (const k of [0, 2]) body.vel[k] -= (t * vel[k]) / B
          this.world.impulse.walls[1] += t * vel[1]
          this.world.pour(parts, pos, 0, vel)
        }
        if (energy > 0) {
          caster.powerLeft -= energy
          caster.strain += energy
          this.transformed += energy
          this.spent.push += spent
        } else this.world.warm('braking', -energy)
        return next()
      }
      case 'WPOS': {
        const c = centreOf(this.felt(caster, weave(1)))
        ;(c ? c.pos : [0, 0, 0]).forEach((v, k) => set(f, a[0] + k, v))
        return next()
      }
      case 'WVEL': {
        const wv = weave(1)
        const c = centreOf(this.felt(caster, wv))
        wv.toFrame(c ? c.vel : [0, 0, 0]).forEach((v, k) => set(f, a[0] + k, v))
        return next()
      }

      // Weave
      case 'WEAV': {
        const weave = new Weave(this.nextWeave++, caster, cast.program, vec(1))
        this.weaves.set(weave.id, weave)
        set(f, a[0], weave.id)
        this.log({ kind: 'weave', caster: caster.name, weave: weave.id })
        return next()
      }
      case 'TURN': {
        const wv = weave(0)
        const d = vec(1)
        if (Math.abs(d[0]) + Math.abs(d[2]) > PHYSICS.epsilon) wv.yaw = Math.atan2(d[0], d[2])
        for (const p of this.felt(caster, wv)) p.yaw = wv.yaw // what it touches is told which way it faces
        return next()
      }
      case 'EMIT': {
        const r = m(0)
        const amount = Math.max(0, n(1))
        const wv = weave(2)
        if (wv.locks.input) throw new Fault('LOCKED', `weave ${wv.id}'s input is locked`)
        // In hand, from where it was begun; set loose, from the middle of what the caster feels of it.
        const from = wv.inHand ? wv.origin : centreOf(this.felt(caster, wv))?.pos
        if (!from) return next() // nothing of it within reach to pour into
        const off = wv.toWorld(vec(3))
        const pos: Vec = [from[0] + off[0], from[1] + off[1], from[2] + off[2]]
        if (this.world.cellOf(pos) < 0 || !this.reaches(caster, pos)) return next() // nowhere it can pour
        const gone = this.gone.has(wv.id)
        const poured = this.world.pour(take(r.parts, amount), pos, gone ? 0 : wv.id)
        // The mana carries what its caster has written into the weave, and which way it faces.
        for (const p of poured) {
          p.regs.set(wv.regs)
          p.stamp.set(wv.stamp)
          p.yaw = wv.yaw
        }
        if (!gone) wv.particles.push(...poured)
        this.version++
        return next()
      }
      case 'WSET': {
        // Written into what of it the caster touches; the rest hears of it from them, by contact.
        const wv = weave(0)
        const k = a[1]
        if (k >= wv.regs.length) throw new Fault('NO_ROOM', `a weave has w0–w${wv.regs.length - 1}`)
        const v = s(2)
        const stamp = stampOf(this.tick, 0)
        wv.regs[k] = v
        wv.stamp[k] = stamp
        for (const p of this.felt(caster, wv)) {
          p.regs[k] = v
          p.stamp[k] = stamp
        }
        return next()
      }
      case 'WGET': {
        // The newest copy among what of it the caster touches; touching none of it, what they wrote last.
        const wv = weave(1)
        const k = a[2]
        if (k >= wv.regs.length) throw new Fault('NO_ROOM', `a weave has w0–w${wv.regs.length - 1}`)
        let best = wv.stamp[k]
        let v = wv.regs[k]
        for (const p of this.felt(caster, wv))
          if (p.stamp[k] > best) {
            best = p.stamp[k]
            v = p.regs[k]
          }
        set(f, a[0], v)
        return next()
      }
      case 'ORDR': {
        const wv = weave(0)
        if (wv.locks.order) throw new Fault('LOCKED', `weave ${wv.id}'s order is locked`)
        wv.order = a[1]
        return next()
      }
      case 'MANI': {
        const wv = weave(0)
        if (wv.inHand) {
          wv.inHand = false
          wv.manifestedAt = this.tick
          const c = wv.centre()
          if (c) wv.origin = c.pos
          this.log({ kind: 'manifest', caster: caster.name, weave: wv.id })
        }
        return next()
      }
      case 'LOCK': {
        const wv = weave(0)
        if (wv.inHand) throw new Fault('NOT_LOOSE', `weave ${wv.id} has to be manifested before it's locked`)
        const what = ([undefined, 'input', 'order'] as const)[a[1]]
        if (!what) throw new Fault('NO_ROOM', `there's nothing to lock called #${a[1]}`)
        wv.locks[what] = true
        this.log({ kind: 'lock', caster: caster.name, weave: wv.id, detail: what })
        return next()
      }
      case 'RELS': {
        // What of it the caster touches goes loose. What they can't reach, and carries its order, keeps going.
        const wv = weave(0)
        this.release(wv, this.felt(caster, wv), 'let go')
        if (!wv.particles.length) this.end_(wv)
        return next()
      }
      // A weave's particles, as the caster feels them: those within reach, counted 0, 1, 2… in the order they were
      // laid. Further out, the caster can't tell they're there.
      case 'PCNT':
        set(f, a[0], this.felt(caster, weave(1)).length)
        return next()
      case 'PPOS':
      case 'PVEL': {
        const wv = weave(1)
        const ps = this.felt(caster, wv)
        const p = ps[Math.floor(s(2))]
        let v: Vec = [0, 0, 0]
        if (p) {
          // Where it is from the middle of what the caster feels of the weave.
          const c = centreOf(ps)!.pos
          v = op.name === 'PVEL' ? wv.toFrame(p.vel) : wv.toFrame(p.pos.map((x, k) => x - c[k]) as Vec)
        }
        v.forEach((x, k) => set(f, a[0] + k, x))
        return next()
      }
      case 'SHOV': {
        const r = m(0)
        const wv = weave(1)
        const p = this.felt(caster, wv)[Math.floor(n(2))]
        if (!p) return next()
        // Through the body's reach: the push goes off the body, which feels it, and the ground under its feet.
        const power = { left: caster.powerLeft }
        caster.strain += this.push(p, wv.toWorld(vec(3)), { kind: 'body', body: caster.body }, (want) => take(r.parts, want), power, 'push')
        caster.powerLeft = power.left
        return next()
      }
      case 'INGR': {
        const wv = weave(0)
        if (wv.order === null) throw new Fault('NO_ORDER', `weave ${wv.id} has no order to ingrain: give it one with ORDR`)
        this.extra = orderLength(wv.program, wv.order)
        const p = this.felt(caster, wv)[Math.floor(s(1))]
        if (p) {
          p.order = { program: wv.program, addr: wv.order }
          p.yaw = wv.yaw
        }
        return next()
      }
    }
    throw new Fault('ORDER_ONLY', `${op.name} can't run in a mind`)
  }

  private port(caster: Caster, name: string): number[] {
    switch (name) {
      case 'AIM':
        return [...caster.will.aim]
      case 'HAND':
        return caster.hand
      case 'SELF':
        return [...caster.body.pos]
      case 'AMOUNT':
        return [caster.will.amount]
      case 'FORCE':
        return [caster.will.force]
      case 'MAINTAIN':
        return [caster.will.maintain ? 1 : 0]
      case 'CELL':
        return [this.world.cell]
      case 'DEPTH':
        return [this.world.d]
      case 'LOAD':
        return [this.load(caster)]
      case 'CAPACITY':
        return [caster.capacity]
      case 'REACH':
        return [caster.reach]
    }
    throw new Fault('BAD_PORT', `${name} can only be read inside a weave's order`)
  }

  /** Draws up to `amount` of air mana from around the body, from every cell in reach in proportion. */
  private gather(caster: Caster, amount: number): Parts {
    const w = this.world
    const reach = Math.min(PHYSICS.gatherRadius, caster.reach)
    const p = caster.body.pos
    const cells: number[] = []
    let available = 0
    const lo = (v: number) => Math.floor((v - reach) / w.cell)
    const hi = (v: number) => Math.floor((v + reach) / w.cell)
    for (let z = Math.max(0, lo(p[2])); z <= Math.min(w.d - 1, hi(p[2])); z++)
      for (let y = Math.max(0, lo(p[1])); y <= Math.min(w.h - 1, hi(p[1])); y++)
        for (let x = Math.max(0, lo(p[0])); x <= Math.min(w.w - 1, hi(p[0])); x++) {
          const c = w.centre(x, y, z)
          if (w.d === 1) c[2] = p[2]
          if (Math.hypot(c[0] - p[0], c[1] - p[1], c[2] - p[2]) > reach) continue
          const i = w.index(x, y, z)
          cells.push(i)
          available += total(w.air[i])
        }
    const got = zero()
    if (available <= PHYSICS.epsilon || amount <= 0) return got
    const f = Math.min(1, amount / available)
    // What's drawn in brings its momentum into the body, across the ground; the ground takes the rest.
    const body = caster.body
    for (const i of cells) {
      const { parts, momentum } = w.takeAir(i, f)
      add(got, parts)
      body.vel[0] += momentum[0] / body.mass
      body.vel[2] += momentum[2] / body.mass
      w.impulse.walls[1] -= momentum[1]
    }
    return got
  }

  private load(caster: Caster): number {
    let inHand = 0
    for (const wv of this.weaves.values()) if (wv.maker === caster && wv.inHand) inHand += wv.mana()
    return caster.held() + inHand
  }

  /** A point inside the world: the nearest cell's centre, if it's outside. */
  private inside(pos: Vec): Vec {
    if (this.world.cellOf(pos) >= 0) return pos
    const [x, y, z] = this.world.coords(this.world.clampedCellOf(pos))
    return this.world.centre(x, y, z)
  }

  /** Is a point within the caster's reach? Everything their body does or senses is (SPEC §0). */
  private reaches(caster: Caster, at: Vec): boolean {
    const b = caster.body.pos
    return Math.hypot(at[0] - b[0], at[1] - b[1], at[2] - b[2]) <= caster.reach
  }

  /** What a caster feels of a weave: its particles within their reach, in the order they were laid. */
  felt(caster: Caster, weave: Weave): Particle[] {
    const b = caster.body.pos
    const key = `${this.tick}:${this.version}:${weave.particles.length}:${b[0]},${b[1]},${b[2]}:${caster.reach}`
    const known = this.feltCache.get(weave)
    if (known?.key === key) return known.ps
    const ps = weave.particles.filter((p) => this.reaches(caster, p.pos))
    this.feltCache.set(weave, { key, ps })
    return ps
  }

  /**
   * A push: a mind transforms mana into Energy (D32) to change particle `p`'s velocity by `dv`, pushing it off something
   * that takes the push back, equally and oppositely: a caster's body, through their reach; the air a particle is in;
   * or the ground it's against. It costs the kinetic energy it adds to both, at PHYSICS.pushEnergy for each M, no more
   * than `power` has left this tick and `pay` can give; what's asked beyond that, it doesn't get. Slowing down costs
   * nothing: what it takes out of the motion is heat. The mana poured goes loose into the air there, still mana.
   * Returns the Energy transformed.
   */
  private push(p: Particle, dv: Vec, off: Against, pay: (want: number) => Parts, power: { left: number }, from: 'push' | 'kick'): number {
    const size2 = dv[0] * dv[0] + dv[1] * dv[1] + dv[2] * dv[2]
    if (size2 < 1e-24) return 0
    const w = this.world
    const m = massOf(p)
    // What it's pushed off: its mass, and how it moves, along each axis (a body stands: the ground holds it up, so up and
    // down the push goes into the ground).
    let M = Infinity
    let v: Vec = [0, 0, 0]
    let cell = -1
    if (off.kind === 'body') {
      M = off.body.mass
      v = [off.body.vel[0], 0, off.body.vel[2]]
    } else if (off.kind === 'air') {
      cell = off.cell
      M = w.airMass(cell)
      if (M <= PHYSICS.epsilon) return 0 // nothing there to push off
      v = [w.airVel[cell * 3], w.airVel[cell * 3 + 1], w.airVel[cell * 3 + 2]]
    }
    const share = (k: number) => (off.kind === 'body' && k === 1 ? 0 : m / M)
    // The energy a share k of the push adds, with impulse J = k·m·dv: k·lin + k²·quad.
    let lin = 0
    let quad = 0
    for (let k = 0; k < 3; k++) {
      lin += m * dv[k] * (p.vel[k] - v[k])
      quad += 0.5 * m * dv[k] * dv[k] * (1 + share(k))
    }
    let k = 1
    let energy = lin + quad
    if (energy > 0) {
      // As much of it as the power left, and what can be paid, allow: solve k·lin + k²·quad = what there is.
      const can = Math.min(energy, power.left)
      const paid = can > 0 ? pay(can / PHYSICS.pushEnergy) : zero()
      const got = total(paid) * PHYSICS.pushEnergy
      if (got <= 0) return 0
      // The mana poured: a caster's comes from their body, at rest; a particle's own leaves it at its own speed.
      const b = massOfParts(paid)
      w.addAir(w.clampedCellOf(p.pos), paid, from === 'kick' ? (p.vel.map((u) => u * b) as Vec) : [0, 0, 0])
      this.spent[from] += total(paid)
      if (got < energy) k = Math.max(0, Math.min(1, (-lin + Math.sqrt(lin * lin + 4 * quad * got)) / (2 * quad)))
      energy = got
      power.left -= got
    } else w.warm('braking', -energy)
    // The push, and its reaction. (The air there now holds the mana poured, too.)
    const mass = massOf(p)
    if (off.kind === 'air') M = w.airMass(cell)
    for (let i = 0; i < 3; i++) {
      const J = mass * dv[i] * k
      p.vel[i] += dv[i] * k
      if (off.kind === 'body' && i !== 1) off.body.vel[i] -= J / M
      else if (off.kind === 'air') w.airVel[cell * 3 + i] -= J / M
      else w.impulse.walls[i] += J // the ground gives it
    }
    p.pushedAt = this.tick
    if (energy > 0) this.transformed += energy
    return Math.max(0, energy)
  }

  /**
   * What a particle's own push goes off (KICK): the ground, or other matter that holds still, where it's against it on
   * the side it pushes away from; otherwise the air it's in.
   */
  private footing(p: Particle, dv: Vec): Against {
    const w = this.world
    const i = w.cellOf(p.pos)
    if (i < 0) return { kind: 'ground' }
    const [x, y, z] = w.coords(i)
    // Along its push's strongest axis, the cell behind it.
    let ax = 0
    for (let k = 1; k < 3; k++) if (Math.abs(dv[k]) > Math.abs(dv[ax])) ax = k
    const back: Vec = [x, y, z]
    back[ax] -= Math.sign(dv[ax])
    const j = w.index(back[0], back[1], back[2])
    if (j < 0 ? !(ax === 2 && w.d === 1) : w.solidAt(j)) return { kind: 'ground' }
    return { kind: 'air', cell: i }
  }

  // The body, each tick

  private breathe(caster: Caster) {
    const w = this.world
    // Holds run down: unheld mana joins the body's flow.
    for (const r of caster.regs)
      if (total(r.parts) > PHYSICS.epsilon && r.holdUntil <= this.tick) add(caster.flow, take(r.parts, Infinity))
    // The flow drains back to the air, down to its baseline.
    const above = total(caster.flow) - caster.baseline
    if (above > 0) w.addAir(w.clampedCellOf(caster.body.pos), take(caster.flow, Math.min(caster.drain, above)))
    // Overcharge harms.
    const excess = this.load(caster) - caster.capacity
    if (excess > 0) {
      caster.harm += excess
      caster.condition.body = Math.max(0.2, caster.condition.body - (excess / caster.capacity) * 0.01)
      this.log({ kind: 'overcharge', caster: caster.name, detail: `${round(excess)} M past capacity` })
    }
    // Strain eases as the mind rests. Past its capacity, it harms the mind (D32).
    caster.strain = Math.max(0, caster.strain - caster.recovery)
    const over = caster.strain - caster.mindCapacity
    if (over > 0) {
      caster.madness += over
      caster.condition.mind = Math.max(0.2, caster.condition.mind - (over / Math.max(caster.mindCapacity, 1e-9)) * 0.01)
      this.log({ kind: 'overstrain', caster: caster.name, detail: `${round(over * 900)} J past capacity` })
    }
  }

  // Weaves

  /** Matter held by mana, cell by cell: all of it, and each weave's own. */
  private measureCarried() {
    this.carried.fill(0)
    this.carriedBy.clear()
    for (const p of this.world.particles) {
      const i = this.world.cellOf(p.pos)
      const m = fillOf(p.carried)
      if (i < 0 || m <= 0) continue
      this.carried[i] += m
      let own = this.carriedBy.get(p.weave)
      if (!own) this.carriedBy.set(p.weave, (own = new Map()))
      own.set(i, (own.get(i) ?? 0) + m)
    }
  }
  private carriedBy = new Map<number, Map<number, number>>()

  private carriedPart(i: number, k: number): number {
    let v = 0
    for (const p of this.world.particles) if (this.world.cellOf(p.pos) === i) v += p.carried[k]
    return v
  }

  /**
   * A weave's free mana holds what matter it can of its own parts in each cell, shared among its particles there by how
   * much of that part each has. What it can no longer hold, it lets go of.
   */
  private hold(weave: Weave) {
    const w = this.world
    const byCell = new Map<number, Particle[]>()
    for (const p of weave.particles) {
      const i = w.cellOf(p.pos)
      if (i < 0) continue
      let list = byCell.get(i)
      if (!list) byCell.set(i, (list = []))
      list.push(p)
    }
    const before = new Map(weave.particles.map((p) => [p, massOf(p)]))
    for (const [i, ps] of byCell)
      for (let k = 0; k < 4; k++) {
        let free = 0
        let have = 0
        for (const p of ps) {
          free += p.free[k]
          have += p.carried[k]
        }
        const can = (free * PHYSICS.bind) / PHYSICS.manaMass[k] // M of matter: `bind` kilograms for each M
        if (have < can && free > 0) {
          const got = Math.min(can - have, w.matter[i][k])
          if (got <= 0) continue
          w.matter[i][k] -= got
          for (const p of ps) p.carried[k] += (got * p.free[k]) / free
        } else if (have > can && have > 0) {
          const out = have - can
          w.matter[i][k] += out
          for (const p of ps) p.carried[k] -= (out * p.carried[k]) / have
        }
      }
    // Matter taken up was at rest: the particle carries it at its own speed now, and slows for it. Matter let go of
    // stops dead in the ground, and its momentum with it.
    for (const [p, m] of before) {
      const now = massOf(p)
      if (now > m && now > 0) for (let k = 0; k < 3; k++) p.vel[k] *= m / now
      else if (now < m) this.massChanged(p, m)
    }
  }

  /**
   * Every particle carrying an order runs it, on its own: what it writes into its copy of the weave's registers counts
   * from the next tick, and if it lets go (DISS) or frays, only it does. The rest of its weave goes on.
   */
  private runOrders(weave: Weave) {
    const age = this.tick - weave.manifestedAt
    const writes: { p: Particle; k: number; v: number }[] = []
    const leaving: { p: Particle; why: string }[] = []
    let frayed = 0
    let fault = ''
    const traces: OrderTrace[] = []
    if (this.traceOrders) this.traces.set(weave.id, traces)

    for (const [k, p] of weave.particles.entries()) {
      if (!p.order) continue
      const f = frame(PHYSICS.orderRegisters, 16, p.order.addr)
      const off = weave.toFrame(p.pos.map((v, i) => v - weave.origin[i]) as Vec)
      // What it's told: its mana and its age. Not where it is: it only feels (D31). The rest it reads from its copy of
      // its weave's registers, and what it senses.
      f.n.set([0, 0, 0, total(p.free), age])
      const trace: OrderTrace | undefined = this.traceOrders
        ? { tick: this.tick, weave: weave.id, particle: k, id: p.id, off, w: p.regs.slice(), steps: [], n: f.n, outcome: 'done', beats: 0, burned: 0, kick: [0, 0, 0], cnds: 0 }
        : undefined
      let beats = 0
      try {
        const result = this.execOrder(weave, p, f, p.order.program, writes, trace, (b) => (beats += b))
        if (result === 'diss') {
          leaving.push({ p, why: 'its order let it go' })
          if (trace) trace.outcome = 'let go (DISS)'
        }
      } catch (e) {
        if (!(e instanceof Fault)) throw e
        leaving.push({ p, why: 'frayed' })
        frayed++
        fault = e.message
        if (trace) trace.outcome = e.message
      }
      // Thinking burns the particle's own mana, beat by beat. It goes into the air, carrying its momentum.
      const burned = take(p.free, beats * PHYSICS.orderBurn)
      const b = massOfParts(burned)
      this.spent.burn += total(burned)
      this.world.addAir(this.world.clampedCellOf(p.pos), burned, p.vel.map((v) => v * b) as Vec)
      if (trace) {
        trace.n = f.n.slice(0, PHYSICS.orderRegisters)
        trace.burned = total(burned)
        traces.push(trace)
      }
    }
    for (const { p, k, v } of writes) {
      p.regs[k] = v
      p.stamp[k] = stampOf(this.tick, p.id)
    }
    if (frayed) this.log({ kind: 'fray', caster: weave.maker.name, weave: weave.id, detail: `${frayed === 1 ? 'a particle' : `${frayed} particles`}: ${fault}` })
    for (const { p, why } of leaving) {
      p.order = null
      this.loosen(p)
      weave.lastLeft = why
    }
    if (leaving.length) weave.particles = weave.particles.filter((p) => p.weave === weave.id)
  }

  /**
   * A weave's particles pass their copies of its registers on to those touching them (within the smoothing length): the
   * newest write wins. `relay` times a tick, so a write crosses a ball of fire in a tick or two, and nothing it doesn't
   * touch hears of it.
   */
  private relay() {
    const ps = this.world.particles.filter((p) => p.weave)
    if (ps.length < 2) return
    const pairs = findPairs(this.world, ps)
    const R = PHYSICS.weaveRegisters
    const regs = new Float64Array(ps.length * R)
    const stamps = new Float64Array(ps.length * R)
    for (let hop = 0; hop < PHYSICS.relay; hop++) {
      for (let i = 0; i < ps.length; i++) {
        regs.set(ps[i].regs, i * R)
        stamps.set(ps[i].stamp, i * R)
      }
      let changed = false
      for (let e = 0; e < pairs.n; e++) {
        const a = ps[pairs.a[e]]
        const b = ps[pairs.b[e]]
        if (a.weave !== b.weave) continue
        for (let k = 0; k < R; k++) {
          // From what each had before this hop: one hop at a time.
          const sa = stamps[pairs.a[e] * R + k]
          const sb = stamps[pairs.b[e] * R + k]
          if (sa > sb && sa > b.stamp[k]) {
            b.stamp[k] = sa
            b.regs[k] = regs[pairs.a[e] * R + k]
            changed = true
          } else if (sb > sa && sb > a.stamp[k]) {
            a.stamp[k] = sb
            a.regs[k] = regs[pairs.b[e] * R + k]
            changed = true
          }
        }
      }
      if (!changed) break
    }
  }

  /** Runs one particle's order for this tick. */
  private execOrder(
    weave: Weave,
    p: Particle,
    f: Frame,
    program: Program,
    writes: { p: Particle; k: number; v: number }[],
    trace: OrderTrace | undefined,
    spend: (beats: number) => void,
  ): 'done' | 'diss' {
    const w = this.world
    const triple = (r: number, v: Vec) => v.forEach((x, k) => set(f, r + k, x))
    // A particle's mind is as strong as the mana it's in.
    const power = { left: PHYSICS.orderPower * total(p.free) }
    for (let budget = PHYSICS.orderBudget; ; ) {
      const instr = fetch(program, f.pc)
      if (trace) {
        trace.steps.push({ addr: instr.addr, n: f.n.slice(0, PHYSICS.orderRegisters) })
        trace.beats += instr.op.beats
      }
      budget -= instr.op.beats
      spend(instr.op.beats)
      if (budget < 0) throw new Fault('FRAYED', `an order thought more than ${PHYSICS.orderBudget} beats in one tick`)
      const a = instr.args
      const common = execCommon(f, instr)
      if (common === 'ret-empty' || common === 'halt' || common === 'tick') return 'done'
      if (common !== 'unhandled') continue
      const next = () => (f.pc = instr.addr + instr.size)
      switch (instr.op.name) {
        case 'KICK': {
          const dv = turnToWorld(p.yaw, [get(f, a[0]), get(f, a[0] + 1), get(f, a[0] + 2)])
          // It pays with its own mana, and pushes off what's around it: the ground behind it, or the air it's in.
          this.push(p, dv, this.footing(p, dv), (want) => take(p.free, want), power, 'kick')
          if (trace) for (let k = 0; k < 3; k++) trace.kick[k] += get(f, a[0] + k)
          next()
          continue
        }
        case 'TUCH':
          // Whether something stopped it or struck it when the world last moved: matter, the ground, a body, anyone's.
          set(f, a[0], p.touchedAt >= this.tick - 1 ? 1 : 0)
          next()
          continue
        case 'GETW':
          if (a[1] >= p.regs.length) throw new Fault('NO_ROOM', `a weave has w0–w${p.regs.length - 1}`)
          set(f, a[0], p.regs[a[1]])
          next()
          continue
        case 'PUTW':
          if (a[0] >= p.regs.length) throw new Fault('NO_ROOM', `a weave has w0–w${p.regs.length - 1}`)
          writes.push({ p, k: a[0], v: source(f, instr, 1) })
          next()
          continue
        case 'DISS':
          return 'diss'
        case 'CNDS': {
          // As much as asked, up to the room the cell has. Nothing asks whether the amount is above nothing, and below
          // nothing condensing runs backwards: the matter the particle holds comes apart into free mana. Nobody designed
          // that; it is the flaw that makes freeing matter possible (SPEC D16).
          const amount = source(f, instr, 0)
          const i = w.cellOf(p.pos)
          if (trace) trace.cnds += amount
          if (amount > 0 && i >= 0) {
            // As much as the room left in the cell takes, each part of it by its density.
            const room = Math.max(0, 1 - w.fill(i) - this.carried[i])
            const each = total(p.free) > 0 ? fillOf(p.free) / total(p.free) : 0
            const made = take(p.free, Math.min(amount, each > 0 ? room / each : amount))
            add(p.carried, made)
            this.carried[i] += fillOf(made)
          } else if (amount < 0) {
            const freed = take(p.carried, -amount)
            add(p.free, freed)
            if (i >= 0) this.carried[i] -= fillOf(freed)
          }
          // Matter weighs what the mana it was made of did (D40): its mass, and its momentum, don't change.
          next()
          continue
        }
        case 'DENS':
          set(f, a[0], p.felt)
          next()
          continue
        case 'GRAD':
          triple(a[0], turnToFrame(p.yaw, p.grad))
          next()
          continue
        case 'NVEL':
          triple(a[0], turnToFrame(p.yaw, p.nvel))
          next()
          continue
        case 'IN': {
          const port = PORT_BY_CODE.get(a[1])!.name
          let values: number[]
          if (port === 'CELL') values = [w.cell]
          else if (port === 'DEPTH') values = [w.d]
          else if (port === 'VEL') values = turnToFrame(p.yaw, p.vel)
          else throw new Fault('BAD_PORT', `an order can't read ${port}`)
          values.forEach((v, k) => set(f, a[0] + k, v))
          next()
          continue
        }
      }
      throw new Fault('NOT_IN_ORDER', `${instr.op.name} can't run inside an order`)
    }
  }

  /**
   * After the world moves: a weave keeps the particles that carry its order, and those its caster can still reach. One
   * with no order that has got out of their reach, or one with no mana left, leaves it. A weave set loose is measured
   * from its centre again, for whoever watches, and one with nothing left in it is gone.
   */
  private keep(weave: Weave) {
    for (const p of weave.particles) {
      if (total(p.free) <= PHYSICS.epsilon) {
        this.loosen(p)
        weave.lastLeft = 'its mana is gone'
      } else if (!weave.inHand && !p.order && !this.reaches(weave.maker, p.pos)) {
        this.strayed += total(p.free)
        this.loosen(p)
        weave.lastLeft = 'out of reach'
      }
    }
    weave.particles = weave.particles.filter((p) => p.weave === weave.id)
    if (!weave.inHand) weave.origin = weave.centre()?.pos ?? weave.origin
    if (!weave.inHand && !weave.particles.length) this.end_(weave)
  }

  /** A weave with nothing left in it is gone. Its caster can still name it. */
  private end_(weave: Weave) {
    if (!this.weaves.has(weave.id)) return
    this.weaves.delete(weave.id)
    this.gone.set(weave.id, weave)
    this.log({ kind: 'dissolve', caster: weave.maker.name, weave: weave.id, detail: weave.lastLeft || 'its mana is gone' })
  }

  /** A particle leaves its weave. It drops the matter it held; its order stays with it. */
  private loosen(p: Particle) {
    p.weave = 0
    this.dropMatter(p)
  }

  /** Loose mana that has come to the speed of the air around it settles into it. */
  private settleLoose() {
    const w = this.world
    for (const p of w.particles) {
      if (p.weave) continue
      const c = w.clampedCellOf(p.pos)
      const rel = Math.hypot(p.vel[0] - w.airVel[c * 3], p.vel[1] - w.airVel[c * 3 + 1], p.vel[2] - w.airVel[c * 3 + 2])
      if (rel >= PHYSICS.looseRest && total(p.free) > PHYSICS.epsilon) continue
      const m = massOfParts(p.free)
      w.addAir(c, take(p.free, Infinity), p.vel.map((v) => v * m) as Vec)
      this.dropMatter(p)
    }
    this.removeEmpty()
  }

  /** Particles with no mana left are gone. */
  private removeEmpty() {
    this.world.particles = this.world.particles.filter((p) => total(p.free) > PHYSICS.epsilon || total(p.carried) > PHYSICS.epsilon)
  }

  /** Some of a weave's particles go loose, and forget its order. */
  private release(weave: Weave, ps: Particle[], why: string) {
    if (!ps.length) return
    for (const p of ps) {
      this.loosen(p)
      p.order = null
    }
    weave.particles = weave.particles.filter((p) => p.weave === weave.id)
    weave.lastLeft = why
    this.version++
  }

  /** A particle lets go of the matter it holds. It stops dead where it is: its momentum goes into the ground. */
  private dropMatter(p: Particle) {
    const before = massOf(p)
    add(this.world.matter[this.world.clampedCellOf(p.pos)], take(p.carried, Infinity))
    this.massChanged(p, before)
  }

  /**
   * A particle has let go of matter, which stops dead in the ground where it is (the ground's matter doesn't move): the
   * ground takes the momentum it had.
   */
  private massChanged(p: Particle, before: number) {
    const d = massOf(p) - before
    for (let k = 0; k < 3; k++) this.world.impulse.walls[k] += d * p.vel[k]
  }

  /**
   * Earth held by mana becomes rock where it's packed and still (D30): each particle holding earth, in a cell as full of
   * earth as solid ground is, binds to its neighbours that do too (PHYSICS.bondRange), when they hardly move against each
   * other. A bond keeps the length it was made at. Whose mana it is doesn't matter.
   */
  private bond() {
    const w = this.world
    const earthy = (p: Particle) => p.carried[EARTH] > 0.5 * total(p.carried) && p.carried[EARTH] > PHYSICS.epsilon
    const ps = w.particles.filter(earthy)
    if (ps.length < 2) return
    // Earth held in each cell, by anyone.
    const held = new Map<number, number>()
    for (const p of w.particles) {
      const i = w.cellOf(p.pos)
      if (i >= 0 && p.carried[EARTH] > 0) held.set(i, (held.get(i) ?? 0) + p.carried[EARTH])
    }
    const tight = (p: Particle) => {
      const i = w.cellOf(p.pos)
      return i >= 0 && (w.matter[i][EARTH] + (held.get(i) ?? 0)) / packed(EARTH) >= PHYSICS.solid
    }
    const known = new Set(w.bonds.map((b) => (b.a.id < b.b.id ? `${b.a.id}:${b.b.id}` : `${b.b.id}:${b.a.id}`)))
    const pairs = findPairs(w, ps)
    const isPacked = new Map<Particle, boolean>()
    const check = (p: Particle) => {
      let v = isPacked.get(p)
      if (v === undefined) isPacked.set(p, (v = tight(p)))
      return v
    }
    for (let e = 0; e < pairs.n; e++) {
      const r = pairs.r[e]
      if (r >= PHYSICS.bondRange || r < 1e-6) continue
      const a = ps[pairs.a[e]]
      const b = ps[pairs.b[e]]
      const key = a.id < b.id ? `${a.id}:${b.id}` : `${b.id}:${a.id}`
      if (known.has(key)) continue
      if (Math.hypot(a.vel[0] - b.vel[0], a.vel[1] - b.vel[1], a.vel[2] - b.vel[2]) > PHYSICS.bondSpeed) continue
      if (!check(a) || !check(b)) continue
      w.bonds.push({ a, b, rest: r })
      known.add(key)
    }
  }

  /**
   * After merging and splitting, each weave's particles are those that are its own. A weave's particle that took in
   * someone else's mana took it over: their weave lost it, and it's this weave's now, with this weave's order (the second
   * flaw, SPEC §11). Each tick's takeovers are logged, one event for each weave that took and whose it took.
   */
  private regroup(changes: Change[]) {
    if (!changes.length) return
    const taken = new Map<string, { into: number; from: number; mana: number; ordered: boolean }>()
    for (const c of changes) {
      if (c.kind !== 'merge' || !c.into.weave || c.weave === c.into.weave) continue
      const key = `${c.into.weave}:${c.weave}`
      const t = taken.get(key) ?? { into: c.into.weave, from: c.weave, mana: 0, ordered: false }
      t.mana += total(c.into.free) // what the merged particle now holds
      t.ordered ||= !!c.into.order
      taken.set(key, t)
    }
    for (const weave of this.weaves.values()) weave.particles = this.world.particles.filter((p) => p.weave === weave.id)
    for (const t of taken.values()) {
      const wv = this.weaves.get(t.into)
      this.log({
        kind: 'taken',
        caster: wv?.maker.name ?? '',
        weave: t.into,
        detail: `took in ${t.from ? `weave ${t.from}'s` : 'loose'} mana${t.ordered ? ', and gave it its order' : ''}`,
      })
    }
  }

  /** Bonds hold only particles that are still in the world, and still hold earth. */
  private unbind() {
    const alive = new Set(this.world.particles)
    this.world.bonds = this.world.bonds.filter((b) => alive.has(b.a) && alive.has(b.b) && b.a.carried[EARTH] > PHYSICS.epsilon && b.b.carried[EARTH] > PHYSICS.epsilon)
  }
}

// What minds and orders share: numbers, jumps, calls, the stack and memory.

function frame(registers: number, memory: number, pc: number): Frame {
  return { n: new Float64Array(32), limit: registers, flags: 0, stack: [], calls: [], memory: new Float64Array(memory), pc }
}

function get(f: Frame, r: number): number {
  if (r >= f.limit) throw new Fault('NO_ROOM', `n${r}: this mind thinks with ${f.limit} registers`)
  return f.n[r]
}

function set(f: Frame, r: number, v: number) {
  if (r >= f.limit) throw new Fault('NO_ROOM', `n${r}: this mind thinks with ${f.limit} registers`)
  f.n[r] = Number.isFinite(v) ? v : 0
}

function source(f: Frame, instr: Instr, k: number): number {
  const last = k === instr.op.operands.length - 1
  return last && instr.immediate ? instr.args[k] : get(f, instr.args[k])
}

/** Runs the instructions every frame shares. Returns 'unhandled' for the rest. */
function execCommon(f: Frame, instr: Instr): 'next' | 'jump' | 'ret-empty' | 'halt' | 'tick' | 'unhandled' {
  const a = instr.args
  const next = instr.addr + instr.size
  const s = (k: number) => source(f, instr, k)
  const arith = (fn: (x: number, y: number) => number) => {
    set(f, a[0], fn(get(f, a[0]), s(1)))
    f.pc = next
    return 'next' as const
  }
  const unary = (fn: (x: number) => number) => {
    set(f, a[0], fn(get(f, a[0])))
    f.pc = next
    return 'next' as const
  }
  const jump = (when: boolean) => {
    f.pc = when ? a[0] : next
    return 'jump' as const
  }
  const depth = f.memory.length

  switch (instr.op.name) {
    case 'NOP':
      f.pc = next
      return 'next'
    case 'HALT':
      f.pc = next
      return 'halt'
    case 'TICK':
      f.pc = next
      return 'tick'
    case 'JMP':
      return jump(true)
    case 'JEQ':
      return jump(f.flags === 0)
    case 'JNE':
      return jump(f.flags !== 0)
    case 'JLT':
      return jump(f.flags < 0)
    case 'JLE':
      return jump(f.flags <= 0)
    case 'JGT':
      return jump(f.flags > 0)
    case 'JGE':
      return jump(f.flags >= 0)
    case 'CALL':
      if (f.calls.length >= Math.max(16, depth)) throw new Fault('NO_ROOM', 'calls nested too deep for this mind')
      f.calls.push(next)
      f.pc = a[0]
      return 'jump'
    case 'RET':
      if (!f.calls.length) return 'ret-empty'
      f.pc = f.calls.pop()!
      return 'jump'
    case 'LDI':
    case 'MOV':
      set(f, a[0], s(1))
      f.pc = next
      return 'next'
    case 'ADD':
      return arith((x, y) => x + y)
    case 'SUB':
      return arith((x, y) => x - y)
    case 'MUL':
      return arith((x, y) => x * y)
    case 'DIV':
      return arith((x, y) => (y === 0 ? 0 : x / y))
    case 'MOD':
      return arith((x, y) => (y === 0 ? 0 : x % y))
    case 'ATN2':
      return arith((x, y) => Math.atan2(x, y))
    case 'MIN':
      return arith(Math.min)
    case 'MAX':
      return arith(Math.max)
    case 'NEG':
      return unary((x) => -x)
    case 'ABS':
      return unary(Math.abs)
    case 'SQRT':
      return unary((x) => (x < 0 ? 0 : Math.sqrt(x)))
    case 'FLOOR':
      return unary(Math.floor)
    case 'ROUND':
      return unary(Math.round)
    case 'SIN':
      return unary(Math.sin)
    case 'COS':
      return unary(Math.cos)
    case 'TAN':
      return unary(Math.tan)
    case 'ATAN':
      return unary(Math.atan)
    case 'CMP': {
      const d = get(f, a[0]) - s(1)
      f.flags = d < 0 ? -1 : d > 0 ? 1 : 0
      f.pc = next
      return 'next'
    }
    case 'PUSH':
      if (f.stack.length >= Math.max(16, depth)) throw new Fault('NO_ROOM', 'the stack is full')
      f.stack.push(get(f, a[0]))
      f.pc = next
      return 'next'
    case 'POP':
      if (!f.stack.length) throw new Fault('NO_ROOM', 'the stack is empty')
      set(f, a[0], f.stack.pop()!)
      f.pc = next
      return 'next'
    case 'LD':
    case 'ST': {
      const addr = Math.floor(get(f, a[1]))
      if (addr < 0 || addr >= depth) throw new Fault('NO_ROOM', `memory has ${depth} numbers, not [${addr}]`)
      if (instr.op.name === 'LD') set(f, a[0], f.memory[addr])
      else f.memory[addr] = get(f, a[0])
      f.pc = next
      return 'next'
    }
  }
  return 'unhandled'
}

function round(v: number) {
  return Math.round(v * 100) / 100
}
