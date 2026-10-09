// The sandbox: mana as a fluid of particles, in 2D, outside the machine (SPEC §11). A ball of mana pushes itself apart
// with pressure; a caster holds it by pushing its particles back in, paying beats and mana for every push.
//
// The fluid is smoothed-particle hydrodynamics: each particle feels its neighbours within `smoothing` metres. Every force
// between two particles is equal and opposite, so momentum only changes by what comes from outside (pushes, the ground,
// the air), and the ledger checks it every tick.

export type Settings = {
  /** M in one particle: the crowd of real mana particles that one simulated particle stands for. */
  mote: number
  /** Metres over which particles feel each other. */
  smoothing: number
  /** How hard mana presses outward: pressure = stiffness × density. The speed it spreads at is about √stiffness. */
  stiffness: number
  /** How much neighbours' motions even out. */
  viscosity: number
  /** How much an exposed particle is slowed by the air, per tick. Particles inside the ball are sheltered. */
  drag: number
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
  substeps: number
}

export const DEFAULTS: Settings = {
  mote: 1,
  smoothing: 0.16,
  stiffness: 0.0009,
  viscosity: 0.0005,
  drag: 0.01,
  rise: 0.0003,
  pushYield: 2,
  pushRate: 0.05,
  field: 2,
  reach: 3,
  substeps: 8,
}

/** The test world, in metres: ground at y = 0, a wall from x = WALL to the edge. */
export const WORLD = { width: 14, height: 6, cell: 0.25, wall: 12 }

export type Particle = {
  x: number
  y: number
  vx: number
  vy: number
  /** M of mana. */
  m: number
  /** Still in the weave: within its caster's field. */
  held: boolean
  /** Carries the hold-yourself order. */
  order: boolean
  /** The tick it was last pushed, by the caster or by its order. */
  pushedAt: number
  /** How much more its velocity can be changed by pushes this tick. */
  dvLeft: number
  rho: number
  ax: number
  ay: number
}

export type Ball = { x: number; y: number; radius: number; amount: number; order?: boolean }

export type Origin = { x: number; y: number; vx: number; vy: number }

const GOLDEN = Math.PI * (3 - Math.sqrt(5))

export class Sandbox {
  particles: Particle[] = []
  /** Mana spent on pushes, loose in the air: M per cell. */
  haze: Float64Array
  readonly hazeW = Math.round(WORLD.width / WORLD.cell)
  readonly hazeH = Math.round(WORLD.height / WORLD.cell)
  tick = 0
  /** The weave's centre of mass and its velocity, as of the start of this tick. */
  origin: Origin = { x: 0, y: 0, vx: 0, vy: 0 }
  /** Momentum given to the particles from outside: pushes, orders, rising, the ground and walls, the air. */
  impulse = { push: [0, 0], order: [0, 0], rise: [0, 0], walls: [0, 0], air: [0, 0] }
  /** The body, which the field reaches out from. */
  caster = { x: 1, y: 1.5 }
  /** Density of a particle deep inside the ball as it was laid out: what counts as sheltered from the air. */
  readonly rhoFull: number
  readonly radius: number
  readonly amount: number
  /** Mana that isn't in the sandbox's particles or haze: the caster's reserve, which pushes are paid from. */
  outside = 0
  /** Mana the particles have spent on their own orders. */
  selfSpent = 0

