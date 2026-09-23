#!/usr/bin/env python3
"""Rebuild src/sprites/frames/{soccer,typing}.ts from the reference videos.

Usage (from the repository root):
    swift tools/extract-reference-frames/extract-frames.swift <video> <dir> <fps>
    python3 tools/extract-reference-frames/build-frames.py <soccer-dir> <typing-dir>

The reference videos are not part of the repository; drop them in references/
and pass their frame directories. The calibration below was measured from the
videos: the pet is a 24x16 cell sprite, K is the cell size in pixels and
(OX, OY) is the top-left corner of the pet's box in the video frame.

The decoder samples the video onto a 36x26 canvas that covers everything the
pet and the ball ever draw: columns -6..29 and rows -10..15 relative to the
pet's box.
"""

import sys
from collections import Counter, deque
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from png import readpng  # noqa: E402

COLS = list(range(-6, 30))
ROWS = list(range(-10, 16))
INK = set("OWEG")

# video -> (cell size, pet box origin, animation rows)
CALIBRATION = {
    "soccer": (148 / 24.0, 72.0, 63.0),
    "typing": (3.85, 61.5, 37.5),
}


def classify(r, g, b):
    if min(r, g, b) > 140:
        return "W"  # the ball's white checks
    if r > 92 and r - b > 25:
        return "O"  # body
    if max(r, g, b) < 72:
        return "K"  # eyes, the ball's dark checks or the background
    if max(r, g, b) - min(r, g, b) < 45 and r > 60:
        return "G"  # laptop
    return "?"


def decode(path, ox, oy, k):
    w, h, px = readpng(path)
    grid = []
    for r in ROWS:
        line = ""
        for c in COLS:
            cx, cy = int(ox + (c + 0.5) * k), int(oy + (r + 0.5) * k)
            votes = {}
            for dy in (-2, -1, 0, 1, 2):
                for dx in (-2, -1, 0, 1, 2):
                    x, y = cx + dx, cy + dy
                    if not (0 <= x < w and 0 <= y < h):
                        continue
                    ch = classify(*px[y][x * 4 : x * 4 + 3])
                    votes[ch] = votes.get(ch, 0) + 1
            # ink wins ties against the background
            best = max(votes.items(), key=lambda kv: (kv[1] + (0.5 if kv[0] in "OWG" else 0), kv[0] != "?"))
            line += best[0]
        grid.append(line)
    return clean(grid)


def clean(grid):
    """Snap unknown cells to a neighbour, drop specks, fill one-cell holes, and
    turn the dark cells that the border can reach into empty space."""
    height, width = len(grid), len(grid[0])

    def neighbours(i, j):
        out = []
        for dj in (-1, 0, 1):
            for di in (-1, 0, 1):
                if (di, dj) != (0, 0) and 0 <= i + di < width and 0 <= j + dj < height:
                    out.append(grid[j + dj][i + di])
        return out

    rows = [list(r) for r in grid]
    for j in range(height):
        for i in range(width):
            ch, nb = rows[j][i], neighbours(i, j)
            ink = sum(1 for c in nb if c in INK)
            if ch == "?":
                cand = [c for c in nb if c in INK]
                rows[j][i] = Counter(cand).most_common(1)[0][0] if cand else "."
            elif ch in INK and ink <= 1:
                rows[j][i] = "."  # speck
            elif ch == "." and ink >= 7:
                rows[j][i] = Counter(c for c in nb if c in INK).most_common(1)[0][0]  # hole
    grid = ["".join(r) for r in rows]

    seen = [[False] * width for _ in range(height)]
    queue = deque()
    for i in range(width):
        for j in (0, height - 1):
            if grid[j][i] == "K":
                queue.append((i, j))
                seen[j][i] = True
    for j in range(height):
        for i in (0, width - 1):
            if grid[j][i] == "K" and not seen[j][i]:
                queue.append((i, j))
                seen[j][i] = True
    while queue:
        i, j = queue.popleft()
        for di, dj in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            x, y = i + di, j + dj
            if 0 <= x < width and 0 <= y < height and grid[y][x] == "K" and not seen[y][x]:
                seen[y][x] = True
                queue.append((x, y))
    return [
        "".join("." if ch == "K" and seen[j][i] else ("E" if ch == "K" else ch) for i, ch in enumerate(row))
        for j, row in enumerate(grid)
    ]


