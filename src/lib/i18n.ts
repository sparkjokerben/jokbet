// zh/en UI strings, following the system language.

const zh = {
  today: "今天",
  keys: "按键",
  clicks: "点击",
  clickSplit: "左 {l} · 右 {r} · 中 {m}",
  scrolls: "滚动",
  distance: "移动",
  speed: "手速",
  perMinute: "{n} 次/分",
  meters: "{n} 米",
  kilometers: "{n} 公里",
  paused: "已暂停计数",
} as const;

export type MessageKey = keyof typeof zh;

const en: Record<MessageKey, string> = {
  today: "Today",
  keys: "Keys",
  clicks: "Clicks",
  clickSplit: "L {l} · R {r} · M {m}",
  scrolls: "Scrolls",
  distance: "Mouse",
  speed: "Speed",
  perMinute: "{n}/min",
  meters: "{n} m",
  kilometers: "{n} km",
  paused: "Counting paused",
};

export const messages = { zh, en } as const;
export type Lang = keyof typeof messages;

export function detectLang(locale: string | undefined): Lang {
  return locale?.toLowerCase().startsWith("zh") ? "zh" : "en";
}

export const lang: Lang = detectLang(typeof navigator === "undefined" ? undefined : navigator.language);

export function t(key: MessageKey, vars: Record<string, string | number> = {}, l: Lang = lang): string {
  return messages[l][key].replace(/\{(\w+)\}/g, (_, v: string) => String(vars[v] ?? `{${v}}`));
}
