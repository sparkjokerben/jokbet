<script lang="ts">
  import { t } from "../../lib/i18n";
  import type { CustomMilestone, Metric } from "../../lib/types";

  let { items, onchange }: { items: CustomMilestone[]; onchange: (next: CustomMilestone[]) => void } = $props();

  /** Mirrors Metric::min_repeat_step in settings.rs. */
  const MIN_STEP: Record<Metric, number> = { keys: 500, clicks: 500, scrolls: 200, distance: 50 };
  const METRICS: Array<[Metric, Parameters<typeof t>[0]]> = [
    ["keys", "metricKeys"],
    ["clicks", "metricClicks"],
    ["scrolls", "metricScrolls"],
    ["distance", "metricDistance"],
  ];

  const tooSmall = (m: CustomMilestone) => m.repeat && m.threshold < MIN_STEP[m.metric];

  function edit(i: number, patch: Partial<CustomMilestone>) {
    const next = items.map((m, j) => (j === i ? { ...m, ...patch } : m));
    // Only save states the backend accepts; the row shows the hint meanwhile.
    if (!next.some((m) => tooSmall(m) || !(m.threshold > 0))) onchange(next);
    else items = next;
  }

  function add() {
    onchange([
      ...items,
      { id: crypto.randomUUID(), period: "daily", metric: "keys", threshold: 10_000, repeat: false },
    ]);
  }
</script>

<div class="list">
  {#each items as m, i (m.id)}
    <div class="row">
      <select value={m.period} onchange={(e) => edit(i, { period: e.currentTarget.value as CustomMilestone["period"] })}>
        <option value="daily">{t("periodDaily")}</option>
        <option value="lifetime">{t("periodLifetime")}</option>
      </select>
      <select value={m.metric} onchange={(e) => edit(i, { metric: e.currentTarget.value as Metric })}>
        {#each METRICS as [value, key] (value)}
          <option {value}>{t(key)}</option>
        {/each}
      </select>
      <input
        type="number"
        min="1"
        aria-label={t("thresholdUnit")}
        value={m.threshold}
        onchange={(e) => edit(i, { threshold: Number(e.currentTarget.value) })}
      />
      <span class="unit muted">{m.metric === "distance" ? "m" : ""}</span>
      <label class="repeat">
        <input type="checkbox" checked={m.repeat} onchange={(e) => edit(i, { repeat: e.currentTarget.checked })} />
        {t("repeat")}
      </label>
      <button class="btn" onclick={() => onchange(items.filter((_, j) => j !== i))}>{t("remove")}</button>
      {#if tooSmall(m)}
        <div class="hint">{t("minStep", { n: MIN_STEP[m.metric] })}</div>
      {/if}
    </div>
  {/each}
  <button class="btn add" onclick={add}>+ {t("addMilestone")}</button>
</div>

<style>
  .list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }
  select,
  input[type="number"] {
    padding: 3px 6px;
    border: 1px solid var(--line);
    border-radius: 6px;
    background: var(--surface-1);
  }
  input[type="number"] {
    width: 90px;
  }
  .unit {
    width: 12px;
  }
  .repeat {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
  }
  .hint {
    flex-basis: 100%;
    color: var(--danger);
    font-size: 12px;
  }
  .add {
    align-self: flex-start;
  }
</style>
