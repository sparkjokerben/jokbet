<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onDestroy } from "svelte";
  import { t } from "../../lib/i18n";
  import { describeShortcut, fromKeyPress } from "../../lib/shortcut";

  let {
    value,
    label,
    onchange,
  }: { value: string | null; label: string; onchange: (v: string | null) => void } = $props();

  let recording = $state(false);

  // While recording, the app lets go of its shortcuts: pressing one should
  // record it, not run it.
  function start() {
    recording = true;
    void invoke("suspend_shortcuts", { suspended: true });
  }

  function stop() {
    if (!recording) return;
    recording = false;
    void invoke("suspend_shortcuts", { suspended: false });
  }

  function onkeydown(e: KeyboardEvent) {
    if (!recording) return;
    e.preventDefault();
    e.stopPropagation();
    if (e.code === "Escape") return stop();
    const accelerator = fromKeyPress(e);
    if (accelerator) {
      stop();
      onchange(accelerator);
    }
  }

  onDestroy(stop);
</script>

<span class="shortcut">
  <button
    class="btn key"
    class:recording
    aria-label={label}
    onclick={() => (recording ? stop() : start())}
    {onkeydown}
    onblur={stop}
  >
    {recording ? t("shortcutRecording") : value ? describeShortcut(value) : t("shortcutNone")}
  </button>
  {#if value && !recording}
    <button class="btn clear" aria-label={t("shortcutClear")} title={t("shortcutClear")} onclick={() => onchange(null)}>
      ×
    </button>
  {/if}
</span>

<style>
  .shortcut {
    display: flex;
    gap: 4px;
  }
  .key {
    min-width: 120px;
    font-variant-numeric: tabular-nums;
  }
  .recording {
    border-color: var(--accent);
    color: var(--accent);
  }
  .clear {
    padding: 5px 8px;
  }
</style>
