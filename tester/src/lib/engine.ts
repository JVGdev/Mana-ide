// The engine, for the tester: the Rust machine compiled to WebAssembly (SPEC D41), and what the tester reads of it in
// the shapes the panels use. It's loaded before anything else runs (the top-level await below).

import init, * as wasm from '../../../engine/mana-wasm/pkg/mana_wasm.js'

await init()

export type Parts = [number, number, number, number]
export type Vec = [number, number, number]
export type SourceLine = { file: string; line: number; text: string }

/** A spell, assembled: its bytes, its labels and constants, and where each instruction came from. */
export type Program = {
  bytes: Uint8Array
  labels: Map<string, number>
  consts: Map<string, number>
  lines: Map<number, SourceLine>
}

/** Finds a library's source by name (`Shapes` → the text of Shapes.masm). */
export type LibraryResolver = (name: string) => string | undefined

export class AsmError extends Error {
  constructor(public problems: string[]) {
    super(problems.join('\n'))
  }
}

// The instruction set

export type OperandKind = 'n' | 'm' | 's' | 't' | 'a' | 'L' | 'P' | 'K' | 'I'
export type Op = { code: number; name: string; operands: OperandKind[]; beats: number; order: boolean }
export type Port = { name: string; code: number; size: number; order: boolean }
export type OpDoc = { syntax: string; group: string; doc: string; step?: string }

const isa: { ops: Op[]; ports: Port[]; locks: string[]; opDocs: Record<string, OpDoc>; portDocs: Record<string, string> } = JSON.parse(wasm.isa())
export const OPS = isa.ops
export const PORTS = isa.ports
export const LOCK_NAMES = isa.locks
export const OP_BY_NAME = new Map(OPS.map((op) => [op.name, op]))
export const PORT_BY_NAME = new Map(PORTS.map((p) => [p.name, p]))
export const OP_DOCS = isa.opDocs
export const PORT_DOCS = isa.portDocs

/** The numbers the world runs on that the tester shows or draws by. */
export const PHYSICS: {
  cell: number
  density: Parts
  airMana: number
  bind: number
  solid: number
  mote: number
  manaMass: Parts
  orderBudget: number
  orderRegisters: number
  weaveRegisters: number
  joules: number
} = JSON.parse(wasm.physics_table())

export const total = (p: ArrayLike<number>) => p[0] + p[1] + p[2] + p[3]

/** Which part a mix is mostly made of. */
export function dominant(p: ArrayLike<number>): number {
  let best = 0
  for (let k = 1; k < 4; k++) if (p[k] > p[best]) best = k
  return best
}

/** The share of a cell's room some matter takes: each part's amount over what a full cell of it holds. */
export function fillOf(m: ArrayLike<number>): number {
  let f = 0
  for (let k = 0; k < 4; k++) f += m[k] / ((PHYSICS.density[k] * PHYSICS.cell ** 3) / PHYSICS.manaMass[k])
  return f
}

// Casters

export type Stat = { genetics: number; training: number }
export type CasterStats = {
  body: { capacity: Stat; baseline: Stat; drain: Stat; focus: Stat; streams: Stat; reach: Stat; affinity: [Stat, Stat, Stat, Stat] }
  mind: { speed: Stat; registers: Stat; memory: Stat; power: Stat; capacity: Stat; recovery: Stat }
}
const presets: Record<'child' | 'adept' | 'master', CasterStats> = JSON.parse(wasm.presets())
export const child = (): CasterStats => structuredClone(presets.child)
export const adept = (): CasterStats => structuredClone(presets.adept)
export const master = (): CasterStats => structuredClone(presets.master)

/** The scenes a spell can be cast in, beside the plain field. */
export const SCENE_LIST: string[] = JSON.parse(wasm.scene_names())

// Assembling

type ProgramJson = { ok: true; bytes: number[]; labels: [string, number][]; consts: [string, number][]; lines: [number, string, number, string][] }
type Assembled = ProgramJson | { ok: false; problems: string[] }

function toProgram(json: Assembled): Program {
  if (!json.ok) throw new AsmError(json.problems)
  return {
    bytes: Uint8Array.from(json.bytes),
    labels: new Map(json.labels),
    consts: new Map(json.consts),
    lines: new Map(json.lines.map(([addr, file, line, text]) => [addr, { file, line, text }])),
  }
}

/** Assembles a spell and the libraries it uses. Throws an AsmError naming each problem. */
export function assemble(source: string, file: string, libraries: LibraryResolver = () => undefined): Program {
  return toProgram(JSON.parse(wasm.assemble(source, file, libraries)))
}

