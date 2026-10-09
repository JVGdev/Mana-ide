<script lang="ts">
  import { session } from '../session.svelte.ts'
  import { num } from '../format.ts'
  import Parts from '../Parts.svelte'
  import { total, type Parts as P } from '../../../../src/vm/parts.ts'

  const sum = (ps: P[]): P => ps.reduce((a, p) => [a[0] + p[0], a[1] + p[1], a[2] + p[2], a[3] + p[3]], [0, 0, 0, 0] as P)

  const weaves = $derived.by(() => {
    void session.version
    const sim = session.sim
    if (!sim) return []
    return [...sim.weaves.values()].map((w) => {
      const free = sum(w.particles.map((p) => p.free))
      const carried = sum(w.particles.map((p) => p.carried))
      const ingrained = w.particles.filter((p) => p.order).length
      let order = ''
      if (w.order !== null) for (const [name, addr] of w.program.labels) if (addr === w.order) order = name
      return {
        id: w.id,
        maker: w.maker.name,
        inHand: w.inHand,
        age: w.inHand ? 0 : sim.tick - w.manifestedAt,
        origin: w.origin.map((v) => num(v, 2)).join(', '),
        particles: w.particles.length,
        ingrained,
        free,
        carried,
        locks: Object.entries(w.locks).filter(([, v]) => v).map(([k]) => k),
        order,
        // Each particle keeps its own copy: this is the newest among them.
        regs: Array.from(w.regs, (_, k) => w.reg(k)),
      }
    })
  })
</script>

{#if !weaves.length}
  <p class="empty">No weaves. <code>WEAV</code> begins one.</p>
{:else}
  {#each weaves as w (w.id)}
    <div class="weave">
      <div class="head">
        <b class="mono">weave {w.id}</b>
        <span class="tag" class:loose={!w.inHand}>{w.inHand ? 'in hand' : `loose · ${w.age} ticks`}</span>
        {#each w.locks as l}<span class="tag lock">{l}</span>{/each}
        {#if w.order}<span class="muted">order <span class="mono">{w.order}</span></span>{/if}
      </div>
      <div class="facts">
        <span>{w.particles} particles, {w.ingrained} ingrained</span>
        <span>at <span class="mono">{w.origin}</span></span>
      </div>
      <div class="line"><span>mana {num(total(w.free))}</span><Parts parts={w.free} scale={total(w.free)} /></div>
      {#if total(w.carried) > 0.01}
        <div class="line"><span>holds {num(total(w.carried))}</span><Parts parts={w.carried} scale={total(w.carried)} /></div>
      {/if}
      <div class="regs mono">
        {#each w.regs as v, i}<span class:set={v !== 0}>w{i} {num(v, 3)}</span>{/each}
      </div>
    </div>
  {/each}
{/if}

<style>
  .weave {
    padding: 8px 10px;
    border: 1px solid var(--line);
    border-radius: var(--radius-sm);
    background: var(--panel-2);
    margin-bottom: 8px;
  }
  .head {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
    font-size: 13px;
  }
  .tag {
    font-size: 11px;
    padding: 0 7px;
    border-radius: 99px;
    background: var(--panel-3);
    color: var(--text-2);
  }
  .tag.loose {
    background: var(--info-soft);
    color: var(--info);
  }
  .tag.lock {
    background: var(--special-soft);
    color: var(--special);
  }
  .facts {
    display: flex;
    gap: 12px;
    font-size: 12px;
    color: var(--muted);
    margin: 4px 0;
  }
  .line {
    display: grid;
    grid-template-columns: 90px 1fr;
    gap: 8px;
    align-items: center;
    font-size: 12px;
    color: var(--text-2);
    margin: 3px 0;
  }
  .regs {
    display: flex;
    flex-wrap: wrap;
    gap: 2px 10px;
    font-size: 11px;
    color: var(--muted);
    margin-top: 4px;
  }
  .regs .set {
    color: var(--text);
  }
</style>
