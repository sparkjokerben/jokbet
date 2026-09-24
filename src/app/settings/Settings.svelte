<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
  import { onMount } from "svelte";
  import { t } from "../../lib/i18n";
  import {
    GLASS_TINT_MAX,
    PET_SCALE_MAX,
    PET_SCALE_MIN,
    type ActionAnim,
    type IdleAnim,
    type Language,
    type Settings,
  } from "../../lib/types";
  import Segmented from "../ui/Segmented.svelte";
  import Toggle from "../ui/Toggle.svelte";
  import MilestoneEditor from "./MilestoneEditor.svelte";

  let s = $state<Settings | null>(null);
  let error = $state("");
  let autostart = $state<boolean | null>(null);
  /** Which system glass is available: "none" hides the switch. */
  let glass = $state("none");
  const isMac = navigator.userAgent.includes("Mac");

  const IDLE_OPTIONS: { value: IdleAnim; label: string }[] = [
    { value: "breathe", label: t("animBreathe") },
    { value: "soccer", label: t("animSoccer") },
    { value: "lookAround", label: t("animLookAround") },
  ];
  // Each language names itself, so it can be found from either.
  const LANGUAGE_OPTIONS: { value: Language; label: string }[] = [
    { value: "system", label: t("languageSystem") },
    { value: "zh", label: "中文" },
    { value: "en", label: "English" },
  ];
  const ACTION_OPTIONS: { value: ActionAnim; label: string }[] = [
    { value: "poke", label: t("animPoke") },
    { value: "hearts", label: t("animHearts") },
    { value: "soccer", label: t("animSoccer") },
    { value: "wave", label: t("animWave") },
  ];

  async function setAutostart(on: boolean) {
    try {
      await (on ? enable() : disable());
      autostart = await isEnabled();
    } catch (e) {
      error = t("saveFailed", { error: String(e) });
    }
  }

  async function update(patch: Record<string, unknown>) {
    try {
      s = await invoke<Settings>("update_settings", { patch });
      error = "";
    } catch (e) {
      error = t("saveFailed", { error: String(e) });
    }
  }

  onMount(() => {
    invoke<Settings>("get_settings").then(
      (v) => (s = v),
      (e) => (error = t("loadFailed", { error: String(e) })),
    );
    invoke<string>("glass_support").then((v) => (glass = v));
    isEnabled().then((v) => (autostart = v), () => {});
    const un = listen<Settings>("settings://changed", (e) => (s = e.payload));
    return () => un.then((f) => f());
  });
</script>

