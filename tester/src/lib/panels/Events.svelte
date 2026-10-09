<script lang="ts">
  import { session } from '../session.svelte.ts'
  import { num } from '../format.ts'

  const view = $derived.by(() => {
    void session.version
    const sim = session.sim
    if (!sim) return null
    const l = sim.ledger()
    return {
      events: sim.events.slice(-200).toReversed(),
      ledger: l,
      drift: l.total - session.ledgerAtCast,
    }
  })
</script>

{#if view}
  <div class="ledger">
    <div><span>free</span><b>{num(view.ledger.free)}</b></div>
    <div><span>condensed</span><b>{num(view.ledger.condensed)}</b></div>
    <div><span>total</span><b>{num(view.ledger.total)}</b></div>
    <div class:bad={Math.abs(view.drift) > 1e-3}><span>made or lost</span><b>{num(view.drift, 6)}</b></div>
  </div>
  <p class="note">The Law of Conservation: the total never changes.</p>
  {#if view.events.length}
    <table>
      <tbody>
        {#each view.events as e}
          <tr class={e.kind}>
            <td class="mono tick">{e.tick}</td>
            <td class="kind">{e.kind}</td>
            <td>{[e.weave ? `weave ${e.weave}` : '', e.detail ?? ''].filter(Boolean).join(' · ')}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {:else}
    <p class="empty">Nothing has happened yet.</p>
  {/if}
{/if}

<style>
  .ledger {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 6px;
  }
  .ledger div {
    display: flex;
    flex-direction: column;
    padding: 6px 8px;
    border-radius: var(--radius-xs);
    background: var(--panel-2);
  }
  .ledger span {
    font-size: 11px;
    color: var(--muted);
  }
  .ledger b {
    font-family: var(--font-mono);
    font-size: 12.5px;
    font-weight: 500;
  }
  .ledger .bad b {
    color: var(--danger);
  }
  .note {
    font-size: 12px;
    color: var(--muted);
    margin: 6px 0 10px;
  }
  table {
    width: 100%;
    border-collapse: collapse;
  }
  td {
    padding: 2px 5px;
    font-size: 12px;
    vertical-align: top;
  }
  .tick {
    color: var(--muted);
    width: 36px;
    text-align: right;
  }
  .kind {
    width: 84px;
    color: var(--text-2);
  }
  tr.fault .kind,
  tr.fail .kind,
  tr.fray .kind,
  tr.overcharge .kind {
    color: var(--danger);
  }
  tr.halt .kind {
    color: var(--ok);
  }
  tr.manifest .kind,
  tr.lock .kind {
    color: var(--info);
  }
</style>
