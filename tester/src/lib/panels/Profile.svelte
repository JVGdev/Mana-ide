<script lang="ts">
  import { session } from '../session.svelte.ts'

  let { onjump }: { onjump: (file: string, line: number) => void } = $props()

  const view = $derived.by(() => {
    void session.version
    const c = session.castView
    const m = session.machine
    if (!c || !m || c.beats === 0) return null
    const p = m.profile()
    if (!p) return null
    const ticks = (c.endedAt ?? m.tick) - c.startedAt + 1
    return { ...p, ticks, state: c.state }
  })
  const pct = (x: number) => `${(x * 100).toFixed(1)}%`
</script>

{#if !view}
  <p class="empty">Cast a spell, then see where its thought went. The editor's gutter shows the same, line by line.</p>
{:else}
  <p class="sum">
    <b>{view.beats}</b> beats over <b>{view.ticks}</b> tick{view.ticks === 1 ? '' : 's'}{view.state === 'running' ? ' so far' : ''}
  </p>
  <h4>routines</h4>
  {#each view.routines as r}
    <div class="routine">
      <span class="mono name">{r.name}</span>
      <span class="track"><span style="width: {r.share * 100}%"></span></span>
      <span class="mono n">{r.beats}</span>
      <span class="mono p">{pct(r.share)}</span>
    </div>
  {/each}
  <h4>costliest lines</h4>
  <table>
    <tbody>
      {#each view.lines.slice(0, 15) as l}
        <tr onclick={() => l.source && onjump(l.source.file, l.source.line)} class:link={!!l.source}>
          <td class="mono n">{l.beats}</td>
          <td class="mono p">{pct(l.share)}</td>
          <td class="mono runs">×{l.runs}</td>
          <td class="mono where">{l.source ? `${l.source.file}:${l.source.line}` : ''}</td>
          <td class="mono code">{l.source?.text.replace(/\s*;.*$/, '') ?? ''}</td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}

<style>
  .sum {
    margin: 0 0 10px;
    color: var(--text-2);
  }
  h4 {
    margin: 12px 0 6px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--muted);
  }
  .routine {
    display: grid;
    grid-template-columns: 110px 1fr 60px 52px;
    gap: 8px;
    align-items: center;
    font-size: 12px;
    margin: 3px 0;
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
    background: var(--danger);
  }
  .n,
  .p {
    text-align: right;
  }
  table {
    width: 100%;
    border-collapse: collapse;
  }
  td {
    padding: 2px 5px;
    font-size: 11.5px;
    white-space: nowrap;
  }
  .runs,
  .where {
    color: var(--muted);
  }
  .code {
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 220px;
  }
  tr.link {
    cursor: pointer;
  }
  tr.link:hover td {
    background: var(--panel-2);
  }
</style>
