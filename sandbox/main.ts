// The sandbox page: the world on a canvas, the caster's choices, the physics as sliders, and every strategy compared.

import { DEFAULTS, WORLD } from './sim.ts'
import { STRATEGIES, strategy } from './strategies.ts'
import { CASTERS, type CasterKind } from './mind.ts'
import { DEFAULT_SETUP, Run, play, type Kind, type Setup } from './scenario.ts'

const $ = <T extends HTMLElement = HTMLElement>(id: string) => document.getElementById(id) as T

const setup: Setup = { ...DEFAULT_SETUP, kind: 'throw', caster: 'master', strategy: 'order', settings: { ...DEFAULTS } }
let run = new Run(setup)
let playing = !matchMedia('(prefers-reduced-motion: reduce)').matches
let tps = 10

// Controls

const kinds: Record<Kind, HTMLButtonElement> = { hold: $('kind-hold'), throw: $('kind-throw') }
const casterSel = $<HTMLSelectElement>('caster')
const strategySel = $<HTMLSelectElement>('strategy')
const playBtn = $<HTMLButtonElement>('play')
const letBtn = $<HTMLButtonElement>('let')

function fillStrategies() {
  strategySel.innerHTML = ''
  for (const s of STRATEGIES) {
    if (setup.kind === 'hold' && s.throwOnly) continue
    strategySel.add(new Option(s.name, s.id))
  }
  if (setup.kind === 'hold' && strategy(setup.strategy).throwOnly) setup.strategy = 'surface'
  strategySel.value = setup.strategy
}

function recast() {
  run = new Run({ ...setup, settings: { ...setup.settings } })
  for (const k of Object.keys(kinds) as Kind[]) kinds[k].setAttribute('aria-pressed', String(k === setup.kind))
  casterSel.value = setup.caster
  $('about').textContent = strategy(setup.strategy).about
  readouts()
  draw()
}

for (const k of Object.keys(kinds) as Kind[]) {
  kinds[k].addEventListener('click', () => {
    setup.kind = k
    fillStrategies()
    recast()
    compare()
  })
}
casterSel.addEventListener('change', () => {
  setup.caster = casterSel.value as CasterKind
  recast()
  compare()
})
strategySel.addEventListener('change', () => {
  setup.strategy = strategySel.value
  recast()
  markCurrent()
})
playBtn.addEventListener('click', () => {
  if (finished()) recast()
  playing = !playing
  readouts()
})
$('step').addEventListener('click', () => {
  playing = false
  tick()
})
letBtn.addEventListener('click', () => {
  run.letGo()
  readouts()
})
$('recast').addEventListener('click', recast)
$('tps').addEventListener('change', (e) => (tps = Number((e.target as HTMLSelectElement).value)))
$('compare').addEventListener('click', compare)

// The physics, as sliders

type Slider = {
  id: string
  label: string
  min: number
  max: number
  step: number
  get: () => number
  set: (v: number) => void
  show: (v: number) => string
  help?: (v: number) => string
}

const S = setup.settings
const SLIDERS: Slider[] = [
  {
    id: 'amount',
    label: 'Mana in the ball',
    min: 40,
    max: 400,
    step: 10,
    get: () => setup.amount,
    set: (v) => (setup.amount = v),
    show: (v) => `${v} M`,
    help: (v) => `${Math.round(v / S.mote)} particles`,
  },
  {
    id: 'radius',
    label: 'Radius',
    min: 0.3,
    max: 1,
    step: 0.05,
    get: () => setup.radius,
    set: (v) => (setup.radius = v),
    show: (v) => `${v.toFixed(2)} m`,
  },
  {
    id: 'speed',
    label: 'Throw at',
    min: 0.1,
    max: 0.8,
    step: 0.05,
    get: () => setup.speed,
    set: (v) => (setup.speed = v),
    show: (v) => `${v.toFixed(2)} m/tick`,
  },
  {
    id: 'stiffness',
    label: 'Pressure',
    min: 1,
    max: 80,
    step: 1,
    get: () => S.stiffness * 1e4,
    set: (v) => (S.stiffness = v * 1e-4),
    show: (v) => `${v} × 10⁻⁴`,
    help: (v) => `spreads at about ${Math.sqrt(v * 1e-4).toFixed(3)} m/tick`,
  },
  {
    id: 'drag',
    label: 'Air drag',
    min: 0,
    max: 0.06,
    step: 0.005,
    get: () => S.drag,
    set: (v) => (S.drag = v),
    show: (v) => `${(v * 100).toFixed(1)}%/tick`,
    help: () => 'on the exposed outside only',
  },
  {
    id: 'rise',
    label: 'Fire rises',
    min: 0,
    max: 0.002,
    step: 0.0001,
    get: () => S.rise,
    set: (v) => (S.rise = v),
    show: (v) => `${(v * 1000).toFixed(1)} mm/tick²`,
  },
  {
    id: 'yield',
    label: 'Push yield',
    min: 0.5,
    max: 8,
    step: 0.5,
    get: () => S.pushYield,
    set: (v) => (S.pushYield = v),
    show: (v) => v.toFixed(1),
    help: (v) => `1 M poured moves 1 M of mana by ${v.toFixed(1)} m/tick`,
  },
  {
    id: 'rate',
    label: 'Push rate',
    min: 0.01,
    max: 0.15,
    step: 0.01,
    get: () => S.pushRate,
    set: (v) => (S.pushRate = v),
    show: (v) => `${v.toFixed(2)} m/tick`,
    help: () => 'the most one particle can be sped up in a tick',
  },
  {
    id: 'mote',
    label: 'Mana per particle',
    min: 0.5,
    max: 4,
    step: 0.25,
    get: () => S.mote,
    set: (v) => (S.mote = v),
    show: (v) => `${v.toFixed(2)} M`,
    help: () => 'one particle stands for a crowd of real ones',
  },
]

