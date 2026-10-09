<script lang="ts">
  import { session } from '../session.svelte.ts'
  import { num } from '../format.ts'
  import { disassembly, PHYSICS } from '../engine.ts'

  const STARTS = ['from centre x', 'from centre y', 'from centre z', 'its mana', 'weave age']

  /** Last tick's orders: which weaves ran them, and how many of each weave's particles did. */
  const traces = $derived.by(() => {
    void session.version
    return session.machine?.traceCounts() ?? new Map<number, number>()
  })
  const all = $derived({ length: traces.get(session.orderSel.weave) ?? 0 })
  const trace = $derived(session.orderTrace())
  const text = (addr: number) => {
    const p = session.program
    if (!p) return ''
    return disassembly(p).at.get(addr)?.text ?? '?'
  }
  const beatsOf = (addr: number) => {
    const p = session.program
    return (p && disassembly(p).at.get(addr)?.beats) ?? 0
  }

  const at = $derived(Math.min(session.orderSel.step, trace ? trace.steps.length : 0))
  const regs = $derived(trace ? (at < trace.steps.length ? trace.steps[at].n : trace.n) : null)
  const prev = $derived(trace && at > 0 ? trace.steps[at - 1].n : null)
  const line = $derived(trace && at < trace.steps.length ? session.program?.lines.get(trace.steps[at].addr) : undefined)

  function go(step: number) {
    if (!trace) return
    session.orderSel.step = Math.max(0, Math.min(trace.steps.length, step))
    if (session.follow) session.goOrder()
  }
  function particle(k: number) {
    if (!all.length) return
    session.orderSel.particle = (k + all.length) % all.length
    session.orderSel.step = 0
    if (session.follow) session.goOrder()
  }

  let list = $state<HTMLDivElement>()
  // Keep the current instruction in sight, scrolling only the list.
  $effect(() => {
    void at
    const on = list?.querySelector<HTMLElement>('.on')
    if (!list || !on) return
    const top = on.offsetTop
    if (top < list.scrollTop) list.scrollTop = top
    else if (top + on.offsetHeight > list.scrollTop + list.clientHeight) list.scrollTop = top + on.offsetHeight - list.clientHeight
  })

  function keys(e: KeyboardEvent) {
    if (e.key === 'ArrowDown' || e.key === 'ArrowRight') go(at + 1)
    else if (e.key === 'ArrowUp' || e.key === 'ArrowLeft') go(at - 1)
    else return
    e.preventDefault()
  }
</script>

