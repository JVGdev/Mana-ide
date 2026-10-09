// The tester's state: which spell, which world, which caster, and the machine running it.

import {
  assemble,
  AsmError,
  adept,
  child,
  master,
  Machine,
  SCENE_LIST,
  type CastView,
  type CasterStats,
  type OrderTrace,
  type Program,
  type SourceLine,
  type Vec,
} from './engine.ts'
import { baseName, files } from './files.svelte.ts'

export type Problem = { file: string; line: number; message: string }

export function parseProblem(p: string): Problem {
  const m = /^(.+?):(\d+): (.*)$/.exec(p)
  return m ? { file: m[1], line: Number(m[2]), message: m[3] } : { file: '', line: 0, message: p }
}

export const SCENE_NAMES = ['Field', ...SCENE_LIST]
export type SceneName = string
export const PRESETS = { child, adept, master } as const
export const PANELS = ['mind', 'body', 'weaves', 'order', 'energy', 'profile', 'events', 'caster', 'bytes', 'reference'] as const
export type Panel = (typeof PANELS)[number]
export type Preset = keyof typeof PRESETS | 'custom'


const clone = <T>(v: T): T => JSON.parse(JSON.stringify(v))

class Session {
  /** The spell being cast. */
  spell = $state('spells/Fireball.masm')
  /** The file in the editor: the spell, or a library it uses. */
  open = $state('spells/Fireball.masm')
  scene = $state<SceneName>('Fireball')
  dims = $state<2 | 3>(2)

  preset = $state<Preset>('adept')
  stats = $state<CasterStats>(adept())
  condition = $state({ body: 1, mind: 1 })
  /** Times the caster has cast this spell before (the Law of Conditioning). */
  practice = $state(0)
  will = $state({ amount: 120, force: 0.5, maintain: false, aim: [0, 0, 0] as Vec })

  problems = $state<Problem[]>([])
  /** `File.masm:line` */
  breakpoints = $state<string[]>([])
  playing = $state(false)
  /** Ticks a second while playing. */
  tps = $state(8)
  /** Open the file of the line the mind is on whenever it stops. */
  follow = $state(true)
  /** Keep the Energy ledger (it costs time, most in 3D). */
  keepEnergy = $state(true)
  /** The slice of a 3D world on screen. */
  sliceZ = $state(0)
  /** Bumped whenever the machine moves: everything that shows it reads this. */
  version = $state(0)
  panel = $state<Panel>('mind')
  /** The particle whose order is shown (which of its weave's traces), and which of its instructions. */
  orderSel = $state({ weave: 0, particle: 0, step: 0 })
  /** The next click on the world picks a particle of a weave instead of aiming. */
  picking = $state(false)
  /** A line for the editor to scroll to; `seq` changes each time it's asked. */
  goto = $state<{ line: number; seq: number }>({ line: 0, seq: 0 })

  /** The world and the spell cast in it: the Rust machine (engine.ts). */
  machine = $state.raw<Machine | null>(null)
  /** Whether a spell has been cast in it. */
  cast = $state(false)
  program = $state.raw<Program | null>(null)
  /** The number registers as they were before the last step, to show what changed. */
  before = $state.raw<Float64Array>(new Float64Array(32))
  ledgerAtCast = 0

  private frameReq = 0

  /** Sets the world up again from the scene, with no spell cast. */
  reset() {
    this.pause()
    this.machine?.free()
    const m = new Machine(this.scene, this.dims)
    m.setStats(clone($state.snapshot(this.stats)) as CasterStats)
    m.setCondition(this.condition.body, this.condition.mind)
    m.traceOrders(true)
    if (this.keepEnergy) m.keepEnergy(true)
    this.machine = m
    this.orderSel = { weave: 0, particle: 0, step: 0 }
    this.picking = false
    this.cast = false
    this.syncWill()
    this.sliceZ = Math.floor(m.scene.pos[2] / m.world.cell)
    this.ledgerAtCast = m.ledgerTotal()
    this.before = new Float64Array(32)
    this.version++
  }

