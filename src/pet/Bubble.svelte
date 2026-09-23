<script lang="ts">
  import { clicks, formatCount, formatDistance, formatRate } from "../lib/format";
  import { t } from "../lib/i18n";
  import type { Tick } from "../lib/types";

  let { tick, showSpeed, paused }: { tick: Tick; showSpeed: boolean; paused: boolean } = $props();

  const d = $derived(tick.today);
</script>

<div class="bubble">
  <div class="title">{paused ? t("paused") : t("today")}</div>
  <dl>
    <dt>{t("keys")}</dt>
    <dd>{formatCount(d.keys)}</dd>
    <dt>{t("clicks")}</dt>
    <dd>
      {formatCount(clicks(d))}
      <span class="sub">{t("clickSplit", { l: d.clickLeft, r: d.clickRight, m: d.clickMiddle })}</span>
    </dd>
    <dt>{t("scrolls")}</dt>
    <dd>{formatCount(d.scrolls)}</dd>
    <dt>{t("distance")}</dt>
    <dd>{formatDistance(d.moveMm)}</dd>
    {#if showSpeed}
      <dt>{t("speed")}</dt>
      <dd>{t("perSecond", { n: formatRate(tick.kps) })}</dd>
    {/if}
  </dl>
</div>

<style>
  .bubble {
    --bg: #f5f0e8;
    --fg: #1f1e1d;
    --muted: #6b6a66;
    --line: rgba(31, 30, 29, 0.14);
    position: relative;
    min-width: 150px;
    padding: 8px 10px;
    border-radius: 10px;
    border: 1px solid var(--line);
    background: var(--bg);
    color: var(--fg);
    font: 12px/1.45 -apple-system, "Segoe UI", system-ui, sans-serif;
    font-variant-numeric: tabular-nums;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.18);
  }
  .bubble::after {
    content: "";
    position: absolute;
    left: 50%;
    bottom: -6px;
    width: 10px;
    height: 10px;
    background: var(--bg);
    border-right: 1px solid var(--line);
    border-bottom: 1px solid var(--line);
    transform: translateX(-50%) rotate(45deg);
  }
  @media (prefers-color-scheme: dark) {
    .bubble {
      --bg: #262624;
      --fg: #f5f0e8;
      --muted: #a8a59e;
      --line: rgba(245, 240, 232, 0.16);
    }
  }
  .title {
    font-weight: 600;
    margin-bottom: 2px;
  }
  dl {
    display: grid;
    grid-template-columns: auto 1fr;
    column-gap: 10px;
    margin: 0;
  }
  dt {
    color: var(--muted);
  }
  dd {
    margin: 0;
    text-align: right;
  }
  .sub {
    display: block;
    font-size: 10px;
    color: var(--muted);
  }
</style>