const slidersEl = $('sliders')
for (const s of SLIDERS) {
  const wrap = document.createElement('div')
  wrap.className = 'slider'
  wrap.innerHTML = `<label for="s-${s.id}">${s.label}</label><output id="o-${s.id}"></output>
    <input type="range" id="s-${s.id}" min="${s.min}" max="${s.max}" step="${s.step}" value="${s.get()}" />
    <small id="h-${s.id}"></small>`
  slidersEl.append(wrap)
  const input = wrap.querySelector('input')!
  const show = () => {
    const v = Number(input.value)
    $(`o-${s.id}`).textContent = s.show(v)
    $(`h-${s.id}`).textContent = s.help?.(v) ?? ''
  }
  show()
  input.addEventListener('input', () => {
    s.set(Number(input.value))
    show()
    for (const o of SLIDERS) if (o.help) $(`h-${o.id}`).textContent = o.help(o.get())
    recast()
  })
  input.addEventListener('change', compare)
}

// Running

function finished(): boolean {
  const { sim } = run
  if (setup.kind === 'throw') return (run.hitAt >= 0 && sim.tick > run.hitAt + 40) || sim.tick > 260
  return sim.tick >= 200
}

function tick() {
  if (finished()) {
    playing = false
    readouts()
    return
  }
  run.step()
  readouts()
  draw()
}

let last = performance.now()
let owed = 0
function frame(now: number) {
  const dt = Math.min(0.25, (now - last) / 1000)
  last = now
  if (playing) {
    owed += dt * tps
    while (owed >= 1 && playing) {
      owed -= 1
      tick()
    }
  } else owed = 0
  requestAnimationFrame(frame)
}

// Readouts

function readouts() {
  const { sim, mind } = run
  const c = CASTERS[setup.caster]
  $('r-tick').textContent = String(sim.tick)
  $('r-held').textContent = `${(sim.held() * 100).toFixed(0)}%`
  $('r-spread').textContent = sim.spread().toFixed(2)
  $('r-speed').textContent = `${Math.hypot(sim.origin.vx, sim.origin.vy).toFixed(2)} m/t`
  $('r-pushes').textContent = String(mind.pushesThisTick)
  $('r-spent').textContent = `${(mind.spent + sim.selfSpent).toFixed(1)} M`
  $('r-mana').textContent = `${mind.mana.toFixed(0)} M`
  const used = Math.min(mind.usedThisTick, c.beats)
  $('r-beats-label').textContent = `Beats this tick: ${used} of ${c.beats}`
  $('r-beats').style.width = `${(used / c.beats) * 100}%`
  const manaOk = run.manaError() < 1e-6
  const momOk = sim.momentumError() < 1e-6
  $('c-mana').textContent = manaOk ? 'Mana balanced' : `Mana off by ${run.manaError().toExponential(1)}`
  $('c-mana').classList.toggle('bad', !manaOk)
  $('c-momentum').textContent = momOk ? 'Momentum balanced' : `Momentum off by ${sim.momentumError().toExponential(1)}`
  $('c-momentum').classList.toggle('bad', !momOk)
  let state = mind.holding ? 'holding' : `let go at tick ${run.letGoAt}`
  if (run.hitAt >= 0) state = `hit the wall at tick ${run.hitAt}`
  if (strategy(setup.strategy).id === 'none' && setup.kind === 'hold') state = 'nobody holds it'
  $('state').textContent = state
  letBtn.disabled = !mind.holding
  playBtn.textContent = playing ? 'Pause' : finished() ? 'Cast again and play' : 'Play'
}