  /** Loads a scene's world and its will, and its caster when the scene needs a particular one. */
  loadScene(name: SceneName, dims = this.dims) {
    this.scene = name
    this.dims = dims
    const s = new Machine(name, dims)
    const w = s.scene.will
    this.will = { amount: w.amount, force: w.force, maintain: w.maintain, aim: [...w.aim] as Vec }
    const stats = clone(s.scene.stats)
    s.free()
    const same = (p: () => CasterStats) => JSON.stringify(p()) === JSON.stringify(stats)
    this.preset = same(adept) ? 'adept' : same(master) ? 'master' : same(child) ? 'child' : 'custom'
    this.stats = stats
    this.reset()
  }

  setKeepEnergy(on: boolean) {
    this.keepEnergy = on
    this.machine?.keepEnergy(on)
    this.version++
  }

  setPreset(p: Exclude<Preset, 'custom'>) {
    this.preset = p
    this.stats = PRESETS[p]()
    this.reset()
  }

  /** Stats and condition, edited: the caster changes now, mid-spell if need be. */
  applyCaster(edited = true) {
    if (edited) this.preset = 'custom'
    if (!this.machine) return
    this.machine.setStats(clone($state.snapshot(this.stats)) as CasterStats)
    this.machine.setCondition(this.condition.body, this.condition.mind)
    this.version++
  }

  /** The will is live: a spell reads it while it runs. */
  syncWill() {
    if (!this.machine) return
    const w = this.will
    this.machine.setWill({ amount: w.amount, force: w.force, maintain: w.maintain, aim: [...w.aim] as Vec })
  }

  /** Assembles the spell as the editor has it. */
  assemble(): Program | null {
    const f = files.get(this.spell)
    if (!f) return null
    try {
      const p = assemble(f.text, baseName(this.spell), files.resolver)
      this.problems = []
      return p
    } catch (e) {
      if (!(e instanceof AsmError)) throw e
      this.problems = e.problems.map(parseProblem)
      return null
    }
  }

  /** Checks the spell, and the open file on its own if it's a library the spell doesn't use. */
  lint() {
    const program = this.assemble()
    if (this.open === this.spell) return
    const f = files.get(this.open)
    if (!f) return
    const used = program && [...program.lines.values()].some((l) => l.file === baseName(this.open))
    if (used) return
    try {
      assemble(f.text, baseName(this.open), files.resolver)
    } catch (e) {
      if (e instanceof AsmError) this.problems = [...this.problems, ...e.problems.map(parseProblem)]
    }
  }

  /** A fresh world, and the spell cast in it. */
  castSpell(): boolean {
    const program = this.assemble()
    this.reset()
    const f = files.get(this.spell)
    if (!program || !this.machine || !f) return false
    try {
      this.program = this.machine.cast(f.text, baseName(this.spell), files.resolver, this.practice)
    } catch (e) {
      if (!(e instanceof AsmError)) throw e
      this.problems = e.problems.map(parseProblem)
      return false
    }
    this.cast = true
    this.version++
    return true
  }

  /** The cast's mind as it is now: read again whenever the machine moves. */
  castView = $derived.by((): CastView | null => {
    void this.version
    return this.cast && this.machine ? this.machine.castView() : null
  })

  get running() {
    return this.castView?.state === 'running'
  }

  /** Where the mind is: the line of the instruction it runs next. */
  here(): SourceLine | undefined {
    void this.version
    const c = this.castView
    if (!c || !this.program || c.state !== 'running') return undefined
    return this.program.lines.get(c.pc)
  }

  isBreak = (addr: number) => {
    const l = this.program?.lines.get(addr)
    return !!l && this.breakpoints.includes(`${l.file}:${l.line}`)
  }

  /** The addresses of the instructions on lines with a breakpoint. */
  private breakAddrs(): number[] {
    if (!this.program || !this.breakpoints.length) return []
    return [...this.program.lines.keys()].filter(this.isBreak)
  }

  toggleBreakpoint(file: string, line: number) {
    const key = `${file}:${line}`
    this.breakpoints = this.breakpoints.includes(key) ? this.breakpoints.filter((b) => b !== key) : [...this.breakpoints, key]
  }

  private remember() {
    const c = this.castView
    this.before = c ? Float64Array.from(c.n) : new Float64Array(32)
  }

  private moved(stopped: boolean) {
    this.settleOrder()
    this.version++
    if (stopped && this.follow) this.goHere()
  }