export type Instr = { addr: number; size: number; beats: number; text: string }
const listings = new WeakMap<Program, { all: Instr[]; at: Map<number, Instr> }>()

/** A program's labels by address: the global name where two share one. */
export function names(p: Program): Map<number, string> {
  const out = new Map<number, string>()
  for (const [name, addr] of p.labels) if (!out.has(addr) || !name.includes('.')) out.set(addr, name)
  return out
}

/** Every instruction of a program, as text: worked out once. */
export function disassembly(p: Program): { all: Instr[]; at: Map<number, Instr> } {
  let d = listings.get(p)
  if (!d) {
    const all: Instr[] = JSON.parse(wasm.disassemble(p.bytes, JSON.stringify([...names(p)])))
    d = { all, at: new Map(all.map((i) => [i.addr, i])) }
    listings.set(p, d)
  }
  return d
}

// The machine

export type CastState = 'running' | 'halted' | 'failed' | 'fault'
export type ManaRegister = { parts: Parts; holdUntil: number }
export type CastView = {
  state: CastState
  fault: string | null
  code: number | null
  name: string
  pc: number
  n: number[]
  limit: number
  flags: number
  stack: number[]
  calls: number[]
  memory: number[]
  left: number
  perTick: number
  beats: number
  startedAt: number
  endedAt: number | null
  /** How many times each instruction ran, and its beats: [address, runs, beats]. */
  profile: [number, number, number][]
  regs: ManaRegister[]
}
export type CasterView = {
  held: number
  capacity: number
  baseline: number
  drain: number
  focus: number
  streams: number
  reach: number
  affinity: Parts
  speed: number
  registers: number
  memory: number
  power: number
  mindCapacity: number
  recovery: number
  flow: Parts
  harm: number
  strain: number
  madness: number
  condition: { body: number; mind: number }
  regs: ManaRegister[]
  inHand: number
  hand: Vec
  body: number
  tick: number
}
export type WeaveView = {
  id: number
  maker: string
  inHand: boolean
  manifestedAt: number
  origin: Vec
  particles: number
  ingrained: number
  free: Parts
  carried: Parts
  locks: { input: boolean; order: boolean }
  order: number | null
  regs: number[]
}
export type SimEvent = { tick: number; kind: string; caster: string | null; weave: number | null; detail: string | null }
export type Ledger = { air: number; matter: number; loose: number; weaves: number; carried: number; casters: number; free: number; condensed: number; total: number }
export type Stored = { motion: number; height: number; gas: number; packing: number; cohesion: number; air: number; bodies: number }
export type EnergyView = {
  held: Stored
  total: number
  heat: number
  start: number
  outside: number
  minds: number
  bodies: number
  error: [string, number][]
  errorTotal: number
  heatBy: [string, number][]
}
export type OrderStep = { addr: number; n: number[] }
export type OrderTrace = {
  tick: number
  weave: number
  particle: number
  id: number
  off: Vec
  w: number[]
  steps: OrderStep[]
  n: number[]
  outcome: string
  beats: number
  burned: number
  kick: Vec
  cnds: number
}
export type RoutineCost = { name: string; beats: number; share: number }
export type LineCost = { addr: number; source: SourceLine | null; runs: number; beats: number; share: number }
export type Profile = { beats: number; routines: RoutineCost[]; lines: LineCost[] }
export type Body = { name: string; pos: Vec; half: Vec; caster: boolean }
export type WorldInfo = { w: number; h: number; d: number; cell: number; size: number }

/** Numbers each particle takes in `Machine.particles()`: see `Particle`. */
export const STRIDE = wasm.particle_stride()

/** A particle, read out of `Machine.particles()` at offset `o`. */
export class Particle {
  constructor(
    private a: Float64Array,
    private o: number,
  ) {}
  get id() {
    return this.a[this.o]
  }
  get pos(): Vec {
    return [this.a[this.o + 1], this.a[this.o + 2], this.a[this.o + 3]]
  }
  get vel(): Vec {
    return [this.a[this.o + 4], this.a[this.o + 5], this.a[this.o + 6]]
  }
  get free(): Parts {
    return [this.a[this.o + 7], this.a[this.o + 8], this.a[this.o + 9], this.a[this.o + 10]]
  }
  get carried(): Parts {
    return [this.a[this.o + 11], this.a[this.o + 12], this.a[this.o + 13], this.a[this.o + 14]]
  }
  get weave() {
    return this.a[this.o + 15]
  }
  get ordered() {
    return this.a[this.o + 16] !== 0
  }
  get pushedAt() {
    return this.a[this.o + 17]
  }
  get rhoM() {
    return this.a[this.o + 18]
  }
  get mass() {
    return this.a[this.o + 19]
  }
}

