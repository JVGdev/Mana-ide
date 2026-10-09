<script lang="ts">
  import { session } from './session.svelte.ts'
  import { dominant, total } from '../../../src/vm/parts.ts'
  import { PHYSICS } from '../../../src/vm/physics.ts'
  import type { Parts } from '../../../src/vm/parts.ts'

  let canvas = $state<HTMLCanvasElement>()
  let wrap: HTMLDivElement
  let width = $state(600)
  let hover = $state<{ x: number; y: number } | null>(null)

  // Fire, water, air, earth.
  const MANA = ['#ff8a3d', '#4fa3ff', '#d9ecff', '#d2a865']
  const MATTER = ['#ffb347', '#2f6fb3', '#9fb4c7', '#7a5b3a']

  const rgba = (hex: string, a: number) => {
    const n = parseInt(hex.slice(1), 16)
    return `rgba(${n >> 16}, ${(n >> 8) & 255}, ${n & 255}, ${Math.max(0, Math.min(1, a))})`
  }

  const world = $derived(session.sim?.world)
  const px = $derived(world ? Math.max(4, Math.floor(width / world.w)) : 10)

  /** What weaves hold in each world cell of the slice: their free mana and the matter they carry. */
  function weaveCells(z: number) {
    const sim = session.sim!
    const w = sim.world
    const out = new Map<number, { free: Parts; carried: Parts; weaves: Set<number>; particles: number }>()
    for (const p of w.particles) {
      if (!p.weave) continue
      const i = w.cellOf(p.pos)
      if (i < 0 || w.coords(i)[2] !== z) continue
      const e = out.get(i) ?? { free: [0, 0, 0, 0], carried: [0, 0, 0, 0], weaves: new Set(), particles: 0 }
      for (let k = 0; k < 4; k++) {
        e.free[k] += p.free[k]
        e.carried[k] += p.carried[k]
      }
      e.weaves.add(p.weave)
      e.particles++
      out.set(i, e)
    }
    return out
  }

  /** The particle whose order is being looked at. */
  function selected() {
    const sim = session.sim
    const t = sim?.traces.get(session.orderSel.weave)?.[session.orderSel.particle]
    return t ? sim!.weaves.get(t.weave)?.particles[t.particle] : undefined
  }

  function draw() {
    void session.version
    const sim = session.sim
    if (!sim || !canvas) return
    const w = sim.world
    const z = Math.min(w.d - 1, Math.max(0, session.sliceZ))
    const dpr = window.devicePixelRatio || 1
    canvas.width = w.w * px * dpr
    canvas.height = w.h * px * dpr
    canvas.style.width = `${w.w * px}px`
    canvas.style.height = `${w.h * px}px`
    const g = canvas.getContext('2d')!
    g.setTransform(dpr, 0, 0, dpr, 0, 0)
    const style = getComputedStyle(canvas)
    g.fillStyle = style.getPropertyValue('--sky').trim() || '#120f0c'
    g.fillRect(0, 0, w.w * px, w.h * px)
    const X = (x: number) => x * px
    const Y = (y: number) => (w.h - 1 - y) * px

    // Air mana: brighter where there's more than the world's usual.
    for (let y = 0; y < w.h; y++)
      for (let x = 0; x < w.w; x++) {
        const a = w.air[w.index(x, y, z)]
        const extra = (total(a) - PHYSICS.airMana) / PHYSICS.airMana
        if (Math.abs(extra) < 0.03) continue
        g.fillStyle = extra > 0 ? rgba(MANA[dominant(a)], Math.min(0.35, extra * 0.5)) : `rgba(0,0,0,${Math.min(0.5, -extra)})`
        g.fillRect(X(x), Y(y), px, px)
      }

    // Matter.
    for (let y = 0; y < w.h; y++)
      for (let x = 0; x < w.w; x++) {
        const m = w.matter[w.index(x, y, z)]
        const t = total(m)
        if (t < 0.5) continue
        g.fillStyle = rgba(MATTER[dominant(m)], 0.25 + 0.75 * Math.min(1, t / PHYSICS.cellMatter))
        g.fillRect(X(x), Y(y), px, px)
      }

    // Weaves: the matter they hold, cell by cell.
    const cells = weaveCells(z)
    for (const [i, c] of cells) {
      const [x, y] = w.coords(i)
      const held = total(c.carried)
      if (held > 0.5) {
        g.fillStyle = rgba(MATTER[dominant(c.carried)], 0.35 + 0.65 * Math.min(1, held / PHYSICS.cellMatter))
        g.fillRect(X(x), Y(y), px, px)
      }
    }

    // Wind: the air, where it moves.
    g.lineWidth = 1
    for (let y = 0; y < w.h; y++)
      for (let x = 0; x < w.w; x++) {
        const i = w.index(x, y, z)
        const vx = w.airVel[i * 3]
        const vy = w.airVel[i * 3 + 1]
        const v = Math.hypot(vx, vy)
        if (v < 0.001) continue
        const len = Math.min(px * 0.9, px * (0.2 + v * 20)) / v
        const cx = X(x) + px / 2
        const cy = Y(y) + px / 2
        g.strokeStyle = rgba('#8db0cf', Math.min(0.8, 0.2 + v * 30))
        g.beginPath()
        g.moveTo(cx - (vx * len) / 2, cy + (vy * len) / 2)
        g.lineTo(cx + (vx * len) / 2, cy - (vy * len) / 2)
        g.stroke()
      }

    // Mana: every particle. In a weave it glows; loose, it's dim; pushed this tick, it's bright.
    g.globalCompositeOperation = 'lighter'
    for (const p of w.particles) {
      if (w.d > 1 && Math.floor(p.pos[2] / w.cell) !== z) continue
      const m = total(p.free)
      if (m <= 0) continue
      const r = Math.max(1.2, Math.sqrt(m / PHYSICS.mote) * px * 0.16)
      const pushed = p.pushedAt === sim.tick - 1
      g.fillStyle = rgba(pushed ? '#fff1d6' : MANA[dominant(p.free)], p.weave ? 0.85 : 0.4)
      g.beginPath()
      g.arc((p.pos[0] / w.cell) * px, (w.h - p.pos[1] / w.cell) * px, r, 0, Math.PI * 2)
      g.fill()
    }
    g.globalCompositeOperation = 'source-over'

    // Bodies.
    const caster = session.caster
    for (const b of w.bodies) {
      const zc = b.pos[2] / w.cell
      if (Math.abs(zc - (z + 0.5)) > b.half[2] / w.cell + 0.5) continue
      const left = ((b.pos[0] - b.half[0]) / w.cell) * px
      const top = (w.h - (b.pos[1] + b.half[1]) / w.cell) * px
      const isCaster = caster?.body === b
      g.fillStyle = isCaster ? 'rgba(214, 167, 92, 0.85)' : 'rgba(192, 179, 156, 0.75)'
      g.fillRect(left, top, ((2 * b.half[0]) / w.cell) * px, ((2 * b.half[1]) / w.cell) * px)
      g.fillStyle = 'rgba(235, 227, 210, 0.9)'
      g.font = `${Math.max(10, px)}px var(--font-ui), sans-serif`
      g.fillText(b.name, left, top - 4)
    }

    // The particle whose order is being looked at.
    if (session.panel === 'order') {
      const p = selected()
      if (p && (w.d === 1 || Math.floor(p.pos[2] / w.cell) === z)) {
        g.strokeStyle = '#8db0cf'
        g.lineWidth = 2
        g.beginPath()
        g.arc((p.pos[0] / w.cell) * px, (w.h - p.pos[1] / w.cell) * px, Math.max(5, px * 0.5), 0, Math.PI * 2)
        g.stroke()
      }
    }

    // The hand and the aim.
    if (caster) {
      const [hx, hy] = caster.hand
      g.fillStyle = '#e6bd7a'
      g.beginPath()
      g.arc((hx / w.cell) * px, (w.h - hy / w.cell) * px, Math.max(2, px / 4), 0, Math.PI * 2)
      g.fill()
      const [ax, ay] = session.will.aim
      const cx = (ax / w.cell) * px
      const cy = (w.h - ay / w.cell) * px
      g.strokeStyle = '#e58a72'
      g.lineWidth = 1.5
      g.beginPath()
      g.arc(cx, cy, px * 0.6, 0, Math.PI * 2)
      g.moveTo(cx - px, cy)
      g.lineTo(cx + px, cy)
      g.moveTo(cx, cy - px)
      g.lineTo(cx, cy + px)
      g.stroke()
    }
  }

  $effect(draw)

  $effect(() => {
    const ro = new ResizeObserver(() => (width = wrap.clientWidth))
    ro.observe(wrap)
    return () => ro.disconnect()
  })

  function cellAt(e: MouseEvent) {
    const r = canvas!.getBoundingClientRect()
    const w = world!
    const x = Math.floor((e.clientX - r.left) / px)
    const y = w.h - 1 - Math.floor((e.clientY - r.top) / px)
    return { x, y, mx: ((e.clientX - r.left) / px) * w.cell, my: (w.h - (e.clientY - r.top) / px) * w.cell }
  }

  function click(e: MouseEvent) {
    if (!world) return
    const c = cellAt(e)
    if (session.picking) {
      // The nearest particle that ran its order last tick.
      const sim = session.sim!
      let best: { weave: number; k: number; d: number } | undefined
      for (const [id, traces] of sim.traces) {
        const wv = sim.weaves.get(id)
        if (!wv) continue
        traces.forEach((t, k) => {
          const p = wv.particles[t.particle]
          if (!p || (world.d > 1 && Math.floor(p.pos[2] / world.cell) !== session.sliceZ)) return
          const d = Math.hypot(p.pos[0] - c.mx, p.pos[1] - c.my)
          if (!best || d < best.d) best = { weave: id, k, d }
        })
      }
      if (best && best.d < world.cell * 2) {
        session.picking = false
        session.selectOrder(best.weave, best.k)
      }
      return
    }
    session.setAim([c.mx, c.my, (session.sliceZ + 0.5) * world.cell])
  }

  const fmt = (p: Parts) => p.map((v) => v.toFixed(1)).join(' / ')

  const info = $derived.by(() => {
    void session.version
    const sim = session.sim
    if (!hover || !sim) return ''
    const w = sim.world
    const i = w.index(hover.x, hover.y, session.sliceZ)
    if (i < 0) return ''
    const bits = [`cell ${hover.x}, ${hover.y}${w.d > 1 ? `, ${session.sliceZ}` : ''}`, `air ${fmt(w.air[i])}`]
    if (total(w.matter[i]) > 0.01) bits.push(`matter ${fmt(w.matter[i])}`)
    const wc = weaveCells(session.sliceZ).get(i)
    if (wc) bits.push(`weave ${[...wc.weaves].join(', ')}: ${wc.particles} particles, mana ${fmt(wc.free)}, holds ${fmt(wc.carried)}`)
    const v = [w.airVel[i * 3], w.airVel[i * 3 + 1], w.airVel[i * 3 + 2]]
    if (Math.hypot(...v) > 0.0005) bits.push(`wind ${v.map((x) => x.toFixed(3)).join(', ')} m/t`)
    return bits.join('  ·  ')
  })
