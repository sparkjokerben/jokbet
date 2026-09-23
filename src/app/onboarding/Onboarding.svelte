<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
  import { onMount } from "svelte";
  import { t } from "../../lib/i18n";
  import type { Settings, Status } from "../../lib/types";
  import Sprite from "../../pet/Sprite.svelte";
  import { compose } from "../../sprites/jokbet";

  type Step = "welcome" | "permission" | "autostart";

  /** A granted permission that still yields no events this long needs a restart. */
  const RESTART_HINT_AFTER_MS = 3000;
  /** Opened from the settings to see every step again. */
  const tour = new URLSearchParams(location.search).has("tour");

  let settings = $state<Settings | null>(null);
  let status = $state<Status | null>(null);
  let step = $state<Step>("welcome");
  let autostart = $state(true);
  let grantedAt = $state<number | null>(null);
  let now = $state(Date.now());

  const needsPermission = $derived(status?.permission !== "notRequired");
  const steps = $derived<Step[]>(
    settings?.onboarded && !tour
      ? ["permission"]
      : needsPermission
        ? ["welcome", "permission", "autostart"]
        : ["welcome", "autostart"],
  );
  const index = $derived(steps.indexOf(step));
  const last = $derived(index === steps.length - 1);
  const needsRestart = $derived(
    status?.permission === "granted" && !status.listening && grantedAt !== null && now - grantedAt > RESTART_HINT_AFTER_MS,
  );
  // Always open-eyed here; a question mark while it cannot see input yet.
  const face = $derived(
    compose(step === "permission" && !status?.listening ? { fx: ["question"] } : {}),
  );
  let error = $state("");

  function applyStatus(s: Status) {
    if (s.permission === "granted" && status?.permission !== "granted") grantedAt = Date.now();
    status = s;
  }

  async function finish() {
    try {
      if (!settings?.onboarded || tour) await (autostart ? enable() : disable()).catch(() => {});
      if (!settings?.onboarded) await invoke("update_settings", { patch: { onboarded: true } });
      await getCurrentWindow().close();
    } catch (e) {
      error = String(e);
    }
  }

  onMount(() => {
    invoke<Settings>("get_settings").then((s) => {
      settings = s;
      if (s.onboarded && !tour) step = "permission";
    });
    // A replayed tour starts from the current autostart state.
    if (tour) isEnabled().then((on) => (autostart = on), () => {});
    invoke<Status>("get_status").then(applyStatus, () => {});
    const un = listen<Status>("app://status", (e) => applyStatus(e.payload));
    const tick = setInterval(() => (now = Date.now()), 1000);
    return () => {
      un.then((f) => f());
      clearInterval(tick);
    };
  });
</script>

<main>
  <div class="hero"><Sprite rows={face} scale={6} /></div>

  {#if step === "welcome"}
    <h1>{t("welcomeTitle")}</h1>
    <p>{t("welcomeBody")}</p>
    <p class="note">{t("welcomePrivacy")}</p>
  {:else if step === "permission"}
    <h1>{t("permTitle")}</h1>
    {#if status?.permission === "unsupported"}
      <p class="warn">{t("permUnsupported")}</p>
    {:else}
      <p>{t("permBody")}</p>
      <div class="perm">
        <button class="btn primary" onclick={() => invoke("open_input_monitoring_settings")}>{t("permOpen")}</button>
        {#if status?.listening}
          <span class="ok">✓ {t("permGranted")}</span>
        {:else if needsRestart}
          <span class="warn">{t("permRestart")}</span>
          <button class="btn" onclick={() => invoke("restart_app")}>{t("restart")}</button>
        {:else}
          <span class="muted">{t("permWaiting")}</span>
        {/if}
      </div>
      <p class="note">{t("permUpdateHint")}</p>
    {/if}
  {:else}
    <h1>{t("autostartTitle")}</h1>
    <p>{t("autostartBody")}</p>
    <label class="check"><input type="checkbox" bind:checked={autostart} /> {t("autostartTitle")}</label>
  {/if}

  {#if error}
    <p class="warn" role="alert">{error}</p>
  {/if}

  <footer>
    {#if index > 0}
      <button class="btn" onclick={() => (step = steps[index - 1])}>{t("back")}</button>
    {/if}
    <span class="dots" aria-hidden="true">
      {#each steps as s (s)}<span class:on={s === step}></span>{/each}
    </span>
    {#if last}
      <button class="btn primary" onclick={finish}>{t("done")}</button>
    {:else}
      <button class="btn primary" onclick={() => (step = steps[index + 1])}>{t("next")}</button>
    {/if}
  </footer>
</main>

<style>
  main {
    height: 100vh;
    padding: 24px 28px 20px;
    display: flex;
    flex-direction: column;
  }
  .hero {
    display: flex;
    justify-content: center;
    margin-bottom: 8px;
  }
  h1 {
    font-size: 18px;
    margin: 0 0 8px;
  }
  p {
    margin: 0 0 10px;
    color: var(--text-secondary);
  }
  .note {
    font-size: 12px;
    color: var(--text-muted);
  }
  .perm {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 10px;
    margin: 6px 0 14px;
  }
  .ok {
    color: #1a8a4a;
    font-weight: 600;
  }
  .warn {
    color: var(--danger);
  }
  .check {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  footer {
    margin-top: auto;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .dots {
    flex: 1;
    display: flex;
    justify-content: center;
    gap: 6px;
  }
  .dots span {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--line);
  }
  .dots span.on {
    background: var(--accent);
  }
  .primary {
    background: var(--accent-ink);
    border-color: var(--accent-ink);
    color: #fff;
  }
  .primary:hover {
    background: var(--accent-ink);
    filter: brightness(1.08);
  }
</style>