  constructor(
    readonly settings: Settings,
    ball: Ball,
  ) {
    this.haze = new Float64Array(this.hazeW * this.hazeH)
    this.radius = ball.radius
    this.amount = ball.amount
    // A sunflower: points spread evenly over a disc, each with the same share.
    const n = Math.max(1, Math.round(ball.amount / settings.mote))
    for (let k = 0; k < n; k++) {
      const r = ball.radius * Math.sqrt((k + 0.5) / n)
      const t = k * GOLDEN
      this.particles.push({
        x: ball.x + r * Math.cos(t),
        y: ball.y + r * Math.sin(t),
        vx: 0,
        vy: 0,
        m: ball.amount / n,
        held: true,
        order: !!ball.order,
        pushedAt: -1,
        dvLeft: settings.pushRate,
        rho: 0,
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

  /** One tick of the world, after the caster has thought: orders, then the fluid, then who's still in the weave. */
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

  private density() {
    const h = this.settings.smoothing
    const poly6 = 4 / (Math.PI * h ** 8)
    this.neighbours()
    for (const p of this.particles) p.rho = p.m * poly6 * h ** 6
    this.pairs((a, b, _dx, _dy, r) => {
      const w = poly6 * (h * h - r * r) ** 3
      a.rho += b.m * w
      b.rho += a.m * w
    })
  }

  private forces() {
    const s = this.settings
    const h = s.smoothing
    const spiky = -30 / (Math.PI * h ** 5)
    const lap = 40 / (Math.PI * h ** 5)
    for (const p of this.particles) {
      p.ax = 0
      p.ay = s.rise
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
    // The air slows what it can reach: the outside of the ball, not its sheltered middle.
    for (const p of this.particles) {
      const exposed = Math.min(1, Math.max(0, 1 - p.rho / this.rhoFull))
      const k = s.drag * exposed
      p.ax -= k * p.vx
      p.ay -= k * p.vy
    }
    const dt = 1 / s.substeps
    for (const p of this.particles) {
      // What came from outside: rising and the air.
      const k = s.drag * Math.min(1, Math.max(0, 1 - p.rho / this.rhoFull))
      this.impulse.rise[1] += p.m * s.rise * dt
      this.impulse.air[0] -= p.m * k * p.vx * dt
      this.impulse.air[1] -= p.m * k * p.vy * dt
    }
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
      if (!p.held && Math.hypot(p.vx, p.vy) < 0.01 && p.rho < 0.3 * this.rhoFull) {
        this.impulse.air[0] -= p.m * p.vx
        this.impulse.air[1] -= p.m * p.vy
        this.loose(p.x, p.y, p.m)
      } else keep.push(p)
    }
    this.particles = keep
  }

  /** Mana let loose into the air at a point. */
  loose(x: number, y: number, m: number) {
    const i = Math.min(this.hazeW - 1, Math.max(0, Math.floor(x / WORLD.cell)))
    const j = Math.min(this.hazeH - 1, Math.max(0, Math.floor(y / WORLD.cell)))
    this.haze[j * this.hazeW + i] += m
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
    this.loose(p.x, p.y, got)
    return got
  }

  /**
   * The hold-yourself order, run by every particle that carries it: past the ball's radius and moving out, it pushes itself
   * back in, paying with its own mana.
   */
  private runOrders() {
    const o = this.origin
    const keep: Particle[] = []
    for (const p of this.particles) {
      if (p.order && p.held) {
        const dx = p.x - o.x
        const dy = p.y - o.y
        const r = Math.hypot(dx, dy) || 1e-9
        const nx = dx / r
        const ny = dy / r
        const ur = (p.vx - o.vx) * nx + (p.vy - o.vy) * ny
        const want = -HOLD_GAIN * Math.max(0, r - this.radius)
        if (ur > want) {
          this.push(p, (want - ur) * nx, (want - ur) * ny, (m) => this.spendSelf(p, m), 'order')
        }
      }
      if (p.m > 1e-3) keep.push(p)
      else {
        this.impulse.air[0] -= p.m * p.vx
        this.impulse.air[1] -= p.m * p.vy
        this.loose(p.x, p.y, p.m)
      }
    }
    this.particles = keep
  }

  /** A particle paying with itself. The mana it spends leaves carrying its share of the particle's momentum, into the air. */
  private spendSelf(p: Particle, m: number): number {
    const got = Math.min(m, p.m * 0.5)
    this.impulse.air[0] -= got * p.vx
    this.impulse.air[1] -= got * p.vy
    p.m -= got
    this.selfSpent += got
    return got
  }

  // Ledgers

  /** Mana: in particles, loose in the air, and outside (the caster's reserve). Always the same. */
  mana(): number {
    let m = this.outside
    for (const p of this.particles) m += p.m
    for (const h of this.haze) m += h
    return m
  }

  /** Momentum the particles have, less what came from outside. Always zero: they started still. */
  momentumError(): number {
    let px = 0
    let py = 0
    for (const p of this.particles) {
      px += p.m * p.vx
      py += p.m * p.vy
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
