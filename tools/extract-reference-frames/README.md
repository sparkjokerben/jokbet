# extract-reference-frames

The soccer and typing animations in `src/sprites/frames/` follow two screen
recordings of Anthropic's own Clawd mascot (a ball juggling loop and the
laptop/typing loop), so they move the way it does. This directory holds the
tooling that turns those recordings into pixel frames.

The recordings themselves are not in the repository.

- `soccer.ts` is decoded from the recording frame by frame (`build-frames.py`).
- `typing.ts` is drawn pose by pose (`draw-typing.py`). That recording is too
  small to decode cleanly: under 4 video pixels per cell, and the cells are not
  square. Sampled cell by cell, the paws, the eyes and the laptop came out
  smeared. So each pose is drawn from the pet's parts, with the positions
  measured off the recording.

Both land on the app's 40x26 canvas: columns -6..33 and rows -10..15 relative
to the pet's 24x16 box. The canvas is bigger than the pet because the juggled
ball goes above it and the laptop sits beside it.

## Decoding (soccer)

The mascot is a 24x16 cell sprite. The recording shows it at a fixed scale, so
once the cell size `K` and the top-left corner `(OX, OY)` of the pet's box in
the video frame are known, every frame can be sampled back onto the cell grid.

Sampling reads a 5x5 patch at each cell's centre and takes the majority:

| character | meaning | test |
|---|---|---|
| `O` | body | warm orange |
| `W` | the ball's white checks | near white |
| `G` | the laptop | neutral grey, mid brightness |
| `K` | dark: eyes, the ball's dark checks, or the background | near black |

`clean()` then fixes up the grid:

- It snaps stray cells and drops single-cell specks.
- It fills single-cell holes.
- It turns the dark cells that the canvas border can reach into empty space.
  Dark cells enclosed by the pet or the ball stay as `E`.

The recording runs at 15 fps held over a 30 fps video, so consecutive identical
frames are collapsed into one pose with a duration.

## Drawing (typing)

`draw-typing.py` builds each pose from a few parts:

- the front view (`front()`);
- the sitting profile with its shaded far side and folded legs (`profile()`);
- the open laptop on the floor (`laptop_ground()`).

On top of those parts, it adds the cells that are specific to each pose. The
coordinates were measured by fitting the pet's edges in the recording. The cells
there are 3.875 video pixels wide and 3.75 tall, so a single `K` drifts by
almost a cell across the laptop.

## Usage

```sh
# soccer: video -> PNG frames -> src/sprites/frames/soccer.ts
# (macOS; uses AVFoundation, no ffmpeg needed)
swift tools/extract-reference-frames/extract-frames.swift 踢足球.mp4 /tmp/soccer 30
python3 tools/extract-reference-frames/build-frames.py /tmp/soccer

# typing: -> src/sprites/frames/typing.ts
python3 tools/extract-reference-frames/draw-typing.py
```

`CALIBRATION` at the top of `build-frames.py` holds the soccer recording's
`(K, OX, OY)`. To calibrate a recording, print the pet's edges and fit them:

- The torso is 16 cells wide.
- The four legs are 2 cells wide on a 4-cell pitch.

So `K = (arm span) / 24`, and `OX` is the left edge of the arms. Fit the
width and the height separately.

## Checking the result

`node scripts/render-sprites.ts anim typing out.png 7 8` renders every frame
of an animation through the app's own sprite pipeline. Put that next to the
recording's frames at the same size and compare them by eye.
