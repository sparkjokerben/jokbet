// Picks the page's language before anything is painted, so the right one is
// shown from the first frame. The rule is the app's own (src/lib/i18n.ts):
// Chinese for Chinese locales, English for everything else.

(function () {
  var saved = null;
  try {
    saved = localStorage.getItem("jokbet.lang");
  } catch (e) {
    // private window, storage disabled: the browser language decides
  }
  var lang = saved === "zh" || saved === "en" ? saved : /^zh/i.test(navigator.language) ? "zh" : "en";
  document.documentElement.dataset.lang = lang;
  document.documentElement.lang = lang === "zh" ? "zh-Hans" : "en";
})();
