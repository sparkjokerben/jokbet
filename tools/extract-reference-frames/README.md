# extract-reference-frames

The soccer and typing animations in `src/sprites/frames/` are not hand-drawn:
they are the frames of Anthropic's own Clawd mascot, recovered from two screen
recordings (a ball juggling loop and the laptop/typing loop). This directory
holds the tooling that turns such a recording back into pixel frames.

The recordings themselves are not in the repository.

## How it works

The mascot is a 24x16 cell sprite. Both recordings show it at a fixed scale, so
once the cell size `K` and the top-left corner `(OX, OY)` of the pet's box in
the video frame are known, every frame can be sampled back onto the cell grid.
The canvas is 36x26 cells — columns -6..29 and rows -10..15 relative to the
pet's box — because the juggled ball leaves the pet's own box.

Sampling reads a 5x5 patch at each cell's centre and takes the majority:

| character | meaning | test |
|---|---|---|
| `O` | body | warm orange |
| `W` | the ball's white checks | near white |
| `G` | the laptop | neutral grey, mid brightness |
| `K` | dark: eyes, the ball's dark checks, or the background | near black |

`clean()` then snaps stray cells, drops single-cell specks, fills single-cell
holes, and turns the dark cells the canvas border can reach into empty space
(`E` stays for dark cells enclosed by the pet or the ball).

The recordings run at 15 fps held over a 30 fps video, so consecutive identical
frames are collapsed into one pose with a duration, and the typing loop's
periodic jitter is averaged back into a 3-frame cycle with `consensus()`.

## Usage

```sh
# 1. video -> PNG frames (macOS; uses AVFoundation, no ffmpeg needed)
swift tools/extract-reference-frames/extract-frames.swift 踢足球.mp4 /tmp/soccer 30

# 2. PNG frames -> src/sprites/frames/*.ts
python3 tools/extract-reference-frames/build-frames.py /tmp/soccer /tmp/keys
```

`CALIBRATION` at the top of `build-frames.py` holds the per-recording `(K, OX,
OY)`. To calibrate a new recording, print the pet's edges and fit them: the
torso is 16 cells wide and the four legs are 2 cells wide on a 4-cell pitch, so
`K = (arm span) / 24` and `OX` is the left edge of the arms.

## Checking the result

`node scripts/render-sprites.ts anim soccer out.png 8 4` renders every frame of
an animation through the app's own sprite pipeline, which is the quickest way to
compare it against the source recording.
