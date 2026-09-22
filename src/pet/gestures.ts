// Tells apart single click, double click, drag and context menu.

export const DRAG_THRESHOLD_PX = 4;
export const DOUBLE_CLICK_MS = 250;

export interface GestureHandlers {
  click(): void;
  doubleClick(): void;
  dragStart(): void;
  context(): void;
}

export function attachGestures(el: HTMLElement, h: GestureHandlers): () => void {
  let down: { x: number; y: number } | null = null;
  let clicks = 0;
  let clickTimer: ReturnType<typeof setTimeout> | undefined;

  const onDown = (e: PointerEvent) => {
    if (e.button !== 0) return;
    down = { x: e.screenX, y: e.screenY };
  };
  const onMove = (e: PointerEvent) => {
    if (!down) return;
    if (Math.hypot(e.screenX - down.x, e.screenY - down.y) > DRAG_THRESHOLD_PX) {
      // The native drag loop swallows pointerup, so forget the press now.
      down = null;
      clicks = 0;
      clearTimeout(clickTimer);
      h.dragStart();
    }
  };
  const onUp = (e: PointerEvent) => {
    if (e.button !== 0 || !down) return;
    down = null;
    clicks++;
    clearTimeout(clickTimer);
    if (clicks >= 2) {
      clicks = 0;
      h.doubleClick();
    } else {
      clickTimer = setTimeout(() => {
        clicks = 0;
        h.click();
      }, DOUBLE_CLICK_MS);
    }
  };
  const onContext = (e: MouseEvent) => {
    e.preventDefault();
    h.context();
  };

  el.addEventListener("pointerdown", onDown);
  el.addEventListener("pointermove", onMove);
  el.addEventListener("pointerup", onUp);
  el.addEventListener("contextmenu", onContext);
  return () => {
    clearTimeout(clickTimer);
    el.removeEventListener("pointerdown", onDown);
    el.removeEventListener("pointermove", onMove);
    el.removeEventListener("pointerup", onUp);
    el.removeEventListener("contextmenu", onContext);
  };
}
