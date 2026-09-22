<script lang="ts">
  import { GRID_H, GRID_W } from "../sprites/clawd";
  import { compileGrid } from "../sprites/compile";

  let { rows, scale }: { rows: string[]; scale: number } = $props();

  const paths = $derived(compileGrid(rows));
</script>

<svg
  width={GRID_W * scale}
  height={GRID_H * scale}
  viewBox="0 0 {GRID_W} {GRID_H}"
  shape-rendering="crispEdges"
  aria-hidden="true"
>
  {#each paths as p (p.color)}
    <path fill={p.color} d={p.d} />
  {/each}
</svg>

<style>
  svg {
    display: block;
  }
</style>
