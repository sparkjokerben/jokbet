<script lang="ts">
  import { t } from "../../lib/i18n";
  import { columnPath, niceMax } from "./chart";

  let {
    counts,
    format,
    label,
  }: { counts: number[]; format: (v: number) => string; label: string } = $props();

  const H = 200;
  const M = { top: 10, right: 8, bottom: 22, left: 48 };
  /** Every third hour is labelled; twenty-four of them would not fit. */
  const LABEL_EVERY = 3;
  const HOURS = Array.from({ length: 24 }, (_, hour) => hour);

  let width = $state(600);
  let hover = $state<number | null>(null);

  /** Always twenty-four of them, whatever the payload holds. */
  const values = $derived(HOURS.map((hour) => counts[hour] ?? 0));

  const innerW = $derived(Math.max(1, width - M.left - M.right));
  const innerH = H - M.top - M.bottom;
  /** The tallest hour, before `niceMax` rounds it up to a whole axis. */
  const most = $derived(Math.max(0, ...values));
  const max = $derived(niceMax(most));
  const band = $derived(innerW / HOURS.length);
  const xAt = (i: number) => M.left + band * (i + 0.5);
  const yAt = (v: number) => M.top + innerH - (v / max) * innerH;

  const barW = $derived(Math.max(1, Math.min(24, band - 2)));
  const ticks = $derived([0, max / 2, max]);
  /** The busiest hour, or null when nothing has been counted at all. */
  const peak = $derived(most > 0 ? values.indexOf(most) : null);

  const clock = (hour: number) => `${String(hour).padStart(2, "0")}:00`;

  function onMove(e: PointerEvent) {
    const rect = (e.currentTarget as SVGElement).getBoundingClientRect();
    const i = Math.floor((e.clientX - rect.left - M.left) / band);
    hover = i >= 0 && i < HOURS.length ? i : null;
  }
</script>

{#if peak === null}
  <div class="empty">
    <p class="muted">{t("noData")}</p>
    <p class="muted note">{t("hourlyHistory")}</p>
  </div>
{:else}
  <div class="hours" bind:clientWidth={width}>
    <svg {width} height={H} role="img" aria-label={label}>
      {#each ticks as tick (tick)}
        <line class="grid" x1={M.left} x2={width - M.right} y1={yAt(tick)} y2={yAt(tick)} />
        <text class="tick" x={M.left - 8} y={yAt(tick)} dy="0.32em" text-anchor="end">{format(tick)}</text>
      {/each}
      {#each HOURS as hour (hour)}
        {#if hour % LABEL_EVERY === 0}
          <text class="tick" x={xAt(hour)} y={H - 6} text-anchor="middle">{clock(hour)}</text>
        {/if}
        <path
          class="bar"
          class:dim={hover !== null && hover !== hour}
          d={columnPath(xAt(hour) - barW / 2, yAt(values[hour]), barW, yAt(0) - yAt(values[hour]))}
        />
      {/each}

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
        style:top="{Math.max(yAt(values[hover]) - 44, 0)}px"
      >
        <strong>{format(values[hover])}</strong>
        <span>{clock(hover)}</span>
      </div>
    {/if}
  </div>
  <!-- Said in words rather than by colouring the bar: a bar that changes hue
       with the range or the metric would read as a different kind of thing. -->
  <p class="muted peak">{t("hourlyPeak", { hour: clock(peak) })}</p>
{/if}

<style>
  .hours {
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
  .peak {
    margin: 6px 0 0;
    font-size: 12px;
  }
  .empty p {
    margin: 0;
  }
  .empty .note {
    margin-top: 4px;
    font-size: 12px;
  }
</style>
