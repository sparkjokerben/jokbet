// Release notes as the About section shows them. They are written in Markdown
// for GitHub, and in two languages; this keeps the reader's language and turns
// the little Markdown they use into blocks the page renders itself (never as
// HTML, since the text comes over the network).

import type { Lang } from "./i18n";

export interface Inline {
  text: string;
  bold?: boolean;
  code?: boolean;
}

export type Block = { kind: "p"; inlines: Inline[] } | { kind: "ul"; items: Inline[][] };

const CJK = /[㐀-鿿]/g;

/** How much of a text is Chinese, from 0 to 1 (letters and ideographs only). */
function chineseShare(text: string): number {
  const cjk = text.match(CJK)?.length ?? 0;
  const latin = text.match(/[A-Za-z]/g)?.length ?? 0;
  return cjk + latin === 0 ? 0 : cjk / (cjk + latin / 4);
}

/**
 * The part of the notes in `lang`. Releases put the Chinese notes, then the
 * English ones, then a bilingual footer, with `---` between them; the footer
 * is dropped, and anything not laid out that way is kept whole.
 */
export function notesFor(notes: string, lang: Lang): string {
  const sections = notes
    .split(/^\s*---\s*$/m)
    .map((s) => s.trim())
    .filter(Boolean);
  if (sections.length < 2) return notes.trim();
  // The most Chinese section is the Chinese notes and the least the English;
  // a bilingual footer sits between the two and goes. English notes naming
  // "中文" stay English this way, where a fixed cut-off could tip them.
  const ranked = [...sections].sort((a, b) => chineseShare(b) - chineseShare(a));
  const zh = ranked[0];
  const en = ranked[ranked.length - 1];
  if (chineseShare(zh) < 0.5 || chineseShare(en) > 0.2) return notes.trim();
  return lang === "zh" ? zh : en;
}

/** **bold**, `code` and [text](url), which keeps only its text. */
export function parseInline(text: string): Inline[] {
  const out: Inline[] = [];
  const pattern = /\*\*(.+?)\*\*|`([^`]+)`|\[([^\]]+)\]\([^)\s]*\)/g;
  let last = 0;
  for (const m of text.matchAll(pattern)) {
    if (m.index > last) out.push({ text: text.slice(last, m.index) });
    if (m[1] !== undefined) out.push({ text: m[1], bold: true });
    else if (m[2] !== undefined) out.push({ text: m[2], code: true });
    else out.push({ text: m[3] });
    last = m.index + m[0].length;
  }
  if (last < text.length) out.push({ text: text.slice(last) });
  return out;
}

/** Paragraphs and "- " lists; a line that continues a list item is indented. */
export function parseNotes(markdown: string): Block[] {
  const blocks: Block[] = [];
  let paragraph: string[] = [];
  let items: string[] | null = null;
  const flush = () => {
    if (paragraph.length) blocks.push({ kind: "p", inlines: parseInline(paragraph.join(" ")) });
    if (items) blocks.push({ kind: "ul", items: items.map(parseInline) });
    paragraph = [];
    items = null;
  };
  for (const raw of markdown.split("\n")) {
    const line = raw.trimEnd();
    const bullet = /^\s*[-*]\s+(.*)$/.exec(line);
    if (!line.trim()) {
      flush();
    } else if (bullet) {
      if (paragraph.length) flush();
      (items ??= []).push(bullet[1]);
    } else if (items && /^\s+/.test(line)) {
      items[items.length - 1] += ` ${line.trim()}`;
    } else {
      if (items) flush();
      paragraph.push(line.trim());
    }
  }
  flush();
  return blocks;
}
