<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { ask, message, open } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";
  import { formatCount, formatDistance } from "../../lib/format";
  import { t } from "../../lib/i18n";
  import { listen } from "@tauri-apps/api/event";
  import type { HeatmapPalette, Metric, Settings, Stats, Status } from "../../lib/types";
  import Segmented from "../ui/Segmented.svelte";
  import Heatmap from "./Heatmap.svelte";
  import Hours from "./Hours.svelte";
  import Trend from "./Trend.svelte";
  import { busiestDay, streaks, weekOverWeek } from "./insights";
  import { METRIC_LABEL, METRICS, valueOf } from "./metric";

  const REFRESH_MS = 15_000;

  let days = $state(30);
  let metric = $state<Metric>("keys");
  let data = $state<Stats | null>(null);
  let loading = $state(false);
  let error = $state("");
  /** False when the database could not be opened: counts live only in memory. */
  let storage = $state(true);
  let palette = $state<HeatmapPalette>("brand");

  const fmt = $derived((v: number) => (metric === "distance" ? formatDistance(v) : formatCount(Math.round(v))));

  const points = $derived(data?.days.map((d) => ({ date: d.date, value: valueOf(d, metric) })) ?? []);
  const rangeTotal = $derived(points.reduce((a, p) => a + p.value, 0));
  /** One value per hour of the day, for the hours chart and its table. */
  const hourValues = $derived((data?.hours ?? []).map((h) => valueOf(h, metric)));

  const tiles = $derived(
    data
      ? [
          { label: t("tileToday"), value: fmt(points.at(-1)?.value ?? 0), sub: "" },
          { label: t("tileRange"), value: fmt(rangeTotal), sub: "" },
          { label: t("tileAverage"), value: fmt(rangeTotal / Math.max(1, points.length)), sub: "" },
          { label: t("tileLifetime"), value: fmt(valueOf(data.lifetime, metric)), sub: "" },
        ]
      : [],
  );

  const change = (now: number, before: number) => {
    if (before <= 0) return "";
    const pct = Math.round(((now - before) / before) * 100);
    return `${pct >= 0 ? "+" : ""}${pct}%`;
  };

  /** What the selected range cannot say, from the history behind it. */
  const insights = $derived.by(() => {
    const today = data?.days.at(-1);
    if (!data || !today) return null;
    const week = weekOverWeek(data.history, metric, today.date);
    return {
      best: busiestDay(data.history, metric, today.date),
      streak: streaks(data.history, metric, today.date, valueOf(today, metric)),
      week,
      weekChange: change(week.thisWeek, week.lastWeek),
    };
  });

  const insightTiles = $derived(
    insights
      ? [
          {
            label: t("insightBusiestDay"),
            value: insights.best ? fmt(valueOf(insights.best, metric)) : "—",
            sub: insights.best?.date ?? "",
          },
          { label: t("insightStreak"), value: t("daysUnit", { n: insights.streak.current }), sub: "" },
          {
            label: t("insightStreakLongest"),
            value: t("daysUnit", { n: insights.streak.longest }),
            sub: "",
          },
          {
            label: t("insightWeek"),
            value: fmt(insights.week.thisWeek),
            sub: [t("vsLastWeek", { value: fmt(insights.week.lastWeek) }), insights.weekChange]
              .filter(Boolean)
              .join(" · "),
          },
        ]
      : [],
  );

  async function load() {
    loading = true;
    try {
      data = await invoke<Stats>("get_stats", { days });
      error = "";
    } catch (e) {
      // Keep whatever was on screen; say why it is not fresh.
      error = t("loadFailed", { error: String(e) });
    } finally {
      loading = false;
    }
  }

  async function exportCsv() {
    const dir = await open({ directory: true });
    if (typeof dir !== "string") return;
    try {
      await invoke("export_csv", { dir });
      await message(t("exported", { dir }));
    } catch (e) {
      await message(String(e), { kind: "error" });
    }
  }

  async function clearData() {
    if (!(await ask(t("clearConfirm"), { kind: "warning" }))) return;
    try {
      await invoke("clear_data");
    } catch (e) {
      await message(t("clearFailed", { error: String(e) }), { kind: "error" });
      return;
    }
    await load();
    await message(t("cleared"));
  }

  $effect(() => {
    void days;
    load();
  });

  onMount(() => {
    invoke<Status>("get_status").then((s) => (storage = s.storage), () => {});
    invoke<Settings>("get_settings").then((s) => (palette = s.heatmapPalette), () => {});
    const un = listen<Settings>("settings://changed", (e) => (palette = e.payload.heatmapPalette));
    const id = setInterval(load, REFRESH_MS);
    return () => {
      clearInterval(id);
      un.then((f) => f());
    };
  });
