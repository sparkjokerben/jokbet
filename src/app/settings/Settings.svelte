<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
  import { onMount } from "svelte";
  import { t } from "../../lib/i18n";
  import type { PetSize, Settings } from "../../lib/types";
  import Segmented from "../ui/Segmented.svelte";
  import Toggle from "../ui/Toggle.svelte";
  import MilestoneEditor from "./MilestoneEditor.svelte";

  let s = $state<Settings | null>(null);
  let error = $state("");
  let autostart = $state<boolean | null>(null);
  const isMac = navigator.userAgent.includes("Mac");

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
    invoke<Settings>("get_settings").then((v) => (s = v));
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
      <div class="field">
        <span>{t("petSize")}</span>
        <Segmented
          label={t("petSize")}
          bind:value={
            () => s!.petSize,
            (v: PetSize) => update({ petSize: v })
          }
          options={[
            { value: "small", label: t("sizeSmall") },
            { value: "medium", label: t("sizeMedium") },
            { value: "large", label: t("sizeLarge") },
          ]}
        />
      </div>
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
      {#if autostart !== null}
        <Toggle label={t("autostart")} checked={autostart} onchange={setAutostart} />
      {/if}
      <Toggle label={t("pauseCounting")} checked={s.paused} onchange={(v) => update({ paused: v })} />
      {#if isMac}
        <button class="btn fix" onclick={() => invoke("open_panel", { view: "onboarding" })}>{t("fixPermission")}</button>
      {/if}
    </section>

    {#if error}
      <p class="error" role="alert">{error}</p>
    {/if}
  </main>
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
  .fix {
    margin-top: 6px;
  }
</style>
