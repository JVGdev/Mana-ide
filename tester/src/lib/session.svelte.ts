// The tester's state: which spell, which world, which caster, and the machine running it.

import { assemble, AsmError, type Program, type SourceLine } from '../../../src/asm/assembler.ts'
import { adept, child, master, type CasterStats, type Caster } from '../../../src/vm/caster.ts'
import { Sim, type Cast } from '../../../src/vm/sim.ts'
import { field, SCENES } from '../../../src/scenes.ts'
import type { Vec } from '../../../src/vm/world.ts'
import { baseName, files } from './files.svelte.ts'

export type Problem = { file: string; line: number; message: string }

export function parseProblem(p: string): Problem {
  const m = /^(.+?):(\d+): (.*)$/.exec(p)
  return m ? { file: m[1], line: Number(m[2]), message: m[3] } : { file: '', line: 0, message: p }
}

export const SCENE_NAMES = ['Field', ...Object.keys(SCENES)] as const
export type SceneName = (typeof SCENE_NAMES)[number]
export const PRESETS = { child, adept, master } as const
export type Preset = keyof typeof PRESETS | 'custom'

function buildScene(name: SceneName, dims: 2 | 3) {
  return name === 'Field' ? field(dims) : SCENES[name as keyof typeof SCENES](dims)
}

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
  /** The slice of a 3D world on screen. */
  sliceZ = $state(0)
  /** Bumped whenever the machine moves: everything that shows it reads this. */
  version = $state(0)
  /** A line for the editor to scroll to; `seq` changes each time it's asked. */
  goto = $state<{ line: number; seq: number }>({ line: 0, seq: 0 })

  sim = $state.raw<Sim | null>(null)
  cast = $state.raw<Cast | null>(null)
  caster = $state.raw<Caster | null>(null)
  program = $state.raw<Program | null>(null)
  /** The number registers as they were before the last step, to show what changed. */
  before = $state.raw<Float64Array>(new Float64Array(32))
  ledgerAtCast = 0

  private frameReq = 0

  /** Sets the world up again from the scene, with no spell cast. */
  reset() {
    this.pause()
    const s = buildScene(this.scene, this.dims)
    s.caster.stats = clone($state.snapshot(this.stats)) as CasterStats
    s.caster.condition = { ...this.condition }
    this.sim = s.sim
    this.caster = s.caster
    this.cast = null
    this.syncWill()
    this.sliceZ = Math.floor(s.caster.body.pos[2] / s.sim.world.cell)
    this.ledgerAtCast = s.sim.ledger().total
    this.before = new Float64Array(32)
    this.version++
  }

  /** Loads a scene's world and its will, and its caster when the scene needs a particular one. */
  loadScene(name: SceneName, dims = this.dims) {
    this.scene = name
    this.dims = dims
    const s = buildScene(name, dims)
    const w = s.caster.will
    this.will = { amount: w.amount, force: w.force, maintain: w.maintain, aim: [...w.aim] as Vec }
    const stats = clone(s.caster.stats)
    const same = (p: () => CasterStats) => JSON.stringify(p()) === JSON.stringify(stats)
    this.preset = same(adept) ? 'adept' : same(master) ? 'master' : same(child) ? 'child' : 'custom'
    this.stats = stats
    this.reset()
  }

  setPreset(p: Exclude<Preset, 'custom'>) {
    this.preset = p
    this.stats = PRESETS[p]()
    this.reset()
  }

  /** Stats and condition, edited: the caster changes now, mid-spell if need be. */
  applyCaster(edited = true) {
    if (edited) this.preset = 'custom'
    if (!this.caster) return
    this.caster.stats = clone($state.snapshot(this.stats)) as CasterStats
    this.caster.condition = { ...this.condition }
    this.version++
  }

  /** The will is live: a spell reads it while it runs. */
  syncWill() {
    if (!this.caster) return
    const w = this.will
    this.caster.will = { amount: w.amount, force: w.force, maintain: w.maintain, aim: [...w.aim] as Vec }
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
    if (!program || !this.sim || !this.caster) return false
    this.program = program
    const name = [...program.labels].find(([, a]) => a === 0)?.[0] ?? 'spell'
    if (this.practice > 0) this.caster.conditioning.set(name, this.practice)
    this.cast = this.sim.cast(this.caster, program)
    this.version++
    return true
  }

  get running() {
    return this.cast?.state === 'running'
  }

  /** Where the mind is: the line of the instruction it runs next. */
  here(): SourceLine | undefined {
    void this.version
    if (!this.cast || !this.program || this.cast.state !== 'running') return undefined
    return this.program.lines.get(this.cast.frame.pc)
  }

  private isBreak = (addr: number) => {
    const l = this.program?.lines.get(addr)
    return !!l && this.breakpoints.includes(`${l.file}:${l.line}`)
  }

  toggleBreakpoint(file: string, line: number) {
    const key = `${file}:${line}`
    this.breakpoints = this.breakpoints.includes(key) ? this.breakpoints.filter((b) => b !== key) : [...this.breakpoints, key]
  }

  private remember() {
    this.before = this.cast ? this.cast.frame.n.slice() : new Float64Array(32)
  }

  private moved(stopped: boolean) {
    this.version++
    if (stopped && this.follow) this.goHere()
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
    if (!this.sim) return
    this.pause()
    this.remember()
    this.syncWill()
    this.sim.step()
    this.moved(true)
  }

  stepInstruction() {
    if (!this.sim) return
    this.pause()
    this.remember()
    this.syncWill()
    if (this.cast?.state === 'running') this.sim.stepInstruction(this.cast)
    else this.sim.step()
    this.moved(true)
  }

  /** One tick while playing. False when a breakpoint stopped it. */
  private tickOnce(): boolean {
    const sim = this.sim!
    this.syncWill()
    if (this.cast?.state === 'running' && this.breakpoints.length) return !sim.runUntil(this.cast, this.isBreak, 1)
    sim.step()
    return true
  }

  play() {
    if (this.playing || !this.sim) return
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
            this.moved(true)
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