</script>

<main>
  <div class="filters">
    <Segmented
      label={t("stats")}
      bind:value={days}
      options={[
        { value: 7, label: t("days7") },
        { value: 30, label: t("days30") },
        { value: 90, label: t("days90") },
        { value: 365, label: t("days365") },
      ]}
    />
    <Segmented
      label={t("value")}
      bind:value={metric}
      options={METRICS.map((m) => ({ value: m, label: t(METRIC_LABEL[m]) }))}
    />
  </div>

  {#if !storage}
    <p class="alert" role="alert">{t("storageUnavailable")}</p>
  {:else if error}
    <p class="alert" role="alert">{error}</p>
  {/if}

  <div class="body" class:loading>
    <div class="tiles">
      {#each tiles as tile (tile.label)}
        <div class="card tile">
          <div class="muted">{tile.label}</div>
          <div class="big">{tile.value}</div>
          {#if tile.sub}
            <div class="sub muted">{tile.sub}</div>
          {/if}
        </div>
      {/each}
    </div>

    {#if insightTiles.length}
      <div class="tiles">
        {#each insightTiles as tile (tile.label)}
          <div class="card tile">
            <div class="muted">{tile.label}</div>
            <div class="big">{tile.value}</div>
            {#if tile.sub}
              <div class="sub muted">{tile.sub}</div>
            {/if}
          </div>
        {/each}
      </div>
    {/if}

    <section class="card">
      <h2>{t("trendTitle", { metric: t(METRIC_LABEL[metric]) })}</h2>
      <Trend {points} format={fmt} label={t("trendTitle", { metric: t(METRIC_LABEL[metric]) })} />
      <details>
        <summary class="muted">{t("dataTable")}</summary>
        <table>
          <thead><tr><th>{t("date")}</th><th class="num">{t(METRIC_LABEL[metric])}</th></tr></thead>
          <tbody>
            {#each [...points].reverse() as p (p.date)}
              <tr><td>{p.date}</td><td class="num">{fmt(p.value)}</td></tr>
            {/each}
          </tbody>
        </table>
      </details>
    </section>

    <section class="card">
      <h2>{t("hourlyTitle")}</h2>
      <Hours counts={hourValues} format={fmt} label={t("hourlyTitle")} />
      {#if hourValues.some((v) => v > 0)}
        <details>
          <summary class="muted">{t("dataTable")}</summary>
          <table>
            <thead><tr><th>{t("hour")}</th><th class="num">{t(METRIC_LABEL[metric])}</th></tr></thead>
            <tbody>
              {#each hourValues as value, hour (hour)}
                <tr><td>{String(hour).padStart(2, "0")}:00</td><td class="num">{fmt(value)}</td></tr>
              {/each}
            </tbody>
          </table>
        </details>
      {/if}
    </section>

    <section class="card">
      <h2>{t("heatmapTitle")}</h2>
      <Heatmap counts={data?.keys ?? {}} everPressed={data?.everPressed ?? []} {palette} />
    </section>

    <div class="actions">
      <button class="btn" onclick={exportCsv}>{t("exportCsv")}</button>
      <button class="btn danger" onclick={clearData}>{t("clearData")}</button>
    </div>
  </div>
</main>

<style>
  main {
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .filters {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
  }
  .alert {
    margin: 0;
    padding: 8px 12px;
    border-radius: 8px;
    border: 1px solid var(--danger);
    color: var(--danger);
  }
  .body {
    display: flex;
    flex-direction: column;
    gap: 12px;
    transition: opacity 0.15s;
  }
  .body.loading {
    opacity: 0.85;
  }
  .tiles {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 12px;
  }
  .tile .big {
    font-size: 22px;
    font-weight: 600;
    margin-top: 2px;
  }
  .tile .sub {
    font-size: 12px;
    margin-top: 2px;
  }
  details {
    margin-top: 8px;
  }
  summary {
    cursor: pointer;
    font-size: 12px;
  }
  table {
    border-collapse: collapse;
    font-size: 12px;
    margin-top: 6px;
  }
  th {
    text-align: left;
    font-weight: 600;
    color: var(--text-secondary);
  }
  th,
  td {
    padding: 2px 16px 2px 0;
  }
  .num {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }
  @media (max-width: 640px) {
    .tiles {
      grid-template-columns: repeat(2, 1fr);
    }
  }
</style>