</script>

<div class="world" bind:this={wrap}>
  {#if world}
    <canvas
      bind:this={canvas}
      onclick={click}
      onmousemove={(e) => {
        const c = cellAt(e)
        hover = { x: c.x, y: c.y }
      }}
      onmouseleave={() => (hover = null)}
      title={session.picking ? 'Click a particle' : 'Click to aim'}
      class:picking={session.picking}
    ></canvas>
    <div class="under">
      {#if world.d > 1}
        <label class="slice">
          slice z
          <input type="range" min="0" max={world.d - 1} bind:value={session.sliceZ} />
          <span class="num">{session.sliceZ}</span>
        </label>
      {/if}
      <span class="info">{session.picking ? 'Click a particle of a weave to see its order.' : info || 'Hover a cell to read it. Click to aim.'}</span>
    </div>
  {/if}
</div>

<style>
  .world {
    --sky: #110e0b;
    width: 100%;
    overflow-x: auto;
  }
  canvas.picking {
    cursor: pointer;
    outline: 2px solid var(--info);
  }
  canvas {
    display: block;
    cursor: crosshair;
    border-radius: var(--radius-xs);
    image-rendering: pixelated;
  }
  .under {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 14px;
    align-items: center;
    padding: 6px 2px 0;
    font-size: 12px;
    color: var(--muted);
    min-height: 24px;
  }
  .info {
    font-family: var(--font-mono);
    font-size: 11.5px;
  }
  .slice {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .num {
    font-family: var(--font-mono);
    min-width: 2ch;
  }
</style>
