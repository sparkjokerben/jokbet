<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount, untrack } from "svelte";
  import { celebrationText, headValue } from "../lib/format";
  import { t } from "../lib/i18n";
  import { blockedBy } from "./machine";
  import {
    PET_SCALE_DEFAULT,
    type ActionAnim,
    type IdleAnim,
    type MilestoneHit,
    type Settings,
    type Status,
    type Tick,
    type UpdateStatus,
  } from "../lib/types";
  import type { AnimName } from "../sprites/jokbet";
  import { GRID_H, GRID_W, PET_H, PET_W, PET_X, PET_Y } from "../sprites/jokbet";
  import Bubble from "./Bubble.svelte";
  import { PetController } from "./controller";
  import Counter from "./Counter.svelte";
  import { attachGestures } from "./gestures";
  import { inkSide } from "./ink";
  import Sprite from "./Sprite.svelte";

  const IDLE: Record<IdleAnim, AnimName> = { breathe: "idle", soccer: "soccer", lookAround: "lookAround" };
  /** Typing animation speed (keys per second) when live typing speed is turned off. */
  const FIXED_KPS = 2.5;
  const CELEBRATE_MS = 3500;
  /** How long the pet says a new version is ready. */
  const UPDATE_BANNER_MS = 6000;
  /** Gap between the top of the pet's box and whatever sits above it. */
  const ABOVE_LIFT = 8;
  /** More of it when the card is the only thing up there: the counter under it
      holds it clear of the pet's head, and without one it reads as sitting on
      the head instead of above it. */
  const ABOVE_LIFT_ALONE = 24;

  let rows = $state<string[]>([]);
  let settings = $state<Settings | null>(null);
  let tick = $state<Tick>({
    today: { keys: 0, clickLeft: 0, clickRight: 0, clickMiddle: 0, scrolls: 0, movePx: 0, moveMm: 0 },
    kps: 0,
    cps: 0,
    activity: null,
  });
  let paused = $state(false);
  let storage = $state(true);
  /** A downloaded update's version, until the app restarts into it. */
  let updateReady = $state<string | null>(null);
  let blocked = $state<ReturnType<typeof blockedBy>>(null);
  let hovering = $state(false);
  let bubbleEl: HTMLDivElement | undefined = $state();
  /** What the system can put behind the bubble, from Rust; "none" where there is none. */
  let glassSupport = $state("none");
  /** Bumped on window resize so the glass follows the bubble. */
  let resized = $state(0);
  /** Whether the system is asking for dark, which the card starts from, and
      which stands in for the desktop where nothing can read it. */
  let themeDark = $state(false);
  /** Which way the panel leans, 0 for a pale one and 1 for a dark one. Rust
      reads it off the desktop behind the card, because the material shows that
      through; on a system whose material is its own business it stays the
      system's. */
  let tone = $state(0);
  /** Whether Rust has said which way the panel leans yet. Until it has, the
      system's own light or dark stands in for a reading. */
  let toneRead = false;
  /** The side the card's ink is drawn on, which follows the tone — dark ink over
      a pale panel, light ink over a dark one. */
  let dark = $state(false);
  let banner = $state("");
  let bannerTimer: ReturnType<typeof setTimeout> | undefined;
  let spriteEl: HTMLDivElement;

  const scale = $derived(settings?.petScale ?? PET_SCALE_DEFAULT);
  /** How far the pet's middle sits from the canvas's (the laptop side is wider). */
  const petOffset = $derived((PET_X + PET_W / 2 - GRID_W / 2) * scale);
  const head = $derived(settings?.headCounter);
  const showCounter = $derived(!!head?.enabled && (head.keyboard || head.mouse));
  /** Whether the counter or a banner shares the column under the card. */
  const underCard = $derived(!!banner || showCounter);
  const glassBubble = $derived(glassSupport !== "none" && !!settings?.bubble && !!settings?.liquidGlass);
  /** Whether the card is drawn over the acrylic: the one material whose colour
      is a reading of the desktop behind the card, and so the one card whose ink,
      and how much of its own colour it carries, come from that reading. On a
      material that is the system's own — macOS — and with no material at all,
      the card goes by the system's own light or dark, as it always has. */
  const acrylic = $derived(glassBubble && glassSupport === "acrylic");

  /** Tells Rust where the bubble is; the material is native, behind the page.
      It is told the system's own light or dark, which is what the panel starts
      from and what stands in for the desktop where nothing can read it — and
      the answer, the tone the panel leans by, comes back on `pet://tone`. */
  function reportGlassRect() {
    if (!glassBubble || !hovering || !bubbleEl) {
      void invoke("set_glass_bubble", { rect: null, radius: 0, system: themeDark });
      return;
    }
    // The card, not the plain wrapper around it: the radius lives on the card.
    const card = bubbleEl.querySelector<HTMLElement>(".bubble") ?? bubbleEl;
    const box = card.getBoundingClientRect();
    const radius = parseFloat(getComputedStyle(card).borderTopLeftRadius) || 0;
    // Exactly the card: anything larger would show the material's own edge
    // around it, which reads as a second panel behind the card.
    void invoke("set_glass_bubble", {
      rect: [box.left, box.top, box.width, box.height],
      radius,
      system: themeDark,
    });
  }

  $effect(() => {
    // Re-runs when the bubble comes and goes, changes size, or the window moves.
    void glassBubble;
    void hovering;
    void resized;
    void themeDark;
    // The counter and a banner sit under the card in the same column, so the
    // card is pushed up or let down without its own size changing — nothing for
    // the observer below to notice, and the material would be left behind.
    void showCounter;
    void banner;
    reportGlassRect();
    if (!glassBubble || !hovering || !bubbleEl) return;
    const observer = new ResizeObserver(reportGlassRect);
    observer.observe(bubbleEl);
    return () => observer.disconnect();
  });

  // The ink follows the tone, keeping the side it is on between the two marks:
  // the inks read much the same across that middle, and a tone drifting there
  // would otherwise flicker them.
  $effect(() => {
    dark = inkSide(tone, untrack(() => dark));
  });

  const pet = new PetController((r) => (rows = r));

  function reportHitRect() {
    // Jokbet fills the middle of the canvas: his box is 24x16 cells at (PET_X, PET_Y).
    const box = spriteEl.getBoundingClientRect();
    invoke("set_hit_rect", {
      rect: {
        x: box.left + PET_X * scale,
        y: box.top + PET_Y * scale,
        w: PET_W * scale,
        h: PET_H * scale,
      },
    });
  }

  function applySettings(s: Settings) {
    settings = s;
    pet.setSleepAfter(s.sleepAfterMin * 60_000);
    pet.setIdleAnim(IDLE[s.idleAnim]);
    if (!s.typingSpeed) pet.setKeysPerSecond(FIXED_KPS);
  }

  function applyStatus(s: Status) {
    paused = s.paused;
    storage = s.storage;
    pet.setPaused(s.paused);
    blocked = blockedBy(s);
    pet.setBlocked(blocked);
  }

  function applyTick(t: Tick) {
    tick = t;
    if (settings?.typingSpeed !== false) pet.setKeysPerSecond(t.kps);
    if (t.activity) pet.input(t.activity);
  }

  function say(text: string, ms: number) {
    banner = text;
    clearTimeout(bannerTimer);
    bannerTimer = setTimeout(() => (banner = ""), ms);
  }

  function celebrate(hits: MilestoneHit[]) {
    say(celebrationText(hits), CELEBRATE_MS);
    pet.celebrate(CELEBRATE_MS);
  }

  /** Says once that an update is downloaded; the bubble keeps saying it. */
  function applyUpdate(status: UpdateStatus, announce: boolean) {
    if (status.state !== "ready" || status.version === updateReady) return;
    updateReady = status.version;
    if (!announce) return;
    say(t("updateBanner", { v: status.version }), UPDATE_BANNER_MS);
    pet.oneShot("wave");
  }

  $effect(() => {
    void scale;
    requestAnimationFrame(reportHitRect);
  });

  function react(anim: ActionAnim | undefined) {
    // A pet that cannot see input leads straight to the fix.
    if (blocked === "noperm") invoke("open_panel", { view: "onboarding" });
    else if (anim) pet.oneShot(anim);
  }

  onMount(() => {
    const win = getCurrentWindow();
    // A new pet size resizes the window after the new scale is drawn; measure
    // the hit rect again once the page has the window's new size.
    const onResize = () => {
      reportHitRect();
      resized++;
    };
    window.addEventListener("resize", onResize);

    const detach = attachGestures(spriteEl, {
      press: () => pet.pressed(),
      click: () => react(settings?.clickAnim),
      doubleClick: () => react(settings?.doubleClickAnim),
      dragStart: async () => {
        pet.setDragging(true);
        // Rust watches the mouse button and sends pet://drag-end on release,
        // however long the pointer rests mid-drag.
        await invoke("pet_drag_start");
        await win.startDragging();
      },
      context: async () => {
        await invoke("show_context_menu");
        // The menu holds the main thread while it is up, so anything asked for
        // in that time lands once it is gone: say where the bubble is now.
        reportGlassRect();
      },
    });

    const unlisteners = [
      listen<[number, number]>("pet://gaze", (e) => pet.setGaze(e.payload)),
      listen<boolean>("pet://hover", (e) => (hovering = e.payload)),
      listen<Tick>("pet://tick", (e) => applyTick(e.payload)),
      listen<Status>("app://status", (e) => applyStatus(e.payload)),
      listen<Settings>("settings://changed", (e) => applySettings(e.payload)),
      // Which way the panel leans, read off the desktop behind the card.
      listen<number>("pet://tone", (e) => {
        toneRead = true;
        tone = e.payload;
      }),
      listen<{ hits: MilestoneHit[] }>("pet://celebrate", (e) => celebrate(e.payload.hits)),
      listen("pet://drag-end", () => pet.setDragging(false)),
      listen<UpdateStatus>("app://update", (e) => applyUpdate(e.payload, true)),
    ];
    invoke<UpdateStatus>("update_status").then((s) => applyUpdate(s, false), () => {});

    // The system's light or dark, which the panel leans by until Rust has read
    // what is actually behind the card — and always, where the material takes
    // its colour from the system on its own.
    const scheme = window.matchMedia("(prefers-color-scheme: dark)");
    const onScheme = () => {
      themeDark = scheme.matches;
      // Over the acrylic the tone is a reading of the desktop, and the system's
      // own light or dark is not to talk over it: a panel that is away keeps the
      // tone it went away with, so that it comes back up in that colour rather
      // than in a guess made while it was gone. Everywhere else the system's
      // answer *is* the card's, and it stands as it always has.
      if (!(untrack(() => acrylic) && toneRead)) tone = scheme.matches ? 1 : 0;
    };
    onScheme();
    dark = scheme.matches;
    scheme.addEventListener("change", onScheme);

    invoke<string>("glass_support").then((name) => (glassSupport = name));
    invoke<Settings>("get_settings").then(async (s) => {
      applySettings(s);
      await Promise.all(unlisteners);
      invoke("pet_ready");
    });

    return () => {
      window.removeEventListener("resize", onResize);
      scheme.removeEventListener("change", onScheme);
      detach();
      pet.destroy();
      clearTimeout(bannerTimer);
      unlisteners.forEach((u) => u.then((f) => f()));
    };
  });
