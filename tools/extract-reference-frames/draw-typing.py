"""Draw src/sprites/frames/typing.ts after references/敲键盘.mp4.

Usage (from the repository root):
    python3 tools/extract-reference-frames/draw-typing.py

The typing recording is too small to decode cell by cell (under 4 video pixels
per cell, and not square), so each of its poses is drawn here from the pet's
parts instead, with the positions measured off the recording: the laptop coming
out, the hop into profile, the three-pose typing cycle, and putting it away.

Coordinates are relative to the pet's 24x16 box (x right, y down); the canvas
is 40x26 with the box at (6, 10).
"""
from pathlib import Path

W, H, PX, PY = 40, 26, 6, 10


def blank():
    return [["."] * W for _ in range(H)]


def rect(g, x0, x1, y0, y1, ch):
    """Fills columns x0..x1 and rows y0..y1 inclusive."""
    for y in range(y0, y1 + 1):
        for x in range(x0, x1 + 1):
            X, Y = x + PX, y + PY
            if 0 <= X < W and 0 <= Y < H:
                g[Y][X] = ch


def cells(g, spec, ch):
    """spec: {row: [(x0, x1), ...]}"""
    for y, spans in spec.items():
        for x0, x1 in spans:
            rect(g, x0, x1, y, y, ch)


# ---- the laptop

def laptop_ground(g, lift=0):
    """Open laptop on the floor right of the pet: the base, then the screen
    rising in 2-cell steps. `lift` raises it (it hops when a key lands)."""
    spec = {
        15: [(23, 28)],
        14: [(28, 29)],
        13: [(29, 30)],
        12: [(30, 31)],
        11: [(31, 32)],
        10: [(32, 32)],
    }
    cells(g, {y - lift: s for y, s in spec.items()}, "G")


# ---- front view

def front(g, top=0, legs_top=12, arm_l=4, arm_r=4, eyes=2, left_eye="open", right_eye=True, shade_l=0):
    rect(g, 4, 19, top, legs_top - 1, "O")
    for x in (4, 8, 14, 18):
        rect(g, x, x + 1, legs_top, 15, "O")
    if arm_l is not None:
        rect(g, 0, 3, arm_l, arm_l + 3, "O")
    if arm_r is not None:
        rect(g, 20, 23, arm_r, arm_r + 3, "O")
    if left_eye == "open":
        rect(g, 6, 7, eyes, eyes + 1, "E")
    elif left_eye == "closed":
        rect(g, 6, 8, eyes + 1, eyes + 1, "E")
    if right_eye:
        rect(g, 16, 17, eyes, eyes + 1, "E")
    if shade_l:
        rect(g, 4, 3 + shade_l, top, legs_top - 1, "D")
        rect(g, 4, 5, legs_top, 15, "D")


# ---- profile, facing right

def profile(g, top=2, x=6, eye_h=2, legs_top=13):
    """Sitting in profile: the far side (4 columns) in shade, one eye in the
    face and the other on the edge of it, the legs folded under."""
    rect(g, x, x + 15, top, legs_top - 1, "O")
    rect(g, x, x + 3, top, legs_top - 1, "D")
    ey = top + 3
    rect(g, x + 6, x + 7, ey, ey + eye_h - 1, "E")
    rect(g, x + 14, x + 15, ey, ey + eye_h - 1, "E")
    legs(g, x, legs_top)


def legs(g, x, legs_top):
    """Folded legs: straight down, then a foot that sticks out backwards. The
    far leg is in shade, and so is the back of each foot."""
    for i, lx in enumerate((x, x + 4, x + 10, x + 14)):
        ch = "D" if i == 0 else "O"
        rect(g, lx, lx + 1, legs_top, 13, ch)
        rect(g, lx - 1, lx + 1, 14, 14, ch)
        rect(g, lx - 1, lx - 1, 14, 14, "D")
        rect(g, lx - 1, lx, 15, 15, ch)


def rows(g):
    return ["".join(r) for r in g]


# ---- the frames

def reach(arm_top):
    """Squatting, one eye shut, the right paw behind the back for the laptop."""
    g = blank()
    front(g, top=1, legs_top=13, arm_l=3, arm_r=None, eyes=5, left_eye="closed")
    rect(g, 20, 22, arm_top, arm_top + 2, "O")
    rect(g, 20, 21, arm_top + 3, arm_top + 3, "O")
    return g


