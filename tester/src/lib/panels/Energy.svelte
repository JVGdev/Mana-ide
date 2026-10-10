<script lang="ts">
  import { session } from '../session.svelte.ts'
  import { PHYSICS } from '../engine.ts'

  const HELD: Record<string, string> = {
    motion: 'motion',
    height: 'height',
    gas: 'mana pressed',
    strain: 'matter stretched',
    cohesion: 'held together',
    air: 'air pressed',
    bodies: 'mana in bodies',
  }

  /** Joules, readably. */
  const j = (e: number) => {
    const v = e * PHYSICS.joules
    const a = Math.abs(v)
    return a >= 1e6 ? `${(v / 1e6).toFixed(2)} MJ` : a >= 1e3 ? `${(v / 1e3).toFixed(2)} kJ` : `${v.toFixed(a >= 10 ? 0 : 1)} J`
  }

  const view = $derived.by(() => {
    void session.version
    const m = session.machine
    if (!m || !m.trackEnergy) return null
    const e = m.energy()
    if (!e) return null
    const heat = e.heatBy.filter(([, v]) => Math.abs(v) > 1e-9).sort((a, b) => b[1] - a[1])
    const errors = e.error.filter(([, v]) => Math.abs(v) > 1e-9)
    const moved = e.heat + Math.abs(e.outside)
    return { e, heat, errors, share: moved > 0 ? Math.abs(e.errorTotal) / moved : 0 }
  })
</script>

<label class="keep">
  <input type="checkbox" checked={session.keepEnergy} onchange={(e) => session.setKeepEnergy(e.currentTarget.checked)} />
  Keep the Energy ledger <span class="muted">(from the next cast or reset; it costs time)</span>
</label>

{#if view}
  <div class="facts">
    <span>put in by casters and orders <b>{j(view.e.outside)}</b>: minds transformed <b>{j(view.e.minds)}</b>, bodies <b>{j(view.e.bodies)}</b></span>
    <span>turned to heat <b>{j(view.e.heat)}</b></span>
    <span>gone past the world's edge <b>{j(view.e.beyond)}</b></span>
    <span>held now, beyond the start <b>{j(view.e.total - view.e.start)}</b></span>
    <span class:bad={view.share > 0.05}>off by <b>{j(view.e.errorTotal)}</b> ({(view.share * 100).toFixed(1)}%)</span>
  </div>

  <h4>Held</h4>
  <table>
    <tbody>
      {#each Object.entries(view.e.held) as [k, v]}
        <tr><td>{HELD[k] ?? k}</td><td class="n">{j(v)}</td></tr>
      {/each}
    </tbody>
  </table>

  <h4>Turned to heat, by</h4>
  <table>
    <tbody>
      {#each view.heat as [k, v]}
        <tr><td>{k}</td><td class="n">{j(v)}</td></tr>
      {:else}
        <tr><td class="muted">nothing yet</td><td></td></tr>
      {/each}
    </tbody>
  </table>

  {#if view.errors.length}
    <h4>What the numbers got wrong, by step</h4>
    <table>
      <tbody>
        {#each view.errors as [k, v]}
          <tr><td>{k}</td><td class="n">{j(v)}</td></tr>
        {/each}
      </tbody>
    </table>
  {/if}
  <p class="muted note">
    Motion turns to heat wherever two things even out their speeds: in the mana's thickness, the air dragging, landing on
    the ground, rock keeping its shape. Pushes and kicks put energy in, at what they add (D29). Each step that should
    keep energy is measured, and what it got wrong is shown above, not hidden in the heat.
  </p>
{:else}
  <p class="muted">The ledger isn't being kept.</p>
{/if}

<style>
  .keep {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    margin-bottom: 8px;
  }
  .facts {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 14px;
    margin: 6px 0 10px;
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
  h4 {
    margin: 10px 0 4px;
    font-size: 12px;
    color: var(--text-2);
    font-weight: 600;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 12px;
  }
  td {
    padding: 2px 0;
    border-bottom: 1px solid var(--line);
  }
  td.n {
    text-align: right;
    font-family: var(--font-mono);
  }
  .note {
    font-size: 12px;
    margin-top: 10px;
  }
</style>
