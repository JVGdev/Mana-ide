<script lang="ts">
  import { session } from '../session.svelte.ts'
  import { disassembly } from '../engine.ts'

  let { onjump }: { onjump: (file: string, line: number) => void } = $props()

  const rows = $derived.by(() => {
    const p = session.program
    if (!p) return []
    const starts = new Map<number, string[]>()
    for (const [name, addr] of p.labels) starts.set(addr, [...(starts.get(addr) ?? []), name])
    return disassembly(p).all.map((i) => ({
      addr: i.addr,
      bytes: Array.from(p.bytes.slice(i.addr, i.addr + i.size), (b) => b.toString(16).toUpperCase().padStart(2, '0')).join(' '),
      text: i.text,
      labels: starts.get(i.addr) ?? [],
      source: p.lines.get(i.addr),
    }))
  })
  const pc = $derived.by(() => {
    void session.version
    const c = session.castView
    return c?.state === 'running' ? c.pc : -1
  })
  let list = $state<HTMLDivElement>()
  $effect(() => {
    if (pc < 0 || !list) return
    list.querySelector('.here')?.scrollIntoView({ block: 'nearest' })
  })
</script>

{#if !rows.length}
  <p class="empty">Cast a spell to see the bytes it assembled to.</p>
{:else}
  <p class="sum">{session.program?.bytes.length} bytes. This is what the mind runs.</p>
  <div class="list" bind:this={list}>
    {#each rows as r}
      {#each r.labels as l}<div class="label mono">{l}:</div>{/each}
      <button class="row mono" class:here={r.addr === pc} onclick={() => r.source && onjump(r.source.file, r.source.line)}>
        <span class="addr">{r.addr.toString(16).toUpperCase().padStart(4, '0')}</span>
        <span class="bytes">{r.bytes}</span>
        <span class="text">{r.text}</span>
      </button>
    {/each}
  </div>
{/if}

<style>
  .sum {
    margin: 0 0 8px;
    font-size: 12px;
    color: var(--muted);
  }
  .list {
    font-size: 11.5px;
  }
  .label {
    color: var(--info);
    margin-top: 6px;
  }
  .row {
    display: grid;
    grid-template-columns: 44px minmax(120px, 190px) 1fr;
    gap: 8px;
    width: 100%;
    text-align: left;
    padding: 1px 4px;
    border: none;
    border-radius: var(--radius-xxs);
    background: none;
    color: var(--text);
    font-size: 11.5px;
    min-height: 0;
  }
  .row:hover {
    background: var(--panel-2);
  }
  .row.here {
    background: var(--accent-soft);
    box-shadow: inset 3px 0 0 var(--accent);
  }
  .addr {
    color: var(--muted);
  }
  .bytes {
    color: var(--text-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
