<script lang="ts">
  import { clicks, formatCount, formatDistance, formatRate } from "../lib/format";
  import { t } from "../lib/i18n";
  import type { Tick } from "../lib/types";
  import { cardBody } from "./ink";

  let {
    tick,
    showSpeed,
    paused,
    storage = true,
    updateReady = null,
    glass = false,
    acrylic = false,
    onDark = false,
    tone = 0,
    tint = 0,
  }: {
    tick: Tick;
    showSpeed: boolean;
    paused: boolean;
    /** False when counts cannot be saved: said beside the title, first. */
    storage?: boolean;
    /** The version of a downloaded update, said beside the title. */
    updateReady?: string | null;
    glass?: boolean;
    /** Whether the material behind the card is the Windows acrylic — the one
        material that shows whatever is under the card through it, and so the
        one card drawn from a reading of that (the tone). Every other card, on a
        material that is the system's own or on none at all, goes by the
        system's own light or dark, as it always has. */
    acrylic?: boolean;
    /** The side the ink is drawn on: a card over a dark desktop. */
    onDark?: boolean;
    /** Which way the panel leans, 0 for a pale one and 1 for a dark one, read
        from behind the card — the material shows the desktop through, so this
        is what is actually under the panel. */
    tone?: number;
    tint?: number;
  } = $props();

  const d = $derived(tick.today);
  /** How much of its own colour the card carries, from the tone. */
  const body = $derived(acrylic ? cardBody(tone) : 0);
</script>

<div
  class="bubble"
  class:glass
  class:acrylic
  class:onDark
  style:--tone={String(tone)}
  style:--body={String(body)}
  style:--tint={String(tint)}
>
  <!-- Notices share the title's line: the window above the pet has room for
       the card as it is, not for more lines. -->
  <div class="title">
    <span>{paused ? t("paused") : t("today")}</span>
    {#if !storage}
      <span class="warn">{t("storageWarning")}</span>
    {:else if updateReady}
      <span class="note">{t("updateBubble", { v: updateReady })}</span>
    {/if}
  </div>
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
  /* The material is the fill; the page only tints it, by as much as the
     settings slider asks for. */
  .bubble.glass {
    background: rgba(250, 247, 242, var(--tint));
    --line: rgba(255, 255, 255, 0.34);
  }
  @media (prefers-color-scheme: dark) {
    .bubble.glass {
      background: rgba(20, 20, 19, var(--tint));
      --line: rgba(255, 255, 255, 0.14);
    }
  }
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
  /* The material has no tail to match, so the card goes without one. */
  .bubble.glass::after {
    display: none;
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
  .warn,
  .note {
    font-size: 10px;
    font-weight: 600;
  }
  .warn {
    color: #c7362f;
  }
  .note {
    color: var(--muted);
  }
  @media (prefers-color-scheme: dark) {
    .warn {
      color: #e66767;
    }
  }
  .title {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 10px;
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

  /* Everything above draws the card as it always has, from the system's own
     light or dark — which is the whole of the card where the material is the
     system's own, as on macOS, and where there is no material at all.
     Below is the acrylic: the one material that shows what is under the card
     through it, and so the one card drawn from a reading of that instead. It is
     also the only one whose words may not hold a colour of their own — the panel
     under them can be any colour at all, and a fixed grey is one it can drift
     away from. Nothing below applies to any other card. */
  .bubble.acrylic.glass,
  .bubble.acrylic.glass.onDark {
    /* As much of the panel's own colour as the desktop behind leaves room for:
       none at all where it is plainly dark or plainly pale, and a third of the
       way to its own colour where it is too mixed to read against. */
    background: color-mix(
      in oklab,
      rgb(250 247 242 / calc(var(--tint) + var(--body))),
      rgb(20 20 19 / calc(var(--tint) + var(--body))) calc(var(--tone) * 100%)
    );
  }
  .bubble.acrylic {
    --bg: #f5f0e8;
    --fg: #1f1e1d;
    /* Every word wears the same ink — the one read off the desktop behind the
       card — so the quieter words are the smaller ones rather than greyer ones. */
    --muted: var(--fg);
    --line: color-mix(
      in oklab,
      rgba(31, 30, 29, 0.14),
      rgba(245, 240, 232, 0.16) calc(var(--tone, 0) * 100%)
    );
  }
  /* For a card the material shows a dark desktop through. */
  .bubble.acrylic.onDark {
    --bg: #262624;
    --fg: #f5f0e8;
  }
  /* The ink changes hands in the middle of the band, where the two read much the
     same, and every word fades across rather than stepping: a step between them
     would be the panel flinching. */
  .bubble.acrylic,
  .bubble.acrylic .title,
  .bubble.acrylic dd,
  .bubble.acrylic dt,
  .bubble.acrylic .sub,
  .bubble.acrylic .note,
  .bubble.acrylic .warn {
    transition: color 0.2s ease;
  }
  .bubble.acrylic .warn {
    color: #c7362f;
  }
  .bubble.acrylic.onDark .warn {
    color: #e66767;
  }
</style>
