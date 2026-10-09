// The sandbox: mana as a fluid of particles, in 2D, outside the machine (SPEC §11). A ball of mana pushes itself apart
// with pressure; a caster holds it by pushing its particles back in, paying beats and mana for every push.
//
// The fluid is smoothed-particle hydrodynamics: each particle feels its neighbours within `smoothing` metres. The air is a
// grid of free mana that the particles drag along and are dragged by. Every force inside the world is equal and opposite
// (particle on particle, particle on air, air on air), so momentum only changes by what comes from outside: pushes, orders,
// rising, the ground and walls. The ledger checks it every tick.

export type Settings = {
  /** M in one particle: the crowd of real mana particles that one simulated particle stands for. */
  mote: number
  /** Metres over which particles feel each other. */
  smoothing: number
  /** How hard mana presses outward: pressure = stiffness × density. The speed it spreads at is about √stiffness. */
  stiffness: number
  /** How much neighbours' motions even out. */
  viscosity: number
  /** Upward pull, m/tick²: fire rises. */
  rise: number
  /** m/tick that one M of poured mana gives one M of mana. */
  pushYield: number
  /** The most a particle's velocity can be changed by pushes in one tick, m/tick. */
  pushRate: number
  /** How far the caster's field reaches around the weave, as a multiple of the ball's radius. */
  field: number
  /** Metres from the caster's body that they can still push. */
  reach: number
  /** Free mana in each cell of open air, all four parts: the world's `airMana`. */
  airMana: number
  /** How fast a particle's speed comes to the speed of the air around it, per tick, in air as thick as the world's. */
  airDrag: number
  /** Share of the difference in speed between neighbouring cells of air that evens out each tick. */
  airViscosity: number
  /** M of its own mana an order burns for every beat it thinks. */
  orderBurn: number
  /** Beats it takes the caster to ingrain an order into one particle. */
  ingrain: number
  substeps: number
}

export const DEFAULTS: Settings = {
  mote: 1,
  smoothing: 0.16,
  stiffness: 0.0009,
  viscosity: 0.0005,
  rise: 0.0003,
  pushYield: 2,
  pushRate: 0.05,
  field: 2,
  reach: 3,
  airMana: 40,
  airDrag: 0.01,
  airViscosity: 0.2,
  orderBurn: 0.0002,
  ingrain: 24,
  substeps: 8,
}

/** The world's air, which `airDrag` is measured in. */
const WORLD_AIR = 40

/** The test world, in metres: ground at y = 0, a wall from x = WALL to the edge. */
export const WORLD = { width: 14, height: 6, cell: 0.25, wall: 12 }

/**
 * An order ingrained in a particle. `centre` knows where its weave's centre is (the ORIGIN port) and pushes itself back
 * toward it. `feel` knows only what its particle feels: how dense the mana around it is, which way it thickens, and how its
 * neighbours move.
 */
export type OrderKind = 'centre' | 'feel'

export type Particle = {
  x: number
  y: number
  vx: number
  vy: number
  /** M of mana. */
  m: number
  /** Still in the weave: within its caster's field. */
  held: boolean
  order: OrderKind | null
  /** The tick it was last pushed, by the caster or by its order. */
  pushedAt: number
  /** How much more its velocity can be changed by pushes this tick. */
  dvLeft: number
  rho: number
  /** Which way, and how fast, the mana around it thickens. */
  gx: number
  gy: number
  /** How its neighbours move, on average. */
  nvx: number
  nvy: number
  ax: number
  ay: number
}

export type Ball = { x: number; y: number; radius: number; amount: number; seed?: number }

export type Origin = { x: number; y: number; vx: number; vy: number }

const GOLDEN = Math.PI * (3 - Math.sqrt(5))

/** A small seeded random number generator, so the same seed lays out the same ball. */
function random(seed: number): () => number {
  let a = seed >>> 0
  return () => {
    a = (a + 0x6d2b79f5) >>> 0
    let t = a
    t = Math.imul(t ^ (t >>> 15), t | 1)
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61)
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
}

