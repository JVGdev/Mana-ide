<script lang="ts">
  import { OPS, PORTS } from '../../../../src/asm/isa.ts'
  import { OP_DOCS, PORT_DOCS } from '../../../../src/asm/docs.ts'

  let filter = $state('')
  const GROUPS = [
    ['control', 'Mind: control'],
    ['numbers', 'Mind: numbers'],
    ['body', 'Body'],
    ['reach', 'Reach'],
    ['weave', 'Weave'],
    ['order', 'Order (only inside an order)'],
  ] as const

  const rows = $derived(
    OPS.map((op) => ({ op, doc: OP_DOCS[op.name] })).filter(
      ({ op, doc }) => !filter || `${op.name} ${doc?.doc ?? ''} ${doc?.step ?? ''}`.toLowerCase().includes(filter.toLowerCase()),
    ),
  )
</script>

<input class="filter" placeholder="Find an instruction…" bind:value={filter} />

{#each GROUPS as [key, title]}
  {@const ops = rows.filter((r) => r.doc?.group === key)}
  {#if ops.length}
    <h4>{title}</h4>
    <table>
      <tbody>
        {#each ops as { op, doc }}
          <tr>
            <td class="mono code">{op.code.toString(16).toUpperCase().padStart(2, '0')}</td>
            <td class="mono syntax">{doc.syntax}</td>
            <td class="beats" title="beats">{op.beats}</td>
            <td>{doc.doc}{#if doc.step}<span class="step">{doc.step}</span>{/if}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
{/each}

{#if !filter}
  <h4>Ports <span class="muted">— read with IN</span></h4>
  <table>
    <tbody>
      {#each PORTS as p}
        <tr>
          <td class="mono syntax">{p.name}</td>
          <td class="beats">{p.size}</td>
          <td>{PORT_DOCS[p.name]}</td>
        </tr>
      {/each}
    </tbody>
  </table>
  <h4>Notes</h4>
  <ul>
    <li><span class="mono">n0</span>–<span class="mono">n15</span> are a routine's to use; <span class="mono">n16</span>–<span class="mono">n31</span> are kept for the caller. Shapes take their mana in <span class="mono">m1</span>.</li>
    <li><span class="mono">n4:6</span> is a triple: three registers in a row, for a position or a direction.</li>
    <li>Mana is four parts, 0–3. <span class="mono">.use Elements</span> names them <span class="mono">#FIRE #WATER #AIR #EARTH</span>.</li>
    <li>An order runs in every cell of its weave, every tick, with 64 beats to spend. It starts with <span class="mono">n3</span> its particle's mana and <span class="mono">n4</span> the weave's age, and knows only what its particle feels (<span class="mono">VEL</span>, <span class="mono">DENS</span>, <span class="mono">GRAD</span>, <span class="mono">NVEL</span>, <span class="mono">TUCH</span>) and its weave's registers: not where it is.</li>
  </ul>
{/if}

<style>
  .filter {
    width: 100%;
    margin-bottom: 6px;
  }
  h4 {
    margin: 14px 0 4px;
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
  table {
    width: 100%;
    border-collapse: collapse;
  }
  td {
    padding: 3px 5px;
    font-size: 12.5px;
    vertical-align: top;
    border-bottom: 1px solid var(--line);
  }
  .code {
    color: var(--muted);
    width: 24px;
  }
  .syntax {
    color: var(--accent);
    white-space: nowrap;
  }
  .beats {
    color: var(--muted);
    text-align: center;
    width: 20px;
  }
  .step {
    margin-left: 6px;
    font-size: 10.5px;
    letter-spacing: 0.05em;
    color: var(--special);
  }
  ul {
    margin: 0;
    padding-left: 18px;
    font-size: 12.5px;
    color: var(--text-2);
  }
  li {
    margin: 3px 0;
  }
</style>
