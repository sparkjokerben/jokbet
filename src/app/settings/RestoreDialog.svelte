<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { message, open } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";
  import { formatWhen } from "../../lib/format";
  import { t, type MessageKey } from "../../lib/i18n";
  import type { BackupInfo, BackupKind, BackupList, RestoreReport } from "../../lib/types";

  let { onclose }: { onclose: () => void } = $props();

  let dialog: HTMLDialogElement;
  let items = $state<BackupInfo[]>([]);
  let loaded = $state(false);
  let chosen = $state<string | null>(null);
  let withSettings = $state(false);
  let busy = $state(false);
  let error = $state("");

  const KIND: Record<BackupKind, MessageKey> = {
    auto: "restoreKindAuto",
    safety: "restoreKindSafety",
    manual: "restoreKindManual",
  };

  /** What the backend's errors mean, in words; anything else as it came. */
  function why(e: unknown): string {
    const text = String(e);
    if (text.startsWith("backup-invalid")) return t("backupInvalid");
    if (text.startsWith("backup-too-new")) return t("backupTooNew");
    return text;
  }

  async function chooseFile() {
    const path = await open({ filters: [{ name: t("backupFilter"), extensions: ["zip"] }] });
    if (typeof path !== "string") return;
    try {
      const info = await invoke<BackupInfo>("inspect_backup", { path });
      items = [info, ...items.filter((i) => i.path !== info.path)];
      chosen = info.path;
      error = "";
    } catch (e) {
      error = why(e);
    }
  }

  async function restore() {
    if (!chosen) return;
    busy = true;
    try {
      const report = await invoke<RestoreReport>("restore_backup", { path: chosen, withSettings });
      const notes = [t("restored")];
      if (report.settingsSkipped.includes("*")) notes.push(t("restoreSettingsFailed"));
      else if (report.settingsSkipped.includes("shortcuts")) notes.push(t("restoreShortcutsSkipped"));
      dialog.close();
      await message(notes.join("\n\n"));
    } catch (e) {
      error = t("restoreFailed", { error: why(e) });
    } finally {
      busy = false;
    }
  }

  onMount(() => {
    dialog.showModal();
    invoke<BackupList>("list_backups").then(
      (list) => {
        items = list.items;
        chosen = list.items[0]?.path ?? null;
        loaded = true;
      },
      (e) => {
        error = String(e);
        loaded = true;
      },
    );
  });
</script>

<dialog bind:this={dialog} onclose={onclose} aria-labelledby="restore-title">
  <h2 id="restore-title">{t("restoreTitle")}</h2>
  {#if loaded && !items.length}
    <p class="muted">{t("restoreEmpty")}</p>
  {/if}
  {#if items.length}
    <ul class="list">
      {#each items as item (item.path)}
        <li>
          <label title={item.path}>
            <input type="radio" name="backup" value={item.path} bind:group={chosen} />
            <span class="when">{formatWhen(item.createdAt)}</span>
            <span class="muted">
              {t(KIND[item.kind])} · {t("daysUnit", { n: item.days })}{#if item.otherDevice}
                · {t("restoreOtherDevice")}{/if}
            </span>
          </label>
        </li>
      {/each}
    </ul>
  {/if}
  <button class="btn" onclick={chooseFile}>{t("restoreChooseFile")}</button>

  <label class="check">
    <input type="checkbox" bind:checked={withSettings} />
    {t("restoreSettings")}
  </label>
  <p class="muted small">{t("restoreWarning")}</p>
  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}
  <div class="actions">
    <button class="btn" onclick={() => dialog.close()}>{t("cancel")}</button>
    <button class="btn primary" disabled={!chosen || busy} onclick={restore}>{t("restoreConfirm")}</button>
  </div>
</dialog>

<style>
  dialog {
    width: min(460px, calc(100vw - 32px));
    padding: 16px;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--surface-1);
    color: var(--text-primary);
  }
  dialog::backdrop {
    background: rgba(0, 0, 0, 0.3);
  }
  h2 {
    margin: 0 0 10px;
    font-size: 13px;
    font-weight: 600;
  }
  .list {
    list-style: none;
    margin: 0 0 8px;
    padding: 0;
    max-height: 220px;
    overflow-y: auto;
    border: 1px solid var(--line);
    border-radius: 6px;
  }
  .list li + li {
    border-top: 1px solid var(--line);
  }
  .list label {
    display: flex;
    align-items: baseline;
    gap: 8px;
    padding: 6px 8px;
    cursor: pointer;
  }
  .when {
    font-variant-numeric: tabular-nums;
  }
  .check {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    margin-top: 12px;
  }
  .check input {
    margin-top: 3px;
  }
  .small {
    font-size: 12px;
    margin: 6px 0 0;
  }
  .error {
    color: var(--danger);
    margin: 8px 0 0;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 14px;
  }
  .primary {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }
  .primary:hover {
    background: var(--accent-ink);
  }
  .primary:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>