export class Sandbox {
  particles: Particle[] = []
  /** The air, cell by cell, from x = 0 to the wall: its mana, and how it moves. */
  readonly airW = Math.round(WORLD.wall / WORLD.cell)
  readonly airH = Math.round(WORLD.height / WORLD.cell)
  air: Float64Array
  airVx: Float64Array
  airVy: Float64Array
  tick = 0
  /** The weave's centre of mass and its velocity, as of the start of this tick. */
  origin: Origin = { x: 0, y: 0, vx: 0, vy: 0 }
  /** Momentum given to the world from outside: pushes, orders, rising, the ground and walls. */
  impulse = { push: [0, 0], order: [0, 0], rise: [0, 0], walls: [0, 0] }
  /** The body, which the field reaches out from. */
  caster = { x: 1, y: 1.5 }
  /** Density of a particle deep inside the ball as it was laid out. */
  readonly rhoFull: number
  readonly radius: number
  readonly amount: number
  /** Mana that isn't in the world: the caster's reserve, which pushes are paid from. */
  outside = 0
  /** Mana the particles have spent on their orders: thinking, and pushing themselves. */
  selfSpent = 0
  /** Beats every order thought last tick, all together, and how many particles thought them. */
  orderBeats = 0
  orderParticles = 0

  constructor(
    readonly settings: Settings,
    ball: Ball,
  ) {
    const cells = this.airW * this.airH
    this.air = new Float64Array(cells).fill(settings.airMana)
    this.airVx = new Float64Array(cells)
    this.airVy = new Float64Array(cells)
    this.radius = ball.radius
    this.amount = ball.amount
    // A sunflower: points spread evenly over a disc, each with the same share. The seed turns it and jostles each point a
    // little, so that a result can be checked against other layouts of the same ball.
    const n = Math.max(1, Math.round(ball.amount / settings.mote))
    const rand = random(ball.seed ?? 1)
    const turn = rand() * Math.PI * 2
    const jostle = ball.seed ? 0.25 * Math.sqrt((Math.PI * ball.radius ** 2) / n) : 0
    for (let k = 0; k < n; k++) {
      const r = ball.radius * Math.sqrt((k + 0.5) / n)
      const t = k * GOLDEN + turn
      this.particles.push({
        x: ball.x + r * Math.cos(t) + (rand() - 0.5) * 2 * jostle,
        y: ball.y + r * Math.sin(t) + (rand() - 0.5) * 2 * jostle,
        vx: 0,
        vy: 0,
        m: ball.amount / n,
        held: true,
        order: null,
        pushedAt: -1,
        dvLeft: settings.pushRate,
        rho: 0,
        gx: 0,
        gy: 0,
        nvx: 0,
        nvy: 0,
        ax: 0,
        ay: 0,
      })
    }
    this.density()
    this.rhoFull = Math.max(...this.particles.map((p) => p.rho))
    this.findOrigin()
  }

  get weave(): Particle[] {
    return this.particles.filter((p) => p.held)
  }

  /** One tick of the world, after the caster has thought: orders, then the fluid and the air, then who's still in the weave. */
  step() {
    this.runOrders()
    const s = this.settings
    const dt = 1 / s.substeps
    for (let k = 0; k < s.substeps; k++) {
      this.density()
      this.forces()
      for (const p of this.particles) {
        p.vx += p.ax * dt
        p.vy += p.ay * dt
      }
      this.drag(dt)
      this.airFlow(dt)
      for (const p of this.particles) {
        p.x += p.vx * dt
        p.y += p.vy * dt
        this.walls(p)
      }
    }
    this.density()
    this.leave()
    this.settle()
    this.tick++
    for (const p of this.particles) p.dvLeft = s.pushRate
    this.findOrigin()
  }

  // The fluid

  private grid = new Map<number, Particle[]>()

  private bucket(x: number, y: number): number {
    const h = this.settings.smoothing
    return (Math.floor(y / h) + 1000) * 100000 + (Math.floor(x / h) + 1000)
  }

