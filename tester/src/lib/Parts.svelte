<script lang="ts">
  import { PART_COLORS, PART_NAMES, num } from './format.ts'
  import type { Parts } from './engine.ts'

  /** A bar of the four parts, `scale` M wide. */
  let { parts, scale }: { parts: Parts; scale: number } = $props()
  const title = $derived(parts.map((v, k) => `${PART_NAMES[k]} ${num(v)}`).join(', '))
</script>

<span class="bar" {title}>
  {#each parts as v, k}
    {#if v > 0}
      <span style="width: {Math.min(100, (v / Math.max(scale, 1e-9)) * 100)}%; background: {PART_COLORS[k]}"></span>
    {/if}
  {/each}
</span>

<style>
  .bar {
    display: flex;
    height: 8px;
    width: 100%;
    min-width: 40px;
    border-radius: 4px;
    overflow: hidden;
    background: var(--panel-3);
  }
  .bar span {
    height: 100%;
  }
</style>
