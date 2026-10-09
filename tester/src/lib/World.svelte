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

  /** The weave cells in each world cell of the slice: their free mana and the matter they hold. */
  function weaveCells(z: number) {
    const sim = session.sim!
    const w = sim.world
    const out = new Map<number, { free: Parts; carried: Parts; weaves: Set<number> }>()
    for (const wv of sim.weaves.values())
      for (const c of wv.cells) {
        const i = w.cellOf(wv.worldPos(c))
        if (i < 0 || w.coords(i)[2] !== z) continue
        const e = out.get(i) ?? { free: [0, 0, 0, 0], carried: [0, 0, 0, 0], weaves: new Set() }
        for (let k = 0; k < 4; k++) {
          e.free[k] += c.free[k]
          e.carried[k] += c.carried[k]
        }
        e.weaves.add(wv.id)
        out.set(i, e)
      }
    return out
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

    // Weaves: the matter they hold, then their mana, brightest where there's most.
    const cells = weaveCells(z)
    const most = Math.max(1e-9, ...[...cells.values()].map((c) => total(c.free)))
    for (const [i, c] of cells) {
      const [x, y] = w.coords(i)
      const held = total(c.carried)
      if (held > 0.5) {
        g.fillStyle = rgba(MATTER[dominant(c.carried)], 0.35 + 0.65 * Math.min(1, held / PHYSICS.cellMatter))
        g.fillRect(X(x), Y(y), px, px)
      }
      const free = total(c.free)
      const col = MANA[dominant(c.free)]
      g.fillStyle = rgba(col, 0.2 + 0.6 * Math.sqrt(free / most))
      g.fillRect(X(x) + 1, Y(y) + 1, px - 2, px - 2)
      g.strokeStyle = rgba(col, 0.9)
      g.lineWidth = 1
      g.strokeRect(X(x) + 0.5, Y(y) + 0.5, px - 1, px - 1)
    }

    // Loose mana.
    for (const p of w.loose) {
      const i = w.clampedCellOf(p.pos)
      if (w.coords(i)[2] !== z) continue
      const r = Math.max(1.5, Math.min(px / 2, Math.sqrt(total(p.parts)) * 0.8))
      g.fillStyle = rgba(MANA[dominant(p.parts)], 0.85)
      g.beginPath()
      g.arc(p.pos[0] / w.cell * px, (w.h - p.pos[1] / w.cell) * px, r, 0, Math.PI * 2)
      g.fill()
    }

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
    if (wc) bits.push(`weave ${[...wc.weaves].join(', ')}: mana ${fmt(wc.free)}, holds ${fmt(wc.carried)}`)
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
      title="Click to aim"
    ></canvas>
    <div class="under">
      {#if world.d > 1}
        <label class="slice">
          slice z
          <input type="range" min="0" max={world.d - 1} bind:value={session.sliceZ} />
          <span class="num">{session.sliceZ}</span>
        </label>
      {/if}
      <span class="info">{info || 'Hover a cell to read it. Click to aim.'}</span>
    </div>
  {/if}
</div>

<style>
  .world {
    --sky: #110e0b;
    width: 100%;
    overflow-x: auto;
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