// The world

const canvas = $<HTMLCanvasElement>('world')
const ctx = canvas.getContext('2d')!
const css = getComputedStyle(document.documentElement)
const color = (name: string) => css.getPropertyValue(name).trim()
const C = {
  panel: color('--panel'),
  line: color('--line'),
  line2: color('--line-2'),
  muted: color('--muted'),
  text2: color('--text-2'),
  accent: color('--accent'),
  fire: color('--fire'),
  hot: color('--fire-hot'),
  loose: color('--loose'),
  info: color('--info'),
}

function size() {
  const dpr = window.devicePixelRatio || 1
  const w = Math.max(320, Math.round(canvas.clientWidth * dpr))
  canvas.width = w
  canvas.height = Math.round((w * WORLD.height) / WORLD.width)
  draw()
}
new ResizeObserver(size).observe(canvas)

function draw() {
  const { sim, mind } = run
  const k = canvas.width / WORLD.width
  const X = (x: number) => x * k
  const Y = (y: number) => canvas.height - y * k
  ctx.setTransform(1, 0, 0, 1, 0, 0)
  ctx.fillStyle = C.panel
  ctx.fillRect(0, 0, canvas.width, canvas.height)

  // A metre grid.
  ctx.strokeStyle = C.line
  ctx.lineWidth = 1
  ctx.beginPath()
  for (let x = 1; x < WORLD.width; x++) {
    ctx.moveTo(X(x) + 0.5, 0)
    ctx.lineTo(X(x) + 0.5, canvas.height)
  }
  for (let y = 1; y < WORLD.height; y++) {
    ctx.moveTo(0, Y(y) + 0.5)
    ctx.lineTo(canvas.width, Y(y) + 0.5)
  }
  ctx.stroke()

  // Spent mana, loose in the air.
  const cell = WORLD.cell * k
  for (let j = 0; j < sim.hazeH; j++) {
    for (let i = 0; i < sim.hazeW; i++) {
      const m = sim.haze[j * sim.hazeW + i]
      if (m < 0.01) continue
      ctx.globalAlpha = Math.min(0.55, 0.08 + m * 0.35)
      ctx.fillStyle = C.accent
      ctx.fillRect(X(i * WORLD.cell), Y((j + 1) * WORLD.cell), cell, cell)
    }
  }
  ctx.globalAlpha = 1

  // The wall.
  ctx.fillStyle = C.line2
  ctx.fillRect(X(WORLD.wall), 0, canvas.width - X(WORLD.wall), canvas.height)
  ctx.strokeStyle = C.line
  ctx.lineWidth = Math.max(1, k * 0.03)
  ctx.save()
  ctx.beginPath()
  ctx.rect(X(WORLD.wall), 0, canvas.width, canvas.height)
  ctx.clip()
  ctx.beginPath()
  for (let y = -2; y < WORLD.height + 2; y += 0.4) {
    ctx.moveTo(X(WORLD.wall), Y(y))
    ctx.lineTo(X(WORLD.width), Y(y + (WORLD.width - WORLD.wall)))
  }
  ctx.stroke()
  ctx.restore()

  // The caster, and how far they reach.
  const c = sim.caster
  if (mind.holding) {
    ctx.setLineDash([k * 0.06, k * 0.08])
    ctx.strokeStyle = C.line2
    ctx.lineWidth = Math.max(1, k * 0.012)
    ctx.beginPath()
    ctx.arc(X(c.x), Y(c.y), sim.settings.reach * k, 0, Math.PI * 2)
    ctx.stroke()
    ctx.setLineDash([])
  }
  ctx.strokeStyle = C.text2
  ctx.lineWidth = Math.max(2, k * 0.05)
  ctx.lineCap = 'round'
  ctx.beginPath()
  ctx.moveTo(X(c.x), Y(c.y - 0.05))
  ctx.lineTo(X(c.x), Y(0.45))
  ctx.moveTo(X(c.x), Y(0.45))
  ctx.lineTo(X(c.x - 0.15), Y(0))
  ctx.moveTo(X(c.x), Y(0.45))
  ctx.lineTo(X(c.x + 0.15), Y(0))
  ctx.moveTo(X(c.x), Y(c.y - 0.35))
  ctx.lineTo(X(c.x + 0.45), Y(c.y))
  ctx.stroke()
  ctx.fillStyle = C.text2
  ctx.beginPath()
  ctx.arc(X(c.x), Y(c.y + 0.12), k * 0.13, 0, Math.PI * 2)
  ctx.fill()

  // The field and the radius it's held to.
  if (sim.held() > 0.01) {
    const o = sim.origin
    ctx.lineWidth = Math.max(1, k * 0.012)
    ctx.strokeStyle = C.accent
    ctx.globalAlpha = 0.5
    ctx.setLineDash([k * 0.05, k * 0.05])
    ctx.beginPath()
    ctx.arc(X(o.x), Y(o.y), sim.settings.field * sim.radius * k, 0, Math.PI * 2)
    ctx.stroke()
    ctx.setLineDash([])
    ctx.globalAlpha = 0.35
    ctx.beginPath()
    ctx.arc(X(o.x), Y(o.y), sim.radius * k, 0, Math.PI * 2)
    ctx.stroke()
    ctx.globalAlpha = 1
  }

  // The mana.
  const r0 = Math.max(1.5, 0.042 * k)
  ctx.globalCompositeOperation = 'lighter'
  for (const p of sim.particles) {
    const pushed = p.pushedAt === sim.tick - 1
    ctx.fillStyle = !p.held ? C.loose : pushed ? C.hot : C.fire
    ctx.globalAlpha = p.held ? 0.9 : 0.6
    ctx.beginPath()
    ctx.arc(X(p.x), Y(p.y), r0 * Math.sqrt(p.m / sim.settings.mote), 0, Math.PI * 2)
    ctx.fill()
  }
  ctx.globalCompositeOperation = 'source-over'
  ctx.globalAlpha = 1

  // Scale.
  ctx.fillStyle = C.muted
  ctx.font = `${Math.max(11, k * 0.13)}px ${color('--font-mono')}`
  ctx.fillText('1 m', X(0.08), Y(WORLD.height - 0.25))
  ctx.strokeStyle = C.muted
  ctx.lineWidth = Math.max(1, k * 0.015)
  ctx.beginPath()
  ctx.moveTo(X(0.08), Y(WORLD.height - 0.35))
  ctx.lineTo(X(1.08), Y(WORLD.height - 0.35))
  ctx.stroke()
}