  private neighbours(): void {
    this.grid.clear()
    for (const p of this.particles) {
      const k = this.bucket(p.x, p.y)
      let list = this.grid.get(k)
      if (!list) this.grid.set(k, (list = []))
      list.push(p)
    }
  }

  /** Every pair of particles closer than the smoothing length, once each. */
  private pairs(f: (a: Particle, b: Particle, dx: number, dy: number, r: number) => void) {
    const h = this.settings.smoothing
    for (const [k, list] of this.grid) {
      for (const [ox, oy] of [
        [0, 0],
        [1, 0],
        [-1, 1],
        [0, 1],
        [1, 1],
      ]) {
        const other = this.grid.get(k + oy * 100000 + ox)
        if (!other) continue
        const same = ox === 0 && oy === 0
        for (let i = 0; i < list.length; i++) {
          for (let j = same ? i + 1 : 0; j < other.length; j++) {
            const a = list[i]
            const b = other[j]
            const dx = a.x - b.x
            const dy = a.y - b.y
            const r2 = dx * dx + dy * dy
            if (r2 >= h * h) continue
            f(a, b, dx, dy, Math.sqrt(r2))
          }
        }
      }
    }
  }

  /** Each particle's density, which way it thickens, and how its neighbours move. */
  private density() {
    const h = this.settings.smoothing
    const poly6 = 4 / (Math.PI * h ** 8)
    const grad = -24 / (Math.PI * h ** 8)
    this.neighbours()
    const weight = new Map<Particle, number>()
    for (const p of this.particles) {
      p.rho = p.m * poly6 * h ** 6
      p.gx = p.gy = p.nvx = p.nvy = 0
      weight.set(p, 0)
    }
    this.pairs((a, b, dx, dy, r) => {
      const q = h * h - r * r
      const w = poly6 * q ** 3
      a.rho += b.m * w
      b.rho += a.m * w
      const g = grad * q * q
      a.gx += b.m * g * dx
      a.gy += b.m * g * dy
      b.gx -= a.m * g * dx
      b.gy -= a.m * g * dy
      a.nvx += b.m * w * b.vx
      a.nvy += b.m * w * b.vy
      b.nvx += a.m * w * a.vx
      b.nvy += a.m * w * a.vy
      weight.set(a, weight.get(a)! + b.m * w)
      weight.set(b, weight.get(b)! + a.m * w)
    })
    for (const p of this.particles) {
      const w = weight.get(p)!
      if (w > 0) {
        p.nvx /= w
        p.nvy /= w
      } else {
        p.nvx = p.vx
        p.nvy = p.vy
      }
    }
  }

  private forces() {
    const s = this.settings
    const h = s.smoothing
    const spiky = -30 / (Math.PI * h ** 5)
    const lap = 40 / (Math.PI * h ** 5)
    const dt = 1 / s.substeps
    for (const p of this.particles) {
      p.ax = 0
      p.ay = s.rise
      this.impulse.rise[1] += p.m * s.rise * dt
    }
    this.pairs((a, b, dx, dy, r) => {
      if (r < 1e-9) return
      // Pressure: equal and opposite, so momentum is kept.
      const pa = s.stiffness / a.rho
      const pb = s.stiffness / b.rho
      const g = spiky * (h - r) ** 2
      const f = -(pa + pb) * g // force per (m_a·m_b), along a − b
      let fx = (f * dx) / r
      let fy = (f * dy) / r
      // Viscosity: neighbours' motions even out.
      const v = (s.viscosity * lap * (h - r) * 2) / (a.rho + b.rho)
      fx += v * (b.vx - a.vx)
      fy += v * (b.vy - a.vy)
      a.ax += fx * b.m
      a.ay += fy * b.m
      b.ax -= fx * a.m
      b.ay -= fy * a.m
    })
  }

  // The air