def laptop_out():
    """The closed laptop comes out over the right shoulder."""
    g = blank()
    front(g, arm_l=5, arm_r=None)
    rect(g, 20, 23, 2, 5, "O")
    rect(g, 20, 22, 6, 6, "O")
    rect(g, 20, 21, 7, 7, "O")
    cells(g, {-3: [(21, 22)], -2: [(21, 24)], -1: [(23, 27)], 0: [(25, 27)], 1: [(21, 27)]}, "G")
    return g


def laptop_up():
    """Held up high, open."""
    g = blank()
    front(g, arm_l=5, arm_r=None)
    rect(g, 20, 23, -1, 2, "O")
    rect(g, 26, 26, -7, -3, "G")
    rect(g, 21, 26, -2, -2, "G")
    return g


def laptop_place():
    """Swung down to the floor."""
    g = blank()
    front(g, arm_l=4, arm_r=None, eyes=3)
    cells(g, {5: [(20, 21)], 6: [(20, 22)], 7: [(20, 22)], 8: [(20, 22)], 9: [(20, 23)], 10: [(20, 23)], 11: [(20, 23)]}, "O")
    cells(g, {3: [(30, 31)], 4: [(29, 32)], 5: [(28, 32)], 6: [(28, 31)], 7: [(23, 30)], 8: [(23, 28)]}, "G")
    return g


def laptop_down(arm_l, arm_r, screen):
    g = blank()
    front(g, arm_l=arm_l, arm_r=arm_r, eyes=3)
    if screen == "up":
        laptop_ground(g)
    else:
        # The screen still wobbling from the drop.
        cells(g, {15: [(23, 28)], 14: [(28, 29)], 13: [(30, 31)], 12: [(32, 33)]}, "G")
    return g


def jump():
    """Hops up with both paws raised, turning to the laptop."""
    g = blank()
    cells(g, {-4: [(8, 11), (18, 21)], -3: [(8, 11), (18, 21)], -2: [(7, 11), (18, 22)]}, "O")
    rect(g, 5, 22, -1, 1, "O")
    rect(g, 5, 20, 2, 11, "O")
    rect(g, 5, 6, -1, 11, "D")
    rect(g, 7, 7, -2, -2, "D")
    rect(g, 21, 22, 1, 1, "D")
    rect(g, 8, 9, 3, 4, "E")
    rect(g, 18, 19, 3, 4, "E")
    for x in (5, 9, 15, 19):
        rect(g, x, x + 1, 12, 15, "O")
    rect(g, 5, 6, 12, 15, "D")
    laptop_ground(g, lift=1)
    return g


def land():
    """Lands sitting in profile, eyes wide, paws still up."""
    g = blank()
    laptop_ground(g)
    profile(g, top=1, eye_h=3, legs_top=12)
    cells(g, {2: [(22, 22)], 3: [(22, 22)], 4: [(22, 22)], 5: [(22, 22)], 6: [(22, 22)], 7: [(22, 24)], 8: [(22, 25)]}, "D")
    cells(g, {3: [(23, 24)], 4: [(23, 25)], 5: [(23, 25)], 6: [(23, 25)], 7: [(25, 25)]}, "O")
    cells(g, {10: [(22, 25)], 11: [(22, 25)], 12: [(22, 25)], 13: [(22, 25)], 14: [(22, 24)]}, "O")
    rect(g, 25, 25, 14, 14, "D")
    return g


def typing(pose):
    """The typing cycle. A: the near paw comes down and the laptop hops;
    B: the near paw lifts while the far paw presses; C: the near paw lands."""
    g = blank()
    laptop_ground(g, lift=1 if pose == "A" else 0)
    profile(g)
    if pose == "A":
        rect(g, 22, 24, 8, 11, "O")
        rect(g, 25, 25, 9, 11, "D")
        rect(g, 22, 25, 12, 12, "D")
    elif pose == "B":
        rect(g, 22, 24, 7, 10, "O")
        rect(g, 22, 25, 12, 14, "D")
    else:
        rect(g, 22, 25, 7, 7, "D")
        rect(g, 22, 23, 8, 8, "O")
        rect(g, 24, 25, 8, 8, "D")
        rect(g, 22, 24, 9, 12, "O")
        rect(g, 25, 25, 9, 11, "D")
        rect(g, 22, 24, 13, 13, "D")
    return g