// Comparing every strategy

let comparing = 0
function compare() {
  const token = ++comparing
  const s = { ...setup, settings: { ...setup.settings } }
  const list = STRATEGIES.filter((x) => !(s.kind === 'hold' && x.throwOnly))
  const head = $('compare-head')
  const body = $('compare-body')
  head.innerHTML =
    s.kind === 'hold'
      ? '<tr><th>How</th><th>In the weave</th><th>Spread</th><th>Mana spent</th><th>Beats/tick</th><th>Pushes/tick</th></tr>'
      : '<tr><th>How</th><th>Let go at</th><th>In the weave then</th><th>Reached the wall</th><th>In the weave there</th><th>Mana spent</th></tr>'
  body.innerHTML = ''
  const ticks = s.kind === 'hold' ? 40 : 160
  $('compare-note').textContent =
    s.kind === 'hold'
      ? `${s.caster[0].toUpperCase() + s.caster.slice(1)}, holding ${s.amount} M at rest for ${ticks} ticks.`
      : `${s.caster[0].toUpperCase() + s.caster.slice(1)}, throwing ${s.amount} M at ${s.speed} m/tick. It's let go once it's up to speed or out of reach.`
  let i = 0
  const next = () => {
    if (token !== comparing || i >= list.length) return
    const st = list[i++]
    const r = play({ ...s, strategy: st.id }, ticks)
    const tr = document.createElement('tr')
    tr.dataset.id = st.id
    const pct = (v: number) => `${(v * 100).toFixed(0)}%`
    tr.innerHTML =
      s.kind === 'hold'
        ? `<td>${st.name}</td><td>${pct(r.held)}</td><td>${r.spread.toFixed(2)}</td><td>${r.spent.toFixed(1)} M</td><td>${r.beatsPerTick.toFixed(0)}</td><td>${r.pushesPerTick.toFixed(1)}</td>`
        : `<td>${st.name}</td><td>${r.letGoAt < 0 ? '—' : `tick ${r.letGoAt}`}</td><td>${r.letGoAt < 0 ? '—' : pct(r.heldAtLetGo)}</td><td>${r.hitAt < 0 ? 'never' : `tick ${r.hitAt}`}</td><td>${r.hitAt < 0 ? '—' : pct(r.held)}</td><td>${r.spent.toFixed(1)} M</td>`
    body.append(tr)
    markCurrent()
    setTimeout(next, 0)
  }
  next()
}

function markCurrent() {
  for (const tr of $('compare-body').querySelectorAll('tr')) {
    tr.classList.toggle('current', (tr as HTMLElement).dataset.id === setup.strategy)
  }
}

fillStrategies()
recast()
size()
compare()
requestAnimationFrame(frame)