  /** The air cell a point is in. */
  cellAt(x: number, y: number): number {
    const i = Math.min(this.airW - 1, Math.max(0, Math.floor(x / WORLD.cell)))
    const j = Math.min(this.airH - 1, Math.max(0, Math.floor(y / WORLD.cell)))
    return j * this.airW + i
  }

  /**
   * A particle and the air it's in pull each other's speeds together. What the particle loses the air gains, so a ball
   * flying through still air sets it moving, and air already moving carries what flies through it.
   */
  private drag(dt: number) {
    const s = this.settings
    for (const p of this.particles) {
      const c = this.cellAt(p.x, p.y)
      const M = this.air[c]
      if (M <= 0) continue
      const rate = (s.airDrag * M) / WORLD_AIR
      const mu = (p.m * M) / (p.m + M)
      const k = mu * (1 - Math.exp(-rate * (1 + p.m / M) * dt))
      const jx = k * (p.vx - this.airVx[c])
      const jy = k * (p.vy - this.airVy[c])
      p.vx -= jx / p.m
      p.vy -= jy / p.m
      this.airVx[c] += jx / M
      this.airVy[c] += jy / M
    }
  }

  /**
   * Neighbouring cells of air even out their speeds, and the ground, the sky, the back edge and the wall hold still the
   * air against them. The air doesn't carry itself along yet, and isn't kept from piling up: a wake spreads where it is.
   */
  private airFlow(dt: number) {
    const k = this.settings.airViscosity * dt
    const W = this.airW
    const H = this.airH
    const { air, airVx: vx, airVy: vy } = this
    const share = (a: number, b: number) => {
      if (air[b] <= 0) return
      const mu = (air[a] * air[b]) / (air[a] + air[b] || 1)
      const jx = k * mu * (vx[a] - vx[b])
      const jy = k * mu * (vy[a] - vy[b])
      vx[a] -= jx / air[a]
      vy[a] -= jy / air[a]
      vx[b] += jx / air[b]
      vy[b] += jy / air[b]
    }
    const still = (a: number) => {
      const jx = k * air[a] * vx[a]
      const jy = k * air[a] * vy[a]
      vx[a] -= jx / air[a]
      vy[a] -= jy / air[a]
      this.impulse.walls[0] -= jx
      this.impulse.walls[1] -= jy
    }
    for (let j = 0; j < H; j++) {
      for (let i = 0; i < W; i++) {
        const a = j * W + i
        if (air[a] <= 0) continue
        if (i + 1 < W) share(a, a + 1)
        if (j + 1 < H) share(a, a + W)
        if (i === 0 || i === W - 1) still(a)
        if (j === 0 || j === H - 1) still(a)
      }
    }
  }

  /** Mana let loose into the air at a point, carrying momentum (px, py) into it. */
  loose(x: number, y: number, m: number, px = 0, py = 0) {
    const c = this.cellAt(x, y)
    const M = this.air[c] + m
    if (M <= 0) return
    this.airVx[c] = (this.air[c] * this.airVx[c] + px) / M
    this.airVy[c] = (this.air[c] * this.airVy[c] + py) / M
    this.air[c] = M
  }

  /** Mana in a cell of air beyond what the world began with: what's been spent and let loose there. */
  spentAt(c: number): number {
    return this.air[c] - this.settings.airMana
  }

  /** The ground, the sky, the back edge and the wall stop mana dead: it piles up against them. */
  private walls(p: Particle) {
    const before = [p.vx, p.vy]
    if (p.y < 0) {
      p.y = 0
      if (p.vy < 0) p.vy = 0
    }
    if (p.y > WORLD.height) {
      p.y = WORLD.height
      if (p.vy > 0) p.vy = 0
    }
    if (p.x < 0) {
      p.x = 0
      if (p.vx < 0) p.vx = 0
    }
    if (p.x > WORLD.wall) {
      p.x = WORLD.wall
      if (p.vx > 0) p.vx = 0
    }
    this.impulse.walls[0] += p.m * (p.vx - before[0])
    this.impulse.walls[1] += p.m * (p.vy - before[1])
  }

