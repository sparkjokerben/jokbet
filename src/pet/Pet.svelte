<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount } from "svelte";
  import { celebrationText, headValue } from "../lib/format";
  import { blockedBy } from "./machine";
  import {
    PET_SCALE_DEFAULT,
    type ActionAnim,
    type IdleAnim,
    type MilestoneHit,
    type Settings,
    type Status,
    type Tick,
  } from "../lib/types";
  import type { AnimName } from "../sprites/jokbet";
  import { GRID_H, GRID_W, PET_H, PET_W, PET_X, PET_Y } from "../sprites/jokbet";
  import Bubble from "./Bubble.svelte";
  import { PetController } from "./controller";
  import Counter from "./Counter.svelte";
  import { attachGestures } from "./gestures";
  import Sprite from "./Sprite.svelte";

  const IDLE: Record<IdleAnim, AnimName> = { breathe: "idle", soccer: "soccer", lookAround: "lookAround" };
  /** Typing animation speed (keys per second) when live typing speed is turned off. */
  const FIXED_KPS = 2.5;
  const CELEBRATE_MS = 3500;
  /** Gap between the top of the pet's box and whatever sits above it. */
  const ABOVE_LIFT = 8;

  let rows = $state<string[]>([]);
  let settings = $state<Settings | null>(null);
  let tick = $state<Tick>({
    today: { keys: 0, clickLeft: 0, clickRight: 0, clickMiddle: 0, scrolls: 0, movePx: 0, moveMm: 0 },
    kps: 0,
    cps: 0,
    activity: null,
  });
  let paused = $state(false);
  let blocked = $state<ReturnType<typeof blockedBy>>(null);
  let hovering = $state(false);
  let banner = $state("");
  let bannerTimer: ReturnType<typeof setTimeout> | undefined;
  let spriteEl: HTMLDivElement;

  const scale = $derived(settings?.petScale ?? PET_SCALE_DEFAULT);
  /** How far the pet's middle sits from the canvas's (the laptop side is wider). */
  const petOffset = $derived((PET_X + PET_W / 2 - GRID_W / 2) * scale);
  const head = $derived(settings?.headCounter);
  const showCounter = $derived(!!head?.enabled && (head.keyboard || head.mouse));

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
    pet.setPaused(s.paused);
    blocked = blockedBy(s);
    pet.setBlocked(blocked);
  }

  function applyTick(t: Tick) {
    tick = t;
    if (settings?.typingSpeed !== false) pet.setKeysPerSecond(t.kps);
    if (t.activity) pet.input(t.activity);
  }

  function celebrate(hits: MilestoneHit[]) {
    banner = celebrationText(hits);
    pet.celebrate(CELEBRATE_MS);
    clearTimeout(bannerTimer);
    bannerTimer = setTimeout(() => (banner = ""), CELEBRATE_MS);
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
    window.addEventListener("resize", reportHitRect);

    const detach = attachGestures(spriteEl, {
      click: () => react(settings?.clickAnim),
      doubleClick: () => react(settings?.doubleClickAnim),
      dragStart: async () => {
        pet.setDragging(true);
        // Rust watches the mouse button and sends pet://drag-end on release,
        // however long the pointer rests mid-drag.
        await invoke("pet_drag_start");
        await win.startDragging();
      },
      context: () => invoke("show_context_menu"),
    });

    const unlisteners = [
      listen<[number, number]>("pet://gaze", (e) => pet.setGaze(e.payload)),
      listen<boolean>("pet://hover", (e) => (hovering = e.payload)),
      listen<Tick>("pet://tick", (e) => applyTick(e.payload)),
      listen<Status>("app://status", (e) => applyStatus(e.payload)),
      listen<Settings>("settings://changed", (e) => applySettings(e.payload)),
      listen<{ hits: MilestoneHit[] }>("pet://celebrate", (e) => celebrate(e.payload.hits)),
      listen("pet://drag-end", () => pet.setDragging(false)),
    ];

    invoke<Settings>("get_settings").then(async (s) => {
      applySettings(s);
      await Promise.all(unlisteners);
      invoke("pet_ready");
    });

    return () => {
      window.removeEventListener("resize", reportHitRect);
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
    style:bottom="{(GRID_H - PET_Y) * scale + ABOVE_LIFT}px"
    style:translate="{petOffset}px 0"
  >
    {#if hovering && settings?.bubble}
      <Bubble {tick} showSpeed={settings.typingSpeed} {paused} />
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