/** A scene, and the spell cast in it: the Rust machine. */
export class Machine {
  private m: wasm.Machine
  readonly world: WorldInfo
  /** The scene's caster as the scene set them up: their will, stats, and where they stand. */
  readonly scene: { will: { aim: Vec; amount: number; force: number; maintain: boolean }; stats: CasterStats; body: number; pos: Vec }

  constructor(scene: string, dims: 2 | 3) {
    this.m = new wasm.Machine(scene, dims)
    const info = JSON.parse(this.m.info())
    this.world = { w: info.w, h: info.h, d: info.d, cell: info.cell, size: info.size }
    this.scene = { will: info.will, stats: info.stats, body: info.body, pos: info.pos }
  }

  free() {
    this.m.free()
  }

  setStats(stats: CasterStats) {
    this.m.set_stats(JSON.stringify(stats))
  }
  setCondition(body: number, mind: number) {
    this.m.set_condition(body, mind)
  }
  setWill(w: { aim: Vec; amount: number; force: number; maintain: boolean }) {
    this.m.set_will(w.aim[0], w.aim[1], w.aim[2], w.amount, w.force, w.maintain)
  }
  traceOrders(on: boolean) {
    this.m.trace_orders(on)
  }
  keepEnergy(on: boolean) {
    this.m.keep_energy(on)
  }
  get trackEnergy() {
    return this.m.track_energy()
  }

  /** Casts a spell, practised `practice` times before. Throws an AsmError if it doesn't assemble. */
  cast(source: string, file: string, libraries: LibraryResolver, practice: number): Program {
    return toProgram(JSON.parse(this.m.cast(source, file, libraries, practice)))
  }

  get tick() {
    return this.m.tick()
  }
  get midTick() {
    return this.m.mid_tick()
  }
  get running() {
    return this.m.running()
  }
  step() {
    this.m.step()
  }
  stepInstruction() {
    this.m.step_instruction()
  }
  /** Runs the mind up to `ticks` ticks, stopping before an instruction at one of `breaks`. True if it stopped there. */
  runUntil(breaks: number[], ticks: number) {
    return this.m.run_until(Uint32Array.from(breaks), ticks)
  }

  castView(): CastView | null {
    return JSON.parse(this.m.cast_view())
  }
  casterView(): CasterView {
    return JSON.parse(this.m.caster_view())
  }
  weaves(): WeaveView[] {
    return JSON.parse(this.m.weaves_view())
  }
  events(last: number): { events: SimEvent[]; ledger: Ledger } {
    return JSON.parse(this.m.events_view(last))
  }
  ledgerTotal() {
    return this.m.ledger_total()
  }
  energy(): EnergyView | null {
    return JSON.parse(this.m.energy_view())
  }
  profile(): Profile | null {
    return JSON.parse(this.m.profile_view())
  }

  /** Last tick's orders: which weaves ran them, and how many of each weave's particles did. */
  traceCounts(): Map<number, number> {
    return new Map(JSON.parse(this.m.trace_counts()))
  }
  trace(weave: number, k: number): OrderTrace | undefined {
    return JSON.parse(this.m.trace(weave, k)) ?? undefined
  }
  orderHit(breaks: number[]): { weave: number; particle: number; step: number } | undefined {
    return JSON.parse(this.m.order_hit(Uint32Array.from(breaks))) ?? undefined
  }
  pick(x: number, y: number, z: number): { weave: number; k: number; d: number } | undefined {
    return JSON.parse(this.m.pick(x, y, z)) ?? undefined
  }

  sliceAir(z: number) {
    return this.m.slice_air(z)
  }
  sliceAirVel(z: number) {
    return this.m.slice_air_vel(z)
  }
  /** Five numbers a cell: its matter's four parts, and the share of the cell it fills. */
  sliceMatter(z: number) {
    return this.m.slice_matter(z)
  }
  particles(): Particle[] {
    const a = this.m.particles()
    const out: Particle[] = []
    for (let o = 0; o < a.length; o += STRIDE) out.push(new Particle(a, o))
    return out
  }
  /** Six numbers a bond: where its two particles are. */
  bonds() {
    return this.m.bonds()
  }
  bodies(): Body[] {
    return JSON.parse(this.m.bodies())
  }
}
