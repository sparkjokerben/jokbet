<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount } from "svelte";
  import { headValue } from "../lib/format";
  import { blockedBy } from "./machine";
  import { SCALE, type Settings, type Status, type Tick } from "../lib/types";
  import { GRID_H, GRID_W } from "../sprites/clawd";
  import Bubble from "./Bubble.svelte";
  import { PetController } from "./controller";
  import Counter from "./Counter.svelte";
  import { attachGestures } from "./gestures";
  import Sprite from "./Sprite.svelte";

  /** Drag counts as over once the window stops moving for this long. */
  const DRAG_SETTLE_MS = 250;
  /** Typing animation speed when live typing speed is turned off. */
  const FIXED_KPM = 150;

  let rows = $state<string[]>([]);
  let settings = $state<Settings | null>(null);
  let tick = $state<Tick>({
    today: { keys: 0, clickLeft: 0, clickRight: 0, clickMiddle: 0, scrolls: 0, movePx: 0, moveMm: 0 },
    kpm: 0,
    cpm: 0,
    activity: null,
  });
  let paused = $state(false);
  let hovering = $state(false);
  let spriteEl: HTMLDivElement;

  const scale = $derived(settings ? SCALE[settings.petSize] : SCALE.medium);
  const head = $derived(settings?.headCounter);
  const showCounter = $derived(!!head?.enabled && (head.keyboard || head.mouse));

  const pet = new PetController((r) => (rows = r));

  function reportHitRect() {
    // Clawd's body spans grid columns 4..19 and rows 3..15 (room for jumps).
    const box = spriteEl.getBoundingClientRect();
    invoke("set_hit_rect", {
      rect: { x: box.left + 4 * scale, y: box.top + 3 * scale, w: 16 * scale, h: 13 * scale },
    });
  }

  function applySettings(s: Settings) {
    settings = s;
    pet.setSleepAfter(s.sleepAfterMin * 60_000);
    if (!s.typingSpeed) pet.setKpm(FIXED_KPM);
  }

  function applyStatus(s: Status) {
    paused = s.paused;
    pet.setPaused(s.paused);
    pet.setBlocked(blockedBy(s));
  }

  function applyTick(t: Tick) {
    tick = t;
    if (settings?.typingSpeed !== false) pet.setKpm(t.kpm);
    if (t.activity) pet.input(t.activity);
  }

  $effect(() => {
    void scale;
    requestAnimationFrame(reportHitRect);
  });

  onMount(() => {
    const win = getCurrentWindow();
    let settleTimer: ReturnType<typeof setTimeout> | undefined;
    const endDragSoon = () => {
      clearTimeout(settleTimer);
      settleTimer = setTimeout(() => {
        settleTimer = undefined;
        pet.setDragging(false);
        invoke("pet_drag_end");
      }, DRAG_SETTLE_MS);
    };

    const detach = attachGestures(spriteEl, {
      poke: () => pet.oneShot("poke"),
      special: () => pet.oneShot("special"),
      dragStart: () => {
        pet.setDragging(true);
        win.startDragging();
        endDragSoon();
      },
      context: () => invoke("show_context_menu"),
    });

    const unlisteners = [
      listen<[number, number]>("pet://gaze", (e) => pet.setGaze(e.payload)),
      listen<boolean>("pet://hover", (e) => (hovering = e.payload)),
      listen<Tick>("pet://tick", (e) => applyTick(e.payload)),
      listen<Status>("pet://status", (e) => applyStatus(e.payload)),
      listen<Settings>("settings://changed", (e) => applySettings(e.payload)),
      win.onMoved(() => {
        if (settleTimer !== undefined) endDragSoon();
      }),
    ];

    invoke<Settings>("get_settings").then(async (s) => {
      applySettings(s);
      await Promise.all(unlisteners);
      invoke("pet_ready");
    });

    return () => {
      detach();
      pet.destroy();
      clearTimeout(settleTimer);
      unlisteners.forEach((u) => u.then((f) => f()));
    };
  });
</script>

<div class="stage">
  <div class="above" style:bottom="{(GRID_H - 2) * scale}px">
    {#if hovering && settings?.bubble}
      <Bubble {tick} showSpeed={settings.typingSpeed} {paused} />
    {/if}
    {#if showCounter && head}
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
</style>