  // The weave

  private findOrigin() {
    let m = 0
    const o = { x: 0, y: 0, vx: 0, vy: 0 }
    for (const p of this.particles) {
      if (!p.held) continue
      m += p.m
      o.x += p.m * p.x
      o.y += p.m * p.y
      o.vx += p.m * p.vx
      o.vy += p.m * p.vy
    }
    if (m > 0) this.origin = { x: o.x / m, y: o.y / m, vx: o.vx / m, vy: o.vy / m }
  }

  /** Particles that have got further from the weave's centre than its field reaches leave it, for good. */
  private leave() {
    let m = 0
    let x = 0
    let y = 0
    for (const p of this.particles) {
      if (!p.held) continue
      m += p.m
      x += p.m * p.x
      y += p.m * p.y
    }
    if (m === 0) return
    x /= m
    y /= m
    const far = this.settings.field * this.radius
    for (const p of this.particles) {
      if (p.held && Math.hypot(p.x - x, p.y - y) > far) p.held = false
    }
  }

  /** Loose particles that have slowed and spread thin settle into the air, and leave their order behind. */
  private settle() {
    const keep: Particle[] = []
    for (const p of this.particles) {
      const c = this.cellAt(p.x, p.y)
      const still = Math.hypot(p.vx - this.airVx[c], p.vy - this.airVy[c]) < 0.01
      if (!p.held && still && p.rho < 0.3 * this.rhoFull) this.loose(p.x, p.y, p.m, p.m * p.vx, p.m * p.vy)
      else keep.push(p)
    }
    this.particles = keep
  }

  // Pushing

  /**
   * Pour mana onto a particle to change its velocity by (dvx, dvy), no more than it has left this tick. Pays from `pay`,
   * which gives what it can and returns what it gave. The poured mana goes loose where it was poured. Returns the M spent.
   */
  push(p: Particle, dvx: number, dvy: number, pay: (m: number) => number, from: 'push' | 'order' = 'push'): number {
    let dv = Math.hypot(dvx, dvy)
    if (dv < 1e-12 || p.dvLeft <= 0) return 0
    if (dv > p.dvLeft) {
      dvx *= p.dvLeft / dv
      dvy *= p.dvLeft / dv
      dv = p.dvLeft
    }
    const want = (p.m * dv) / this.settings.pushYield
    const got = pay(want)
    if (got <= 0) return 0
    const k = got / want
    p.vx += dvx * k
    p.vy += dvy * k
    p.dvLeft -= dv * k
    p.pushedAt = this.tick
    this.impulse[from][0] += p.m * dvx * k
    this.impulse[from][1] += p.m * dvy * k
    if (from === 'push') this.loose(p.x, p.y, got)
    return got
  }

  /** The caster ingrains an order into a particle. */
  ingrain(p: Particle, kind: OrderKind) {
    p.order = kind
  }

  /**
   * Every particle carrying an order runs it. Thinking burns the particle's own mana, beat by beat, so an order is
   * written to decide quickly when there's nothing to do. Pushing itself is paid from itself too.
   */
  private runOrders() {
    const o = this.origin
    this.orderBeats = 0
    this.orderParticles = 0
    const keep: Particle[] = []
    for (const p of this.particles) {
      if (p.order === 'centre' && p.held) this.byCentre(p, o)
      else if (p.order === 'feel') this.byFeel(p)
      if (p.m > 1e-3) keep.push(p)
      else this.loose(p.x, p.y, p.m, p.m * p.vx, p.m * p.vy)
    }
    this.particles = keep
  }

  private think(p: Particle, beats: number) {
    this.orderBeats += beats
    this.selfSpendAndLoose(p, beats * this.settings.orderBurn)
  }

