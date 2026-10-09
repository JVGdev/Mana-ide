// The machine running in the world: casters casting, weaves running their orders, the world moving. One tick at a time.

import { decode, type Instr } from '../asm/disassembler.ts'
import { PORT_BY_CODE } from '../asm/isa.ts'
import type { Program } from '../asm/assembler.ts'
import { PHYSICS } from './physics.ts'
import { add, share, take, total, zero, type Parts } from './parts.ts'
import { Caster, type CasterStats, type ManaRegister } from './caster.ts'
import { Weave, type WeaveCell } from './weave.ts'
import { World, type Vec } from './world.ts'

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

/** What one cell's order asked for this tick. */
type CellOrder = { move: Vec; cnds: number }

const decoded = new WeakMap<Uint8Array, Map<number, Instr>>()

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
  /** Matter weaves hold, per world cell, as of the start of this tick's orders. */
  private carried: Float64Array

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

  /** The start of a tick: every running mind gets its beats. */
  private startThinking() {
    if (this.midTick) return
    this.midTick = true
    for (const cast of this.casts)
      if (cast.state === 'running') {
        cast.left = cast.caster.speed * (1 + (cast.caster.conditioning.get(cast.name) ?? 0))
        cast.yielded = false
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
    // 5. Weaves leak, and take hold of what matter they can.
    for (const weave of this.weaves.values()) this.hold(weave)
    // 6. Weaves set loose run their orders.
    this.measureCarried()
    for (const weave of [...this.weaves.values()]) if (!weave.inHand && weave.order !== null) this.runOrder(weave)
    // 7. The world moves.
    this.measureCarried()
    w.moveLoose()
    w.moveBodies()
    w.settleMatter(this.carried)
    w.diffuseAir()
    w.tick++
  }

  /** Every M of mana in the world, wherever it is. Conservation says `total` never changes. */
  ledger() {
    const world = this.world.mana()
    let weaves = 0
    let carried = 0
    for (const wv of this.weaves.values())
      for (const c of wv.cells) {
        weaves += total(c.free)
        carried += total(c.carried)
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
      try {
        if (this.execMind(cast, instr) === 'tick') cast.yielded = true
      } catch (e) {
        if (!(e instanceof Fault)) throw e
        this.end(cast, 'fault', e.message)
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
        for (const c of weave.cells) {
          add(caster.flow, c.free)
          this.dropMatter(weave, c)
        }
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
      const wv = this.weaves.get(n(k))
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
        add(this.world.air[this.world.clampedCellOf(caster.body.pos)], take(m(0).parts, Infinity))
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
        const r = m(0)
        const parts = take(r.parts, Math.max(0, n(1)))
        const pos = vec(2)
        const i = this.world.clampedCellOf(pos)
        const [x, y, z] = this.world.coords(i)
        const inside = this.world.cellOf(pos) >= 0 ? pos : this.world.centre(x, y, z)
        if (total(parts) > PHYSICS.epsilon) this.world.loose.push({ pos: [...inside], vel: vec(3), parts })
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
        const off = vec(3)
        const pos = wv.worldPos({ off: off.map((v) => Math.round(v / this.world.cell) * this.world.cell) as Vec, free: zero(), carried: zero() })
        if (this.world.cellOf(pos) < 0) return next() // outside the world: nothing goes there
        add(wv.cellAt(off, this.world.cell).free, take(r.parts, amount))
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
          this.log({ kind: 'manifest', caster: caster.name, weave: wv.id })
        }
        return next()
      }
      case 'LOCK': {
        const wv = weave(0)
        if (wv.inHand) throw new Fault('NOT_LOOSE', `weave ${wv.id} has to be manifested before it's locked`)
        const what = (['shape', 'input', 'order'] as const)[a[1]]
        if (!what) throw new Fault('NO_ROOM', `there's nothing to lock called #${a[1]}`)
        wv.locks[what] = true
        this.log({ kind: 'lock', caster: caster.name, weave: wv.id, detail: what })
        return next()
      }
      case 'RELS':
        this.dissolve(weave(0), 'let go')
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
    for (const i of cells) add(got, share(w.air[i], f))
    return got
  }

  private load(caster: Caster): number {
    let inHand = 0
    for (const wv of this.weaves.values()) if (wv.maker === caster && wv.inHand) for (const c of wv.cells) inHand += total(c.free)
    return caster.held() + inHand
  }

  // The body, each tick

  private breathe(caster: Caster) {
    const w = this.world
    // Holds run down: unheld mana joins the body's flow.
    for (const r of caster.regs)
      if (total(r.parts) > PHYSICS.epsilon && r.holdUntil <= this.tick) add(caster.flow, take(r.parts, Infinity))
    // The flow drains back to the air, down to its baseline.
    const above = total(caster.flow) - caster.baseline
    if (above > 0) add(w.air[w.clampedCellOf(caster.body.pos)], take(caster.flow, Math.min(caster.drain, above)))
    // Overcharge harms.
    const excess = this.load(caster) - caster.capacity
    if (excess > 0) {
      caster.harm += excess
      caster.condition.body = Math.max(0.2, caster.condition.body - (excess / caster.capacity) * 0.01)
      this.log({ kind: 'overcharge', caster: caster.name, detail: `${round(excess)} M past capacity` })
    }
  }

  // Weaves

  private measureCarried() {
    this.carried.fill(0)
    for (const wv of this.weaves.values())
      for (const c of wv.cells) {
        const i = this.world.cellOf(wv.worldPos(c))
        if (i >= 0) this.carried[i] += total(c.carried)
      }
  }

  private carriedPart(i: number, k: number): number {
    let v = 0
    for (const wv of this.weaves.values()) for (const c of wv.cells) if (this.world.cellOf(wv.worldPos(c)) === i) v += c.carried[k]
    return v
  }

  private runOrder(weave: Weave) {
    const w = this.world
    const program = weave.program
    const age = this.tick - weave.manifestedAt
    const pending = weave.regs.slice()
    const occupied = new Set(weave.cells.map((c) => w.cellOf(weave.worldPos(c))))
    const orders: CellOrder[] = []
    let ending: string | undefined

    for (const cell of weave.cells) {
      const f = frame(PHYSICS.orderRegisters, 16, weave.order!)
      f.n.set([cell.off[0], cell.off[1], cell.off[2], total(cell.free), age])
      const order: CellOrder = { move: [0, 0, 0], cnds: 0 }
      orders.push(order)
      try {
        const result = this.execOrder(weave, cell, f, program, order, pending, occupied)
        if (result === 'diss') ending = 'its order let it go'
      } catch (e) {
        if (!(e instanceof Fault)) throw e
        this.log({ kind: 'fray', caster: weave.maker.name, weave: weave.id, detail: e.message })
        ending = 'frayed'
      }
      if (ending) break
    }
    weave.regs.set(pending)
    if (ending) {
      this.dissolve(weave, ending)
      return
    }
    // Condense: as much as the order asked for, up to the room the cell has. Nothing asks whether the amount is
    // above nothing, and below nothing condensing runs backwards: the matter the cell holds comes apart into free
    // mana. Nobody designed that; it is the flaw that makes freeing matter possible (SPEC D16).
    weave.cells.forEach((cell, k) => {
      const i = w.cellOf(weave.worldPos(cell))
      const room = i < 0 ? 0 : Math.max(0, PHYSICS.cellMatter - total(w.matter[i]) - this.carried[i])
      const amount = Math.min(orders[k].cnds, room)
      if (amount > 0) {
        const made = take(cell.free, amount)
        add(cell.carried, made)
        this.carried[i] += total(made)
      } else if (amount < 0) {
        const freed = take(cell.carried, -amount)
        add(cell.free, freed)
        if (i >= 0) this.carried[i] -= total(freed)
      }
    })
    this.moveWeave(
      weave,
      orders.map((o) => o.move),
    )
  }

  /** Runs one cell's order for this tick. */
  private execOrder(
    weave: Weave,
    cell: WeaveCell,
    f: Frame,
    program: Program,
    order: CellOrder,
    pending: Float64Array,
    occupied: Set<number>,
  ): 'done' | 'diss' {
    const w = this.world
    for (let budget = PHYSICS.orderBudget; ; ) {
      const instr = fetch(program, f.pc)
      budget -= instr.op.beats
      if (budget < 0) throw new Fault('FRAYED', `an order thought more than ${PHYSICS.orderBudget} beats in one tick`)
      const a = instr.args
      const common = execCommon(f, instr)
      if (common === 'ret-empty' || common === 'halt' || common === 'tick') return 'done'
      if (common !== 'unhandled') continue
      const next = () => (f.pc = instr.addr + instr.size)
      switch (instr.op.name) {
        case 'MOVE':
          order.move[0] += get(f, a[0])
          order.move[1] += get(f, a[0] + 1)
          order.move[2] += get(f, a[0] + 2)
          next()
          continue
        case 'TUCH':
          set(f, a[0], this.touches(weave, cell, occupied) ? 1 : 0)
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
        case 'CNDS':
          order.cnds += source(f, instr, 0)
          next()
          continue
        case 'IN': {
          const port = PORT_BY_CODE.get(a[1])!.name
          let values: number[]
          if (port === 'CELL') values = [w.cell]
          else if (port === 'DEPTH') values = [w.d]
          else if (port === 'ORIGIN') values = [...weave.origin]
          else if (port === 'MAKER') values = [...weave.maker.body.pos]
          else throw new Fault('BAD_PORT', `an order can't read ${port}`)
          values.forEach((v, k) => set(f, a[0] + k, v))
          next()
          continue
        }
      }
      throw new Fault('NOT_IN_ORDER', `${instr.op.name} can't run inside an order`)
    }
  }

  /** Is this cell against something that isn't its own: matter, or a body other than its maker's? */
  private touches(weave: Weave, cell: WeaveCell, occupied: Set<number>): boolean {
    const w = this.world
    const p = weave.worldPos(cell)
    const i = w.cellOf(p)
    if (i < 0) return true
    const [x, y, z] = w.coords(i)
    const around: [number, number, number][] = [[0, 0, 0], [1, 0, 0], [-1, 0, 0], [0, 1, 0], [0, -1, 0]]
    if (w.d > 1) around.push([0, 0, 1], [0, 0, -1])
    for (const [dx, dy, dz] of around) {
      const j = w.index(x + dx, y + dy, z + dz)
      if (j < 0) continue
      if ((dx || dy || dz) && occupied.has(j)) continue
      if (w.solidAt(j)) return true
      const [jx, jy, jz] = w.coords(j)
      const c = w.centre(jx, jy, jz)
      if (w.d === 1) c[2] = p[2]
      if (w.bodyAt(c, weave.maker.body)) return true
    }
    return false
  }

  /**
   * Moves each cell as its order asked, a cell's width at a time so nothing passes through a wall.
   * A cell holding matter can't move into a cell that has no room for it. With its shape locked, the weave moves as one.
   */
  private moveWeave(weave: Weave, asked: Vec[]) {
    const w = this.world
    if (asked.every((v) => !v[0] && !v[1] && !v[2])) return
    let moves = asked
    if (weave.locks.shape) {
      const avg: Vec = [0, 0, 0]
      for (const v of asked) for (let k = 0; k < 3; k++) avg[k] += v[k] / asked.length
      moves = asked.map(() => avg)
    }
    const longest = Math.max(...moves.map((v) => Math.hypot(...v)))
    const steps = Math.max(1, Math.ceil(longest / (w.cell * 0.99)))
    const applied = moves.map(() => [0, 0, 0] as Vec)
    const stuck = new Set<number>()

    const blocked = (cell: WeaveCell, step: Vec, occupied: Set<number>) => {
      const p = weave.worldPos(cell)
      const d = weave.toWorld(step)
      const j = w.cellOf([p[0] + d[0], p[1] + d[1], p[2] + d[2]])
      if (j < 0) return true
      const load = total(cell.carried)
      if (load <= PHYSICS.epsilon || occupied.has(j)) return false
      return PHYSICS.cellMatter - total(w.matter[j]) - this.carried[j] < load
    }

    for (let s = 0; s < steps; s++) {
      const occupied = new Set(weave.cells.map((c) => w.cellOf(weave.worldPos(c))))
      const step = (k: number): Vec => [moves[k][0] / steps, moves[k][1] / steps, moves[k][2] / steps]
      if (weave.locks.shape) {
        if (weave.cells.some((c, k) => blocked(c, step(k), occupied))) break
      }
      weave.cells.forEach((c, k) => {
        if (stuck.has(k)) return
        const st = step(k)
        if (!weave.locks.shape && blocked(c, st, occupied)) {
          stuck.add(k)
          return
        }
        for (let i = 0; i < 3; i++) {
          c.off[i] += st[i]
          applied[k][i] += st[i]
        }
      })
    }
    // The origin follows the cells, on average.
    const avg: Vec = [0, 0, 0]
    for (const v of applied) for (let k = 0; k < 3; k++) avg[k] += v[k] / applied.length
    const d = weave.toWorld(avg)
    for (let k = 0; k < 3; k++) weave.origin[k] += d[k]
    for (const c of weave.cells) for (let k = 0; k < 3; k++) c.off[k] -= avg[k]
  }

  /** A weave set loose leaks; every weave's free mana holds what matter it can of its own parts. */
  private hold(weave: Weave) {
    const w = this.world
    for (const c of weave.cells) {
      const i = w.cellOf(weave.worldPos(c))
      if (i < 0) continue
      if (!weave.inHand) add(w.air[i], share(c.free, PHYSICS.leak))
      for (let k = 0; k < 4; k++) {
        const can = c.free[k] * PHYSICS.bind
        if (c.carried[k] < can) {
          const got = Math.min(can - c.carried[k], w.matter[i][k])
          c.carried[k] += got
          w.matter[i][k] -= got
        } else if (c.carried[k] > can) {
          w.matter[i][k] += c.carried[k] - can
          c.carried[k] = can
        }
      }
    }
  }

  private dissolve(weave: Weave, why: string) {
    const w = this.world
    for (const c of weave.cells) {
      add(w.air[w.clampedCellOf(weave.worldPos(c))], take(c.free, Infinity))
      this.dropMatter(weave, c)
    }
    this.weaves.delete(weave.id)
    this.log({ kind: 'dissolve', caster: weave.maker.name, weave: weave.id, detail: why })
  }

  private dropMatter(weave: Weave, c: WeaveCell) {
    add(this.world.matter[this.world.clampedCellOf(weave.worldPos(c))], take(c.carried, Infinity))
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
