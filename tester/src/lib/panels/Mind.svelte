<script lang="ts">
  import { session } from '../session.svelte.ts'
  import { num } from '../format.ts'
  import { decode, format } from '../../../../src/asm/disassembler.ts'

  const view = $derived.by(() => {
    void session.version
    const cast = session.cast
    if (!cast) return null
    const f = cast.frame
    const names = new Map<number, string>()
    for (const [name, addr] of cast.program.labels) if (!names.has(addr) || !name.includes('.')) names.set(addr, name)
    let instr = ''
    try {
      if (cast.state === 'running') instr = format(decode(cast.program.bytes, f.pc), names)
    } catch {
      instr = '?'
    }
    const memory: [number, number][] = []
    f.memory.forEach((v, i) => v !== 0 && memory.push([i, v]))
    const perTick = cast.caster.speed * (1 + (cast.caster.conditioning.get(cast.name) ?? 0))
    return {
      state: cast.state,
      fault: cast.fault,
      code: cast.code,
      pc: f.pc,
      instr,
      line: cast.program.lines.get(f.pc),
      n: Array.from(f.n),
      limit: f.limit,
      flags: f.flags,
      stack: [...f.stack],
      calls: f.calls.map((a) => {
        let best = ''
        let at = -1
        for (const [name, addr] of cast.program.labels) if (!name.includes('.') && addr <= a && addr > at) [best, at] = [name, addr]
        return `${best}+${a - at}`
      }),
      memory,
      left: session.sim?.midTick ? cast.left : perTick,
      perTick,
      beats: cast.beats,
    }
  })
</script>

{#if !view}
  <p class="empty">Cast a spell to see the mind think.</p>
{:else}
  <div class="status">
    <span class="state {view.state}">{view.state}</span>
    {#if view.state === 'fault'}<span class="fault">{view.fault}</span>{/if}
    {#if view.state === 'failed'}<span class="fault">code {view.code}</span>{/if}
    {#if view.state === 'running'}
      <span class="mono">{view.pc.toString(16).toUpperCase().padStart(4, '0')}</span>
      <span class="mono instr">{view.instr}</span>
      {#if view.line}<span class="muted">{view.line.file}:{view.line.line}</span>{/if}
    {/if}
  </div>
  <div class="beats">
    {#if view.state === 'running'}beats this tick <b>{view.left}</b> / {view.perTick} ·{/if} spent <b>{view.beats}</b>
    · flags <b class="mono">{view.flags < 0 ? '<' : view.flags > 0 ? '>' : '='}</b>
  </div>

  <div class="regs">
    {#each view.n as v, i}
      <div
        class="reg"
        class:off={i >= view.limit}
        class:kept={i >= 16}
        class:changed={i < view.limit && v !== session.before[i]}
        title={i >= view.limit ? 'this mind has no such register' : i >= 16 ? 'kept by the caller' : 'argument / scratch'}
      >
        <span class="name">n{i}</span>
        <span class="val">{i < view.limit ? num(v, 4) : '—'}</span>
      </div>
    {/each}
  </div>

  <div class="row">
    <div>
      <h4>returns to</h4>
      {#if view.calls.length}
        {#each view.calls.toReversed() as c}<div class="mono small">{c}</div>{/each}
      {:else}<div class="muted small">none</div>{/if}
    </div>
    <div>
      <h4>stack</h4>
      {#if view.stack.length}
        {#each view.stack.toReversed() as v}<div class="mono small">{num(v, 4)}</div>{/each}
      {:else}<div class="muted small">empty</div>{/if}
    </div>
    <div>
      <h4>memory</h4>
      {#if view.memory.length}
        {#each view.memory.slice(0, 24) as [i, v]}<div class="mono small">[{i}] {num(v, 4)}</div>{/each}
      {:else}<div class="muted small">nothing stored</div>{/if}
    </div>
  </div>
{/if}

<style>
  .status {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: baseline;
    margin-bottom: 6px;
  }
  .state {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    padding: 1px 7px;
    border-radius: 99px;
    background: var(--panel-3);
  }
  .state.running {
    background: var(--accent-soft);
    color: var(--accent);
  }
  .state.halted {
    background: var(--ok-soft);
    color: var(--ok);
  }
  .state.fault,
  .state.failed {
    background: var(--danger-soft);
    color: var(--danger);
  }
  .fault {
    color: var(--danger);
    font-size: 13px;
  }
  .instr {
    color: var(--accent-2);
  }
  .beats {
    font-size: 12px;
    color: var(--muted);
    margin-bottom: 10px;
  }
  .beats b {
    color: var(--text-2);
    font-weight: 600;
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
  .reg.kept {
    background: var(--panel);
    border: 1px solid var(--line);
  }
  .reg.off {
    opacity: 0.3;
  }
  .reg.changed {
    background: var(--accent-soft);
    box-shadow: inset 0 0 0 1px var(--accent-line);
  }
  .name {
    color: var(--muted);
  }
  .val {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .row {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 10px;
    margin-top: 12px;
  }
  h4 {
    margin: 0 0 4px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--muted);
  }
  .small {
    font-size: 11.5px;
  }
</style>