def shut():
    """Both paws come down on the laptop to close it."""
    g = blank()
    laptop_ground(g)
    profile(g)
    rect(g, 22, 25, 7, 8, "D")
    rect(g, 22, 24, 9, 10, "O")
    rect(g, 25, 25, 9, 10, "D")
    rect(g, 22, 25, 11, 15, "O")
    return g


def rise():
    """Getting up, eyes wide, folding the lid down."""
    g = blank()
    profile(g, top=1, x=5, eye_h=3, legs_top=12)
    cells(g, {6: [(31, 31)], 7: [(30, 31)], 8: [(29, 30)], 9: [(28, 29)], 10: [(27, 28)], 11: [(26, 27)]}, "G")
    cells(g, {2: [(21, 21)], 3: [(23, 23)], 4: [(24, 24)], 5: [(21, 21)], 6: [(21, 21)], 7: [(20, 23)], 8: [(21, 24)], 10: [(24, 24)]}, "D")
    cells(g, {3: [(21, 22)], 4: [(21, 23)], 5: [(22, 24)], 6: [(22, 24)], 7: [(24, 24)], 9: [(21, 22)], 10: [(21, 23)], 11: [(21, 25)],
              12: [(22, 25)], 13: [(22, 25)], 14: [(22, 25)]}, "O")
    return g


def carry():
    """Turning back to the front with the closed laptop under the arm."""
    g = blank()
    front(g, arm_l=None, arm_r=None, left_eye=None, right_eye=False, shade_l=2)
    rect(g, 8, 9, 3, 4, "E")
    rect(g, 18, 19, 3, 4, "E")
    rect(g, 2, 3, 5, 8, "O")
    rect(g, 7, 7, 5, 8, "D")
    cells(g, {5: [(20, 21)], 6: [(20, 22)], 7: [(20, 22)], 8: [(20, 25)], 9: [(20, 21)], 10: [(20, 20)]}, "O")
    cells(g, {5: [(22, 22)], 6: [(23, 23)], 7: [(23, 24)], 9: [(22, 25)], 10: [(21, 21)]}, "D")
    cells(g, {5: [(23, 23)], 6: [(24, 24)], 7: [(25, 26)], 8: [(26, 27)], 9: [(26, 27)], 10: [(22, 27)]}, "G")
    return g


def put_away(wide):
    """Squatting, looking down, stowing the laptop behind the back."""
    g = blank()
    front(g, top=1, legs_top=13, arm_l=3, arm_r=None, eyes=5)
    if wide:
        rect(g, 20, 22, 8, 10, "O")
        rect(g, 20, 21, 11, 11, "O")
    else:
        rect(g, 20, 21, 8, 11, "O")
    return g


def settle():
    g = blank()
    front(g, arm_l=5, arm_r=5)
    return g


INTRO = [
    (reach(7), 67), (reach(8), 67), (reach(7), 67), (reach(8), 167),
    (laptop_out(), 100), (laptop_up(), 167), (laptop_place(), 67),
    (laptop_down(5, 9, "up"), 100), (laptop_down(3, 4, "wobble"), 100),
    (jump(), 67), (land(), 67),
]
LOOP = [(typing("A"), 89), (typing("B"), 89), (typing("C"), 89)]
OUTRO = [
    (shut(), 67), (rise(), 100), (carry(), 67),
    (put_away(True), 167), (put_away(False), 100), (settle(), 67),
]

HEADER = """// Generated by tools/extract-reference-frames/draw-typing.py, drawn after
// references/敲键盘.mp4 pose by pose; edit the script, not this file.
import type { RawFrame } from "./types.ts";

"""


def ts_array(name, frames):
    items = []
    for g, ms in frames:
        body = ",\n".join(f'      "{r}"' for r in rows(g))
        items.append(f"  {{ ms: {ms}, rows: [\n{body},\n    ] }}")
    return f"export const {name}: readonly RawFrame[] = [\n" + ",\n".join(items) + ",\n];\n"


if __name__ == "__main__":
    Path("src/sprites/frames/typing.ts").write_text(
        HEADER
        + ts_array("TYPING_INTRO", INTRO)
        + "\n"
        + ts_array("TYPING_LOOP", LOOP)
        + "\n"
        + ts_array("TYPING_OUTRO", OUTRO)
    )
    print(f"typing: {len(INTRO)}+{len(LOOP)}+{len(OUTRO)} frames")