<div class="pick">
  <label>
    weave
    <select value={session.orderSel.weave} onchange={(e) => session.selectOrder(Number(e.currentTarget.value), 0)}>
      {#each [...traces.keys()] as id}<option value={id}>{id}</option>{/each}
      {#if !traces.has(session.orderSel.weave)}<option value={session.orderSel.weave}>—</option>{/if}
    </select>
  </label>
  <span class="cellpick">
    particle
    <button onclick={() => particle(session.orderSel.particle - 1)} disabled={!all.length} aria-label="Previous particle">‹</button>
    <span class="mono">{all.length ? `${session.orderSel.particle + 1} / ${all.length}` : '—'}</span>
    <button onclick={() => particle(session.orderSel.particle + 1)} disabled={!all.length} aria-label="Next particle">›</button>
  </span>
  <button class:on={session.picking} onclick={() => (session.picking = !session.picking)}>
    {session.picking ? 'Click a particle…' : 'Pick in the world'}
  </button>
</div>

{#if !trace}
  <p class="empty">
    {#if !traces.size}
      No particle ran an order last tick. A particle runs its order every tick once it's ingrained (<code>ORDR</code>,
      then <code>INGR</code>) and its weave is set loose (<code>MANI</code>). Run a tick or two after it leaves the
      hand, then look at any of its particles here.
    {:else}
      Weave {session.orderSel.weave} didn't run its order last tick.
    {/if}
  </p>
{:else}
  <div class="facts">
    <span>tick <b>{trace.tick}</b></span>
    <span>particle <b>{trace.particle}</b></span>
    <span>from centre <b class="mono">{trace.off.map((v) => num(v, 3)).join(', ')}</b></span>
    <span>kicked <b class="mono">{trace.kick.map((v) => num(v, 3)).join(', ')}</b></span>
    <span title="Its own mana, burned by thinking">burned <b class="mono">{num(trace.burned, 4)} M</b></span>
    {#if trace.cnds !== 0}<span>condensed <b>{num(trace.cnds)}</b></span>{/if}
    <span class:bad={trace.outcome !== 'done'}>{trace.outcome}</span>
  </div>
  <div class="budget" title="An order that thinks past its budget frays the weave, and every beat burns the particle's mana">
    <span>beats <b>{trace.beats}</b> / {PHYSICS.orderBudget}</span>
    <span class="track"><span class:over={trace.beats > PHYSICS.orderBudget} style="width: {Math.min(100, (trace.beats / PHYSICS.orderBudget) * 100)}%"></span></span>
  </div>

  <div class="stepper">
    <button onclick={() => go(0)} aria-label="First">⏮</button>
    <button onclick={() => go(at - 1)} aria-label="Back">◀</button>
    <input type="range" min="0" max={trace.steps.length} value={at} oninput={(e) => go(Number(e.currentTarget.value))} aria-label="Step" />
    <button onclick={() => go(at + 1)} aria-label="Forward">▶</button>
    <button onclick={() => go(trace.steps.length)} aria-label="Last">⏭</button>
    <span class="mono count">{at} / {trace.steps.length}</span>
  </div>
  <div class="now">
    {#if at < trace.steps.length}
      <span class="mono instr">{text(trace.steps[at].addr)}</span>
      {#if line}<span class="muted">{line.file}:{line.line}</span>{/if}
    {:else}
      <span class="muted">after its last instruction:</span> <span>{trace.outcome}</span>
    {/if}
  </div>

  {#if regs}
    <div class="regs">
      {#each Array.from(regs) as v, i}
        <div class="reg" class:changed={prev !== null && v !== prev[i]} title={STARTS[i] ? `starts as ${STARTS[i]}` : ''}>
          <span class="name">n{i}</span><span class="val">{num(v, 4)}</span>
        </div>
      {/each}
    </div>
  {/if}
  <div class="wregs mono">
    {#each Array.from(trace.w) as v, i}<span class:set={v !== 0}>w{i} {num(v, 3)}</span>{/each}
  </div>

  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div class="steps" bind:this={list} tabindex="0" onkeydown={keys} role="listbox" aria-label="Instructions this cell ran">
    {#each trace.steps as s, i}
      <button class="step mono" class:on={i === at} onclick={() => go(i)} role="option" aria-selected={i === at}>
        <span class="i">{i}</span>
        <span class="addr">{s.addr.toString(16).toUpperCase().padStart(4, '0')}</span>
        <span>{text(s.addr)}</span>
        <span class="b">{beatsOf(s.addr)}</span>
      </button>
    {/each}
    <button class="step mono end" class:on={at === trace.steps.length} onclick={() => go(trace.steps.length)}>
      <span class="i">{trace.steps.length}</span><span></span><span>{trace.outcome}</span><span></span>
    </button>
  </div>
{/if}

<style>
  .pick {
    display: flex;
    flex-wrap: wrap;
    gap: 8px 14px;
    align-items: center;
    margin-bottom: 10px;
    font-size: 13px;
  }
  .pick label,
  .cellpick {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .cellpick button {
    padding: 2px 9px;
  }
  .pick button.on {
    background: var(--info-soft);
    border-color: var(--info);
    color: var(--info);
  }
  .facts {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 14px;
    font-size: 12px;
    color: var(--muted);
  }
  .facts b {
    color: var(--text-2);
    font-weight: 600;
  }
  .bad {
    color: var(--danger);
  }
  .budget {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 10px;
    align-items: center;
    font-size: 12px;
    color: var(--muted);
    margin: 6px 0 10px;
  }
  .budget b {
    color: var(--text-2);
  }
  .track {
    height: 8px;
    border-radius: 4px;
    background: var(--panel-3);
    overflow: hidden;
  }
  .track span {
    display: block;
    height: 100%;
    background: var(--info);
  }
  .track span.over {
    background: var(--danger);
  }
  .stepper {
    display: flex;
    gap: 4px;
    align-items: center;
  }
  .stepper button {
    padding: 2px 8px;
  }
  .stepper input {
    flex: 1;
    min-width: 60px;
  }
  .count {
    font-size: 12px;
    color: var(--muted);
    min-width: 60px;
    text-align: right;
  }
  .now {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: baseline;
    margin: 8px 0;
    min-height: 22px;
  }
  .instr {
    color: var(--info);
  }
  .regs {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(92px, 1fr));
    gap: 3px;
  }
  .reg {
    display: flex;
    justify-content: space-between;
    gap: 6px;
    padding: 2px 6px;
    border-radius: var(--radius-xxs);
    background: var(--panel-2);
    font-family: var(--font-mono);
    font-size: 11.5px;
  }
  .reg.changed {
    background: var(--info-soft);
    box-shadow: inset 0 0 0 1px var(--info);
  }
  .name {
    color: var(--muted);
  }
  .val {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .wregs {
    display: flex;
    flex-wrap: wrap;
    gap: 2px 10px;
    font-size: 11px;
    color: var(--muted);
    margin: 8px 0;
  }
  .wregs .set {
    color: var(--text);
  }
  .steps {
    position: relative;
    max-height: 260px;
    overflow: auto;
    border: 1px solid var(--line);
    border-radius: var(--radius-xs);
    padding: 3px;
  }
  .step {
    display: grid;
    grid-template-columns: 30px 44px 1fr 24px;
    gap: 6px;
    width: 100%;
    text-align: left;
    border: none;
    background: none;
    color: var(--text);
    font-size: 11.5px;
    padding: 1px 4px;
    min-height: 0;
    border-radius: var(--radius-xxs);
  }
  .step:hover {
    background: var(--panel-2);
  }
  .step.on {
    background: var(--info-soft);
    box-shadow: inset 3px 0 0 var(--info);
  }
  .i,
  .addr,
  .b {
    color: var(--muted);
  }
  .b {
    text-align: right;
  }
  .end {
    color: var(--muted);
  }
</style>