  /** The trace of the chosen particle's order, from the last tick its weave ran. */
  orderTrace(): OrderTrace | undefined {
    void this.version
    return this.machine?.trace(this.orderSel.weave, this.orderSel.particle)
  }

  /** Keeps the order selection pointing at something after a tick: the first weave that ran, back to its first step. */
  private lastTraceTick = -1
  private settleOrder() {
    const counts = this.machine?.traceCounts()
    if (!counts?.size) return
    if (!counts.has(this.orderSel.weave)) this.orderSel = { weave: [...counts.keys()][0], particle: 0, step: 0 }
    if (this.orderSel.particle >= counts.get(this.orderSel.weave)!) this.orderSel.particle = 0
    const trace = this.machine!.trace(this.orderSel.weave, this.orderSel.particle)
    if (trace && trace.tick !== this.lastTraceTick) {
      this.lastTraceTick = trace.tick
      this.orderSel.step = 0
    }
  }

  /** The first particle whose order ran a line with a breakpoint last tick. */
  private orderHit(): { weave: number; particle: number; step: number } | undefined {
    if (!this.breakpoints.length || !this.machine) return undefined
    return this.machine.orderHit(this.breakAddrs())
  }

  private stopAtOrder(hit: { weave: number; particle: number; step: number }) {
    this.lastTraceTick = this.machine!.trace(hit.weave, hit.particle)!.tick
    this.orderSel = hit
    this.panel = 'order'
    this.goOrder()
  }

  /** Opens the file of the order instruction being shown, at its line. */
  goOrder() {
    const t = this.orderTrace()
    const step = t?.steps[this.orderSel.step]
    const l = step && this.program?.lines.get(step.addr)
    if (l) this.jump(l.file, l.line)
  }

  /** Show a weave's order, in the particle at `particle` among the ones that ran it. */
  selectOrder(weave: number, particle: number) {
    this.orderSel = { weave, particle, step: 0 }
    this.panel = 'order'
    this.version++
  }

  /** Opens the file the mind is in, at its line. */
  goHere() {
    const l = this.here()
    if (l) this.jump(l.file, l.line)
  }

  /** Opens a file (by the name the assembler records) at a line. */
  jump(file: string, line: number) {
    const f = files.byName(file)
    if (!f) return
    this.open = f.path
    this.goto = { line, seq: this.goto.seq + 1 }
  }

  stepTick() {
    if (!this.machine) return
    this.pause()
    this.remember()
    this.syncWill()
    this.machine.step()
    const hit = this.orderHit()
    if (hit) {
      this.stopAtOrder(hit)
      this.version++
      return
    }
    this.moved(true)
  }

  stepInstruction() {
    if (!this.machine) return
    this.pause()
    this.remember()
    this.syncWill()
    this.machine.stepInstruction()
    this.moved(true)
  }

  /** One tick while playing. False when a breakpoint stopped it, in the mind or in an order. */
  private tickOnce(): boolean {
    const m = this.machine!
    this.syncWill()
    if (m.running && this.breakpoints.length) {
      if (m.runUntil(this.breakAddrs(), 1)) return false
    } else m.step()
    const hit = this.orderHit()
    if (hit) {
      this.stopAtOrder(hit)
      return false
    }
    return true
  }

  play() {
    if (this.playing || !this.machine) return
    this.playing = true
    let last = performance.now()
    let due = 1 // the first tick right away
    const frame = (now: number) => {
      if (!this.playing) return
      due += ((now - last) / 1000) * this.tps
      last = now
      const n = Math.min(Math.floor(due), 30)
      due -= n
      if (n > 0) {
        this.remember()
        for (let i = 0; i < n; i++)
          if (!this.tickOnce()) {
            this.playing = false
            const atOrder = this.panel === 'order' && !this.machine?.midTick
            this.version++
            if (this.follow && !atOrder) this.goHere()
            return
          }
        this.moved(false)
      }
      this.frameReq = requestAnimationFrame(frame)
    }
    this.frameReq = requestAnimationFrame(frame)
  }

  pause() {
    this.playing = false
    cancelAnimationFrame(this.frameReq)
  }

  setAim(p: Vec) {
    this.will.aim = p
    this.syncWill()
    this.version++
  }
}

export const session = new Session()
