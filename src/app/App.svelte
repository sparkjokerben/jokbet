<script lang="ts">
  import { t } from "../lib/i18n";
  import Onboarding from "./onboarding/Onboarding.svelte";
  import Settings from "./settings/Settings.svelte";
  import Stats from "./stats/Stats.svelte";

  // Each on-demand window loads app.html?view=<name>.
  const view = new URLSearchParams(location.search).get("view") ?? "stats";
  document.title = view === "settings" ? t("settings") : view === "onboarding" ? "Jokbet" : t("stats");
</script>

{#if view === "settings"}
  <Settings />
{:else if view === "onboarding"}
  <Onboarding />
{:else}
  <Stats />
{/if}
