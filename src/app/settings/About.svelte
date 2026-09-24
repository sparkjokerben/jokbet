<script lang="ts">
  import { getVersion } from "@tauri-apps/api/app";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import { lang, t } from "../../lib/i18n";
  import { notesFor, parseNotes, type Inline } from "../../lib/notes";
  import type { UpdateStatus } from "../../lib/types";

  let version = $state("");
  let update = $state<UpdateStatus>({ state: "idle" });
  let error = $state("");

  /** The notes in the reader's language, as blocks (never as HTML). */
  const notes = $derived(update.state === "ready" && update.notes ? parseNotes(notesFor(update.notes, lang)) : []);

  const statusText = $derived.by(() => {
    switch (update.state) {
      case "checking":
        return t("updateChecking");
      case "upToDate":
        return t("updateUpToDate");
      case "ready":
        return t("updateReady", { v: update.version });
      case "failed":
        return t("updateFailed", { error: update.error });
      default:
        return "";
    }
  });

  async function check() {
    update = { state: "checking" };
    update = await invoke<UpdateStatus>("check_update");
  }

  async function install() {
    try {
      await invoke("install_update");
    } catch (e) {
      error = String(e);
    }
  }

  function open(target: "site" | "github" | "logs") {
    invoke("open_external", { target }).catch((e) => (error = String(e)));
  }

  onMount(() => {
    getVersion().then((v) => (version = v), () => {});
    invoke<UpdateStatus>("update_status").then((s) => (update = s), () => {});
    // The background check reports here too.
    const un = listen<UpdateStatus>("app://update", (e) => (update = e.payload));
    return () => un.then((f) => f());
  });
</script>

{#snippet inlines(parts: Inline[])}
  {#each parts as part, i (i)}
    {#if part.bold}<strong>{part.text}</strong>{:else if part.code}<code>{part.text}</code>{:else}{part.text}{/if}
  {/each}
{/snippet}

<section class="card">
  <h2>{t("sectionAbout")}</h2>
  <div class="field">
    <span>Jokbet{version ? ` · ${t("version", { v: version })}` : ""}</span>
    {#if update.state === "ready"}
      <button class="btn primary" onclick={install}>{t("updateInstall")}</button>
    {:else}
      <button class="btn" disabled={update.state === "checking"} onclick={check}>{t("checkUpdate")}</button>
    {/if}
  </div>
  {#if statusText}
    <p class="small" class:muted={update.state !== "failed"} class:failed={update.state === "failed"} role="status">
      {statusText}
    </p>
  {/if}
  {#if update.state === "ready" && notes.length}
    <details>
      <summary class="muted">{t("updateNotes")}{update.date ? ` · ${update.date}` : ""}</summary>
      <div class="notes">
        {#each notes as block, i (i)}
          {#if block.kind === "p"}
            <p>{@render inlines(block.inlines)}</p>
          {:else}
            <ul>
              {#each block.items as item, j (j)}
                <li>{@render inlines(item)}</li>
              {/each}
            </ul>
          {/if}
        {/each}
      </div>
    </details>
  {/if}
  <div class="actions">
    <button class="btn" onclick={() => open("site")}>{t("openSite")}</button>
    <button class="btn" onclick={() => open("github")}>{t("openGithub")}</button>
    <button class="btn" onclick={() => open("logs")}>{t("openLogs")}</button>
  </div>
  {#if error}
    <p class="small failed" role="alert">{error}</p>
  {/if}
</section>

<style>
  .field {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 4px 0;
  }
  .small {
    font-size: 12px;
    margin: 4px 0 0;
  }
  .failed {
    color: var(--danger);
  }
  /* The accent's ink shade, with the page colour as text: readable in
     either theme. */
  .primary {
    border-color: var(--accent-ink);
    background: var(--accent-ink);
    color: var(--surface-0);
  }
  .primary:hover {
    background: var(--accent-ink);
    filter: brightness(1.08);
  }
  details {
    margin-top: 6px;
  }
  summary {
    cursor: pointer;
    font-size: 12px;
  }
  .notes {
    margin-top: 6px;
    font-size: 12px;
    line-height: 1.55;
    max-height: 220px;
    overflow: auto;
  }
  .notes p {
    margin: 0 0 6px;
  }
  .notes ul {
    margin: 0 0 6px;
    padding-left: 18px;
  }
  .notes li + li {
    margin-top: 4px;
  }
  .notes strong {
    font-weight: 600;
  }
  .notes code {
    font-size: 11px;
    padding: 0 3px;
    border-radius: 3px;
    background: var(--surface-2);
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 10px;
  }
  .btn:disabled {
    opacity: 0.6;
    cursor: default;
  }
</style>
