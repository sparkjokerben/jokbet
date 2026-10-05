<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
  import { message, open } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";
  import { formatWhen } from "../../lib/format";
  import { t, type MessageKey } from "../../lib/i18n";
  import {
    BACKUP_KEEP_MAX,
    GLASS_TINT_MAX,
    PET_SCALE_MAX,
    PET_SCALE_MIN,
    REST_AFTER_MIN,
    REST_GAP_MIN,
    type ActionAnim,
    type BackupList,
    type IdleAnim,
    type HeatmapPalette,
    type Language,
    type Settings,
    type Shortcuts,
  } from "../../lib/types";
  import { platform } from "../../lib/platform";
  import Segmented from "../ui/Segmented.svelte";
  import ShortcutInput from "../ui/ShortcutInput.svelte";
  import Toggle from "../ui/Toggle.svelte";
  import About from "./About.svelte";
  import MilestoneEditor from "./MilestoneEditor.svelte";
  import RestoreDialog from "./RestoreDialog.svelte";

  let s = $state<Settings | null>(null);
  let error = $state("");
  let backups = $state<BackupList | null>(null);
  let restoring = $state(false);
  /** This computer's newest daily backup, for the line under the switch. */
  const lastAuto = $derived(backups?.items.find((b) => b.kind === "auto" && !b.otherDevice) ?? null);
  let autostart = $state<boolean | null>(null);
  /** Which system material is available: "none" hides the switch. */
  let glass = $state("none");
  /** What to call it, and what to say about it: each material its own. */
  const GLASS_MATERIALS: Record<string, { label: MessageKey; body: MessageKey }> = {
    liquidGlass: { label: "liquidGlass", body: "liquidGlassBody" },
    vibrancy: { label: "liquidGlass", body: "liquidGlassBody" },
    acrylic: { label: "glassAcrylic", body: "glassAcrylicBody" },
  };
  const material = $derived(GLASS_MATERIALS[glass]);
  const isMac = platform === "mac";

  const IDLE_OPTIONS: { value: IdleAnim; label: string }[] = [
    { value: "soccer", label: t("animSoccer") },
    { value: "lookAround", label: t("animLookAround") },
    { value: "walk", label: t("animWalk") },
  ];

  /** Ticks or unticks one idle animation, keeping the list in the order shown. */
  function setIdle(anim: IdleAnim, on: boolean) {
    const now = new Set(s!.idleAnims);
    if (on) now.add(anim);
    else now.delete(anim);
    update({ idleAnims: IDLE_OPTIONS.map((o) => o.value).filter((v) => now.has(v)) });
  }

  /** A whole number of minutes within a range, or nothing if that is not what was typed. */
  const minutes = (value: string, [min, max]: readonly [number, number]) => {
    const n = Math.round(Number(value));
    return Number.isFinite(n) ? Math.min(max, Math.max(min, n)) : null;
  };
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

  /** The modifiers a shortcut needs, as this platform names them. */
  const SHORTCUT_MODS = platform === "mac" ? "⌘ ⌃ ⌥" : platform === "windows" ? "Ctrl Alt Win" : "Ctrl Alt Super";

  async function setShortcut(action: keyof Shortcuts, accelerator: string | null) {
    try {
      s = await invoke<Settings>("update_settings", { patch: { shortcuts: { [action]: accelerator } } });
      error = "";
    } catch (e) {
      const why = String(e);
      error = why.startsWith("shortcut-taken")
        ? t("shortcutTaken")
        : why.startsWith("shortcut-duplicate")
          ? t("shortcutDuplicate")
          : t("saveFailed", { error: why });
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

  function loadBackups() {
    invoke<BackupList>("list_backups").then((v) => (backups = v), () => {});
  }

  async function backupNow() {
    try {
      const path = await invoke<string | null>("backup_create");
      if (!path) return;
      await message(t("backupDone", { path }));
    } catch (e) {
      await message(t("backupFailed", { error: String(e) }), { kind: "error" });
    }
  }

  async function chooseBackupDir() {
    const dir = await open({ directory: true, defaultPath: backups?.dir });
    if (typeof dir === "string") await update({ autoBackup: { dir } });
  }

  onMount(() => {
    invoke<Settings>("get_settings").then(
      (v) => (s = v),
      (e) => (error = t("loadFailed", { error: String(e) })),
    );
    invoke<string>("glass_support").then((v) => (glass = v));
    isEnabled().then((v) => (autostart = v), () => {});
    loadBackups();
    const un = listen<Settings>("settings://changed", (e) => {
      s = e.payload;
      loadBackups();
    });
    const unBackups = listen("app://backups-changed", loadBackups);
    return () => {
      un.then((f) => f());
      unBackups.then((f) => f());
    };
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
      <h2>{t("sectionRest")}</h2>
      <Toggle
        label={t("restEnabled")}
        checked={s.restReminder.enabled}
        onchange={(v) => update({ restReminder: { enabled: v } })}
      />
      <label class="field" class:off={!s.restReminder.enabled}>
        <span>{t("restAfter")}</span>
        <input
          type="number"
          min={REST_AFTER_MIN[0]}
          max={REST_AFTER_MIN[1]}
          value={s.restReminder.afterMin}
          onchange={(e) => {
            const n = minutes(e.currentTarget.value, REST_AFTER_MIN);
            if (n !== null) update({ restReminder: { afterMin: n } });
          }}
        />
      </label>
      <label class="field" class:off={!s.restReminder.enabled}>
        <span>{t("restGap")}</span>
        <input
          type="number"
          min={REST_GAP_MIN[0]}
          max={REST_GAP_MIN[1]}
          value={s.restReminder.gapMin}
          onchange={(e) => {
            const n = minutes(e.currentTarget.value, REST_GAP_MIN);
            if (n !== null) update({ restReminder: { gapMin: n } });
          }}
        />
      </label>
      <p class="muted small">{t("restHint")}</p>
    </section>

    <section class="card">
      <h2>{t("sectionAnims")}</h2>
      <div class="field">
        <span>{t("idleAnim")}</span>
        <div class="checks" role="group" aria-label={t("idleAnim")}>
          {#each IDLE_OPTIONS as option (option.value)}
            <label>
              <input
                type="checkbox"
                checked={s.idleAnims.includes(option.value)}
                onchange={(e) => setIdle(option.value, e.currentTarget.checked)}
              />
              {option.label}
            </label>
          {/each}
        </div>
      </div>
      <p class="muted small">{t("idleAnimHint")}</p>
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
      <Toggle
        label={t("hideInFullscreen")}
        checked={s.hideInFullscreen}
        onchange={(v) => update({ hideInFullscreen: v })}
      />
      <Toggle label={t("bubbleEnabled")} checked={s.bubble} onchange={(v) => update({ bubble: v })} />
      {#if material}
        <!-- Only where the system has a material to put behind the bubble. -->
        <Toggle
          label={t(material.label)}
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
        <p class="muted small">{t(material.body)}</p>
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
      <h2>{t("sectionStats")}</h2>
      <div class="field">
        <span>{t("heatmapPalette")}</span>
        <Segmented
          label={t("heatmapPalette")}
          bind:value={() => s!.heatmapPalette, (v: HeatmapPalette) => update({ heatmapPalette: v })}
          options={[
            { value: "brand", label: t("paletteBrand") },
            { value: "heat", label: t("paletteHeat") },
          ]}
        />
      </div>
    </section>

    <section class="card">
      <h2>{t("sectionShortcuts")}</h2>
      <div class="field">
        <span>{t("shortcutTogglePet")}</span>
        <ShortcutInput
          label={t("shortcutTogglePet")}
          value={s.shortcuts.togglePet}
          onchange={(v) => setShortcut("togglePet", v)}
        />
      </div>
      <div class="field">
        <span>{t("shortcutPause")}</span>
        <ShortcutInput
          label={t("shortcutPause")}
          value={s.shortcuts.pause}
          onchange={(v) => setShortcut("pause", v)}
        />
      </div>
      <p class="muted small">{t("shortcutHint", { mods: SHORTCUT_MODS })}</p>
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

    <section class="card">
      <h2>{t("sectionData")}</h2>
      <Toggle
        label={t("autoBackup")}
        checked={s.autoBackup.enabled}
        onchange={(v) => update({ autoBackup: { enabled: v } })}
      />
      <div class="field" class:off={!s.autoBackup.enabled}>
        <span>{t("backupDir")}</span>
        <span class="dir">
          <span class="path" title={backups?.dir}>{s.autoBackup.dir ?? t("backupDirDefault")}</span>
          <button class="btn" onclick={chooseBackupDir}>{t("chooseDir")}</button>
          {#if s.autoBackup.dir}
            <button class="btn" onclick={() => update({ autoBackup: { dir: null } })}>{t("useDefaultDir")}</button>
          {/if}
        </span>
      </div>
      <label class="field" class:off={!s.autoBackup.enabled}>
        <span>{t("backupKeep")}</span>
        <input
          type="number"
          min="1"
          max={BACKUP_KEEP_MAX}
          value={s.autoBackup.keep}
          onchange={(e) => update({ autoBackup: { keep: Math.round(Number(e.currentTarget.value)) } })}
        />
      </label>
      {#if s.autoBackup.enabled && backups}
        {#if backups.lastError}
          <p class="error small">{t("backupAutoFailed", { error: backups.lastError })}</p>
        {:else}
          <p class="muted small">
            {lastAuto ? t("backupLast", { when: formatWhen(lastAuto.createdAt) }) : t("backupNever")}
          </p>
        {/if}
      {/if}
      <div class="actions">
        <button class="btn" onclick={backupNow}>{t("backupNow")}</button>
        <button class="btn" onclick={() => (restoring = true)}>{t("restoreOpen")}</button>
        <button class="btn" onclick={() => invoke("open_external", { target: "backups" })}>{t("openBackupDir")}</button>
      </div>
    </section>

    {#if restoring}
      <RestoreDialog
        onclose={() => {
          restoring = false;
          loadBackups();
        }}
      />
    {/if}

    <About />

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
  .dir {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .path {
    max-width: 180px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-secondary);
  }
</style>