  /**
   * Knowing the centre: how far am I from it (12 beats: the port, the distance and its square root, a compare)? Past the
   * radius and moving out, push back in (16 more).
   */
  private byCentre(p: Particle, o: Origin) {
    this.orderParticles++
    const dx = p.x - o.x
    const dy = p.y - o.y
    const r = Math.hypot(dx, dy) || 1e-9
    if (r <= this.radius) return this.think(p, 12)
    this.think(p, 28)
    const nx = dx / r
    const ny = dy / r
    const ur = (p.vx - o.vx) * nx + (p.vy - o.vy) * ny
    const want = -HOLD_GAIN * (r - this.radius)
    if (ur > want) this.push(p, (want - ur) * nx, (want - ur) * ny, (m) => this.spendSelf(p, m), 'order')
  }

  /**
   * By feel: is the mana around me as dense as it should be (6 beats)? If it's thin, I'm at the edge: which way does it
   * thicken, and am I drifting away from my neighbours (18 more)? Then push back toward them.
   */
  private byFeel(p: Particle) {
    this.orderParticles++
    const edge = 0.75 * this.rhoFull
    if (p.rho >= edge) return this.think(p, 6)
    this.think(p, 24)
    const g = Math.hypot(p.gx, p.gy)
    if (g < 1e-9) return
    const nx = -p.gx / g // out: where it thins
    const ny = -p.gy / g
    const ur = (p.vx - p.nvx) * nx + (p.vy - p.nvy) * ny
    const want = -FEEL_GAIN * (1 - p.rho / edge)
    if (ur > want) this.push(p, (want - ur) * nx, (want - ur) * ny, (m) => this.spendSelf(p, m), 'order')
  }

  /** A particle paying with itself. The mana it spends goes loose into the air, carrying its share of its momentum. */
  private spendSelf(p: Particle, m: number): number {
    return this.selfSpendAndLoose(p, Math.min(m, p.m * 0.5))
  }

  private selfSpendAndLoose(p: Particle, m: number): number {
    const got = Math.min(m, p.m)
    if (got <= 0) return 0
    this.loose(p.x, p.y, got, got * p.vx, got * p.vy)
    p.m -= got
    this.selfSpent += got
    return got
  }

  // Ledgers

  /** Mana: in particles, in the air, and outside (the caster's reserve). Always the same. */
  mana(): number {
    let m = this.outside
    for (const p of this.particles) m += p.m
    for (const a of this.air) m += a
    return m
  }

  /** Momentum the particles and the air have, less what came from outside. Always zero: everything started still. */
  momentumError(): number {
    let px = 0
    let py = 0
    for (const p of this.particles) {
      px += p.m * p.vx
      py += p.m * p.vy
    }
    for (let c = 0; c < this.air.length; c++) {
      px += this.air[c] * this.airVx[c]
      py += this.air[c] * this.airVy[c]
    }
    for (const j of Object.values(this.impulse)) {
      px -= j[0]
      py -= j[1]
    }
    return Math.hypot(px, py)
  }

  /** Mana still in the weave, as a share of what it began with. */
  held(): number {
    let m = 0
    for (const p of this.particles) if (p.held) m += p.m
    return m / this.amount
  }

  /** How spread the weave is: the root-mean-square distance of its mana from its centre, over the radius it was laid at. */
  spread(): number {
    const o = this.origin
    let m = 0
    let s = 0
    for (const p of this.particles) {
      if (!p.held) continue
      m += p.m
      s += p.m * ((p.x - o.x) ** 2 + (p.y - o.y) ** 2)
    }
    // A disc of radius R, evenly filled, has an RMS radius of R/√2.
    return m > 0 ? Math.sqrt(s / m) / (this.radius / Math.SQRT2) : 0
  }

  kinetic(): number {
    let e = 0
    for (const p of this.particles) e += 0.5 * p.m * (p.vx * p.vx + p.vy * p.vy)
    return e
  }
}

/** How fast a particle past the radius is brought back in: m/tick per metre out. */
export const HOLD_GAIN = 0.2
/** How fast a particle at the thin edge pulls back toward its neighbours, at the thinnest: m/tick. */
export const FEEL_GAIN = 0.05