{#if s}
  {@const head = s.headCounter}
  <main>
    <section class="card">
      <h2>{t("sectionPet")}</h2>
      <label class="field">
        <span>{t("petSize")}</span>
        <span class="size">
          <input
            type="range"
            min={PET_SCALE_MIN}
            max={PET_SCALE_MAX}
            step="0.5"
            aria-label={t("petSize")}
            title={t("sizeHint")}
            value={s.petScale}
            oninput={(e) => update({ petScale: Number(e.currentTarget.value) })}
          />
          <output>{s.petScale}×</output>
        </span>
      </label>
      <label class="field">
        <span>{t("sleepAfter")}</span>
        <input
          type="number"
          min="1"
          max="120"
          value={s.sleepAfterMin}
          onchange={(e) => update({ sleepAfterMin: Math.round(Number(e.currentTarget.value)) })}
        />
      </label>
    </section>

    <section class="card">
      <h2>{t("sectionAnims")}</h2>
      <div class="field">
        <span>{t("idleAnim")}</span>
        <Segmented
          label={t("idleAnim")}
          bind:value={() => s!.idleAnim, (v: IdleAnim) => update({ idleAnim: v })}
          options={IDLE_OPTIONS}
        />
      </div>
      <div class="field">
        <span>{t("clickAnim")}</span>
        <Segmented
          label={t("clickAnim")}
          bind:value={() => s!.clickAnim, (v: ActionAnim) => update({ clickAnim: v })}
          options={ACTION_OPTIONS}
        />
      </div>
      <div class="field">
        <span>{t("doubleClickAnim")}</span>
        <Segmented
          label={t("doubleClickAnim")}
          bind:value={() => s!.doubleClickAnim, (v: ActionAnim) => update({ doubleClickAnim: v })}
          options={ACTION_OPTIONS}
        />
      </div>
    </section>

    <section class="card">
      <h2>{t("sectionHead")}</h2>
      <Toggle
        label={t("headEnabled")}
        checked={head.enabled}
        onchange={(v) => update({ headCounter: { enabled: v } })}
      />
      <div class="field" class:off={!head.enabled}>
        <span>{t("headKind")}</span>
        <Segmented
          label={t("headKind")}
          bind:value={
            () => head.kind,
            (v: Settings["headCounter"]["kind"]) => update({ headCounter: { kind: v } })
          }
          options={[
            { value: "today", label: t("kindToday") },
            { value: "rate", label: t("kindRate") },
          ]}
        />
      </div>
      <div class="field" class:off={!head.enabled}>
        <span>{t("headSources")}</span>
        <div class="checks">
          <label>
            <input
              type="checkbox"
              checked={head.keyboard}
              disabled={head.keyboard && !head.mouse}
              onchange={(e) => update({ headCounter: { keyboard: e.currentTarget.checked } })}
            />
            {t("sourceKeyboard")}
          </label>
          <label>
            <input
              type="checkbox"
              checked={head.mouse}
              disabled={head.mouse && !head.keyboard}
              onchange={(e) => update({ headCounter: { mouse: e.currentTarget.checked } })}
            />
            {t("sourceMouse")}
          </label>
        </div>
      </div>
    </section>

    <section class="card">
      <h2>{t("sectionDisplay")}</h2>
      <Toggle label={t("bubbleEnabled")} checked={s.bubble} onchange={(v) => update({ bubble: v })} />
      {#if glass !== "none"}
        <!-- Only where the system has a material to put behind the bubble. -->
        <Toggle
          label={t("liquidGlass")}
          checked={s.liquidGlass}
          disabled={!s.bubble}
          onchange={(v) => update({ liquidGlass: v })}
        />
        {#if s.liquidGlass}
          <label class="field">
            <span>{t("glassTint")}</span>
            <span class="size">
              <input
                type="range"
                min="0"
                max={GLASS_TINT_MAX}
                step="1"
                aria-label={t("glassTint")}
                value={s.glassTint}
                oninput={(e) => update({ glassTint: Number(e.currentTarget.value) })}
              />
              <output>{s.glassTint}%</output>
            </span>
          </label>
        {/if}
        <p class="muted small">{t("liquidGlassBody")}</p>
      {/if}
      <Toggle label={t("typingSpeedEnabled")} checked={s.typingSpeed} onchange={(v) => update({ typingSpeed: v })} />
    </section>

    <section class="card">
      <h2>{t("sectionMilestones")}</h2>
      <Toggle label={t("milestonesEnabled")} checked={s.milestones} onchange={(v) => update({ milestones: v })} />
      <p class="muted small">{t("milestonesBuiltin")}</p>
      <h3>{t("customMilestones")}</h3>
      <MilestoneEditor items={s.customMilestones} onchange={(next) => update({ customMilestones: next })} />
    </section>

    <section class="card">
      <h2>{t("sectionGeneral")}</h2>
      <div class="field">
        <span>{t("language")}</span>
        <Segmented
          label={t("language")}
          bind:value={() => s!.language, (v: Language) => update({ language: v })}
          options={LANGUAGE_OPTIONS}
        />
      </div>
      {#if autostart !== null}
        <Toggle label={t("autostart")} checked={autostart} onchange={setAutostart} />
      {/if}
      <Toggle label={t("pauseCounting")} checked={s.paused} onchange={(v) => update({ paused: v })} />
      <div class="actions">
        <button class="btn" onclick={() => invoke("open_panel", { view: "tour" })}>{t("replayOnboarding")}</button>
        {#if isMac}
          <button class="btn" onclick={() => invoke("open_panel", { view: "onboarding" })}>{t("fixPermission")}</button>
        {/if}
      </div>
    </section>

    {#if error}
      <p class="error" role="alert">{error}</p>
    {/if}
  </main>
{:else if error}
  <main><p class="error" role="alert">{error}</p></main>
{/if}

<style>
  main {
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .field {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 4px 0;
  }
  .field.off {
    opacity: 0.5;
  }
  .size {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .size input[type="range"] {
    width: 132px;
    accent-color: var(--accent, #d97757);
  }
  .size output {
    width: 34px;
    text-align: right;
    font-variant-numeric: tabular-nums;
    color: var(--text-secondary);
  }
  .field input[type="number"] {
    width: 70px;
    padding: 3px 6px;
    border: 1px solid var(--line);
    border-radius: 6px;
    background: var(--surface-1);
  }
  .checks {
    display: flex;
    gap: 14px;
  }
  .checks label {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  h3 {
    margin: 10px 0 6px;
    font-size: 12px;
    font-weight: 600;
  }
  .small {
    font-size: 12px;
    margin: 4px 0 0;
  }
  .error {
    color: var(--danger);
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 6px;
  }
</style>
