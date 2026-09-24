import { describe, expect, it } from "vitest";
import { notesFor, parseInline, parseNotes } from "./notes";

// Laid out as the release notes are: Chinese, English, then a bilingual footer.
const NOTES = `这一版让它懂得在全屏时让开。

- **全屏时自动让开。** 别的应用全屏时它会先藏起来，可以在 Jokbet 设置里关掉。
- **全局快捷键。** 到「设置 › 快捷键」里录。

---

This release teaches the pet to step aside for full-screen apps.

- **It steps aside in full screen.** A setting turns this off.
- **Global shortcuts.** Follow the system, 中文 or English.

---

Unofficial fan project, not affiliated with Anthropic.
非官方同人作品，与 Anthropic 无关。安装包未经官方签名，首次打开的步骤见 [下载页](https://jokbet.jokerben.top/download)。`;

describe("notesFor", () => {
  it("keeps the reader's language and drops the footer", () => {
    expect(notesFor(NOTES, "zh")).toMatch(/^这一版/);
    expect(notesFor(NOTES, "zh")).not.toContain("This release");
    expect(notesFor(NOTES, "zh")).not.toContain("Unofficial");
    expect(notesFor(NOTES, "en")).toMatch(/^This release/);
    expect(notesFor(NOTES, "en")).not.toContain("非官方");
  });

  it("keeps notes whole when they are not laid out that way", () => {
    expect(notesFor("- One fix\n- Another", "zh")).toBe("- One fix\n- Another");
    expect(notesFor("Only English.\n\n---\n\nStill English.", "zh")).toContain("Still English.");
  });
});

describe("parseInline", () => {
  it("reads bold, code and links, keeping a link's text only", () => {
    expect(parseInline("**Bold.** run `npm test`, see [the page](https://example.com).")).toEqual([
      { text: "Bold.", bold: true },
      { text: " run " },
      { text: "npm test", code: true },
      { text: ", see " },
      { text: "the page" },
      { text: "." },
    ]);
  });

  it("leaves plain text and stray markers alone", () => {
    expect(parseInline("2 * 3 is not **bold")).toEqual([{ text: "2 * 3 is not **bold" }]);
  });
});

describe("parseNotes", () => {
  it("makes paragraphs and lists", () => {
    const blocks = parseNotes(notesFor(NOTES, "en"));
    expect(blocks.map((b) => b.kind)).toEqual(["p", "ul"]);
    const list = blocks[1];
    expect(list.kind === "ul" && list.items.length).toBe(2);
    expect(list.kind === "ul" && list.items[0][0]).toEqual({ text: "It steps aside in full screen.", bold: true });
  });

  it("joins wrapped lines and indented list continuations", () => {
    const blocks = parseNotes("One line\nand the next.\n- An item\n  that wraps\n- Two\nAfter.");
    expect(blocks).toEqual([
      { kind: "p", inlines: [{ text: "One line and the next." }] },
      { kind: "ul", items: [[{ text: "An item that wraps" }], [{ text: "Two" }]] },
      { kind: "p", inlines: [{ text: "After." }] },
    ]);
  });
});