def distance(a, b):
    return sum(1 for ra, rb in zip(a, b) for ca, cb in zip(ra, rb) if ca != cb)


def group(frames, tolerance=4):
    """Collapse runs of identical frames (the videos hold the source's 15 fps
    poses for two 30 fps frames each) into one pose with a duration."""
    poses, durations = [], []
    for grid in frames:
        if poses and distance(poses[-1], grid) <= tolerance:
            durations[-1] += 1
        else:
            poses.append(grid)
            durations.append(1)
    return poses, [int(d * 100 / 3) for d in durations]  # 30 fps frames -> ms


def consensus(grids):
    height, width = len(grids[0]), len(grids[0][0])
    out = []
    for j in range(height):
        row = ""
        for i in range(width):
            counts = Counter(g[j][i] for g in grids)
            row += sorted(counts.items(), key=lambda kv: (kv[1], kv[0] in INK), reverse=True)[0][0]
        out.append(row)
    return out


def load_frames(directory, calibration):
    k, ox, oy = calibration
    paths = sorted(Path(directory).glob("*.png"))
    return group([decode(p, ox, oy, k) for p in paths])


def ts_array(name, frames):
    items = []
    for f in frames:
        rows = ",\n".join(f'      "{r}"' for r in f["rows"])
        items.append(f'  {{ ms: {f["ms"]}, rows: [\n{rows},\n    ] }}')
    return f"export const {name}: readonly RawFrame[] = [\n" + ",\n".join(items) + ",\n];\n"


HEADER = """// Generated by tools/extract-reference-frames from references/{src}
// Recovered frame by frame from the reference video; do not edit by hand.
import type {{ RawFrame }} from "./types.ts";

"""


def main(soccer_dir, typing_dir, out_dir="src/sprites/frames"):
    poses, durations = load_frames(soccer_dir, CALIBRATION["soccer"])
    soccer = [
        {"ms": durations[0], "rows": poses[0]},
        *[{"ms": d, "rows": g} for g, d in zip(poses[1:-1], durations[1:-1])],
        {"ms": durations[-1], "rows": poses[-1]},
    ]
    Path(out_dir, "soccer.ts").write_text(
        HEADER.format(src="踢足球.mp4")
        + ts_array("SOCCER_IDLE_BEFORE", soccer[:1])
        + "\n"
        + ts_array("SOCCER", soccer[1:-1])
        + "\n"
        + ts_array("SOCCER_IDLE_AFTER", soccer[-1:])
    )

    poses, durations = load_frames(typing_dir, CALIBRATION["typing"])
    # intro (idle -> sitting at the laptop), a 3-frame typing cycle, then the outro
    intro = [{"ms": d, "rows": g} for g, d in zip(poses[1:12], durations[1:12])]
    loop = [{"ms": 100, "rows": consensus(poses[i : i + 6])} for i in range(12, 30, 6)]
    outro = [{"ms": d, "rows": g} for g, d in zip(poses[30:], durations[30:])]
    Path(out_dir, "typing.ts").write_text(
        HEADER.format(src="敲键盘.mp4")
        + ts_array("TYPING_INTRO", intro)
        + "\n"
        + ts_array("TYPING_LOOP", loop)
        + "\n"
        + ts_array("TYPING_OUTRO", outro)
    )
    print(f"soccer: {len(soccer)} poses, typing: {len(intro)}+{len(loop)}+{len(outro)} frames")


if __name__ == "__main__":
    if len(sys.argv) < 3:
        raise SystemExit(__doc__)
    main(*sys.argv[1:])
