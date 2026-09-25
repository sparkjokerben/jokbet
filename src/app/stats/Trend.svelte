<script lang="ts">
  import { columnAt, columnPath, labelIndexes, niceMax } from "./chart";

  let {
    points,
    format,
    label,
  }: { points: { date: string; value: number }[]; format: (v: number) => string; label: string } = $props();

  const H = 200;
  const M = { top: 10, right: 8, bottom: 22, left: 48 };
  /** Up to a month reads best as columns; longer ranges as a line. */
  const COLUMN_LIMIT = 31;

  let width = $state(600);
  let hover = $state<number | null>(null);

  const innerW = $derived(Math.max(1, width - M.left - M.right));
  const innerH = H - M.top - M.bottom;
  const max = $derived(niceMax(Math.max(0, ...points.map((p) => p.value))));
  const band = $derived(innerW / Math.max(1, points.length));
  const columns = $derived(points.length <= COLUMN_LIMIT);
  const xAt = (i: number) => M.left + band * (i + 0.5);
  const yAt = (v: number) => M.top + innerH - (v / max) * innerH;

  const barW = $derived(Math.max(1, Math.min(24, band - 2)));
  const linePath = $derived(points.map((p, i) => `${i ? "L" : "M"}${xAt(i)} ${yAt(p.value)}`).join(""));
  const areaPath = $derived(
    points.length ? `${linePath}L${xAt(points.length - 1)} ${yAt(0)}L${xAt(0)} ${yAt(0)}Z` : "",
  );
  const ticks = $derived([0, max / 2, max]);
  const xLabels = $derived(labelIndexes(points.length));

  const shortDate = (d: string) => d.slice(5);

  function onMove(e: PointerEvent) {
    // The handler is on the hit rectangle, so its own left edge is the first
    // column: measuring from it needs no margin of its own.
    const rect = (e.currentTarget as SVGElement).getBoundingClientRect();
    hover = columnAt(e.clientX, rect.left, band, points.length);
  }
</script>

<div class="trend" bind:clientWidth={width}>
  <svg {width} height={H} role="img" aria-label={label}>
    {#each ticks as t (t)}
      <line class="grid" x1={M.left} x2={width - M.right} y1={yAt(t)} y2={yAt(t)} />
      <text class="tick" x={M.left - 8} y={yAt(t)} dy="0.32em" text-anchor="end">{format(t)}</text>
    {/each}
    {#each xLabels as i (i)}
      <text class="tick" x={xAt(i)} y={H - 6} text-anchor="middle">{shortDate(points[i].date)}</text>
    {/each}

    {#if columns}
      {#each points as p, i (p.date)}
        <path
          class="bar"
          class:dim={hover !== null && hover !== i}
          d={columnPath(xAt(i) - barW / 2, yAt(p.value), barW, yAt(0) - yAt(p.value))}
        />
      {/each}
    {:else}
      <path class="area" d={areaPath} />
      <path class="line" d={linePath} />
      {#if hover !== null}
        <line class="crosshair" x1={xAt(hover)} x2={xAt(hover)} y1={M.top} y2={yAt(0)} />
        <circle class="dot" cx={xAt(hover)} cy={yAt(points[hover].value)} r="4" />
      {/if}
    {/if}

    <rect
      role="presentation"
      class="hit"
      x={M.left}
      y={M.top}
      width={innerW}
      height={innerH}
      onpointermove={onMove}
      onpointerleave={() => (hover = null)}
    />
  </svg>
  {#if hover !== null}
    <div
      class="tip"
      style:left="{Math.min(Math.max(xAt(hover), 60), width - 60)}px"
      style:top="{Math.max(yAt(points[hover].value) - 44, 0)}px"
    >
      <strong>{format(points[hover].value)}</strong>
      <span>{points[hover].date}</span>
    </div>
  {/if}
</div>

<style>
  .trend {
    position: relative;
    width: 100%;
  }
  svg {
    display: block;
    overflow: visible;
  }
  .grid {
    stroke: var(--line);
    stroke-width: 1;
  }
  .tick {
    fill: var(--text-muted);
    font-size: 11px;
    font-variant-numeric: tabular-nums;
  }
  .bar {
    fill: var(--accent);
    transition: opacity 0.1s;
  }
  .bar.dim {
    opacity: 0.45;
  }
  .line {
    fill: none;
    stroke: var(--accent);
    stroke-width: 2;
    stroke-linejoin: round;
    stroke-linecap: round;
  }
  .area {
    fill: var(--accent);
    opacity: 0.1;
  }
  .crosshair {
    stroke: var(--text-muted);
    stroke-width: 1;
  }
  .dot {
    fill: var(--accent);
    stroke: var(--surface-1);
    stroke-width: 2;
  }
  .hit {
    fill: transparent;
  }
  .tip {
    position: absolute;
    transform: translateX(-50%);
    pointer-events: none;
    padding: 4px 8px;
    border-radius: 6px;
    background: var(--surface-1);
    border: 1px solid var(--line);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.12);
    white-space: nowrap;
    display: flex;
    flex-direction: column;
    align-items: center;
    font-size: 12px;
  }
  .tip span {
    color: var(--text-muted);
    font-size: 11px;
  }
</style>
