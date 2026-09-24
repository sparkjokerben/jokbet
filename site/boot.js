// Runs from every page's <head>, before anything is painted.
//
// 1. Picks the page's language, so the right one is shown from the first
//    frame. The rule is the app's own (src/lib/i18n.ts): Chinese for Chinese
//    locales, English for everything else — unless the visitor picked one here.
// 2. Sends the home page's old anchors to the pages they moved to, so a link
//    from before the site had pages (/#download, /#install, …) still lands.

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

  /** @type {Record<string, string>} */
  var moved = {
    "#download": "/download",
    "#install": "/download#install",
    "#privacy": "/about#privacy",
    "#limits": "/about#limits",
    "#license": "/about#license",
  };
  if (location.pathname === "/" && moved[location.hash]) location.replace(moved[location.hash]);
})();
