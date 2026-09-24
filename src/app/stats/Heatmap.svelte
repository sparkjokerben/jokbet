<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { formatCount } from "../../lib/format";
  import { t } from "../../lib/i18n";
  import { platform } from "../../lib/platform";
  import { HEAT_BINS, heatBin } from "./chart";
  import { keyboardLayout, layoutFor, type KeyboardKind, type KeyCap } from "./keyboard";

  let { counts, everPressed = [] }: { counts: Record<string, number>; everPressed?: string[] } = $props();

  const GAP = 3;
  /** What the system says the keyboard is; only macOS says. */
  let kind = $state<KeyboardKind>("unknown");
  const board = $derived(keyboardLayout(layoutFor(platform, everPressed, kind)));
  const caps = $derived(board.caps);
  let width = $state(700);
  let hover = $state<KeyCap | null>(null);

  const unit = $derived(width / board.width);
  const max = $derived(Math.max(0, ...Object.values(counts)));
  const total = $derived(Object.values(counts).reduce((a, b) => a + b, 0));
  const top = $derived(
    Object.entries(counts)
      .filter(([, n]) => n > 0)
      .sort((a, b) => b[1] - a[1])
      .slice(0, 10),
  );

  onMount(() => {
    if (platform === "mac") invoke<KeyboardKind>("keyboard_kind").then((k) => (kind = k), () => {});
  });

  const nameOf = (code: string) => {
    const cap = caps.find((k) => k.code === code);
    return cap?.label ? `${cap.label}` : code;
  };
  const fill = (bin: number) => (bin < 0 ? "var(--surface-2)" : `var(--heat-${bin})`);
  const ink = (bin: number) => (bin < 0 ? "var(--text-muted)" : `var(--heat-ink-${bin})`);
  /** The L of an ISO Enter: the cap's box less its bottom-left notch, with
   * the notch's edges on the same gaps as the keys around it. */
  const clip = (k: KeyCap) => {
    if (!k.notch) return undefined;
    const w = k.w * unit - GAP;
    const h = k.h * unit - GAP;
    const nx = k.notch.w * unit;
    const ny = (k.h - k.notch.h) * unit - GAP;
    return `polygon(0 0, ${w}px 0, ${w}px ${h}px, ${nx}px ${h}px, ${nx}px ${ny}px, 0 ${ny}px)`;
  };
</script>

<div class="wrap">
  <div
    class="board"
    bind:clientWidth={width}
    style:height="{board.height * unit}px"
    role="img"
    aria-label={t("heatmapTitle")}
  >
    {#each caps as k (k.code)}
      {@const n = counts[k.code] ?? 0}
      {@const bin = heatBin(n, max)}
      <div
        role="presentation"
        class="cap"
        class:lift={hover?.code === k.code}
        style:left="{k.x * unit}px"
        style:top="{k.y * unit}px"
        style:width="{k.w * unit - GAP}px"
        style:height="{k.h * unit - GAP}px"
        style:background={fill(bin)}
        style:color={ink(bin)}
        style:clip-path={clip(k)}
        onpointerenter={() => (hover = k)}
        onpointerleave={() => (hover = null)}
      >
        {#if unit >= 24}{k.label}{/if}
      </div>
    {/each}
    {#if hover}
      <div
        class="tip"
        style:left="{Math.min(Math.max((hover.x + hover.w / 2) * unit, 50), width - 50)}px"
        style:top="{hover.y * unit - 6}px"
      >
        <strong>{formatCount(counts[hover.code] ?? 0)}</strong>
        <span>{nameOf(hover.code)} · {hover.code}</span>
      </div>
    {/if}
  </div>

  <div class="foot">
    <div class="legend" aria-hidden="true">
      <span class="muted">{t("heatmapLess")}</span>
      <span class="sw" style:background="var(--surface-2)"></span>
      {#each Array(HEAT_BINS) as _, i (i)}
        <span class="sw" style:background="var(--heat-{i})"></span>
      {/each}
      <span class="muted">{t("heatmapMore")}</span>
    </div>
  </div>

  <h3>{t("topKeys")}</h3>
  {#if top.length}
    <table>
      <tbody>
        {#each top as [code, n], i (code)}
          <tr>
            <td class="muted">{i + 1}</td>
            <td>{nameOf(code)}</td>
            <td class="num">{formatCount(n)}</td>
            <td class="num muted">{((n / total) * 100).toFixed(1)}%</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {:else}
    <p class="muted">{t("noData")}</p>
  {/if}
</div>

<style>
  .board {
    position: relative;
    width: 100%;
  }
  .cap {
    position: absolute;
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 11px;
    line-height: 1;
    overflow: hidden;
    white-space: nowrap;
    transition: filter 0.1s;
  }
  .cap.lift {
    filter: brightness(1.12);
    outline: 2px solid var(--text-primary);
    outline-offset: 1px;
    z-index: 1;
  }
  .tip {
    position: absolute;
    transform: translate(-50%, -100%);
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
    z-index: 2;
  }
  .tip span {
    color: var(--text-muted);
    font-size: 11px;
  }
  .foot {
    display: flex;
    justify-content: flex-end;
    margin-top: 10px;
  }
  .legend {
    display: flex;
    align-items: center;
    gap: 3px;
    font-size: 11px;
  }
  .legend .muted {
    margin: 0 4px;
  }
  .sw {
    width: 14px;
    height: 10px;
    border-radius: 2px;
  }
  h3 {
    margin: 12px 0 6px;
    font-size: 12px;
    font-weight: 600;
  }
  table {
    border-collapse: collapse;
    font-size: 12px;
  }
  td {
    padding: 2px 12px 2px 0;
  }
  .num {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
</style>
