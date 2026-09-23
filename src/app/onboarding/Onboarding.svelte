<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
  import { onMount } from "svelte";
  import { t } from "../../lib/i18n";
  import type { ActionAnim, IdleAnim, Settings, Status } from "../../lib/types";
  import { PetController } from "../../pet/controller";
  import { attachGestures } from "../../pet/gestures";
  import Sprite from "../../pet/Sprite.svelte";
  import type { AnimName } from "../../sprites/jokbet";

  type Step = "welcome" | "permission" | "autostart";

  /** The idle animation setting, as the sprite knows it. */
  const IDLE: Record<IdleAnim, AnimName> = { breathe: "idle", soccer: "soccer", lookAround: "lookAround" };
  /** A granted permission that still yields no events this long needs a restart. */
  const RESTART_HINT_AFTER_MS = 3000;
  /** Opened from the settings to see every step again. */
  const tour = new URLSearchParams(location.search).has("tour");

  let rows = $state<string[]>([]);
  let heroEl: HTMLDivElement;
  let settings = $state<Settings | null>(null);
  let status = $state<Status | null>(null);
  let step = $state<Step>("welcome");
  let autostart = $state(true);
  let grantedAt = $state<number | null>(null);
  let now = $state(Date.now());

  // The mascot here is the pet itself, so the steps show what it really does.
  const pet = new PetController((r) => (rows = r));
  // Bar the nap: it should not be asleep while the user reads the steps.
  pet.setSleepAfter(Number.POSITIVE_INFINITY);

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
  // On the permission step it cannot see input yet: the pet's own way of
  // saying so is crossed eyes and a question mark.
  const cannotSee = $derived(step === "permission" && !status?.listening);
  let error = $state("");

  $effect(() => pet.setBlocked(cannotSee ? "noperm" : null));

  function applyStatus(s: Status) {
    if (s.permission === "granted" && status?.permission !== "granted") grantedAt = Date.now();
    status = s;
  }

  function applySettings(s: Settings) {
    settings = s;
    pet.setIdleAnim(IDLE[s.idleAnim]);
  }

  function react(anim: ActionAnim | undefined) {
    if (anim) pet.oneShot(anim);
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
    // Same click timing as the pet: one click reacts, two clicks react twice.
    const detach = attachGestures(heroEl, {
      press: () => {},
      click: () => react(settings?.clickAnim),
      doubleClick: () => react(settings?.doubleClickAnim),
      dragStart: () => {},
      context: () => {},
    });
    invoke<Settings>("get_settings").then((s) => {
      applySettings(s);
      if (s.onboarded && !tour) step = "permission";
    });
    // A replayed tour starts from the current autostart state.
    if (tour) isEnabled().then((on) => (autostart = on), () => {});
    invoke<Status>("get_status").then(applyStatus, () => {});
    const un = listen<Status>("app://status", (e) => applyStatus(e.payload));
    const tick = setInterval(() => (now = Date.now()), 1000);
    return () => {
      detach();
      pet.destroy();
      un.then((f) => f());
      clearInterval(tick);
    };
  });
</script>

<main>
  <div class="hero" bind:this={heroEl}><Sprite {rows} scale={6} /></div>

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
    <!-- The first step has nothing to go back to, but the button keeps its
         place so the dots stay put when the steps change. -->
    <button
      class="btn back"
      class:ghost={index === 0}
      disabled={index === 0}
      onclick={() => (step = steps[index - 1])}
    >
      {t("back")}
    </button>
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
    cursor: pointer;
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
  footer .btn {
    /* One width for both ends, so the dots never shift when "next" becomes
       "done" or when a step gains the back button. */
    min-width: 84px;
  }
  .ghost {
    visibility: hidden;
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
