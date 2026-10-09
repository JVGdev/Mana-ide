<script lang="ts">
  import { session, PRESETS } from '../session.svelte.ts'
  import { num, PART_NAMES } from '../format.ts'
  import type { CasterView, Stat } from '../engine.ts'

  type Row = { label: string; stat: Stat; effective: () => number; hint: string; step?: number }

  /** The caster as they are now: what each stat comes to. */
  const caster = $derived.by((): CasterView | null => {
    void session.version
    return session.machine?.casterView() ?? null
  })
  const c = () => caster
  const body = $derived(session.stats.body)
  const mind = $derived(session.stats.mind)

  const bodyRows: Row[] = $derived([
    { label: 'capacity', stat: body.capacity, effective: () => c()?.capacity ?? 0, hint: 'M the body holds before overcharge' },
    { label: 'baseline', stat: body.baseline, effective: () => c()?.baseline ?? 0, hint: 'its natural flow, M' },
    { label: 'drain', stat: body.drain, effective: () => c()?.drain ?? 0, hint: 'M a tick the flow drains toward baseline' },
    { label: 'focus', stat: body.focus, effective: () => c()?.focus ?? 0, hint: 'ticks a CIRC holds' },
    { label: 'streams', stat: body.streams, effective: () => c()?.streams ?? 0, hint: 'mana registers it can work, m0–m7' },
    { label: 'reach', stat: body.reach, effective: () => c()?.reach ?? 0, hint: 'metres from the body it can push mana' },
    ...PART_NAMES.map((name, k) => ({
      label: name,
      stat: body.affinity[k],
      effective: () => c()?.affinity[k] ?? 0,
      hint: `affinity: how much ${name} survives a FILT, 0–1`,
      step: 0.05,
    })),
  ])
  const mindRows: Row[] = $derived([
    { label: 'speed', stat: mind.speed, effective: () => c()?.speed ?? 0, hint: 'beats of thought a tick' },
    { label: 'registers', stat: mind.registers, effective: () => c()?.registers ?? 0, hint: 'number registers, n0–n31' },
    { label: 'memory', stat: mind.memory, effective: () => c()?.memory ?? 0, hint: 'numbers the memory holds' },
    { label: 'power', stat: mind.power, effective: () => c()?.power ?? 0, hint: 'Energy it can transform out of mana a tick (×900 J)', step: 0.05 },
    { label: 'capacity', stat: mind.capacity, effective: () => c()?.mindCapacity ?? 0, hint: 'strain it bears before transforming harms it (×900 J)' },
    { label: 'recovery', stat: mind.recovery, effective: () => c()?.recovery ?? 0, hint: 'strain it eases a tick (×900 J)', step: 0.005 },
  ])

  const effective = (r: Row) => {
    void session.version
    return num(r.effective(), 2)
  }
  const changed = () => session.applyCaster()
</script>

<section>
  <h4>who's casting</h4>
  <div class="presets">
    {#each Object.keys(PRESETS) as p}
      <button class:on={session.preset === p} onclick={() => session.setPreset(p as keyof typeof PRESETS)}>{p}</button>
    {/each}
    {#if session.preset === 'custom'}<span class="muted">custom</span>{/if}
  </div>
</section>

<section>
  <h4>will <span class="muted">— live: a spell reads it while it runs</span></h4>
  <div class="will">
    <label>amount <input type="number" min="0" step="10" bind:value={session.will.amount} oninput={() => session.syncWill()} /></label>
    <label>force <input type="number" min="0" step="0.1" bind:value={session.will.force} oninput={() => session.syncWill()} /></label>
    <label class="check"><input type="checkbox" bind:checked={session.will.maintain} onchange={() => session.syncWill()} /> maintain</label>
    <span class="muted mono">aim {session.will.aim.map((v) => num(v, 2)).join(', ')}</span>
  </div>
  <label class="practice">
    cast before
    <input type="number" min="0" step="1" bind:value={session.practice} />
    <span class="muted">times — each time, it thinks a tick's worth faster (the Law of Conditioning). Applies on the next cast.</span>
  </label>
</section>

<section>
  <h4>condition <span class="muted">— how they are right now</span></h4>
  <div class="will">
    <label>body <input type="range" min="0.2" max="1" step="0.05" bind:value={session.condition.body} oninput={changed} /> {num(session.condition.body, 2)}</label>
    <label>mind <input type="range" min="0.2" max="1" step="0.05" bind:value={session.condition.mind} oninput={changed} /> {num(session.condition.mind, 2)}</label>
  </div>
</section>

{#each [{ title: 'body', rows: bodyRows }, { title: 'mind', rows: mindRows }] as group}
  <section>
    <h4>{group.title} <span class="muted">— genetics × condition + training</span></h4>
    <table>
      <thead><tr><th></th><th>genetics</th><th>training</th><th>now</th></tr></thead>
      <tbody>
        {#each group.rows as r}
          <tr title={r.hint}>
            <td>{r.label}</td>
            <td><input type="number" step={r.step ?? 1} bind:value={r.stat.genetics} oninput={changed} /></td>
            <td><input type="number" step={r.step ?? 1} bind:value={r.stat.training} oninput={changed} /></td>
            <td class="mono now">{effective(r)}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </section>
{/each}

<style>
  section {
    margin-bottom: 14px;
  }
  h4 {
    margin: 0 0 6px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--muted);
  }
  h4 .muted {
    text-transform: none;
    letter-spacing: 0;
    font-weight: 400;
  }
  .presets {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .presets button.on {
    background: var(--accent);
    color: var(--accent-ink);
    border-color: var(--accent);
  }
  .will {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 16px;
    align-items: center;
    font-size: 13px;
  }
  .will label {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .will input[type='number'] {
    width: 80px;
  }
  .practice {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
    font-size: 13px;
    margin-top: 8px;
  }
  .practice input {
    width: 60px;
  }
  .practice .muted {
    font-size: 12px;
  }
  table {
    width: 100%;
    border-collapse: collapse;
  }
  th {
    font-size: 11px;
    font-weight: 500;
    color: var(--muted);
    text-align: left;
    padding: 0 4px 3px;
  }
  td {
    padding: 2px 4px;
    font-size: 13px;
  }
  td input {
    width: 100%;
    max-width: 96px;
  }
  .now {
    color: var(--accent-2);
  }
</style>
