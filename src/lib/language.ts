// Puts a page in the language the settings ask for. Strings are read once, as
// the page builds, so a change of language reloads the page rather than
// making every string reactive.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { resolveLang, setLang, type Lang } from "./i18n";
import type { Settings } from "./types";

function use(lang: Lang) {
  setLang(lang);
  document.documentElement.lang = lang === "zh" ? "zh-CN" : "en";
}

/** Sets the page's language from the settings; call before mounting. */
export async function applyLanguage(): Promise<void> {
  const settings = await invoke<Settings>("get_settings").catch(() => null);
  const current = resolveLang(settings?.language ?? "system");
  use(current);
  void listen<Settings>("settings://changed", (e) => {
    if (resolveLang(e.payload.language) !== current) location.reload();
  });
}