</script>

<div class="stage">
  <div
    class="above"
    style:bottom="{(GRID_H - PET_Y) * scale + (underCard ? ABOVE_LIFT : ABOVE_LIFT_ALONE)}px"
    style:translate="{petOffset}px 0"
  >
    {#if hovering && settings?.bubble}
      <div bind:this={bubbleEl}>
        <Bubble
          {tick}
          showSpeed={settings.typingSpeed}
          {paused}
          {storage}
          {updateReady}
          glass={glassBubble}
          {acrylic}
          onDark={dark}
          {tone}
          tint={(settings.glassTint ?? 0) / 100}
        />
      </div>
    {/if}
    {#if banner}
      <div class="banner" role="status">{banner}</div>
    {:else if showCounter && head}
      <Counter value={headValue(tick, head)} rate={head.kind === "rate"} dimmed={paused} />
    {/if}
  </div>
  <div class="sprite" bind:this={spriteEl} style:width="{GRID_W * scale}px" style:height="{GRID_H * scale}px">
    <Sprite {rows} {scale} />
  </div>
</div>

<style>
  .stage {
    position: fixed;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: flex-end;
  }
  .above {
    position: absolute;
    left: 0;
    right: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    pointer-events: none;
  }
  .sprite {
    cursor: grab;
  }
  .banner {
    max-width: 210px;
    padding: 4px 10px;
    border-radius: 999px;
    background: #b85a3a;
    color: #fff;
    font: 600 12px/16px -apple-system, "Segoe UI", system-ui, sans-serif;
    text-align: center;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
    animation: pop 0.25s ease-out;
  }
  @keyframes pop {
    from {
      transform: scale(0.6);
      opacity: 0;
    }
  }
</style>
