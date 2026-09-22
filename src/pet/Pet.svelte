<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount } from "svelte";
  import { GRID_H, GRID_W } from "../sprites/clawd";
  import { PetController } from "./controller";
  import { attachGestures } from "./gestures";
  import Sprite from "./Sprite.svelte";

  interface Settings {
    petSize: "small" | "medium" | "large";
  }

  const SCALE = { small: 4, medium: 6, large: 8 } as const;
  /** Drag counts as over once the window stops moving for this long. */
  const DRAG_SETTLE_MS = 250;

  let rows = $state<string[]>([]);
  let scale = $state<number>(SCALE.medium);
  let spriteEl: HTMLDivElement;

  const pet = new PetController((r) => (rows = r));

  function reportHitRect() {
    // Clawd's body spans grid columns 4..19 and rows 3..15 (room for jumps).
    const box = spriteEl.getBoundingClientRect();
    invoke("set_hit_rect", {
      rect: { x: box.left + 4 * scale, y: box.top + 3 * scale, w: 16 * scale, h: 13 * scale },
    });
  }

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
      win.onMoved(() => {
        if (settleTimer !== undefined) endDragSoon();
      }),
    ];

    invoke<Settings>("get_settings").then((s) => {
      scale = SCALE[s.petSize];
      requestAnimationFrame(reportHitRect);
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
  .sprite {
    cursor: grab;
  }
</style>
