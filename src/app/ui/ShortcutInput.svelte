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
  let root: HTMLSpanElement;

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
    e.preventDefault();
    e.stopPropagation();
    if (e.code === "Escape") return stop();
    const accelerator = fromKeyPress(e);
    if (accelerator) {
      stop();
      onchange(accelerator);
    }
  }

  function onpointerdown(e: PointerEvent) {
    if (!root.contains(e.target as Node)) stop();
  }

  // Keys are taken from the whole window while recording: WebKit on macOS
  // does not focus a button that is clicked, so the button itself would hear
  // nothing. Clicking elsewhere, or leaving the window, cancels.
  $effect(() => {
    if (!recording) return;
    window.addEventListener("keydown", onkeydown, true);
    window.addEventListener("pointerdown", onpointerdown, true);
    window.addEventListener("blur", stop);
    return () => {
      window.removeEventListener("keydown", onkeydown, true);
      window.removeEventListener("pointerdown", onpointerdown, true);
      window.removeEventListener("blur", stop);
    };
  });

  onDestroy(stop);
</script>

<span class="shortcut" bind:this={root}>
  <button
    class="btn key"
    class:recording
    aria-label={label}
    aria-pressed={recording}
    onclick={() => (recording ? stop() : start())}
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
