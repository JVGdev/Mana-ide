// The machine running in the world: casters casting, weaves running their orders, the world moving. One tick at a time.

import { decode, type Instr } from '../asm/disassembler.ts'
import { PORT_BY_CODE } from '../asm/isa.ts'
import type { Program } from '../asm/assembler.ts'
import { PHYSICS } from './physics.ts'
import { add, take, total, zero, type Parts } from './parts.ts'
import { Caster, type CasterStats, type ManaRegister } from './caster.ts'
import { Weave } from './weave.ts'
import { EARTH, World, massOf, type Particle, type Vec } from './world.ts'
import { airFlow, mergeAndSplit, stepFluid, type Change, type FluidHooks } from './fluid.ts'

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
  /** The weave's registers as the tick began: what GETW reads. */
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
  /** M spent so far: poured onto particles by casters (`push`) and by orders on themselves (`kick`), and burned thinking. */
  spent = { push: 0, kick: 0, burn: 0 }
  /** M of free mana that has strayed out of its weave's field, and left it. */
  strayed = 0

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
    for (const cast of this.casts)
      if (cast.state === 'running') {
        cast.left = cast.caster.speed * (1 + (cast.caster.conditioning.get(cast.name) ?? 0)) + Math.min(0, cast.left)
        cast.yielded = cast.left <= 0
      }
  }

  /** The rest of a tick: minds still thinking finish, then bodies, weaves and the world. */
  private endTick() {
    const w = this.world
    // 1. Minds think.
    for (const cast of this.casts) if (cast.state === 'running') this.think(cast)
    this.midTick = false
    // 2–4. Bodies: holds run down, flows drain, overcharge.
    for (const c of this.casters) this.breathe(c)
    // 5. Weaves take hold of what matter their mana can bind, and let go of what it can't.
    for (const weave of this.weaves.values()) this.hold(weave)
    // 6. Particles of weaves set loose run their orders.
    if (this.traceOrders) this.traces = new Map()
    this.measureCarried()
    for (const weave of [...this.weaves.values()]) if (!weave.inHand) this.runOrders(weave)
    // 7. The world moves: the mana, the air, bodies, matter.
    this.measureCarried()
    const hooks: FluidHooks = {
      passes: (p) => (p.weave ? this.weaves.get(p.weave)?.maker.body : undefined),
      still: (p) => !!p.weave && !!this.weaves.get(p.weave)?.inHand,
      othersCarried: (p, cell) => this.carried[cell] - (this.carriedBy.get(p.weave)?.get(cell) ?? 0),
    }
    stepFluid(w, hooks)
    airFlow(w)
    // Particles at rest together merge, and big ones spread thin split.
    this.regroup(mergeAndSplit(w, hooks))
    for (const weave of [...this.weaves.values()]) this.keep(weave)
    this.settleLoose()
    this.unbind()
    w.moveBodies()
    this.measureCarried()
    w.settleMatter(this.carried)
    w.diffuseAir()
    for (const p of w.particles) p.dvLeft = PHYSICS.pushRate
    w.tick++
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
        if (i >= 0 && Number.isInteger(k) && k >= 0 && k < 4) {
          v = op.name === 'AIRM' ? this.world.air[i][k] : this.world.matter[i][k] + this.carriedPart(i, k)
        }
        set(f, a[0], v)
        return next()
      }
      case 'SEND': {
        // Mana let out with a speed: the speed is paid for from the mana sent, which goes loose where it was let out.
        const r = m(0)
        const parts = take(r.parts, Math.max(0, n(1)))
        const pos = this.inside(vec(2))
        const vel = vec(3)
        // Each M sent at speed v carries ½v² of kinetic energy, paid for at pushEnergy a M.
        const sent = 1 / (1 + (0.5 * (vel[0] ** 2 + vel[1] ** 2 + vel[2] ** 2)) / PHYSICS.pushEnergy)
        const spent = take(parts, total(parts) * (1 - sent))
        this.world.addAir(this.world.clampedCellOf(pos), spent)
        const t = total(parts)
        if (t > PHYSICS.epsilon) {
          for (let k = 0; k < 3; k++) this.world.impulse.push[k] += t * vel[k]
          this.world.pour(parts, pos, 0, vel)
        }
        return next()
      }
      case 'WPOS': {
        const wv = weave(1)
        const c = wv.centre()
        ;(c ? c.pos : wv.origin).forEach((v, k) => set(f, a[0] + k, v))
        return next()
      }
      case 'WVEL': {
        const wv = weave(1)
        const c = wv.centre()
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
        return next()
      }
      case 'EMIT': {
        const r = m(0)
        const amount = Math.max(0, n(1))
        const wv = weave(2)
        if (wv.locks.input) throw new Fault('LOCKED', `weave ${wv.id}'s input is locked`)
        const pos = wv.worldPos(vec(3))
        if (this.world.cellOf(pos) < 0) return next() // outside the world: nothing goes there
        const gone = this.gone.has(wv.id)
        const poured = this.world.pour(take(r.parts, amount), pos, gone ? 0 : wv.id)
        if (!gone) wv.particles.push(...poured)
        return next()
      }
      case 'WSET': {
        const wv = weave(0)
        if (a[1] >= wv.regs.length) throw new Fault('NO_ROOM', `a weave has w0–w${wv.regs.length - 1}`)
        wv.regs[a[1]] = s(2)
        return next()
      }
      case 'WGET': {
        const wv = weave(1)
        if (a[2] >= wv.regs.length) throw new Fault('NO_ROOM', `a weave has w0–w${wv.regs.length - 1}`)
        set(f, a[0], wv.regs[a[2]])
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
          this.bind(wv)
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
      case 'RELS':
        this.dissolve(weave(0), 'let go')
        return next()
      case 'PCNT':
        set(f, a[0], weave(1).particles.length)
        return next()
      case 'PPOS':
      case 'PVEL': {
        const wv = weave(1)
        const p = wv.particles[Math.floor(s(2))]
        let v: Vec = [0, 0, 0]
        if (p) v = op.name === 'PVEL' ? wv.toFrame(p.vel) : wv.toFrame(p.pos.map((x, k) => x - wv.origin[k]) as Vec)
        v.forEach((x, k) => set(f, a[0] + k, x))
        return next()
      }
      case 'SHOV': {
        const r = m(0)
        const wv = weave(1)
        const p = wv.particles[Math.floor(n(2))]
        if (!p || !this.inReach(caster, p)) return next()
        this.push(p, wv.toWorld(vec(3)), (want) => take(r.parts, want), 'push')
        return next()
      }
      case 'INGR': {
        const wv = weave(0)
        if (wv.order === null) throw new Fault('NO_ORDER', `weave ${wv.id} has no order to ingrain: give it one with ORDR`)
        this.extra = orderLength(wv.program, wv.order)
        const p = wv.particles[Math.floor(s(1))]
        if (p && this.inReach(caster, p)) p.order = { program: wv.program, addr: wv.order }
        return next()
      }
      case 'HOLD':
        weave(0).field = Math.max(0, s(1))
        return next()
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
    const reach = PHYSICS.gatherRadius
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
    for (const i of cells) add(got, w.takeAir(i, f))
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

  private inReach(caster: Caster, p: Particle): boolean {
    const b = caster.body.pos
    return Math.hypot(p.pos[0] - b[0], p.pos[1] - b[1], p.pos[2] - b[2]) <= caster.reach
  }

  /**
   * Pour mana onto a particle to change its velocity by `dv`, no more than it has left this tick. It costs the kinetic
   * energy it adds (PHYSICS.pushEnergy); slowing it costs nothing. `pay` gives what it can of what's asked. What's
   * poured goes loose into the air where it was poured. Returns the M spent.
   */
  private push(p: Particle, dv: Vec, pay: (want: number) => Parts, from: 'push' | 'kick'): number {
    let size = Math.hypot(...dv)
    if (size < 1e-12 || p.dvLeft <= 0) return 0
    if (size > p.dvLeft) {
      dv = dv.map((v) => (v * p.dvLeft) / size) as Vec
      size = p.dvLeft
    }
    // The energy a share k of the push adds: ½M(|v + k·dv|² − |v|²) = k·M(v·dv) + k²·½M|dv|².
    const M = massOf(p)
    const lin = M * (p.vel[0] * dv[0] + p.vel[1] * dv[1] + p.vel[2] * dv[2])
    const quad = 0.5 * M * size * size
    const want = Math.max(0, lin + quad) / PHYSICS.pushEnergy
    let k = 1
    let got = 0
    if (want > 0) {
      const paid = pay(want)
      got = total(paid)
      if (got <= 0) return 0
      const c = this.world.clampedCellOf(p.pos)
      // Mana a particle pays with itself leaves carrying its share of the particle's momentum.
      this.world.addAir(c, paid, from === 'kick' ? (p.vel.map((v) => v * got) as Vec) : [0, 0, 0])
      this.spent[from] += got
      if (got < want) {
        // As much of the push as what was paid buys: solve k·lin + k²·quad = energy.
        const e = got * PHYSICS.pushEnergy
        k = (-lin + Math.sqrt(lin * lin + 4 * quad * e)) / (2 * quad)
        k = Math.max(0, Math.min(1, k))
      }
    }
    const m = massOf(p)
    for (let i = 0; i < 3; i++) {
      p.vel[i] += dv[i] * k
      this.world.impulse[from][i] += m * dv[i] * k
    }
    p.dvLeft -= size * k
    p.pushedAt = this.tick
    return got
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
  }

  // Weaves

  /** Matter held by mana, cell by cell: all of it, and each weave's own. */
  private measureCarried() {
    this.carried.fill(0)
    this.carriedBy.clear()
    for (const p of this.world.particles) {
      const i = this.world.cellOf(p.pos)
      const m = total(p.carried)
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
        const can = free * PHYSICS.bind
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

  /** Every particle carrying an order runs it. */
  private runOrders(weave: Weave) {
    const age = this.tick - weave.manifestedAt
    const pending = weave.regs.slice()
    const w0 = weave.regs.slice()
    let ending: string | undefined
    const traces: OrderTrace[] = []
    if (this.traceOrders) this.traces.set(weave.id, traces)

    for (const [k, p] of weave.particles.entries()) {
      if (!p.order) continue
      const f = frame(PHYSICS.orderRegisters, 16, p.order.addr)
      const off = weave.toFrame(p.pos.map((v, i) => v - weave.origin[i]) as Vec)
      // What it's told: where it is from its weave's centre (unless orders only feel, PHYSICS.orderKnowsCentre), its mana, its age.
      const told = PHYSICS.orderKnowsCentre ? off : [0, 0, 0]
      f.n.set([told[0], told[1], told[2], total(p.free), age])
      const trace: OrderTrace | undefined = this.traceOrders
        ? { tick: this.tick, weave: weave.id, particle: k, id: p.id, off, w: w0, steps: [], n: f.n, outcome: 'done', beats: 0, burned: 0, kick: [0, 0, 0], cnds: 0 }
        : undefined
      let beats = 0
      try {
        const result = this.execOrder(weave, p, f, p.order.program, pending, trace, (b) => (beats += b))
        if (result === 'diss') ending = 'its order let it go'
        if (trace && result === 'diss') trace.outcome = 'let go (DISS)'
      } catch (e) {
        if (!(e instanceof Fault)) throw e
        this.log({ kind: 'fray', caster: weave.maker.name, weave: weave.id, detail: e.message })
        ending = 'frayed'
        if (trace) trace.outcome = e.message
      }
      // Thinking burns the particle's own mana, beat by beat. It goes into the air, carrying its momentum.
      const burned = take(p.free, beats * PHYSICS.orderBurn)
      const b = total(burned)
      this.spent.burn += b
      this.world.addAir(this.world.clampedCellOf(p.pos), burned, p.vel.map((v) => v * b) as Vec)
      if (trace) {
        trace.n = f.n.slice(0, PHYSICS.orderRegisters)
        trace.burned = b
        traces.push(trace)
      }
      if (ending) break
    }
    weave.regs.set(pending)
    if (ending) this.dissolve(weave, ending)
  }

  /** Runs one particle's order for this tick. */
  private execOrder(
    weave: Weave,
    p: Particle,
    f: Frame,
    program: Program,
    pending: Float64Array,
    trace: OrderTrace | undefined,
    spend: (beats: number) => void,
  ): 'done' | 'diss' {
    const w = this.world
    const triple = (r: number, v: Vec) => v.forEach((x, k) => set(f, r + k, x))
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
          const dv = weave.toWorld([get(f, a[0]), get(f, a[0] + 1), get(f, a[0] + 2)])
          // A particle pays with itself, but never more than half of what it has in one push.
          this.push(p, dv, (want) => take(p.free, Math.min(want, total(p.free) * 0.5)), 'kick')
          if (trace) for (let k = 0; k < 3; k++) trace.kick[k] += get(f, a[0] + k)
          next()
          continue
        }
        case 'TUCH':
          set(f, a[0], this.touches(weave, p) ? 1 : 0)
          next()
          continue
        case 'GETW':
          if (a[1] >= weave.regs.length) throw new Fault('NO_ROOM', `a weave has w0–w${weave.regs.length - 1}`)
          set(f, a[0], weave.regs[a[1]])
          next()
          continue
        case 'PUTW':
          if (a[0] >= weave.regs.length) throw new Fault('NO_ROOM', `a weave has w0–w${weave.regs.length - 1}`)
          pending[a[0]] = source(f, instr, 1)
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
          const before = massOf(p)
          if (amount > 0 && i >= 0) {
            const room = Math.max(0, PHYSICS.cellMatter - total(w.matter[i]) - this.carried[i])
            const made = take(p.free, Math.min(amount, room))
            add(p.carried, made)
            this.carried[i] += total(made)
          } else if (amount < 0) {
            const freed = take(p.carried, -amount)
            add(p.free, freed)
            if (i >= 0) this.carried[i] -= total(freed)
          }
          // Matter doesn't weigh what the mana it was made of did: the particle's mass changes, at its own speed.
          this.massChanged(p, before)
          next()
          continue
        }
        case 'DENS':
          set(f, a[0], p.rho)
          next()
          continue
        case 'GRAD':
          triple(a[0], weave.toFrame(p.grad))
          next()
          continue
        case 'NVEL':
          triple(a[0], weave.toFrame(p.nvel))
          next()
          continue
        case 'IN': {
          const port = PORT_BY_CODE.get(a[1])!.name
          let values: number[]
          if (port === 'CELL') values = [w.cell]
          else if (port === 'DEPTH') values = [w.d]
          else if ((port === 'ORIGIN' || port === 'MAKER') && !PHYSICS.orderKnowsCentre) throw new Fault('BAD_PORT', `an order only feels: it can't read ${port}`)
          else if (port === 'ORIGIN') values = [...weave.origin]
          else if (port === 'MAKER') values = [...weave.maker.body.pos]
          else if (port === 'VEL') values = weave.toFrame(p.vel)
          else throw new Fault('BAD_PORT', `an order can't read ${port}`)
          values.forEach((v, k) => set(f, a[0] + k, v))
          next()
          continue
        }
      }
      throw new Fault('NOT_IN_ORDER', `${instr.op.name} can't run inside an order`)
    }
  }

  /** Is this particle against something that isn't its own: matter, or a body other than its maker's? */
  private touches(weave: Weave, p: Particle): boolean {
    const w = this.world
    const i = w.cellOf(p.pos)
    if (i < 0) return true
    const [x, y, z] = w.coords(i)
    const around: [number, number, number][] = [[0, 0, 0], [1, 0, 0], [-1, 0, 0], [0, 1, 0], [0, -1, 0]]
    if (w.d > 1) around.push([0, 0, 1], [0, 0, -1])
    for (const [dx, dy, dz] of around) {
      const j = w.index(x + dx, y + dy, z + dz)
      if (j < 0) continue
      if (w.solidAt(j)) return true
      const [jx, jy, jz] = w.coords(j)
      const c = w.centre(jx, jy, jz)
      if (w.d === 1) c[2] = p.pos[2]
      if (w.bodyAt(c, weave.maker.body)) return true
    }
    return false
  }

  /**
   * After the world moves: a weave's particles that have got further from its centre than its field reaches leave it
   * for good, and a weave set loose is measured from its centre again. A loose weave with nothing left in it is gone.
   */
  private keep(weave: Weave) {
    const c = weave.centre()
    if (c) {
      for (const p of weave.particles) {
        const d = Math.hypot(p.pos[0] - c.pos[0], p.pos[1] - c.pos[1], p.pos[2] - c.pos[2])
        if (d > weave.field) this.strayed += total(p.free)
        if (d > weave.field || total(p.free) <= PHYSICS.epsilon) this.loosen(p)
      }
      weave.particles = weave.particles.filter((p) => p.weave === weave.id)
      if (!weave.inHand) weave.origin = weave.centre()?.pos ?? c.pos
    }
    if (!weave.inHand && !weave.particles.length) {
      this.weaves.delete(weave.id)
      this.gone.set(weave.id, weave)
      this.log({ kind: 'dissolve', caster: weave.maker.name, weave: weave.id, detail: 'its mana is gone' })
    }
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
      const m = total(p.free)
      w.addAir(c, take(p.free, Infinity), p.vel.map((v) => v * m) as Vec)
      this.dropMatter(p)
    }
    this.removeEmpty()
  }

  /** Particles with no mana left are gone. */
  private removeEmpty() {
    this.world.particles = this.world.particles.filter((p) => total(p.free) > PHYSICS.epsilon || total(p.carried) > PHYSICS.epsilon)
  }

  private dissolve(weave: Weave, why: string) {
    for (const p of weave.particles) {
      this.loosen(p)
      p.order = null
    }
    weave.particles = []
    this.weaves.delete(weave.id)
    this.gone.set(weave.id, weave)
    this.log({ kind: 'dissolve', caster: weave.maker.name, weave: weave.id, detail: why })
  }

  /** A particle lets go of the matter it holds. It stops dead where it is: its momentum goes into the ground. */
  private dropMatter(p: Particle) {
    const before = massOf(p)
    add(this.world.matter[this.world.clampedCellOf(p.pos)], take(p.carried, Infinity))
    this.massChanged(p, before)
  }

  /** A particle's mass has changed with nothing pushing it: what it gained or lost, it gained or lost at its own speed. */
  private massChanged(p: Particle, before: number) {
    const d = massOf(p) - before
    for (let k = 0; k < 3; k++) this.world.impulse.matter[k] += d * p.vel[k]
  }

  /**
   * A weave set loose with earth in it is rock: each particle holding earth is bound to its neighbours that do
   * (PHYSICS.bondRange). The bonds hold it in the shape it was laid out in.
   */
  private bind(weave: Weave) {
    const rock = weave.particles.filter((p) => p.carried[EARTH] > 0.5 * total(p.carried) && p.carried[EARTH] > PHYSICS.epsilon)
    const r = PHYSICS.bondRange
    const grid = new Map<string, Particle[]>()
    const at = (p: Particle) => p.pos.map((v) => Math.floor(v / r))
    for (const p of rock) {
      const k = at(p).join()
      grid.set(k, [...(grid.get(k) ?? []), p])
    }
    for (const a of rock) {
      const [x, y, z] = at(a)
      for (let dx = -1; dx <= 1; dx++)
        for (let dy = -1; dy <= 1; dy++)
          for (let dz = -1; dz <= 1; dz++)
            for (const b of grid.get([x + dx, y + dy, z + dz].join()) ?? []) {
              if (b.id <= a.id) continue
              const d = Math.hypot(a.pos[0] - b.pos[0], a.pos[1] - b.pos[1], a.pos[2] - b.pos[2])
              if (d < r && d > 1e-6) this.world.bonds.push({ a, b, rest: d })
            }
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

  /** Bonds hold only particles of the same weave that are still in the world. */
  private unbind() {
    const alive = new Set(this.world.particles)
    this.world.bonds = this.world.bonds.filter((b) => b.a.weave !== 0 && b.a.weave === b.b.weave && alive.has(b.a) && alive.has(b.b))
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
