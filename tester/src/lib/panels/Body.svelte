<script lang="ts">
  import { session } from '../session.svelte.ts'
  import { num } from '../format.ts'
  import Parts from '../Parts.svelte'
  import { total } from '../engine.ts'

  const view = $derived.by(() => {
    void session.version
    const m = session.machine
    if (!m) return null
    const c = m.casterView()
    const regs = c.regs.map((r, i) => ({
      i,
      parts: r.parts,
      amount: total(r.parts),
      hold: r.holdUntil >= c.tick ? r.holdUntil - c.tick + 1 : 0,
      stream: i < c.streams,
    }))
    return {
      held: c.held,
      capacity: c.capacity,
      flow: c.flow,
      harm: c.harm,
      strain: c.strain,
      mindCapacity: c.mindCapacity,
      madness: c.madness,
      condition: c.condition,
      regs,
      biggest: Math.max(1, ...regs.map((r) => r.amount)),
      inHand: c.inHand,
      focus: c.focus,
      drain: c.drain,
      baseline: c.baseline,
    }
  })
</script>

{#if view}
  <div class="load">
    <div class="label">
      load <b>{num(view.held)}</b> / {num(view.capacity)} M
      {#if view.held > view.capacity}<span class="over">overcharged</span>{/if}
    </div>
    <div class="meter">
      <span class="fill" class:over={view.held > view.capacity} style="width: {Math.min(100, (view.held / view.capacity) * 100)}%"></span>
    </div>
  </div>

  <div class="facts">
    <span>flow <b>{num(total(view.flow))}</b> M</span>
    <span>baseline <b>{num(view.baseline)}</b></span>
    <span>drain <b>{num(view.drain)}</b>/tick</span>
    <span>focus <b>{view.focus}</b> ticks</span>
    {#if view.inHand > 0}<span>in hand <b>{num(view.inHand)}</b> M</span>{/if}
    <span class:bad={view.harm > 0}>harm <b>{num(view.harm)}</b></span>
    <span class:bad={view.strain > view.mindCapacity}>strain <b>{num(view.strain * 900)}</b> / {num(view.mindCapacity * 900)} J</span>
    {#if view.madness > 0}<span class="bad">madness <b>{num(view.madness * 900)}</b> J</span>{/if}
    <span>condition <b>{num(view.condition.body, 2)}</b> body · <b>{num(view.condition.mind, 2)}</b> mind</span>
  </div>
  <div class="flow"><span class="muted">flow</span><Parts parts={view.flow} scale={total(view.flow)} /></div>

  <table>
    <tbody>
      {#each view.regs as r}
        <tr class:off={!r.stream}>
          <td class="mono name">m{r.i}</td>
          <td class="bar"><Parts parts={r.parts} scale={view.biggest} /></td>
          <td class="mono amt">{r.stream ? num(r.amount) : '—'}</td>
          <td class="hold">
            {#if !r.stream}<span class="muted">no stream</span>
            {:else if r.amount > 0}{#if r.hold > 0}<span class="held">held {r.hold}</span>{:else}<span class="slip">slipping</span>{/if}{/if}
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
{:else}
  <p class="empty">No caster yet.</p>
{/if}

<style>
  .load .label {
    font-size: 13px;
    color: var(--text-2);
    margin-bottom: 4px;
  }
  .meter {
    height: 10px;
    border-radius: 5px;
    background: var(--panel-3);
    overflow: hidden;
  }
  .fill {
    display: block;
    height: 100%;
    background: var(--accent);
  }
  .fill.over,
  .over {
    background: var(--danger);
  }
  span.over {
    background: none;
    color: var(--danger);
    margin-left: 6px;
  }
  .facts {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 14px;
    margin: 10px 0 8px;
    font-size: 12px;
    color: var(--muted);
  }
  .facts b {
    color: var(--text-2);
    font-weight: 600;
  }
  .bad b {
    color: var(--danger);
  }
  .flow {
    display: grid;
    grid-template-columns: 40px 1fr;
    gap: 8px;
    align-items: center;
    font-size: 12px;
    margin-bottom: 8px;
  }
  table {
    width: 100%;
    border-collapse: collapse;
  }
  td {
    padding: 3px 4px;
    font-size: 12px;
  }
  .name {
    width: 30px;
    color: var(--mana);
  }
  .bar {
    width: 55%;
  }
  .amt {
    text-align: right;
    width: 60px;
  }
  .hold {
    width: 80px;
  }
  .held {
    color: var(--ok);
  }
  .slip {
    color: var(--warn);
  }
  tr.off {
    opacity: 0.4;
  }
</style>
