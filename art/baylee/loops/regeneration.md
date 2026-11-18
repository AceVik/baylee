# Both vista loops, to be made anew · prepared 18 November 2026

The owner rejected both loops of 18.11.2026 the same day. Nothing has been
generated for this yet: the Artlist account held 110 of 40,000 credits on
18.11.2026 and renews on 17.12.2026, and a generation is paid for only after
the owner says so. This file is the preparation: what was wrong, what the
next generations are asked for, and what a clip must pass before it is cut.

## What was wrong

**The gate (`world-loop.mp4`).**

- *The leaves stand still at one point.* Measured on the source: the motion
  between neighbouring frames halves from about frame 240 on (the model lets
  its motion die towards its end frame, because start and end frame were the
  same still). The cut used frames `36..=239`, up to the edge of that, and the
  cross-fade then blends the slowest frames into the liveliest.
- *The loop does not close.* The wrap (last frame to first) measures
  33.8 dB PSNR where neighbouring frames measure 38.8–42.6 dB: a visible
  jump. A 36-frame cross-fade cannot hide leaves that are in different
  places in the two halves; it shows them twice.

**The tavern (`tavern-loop.mp4`).** The owner's words, in substance: the dance
is dull and the dancer's dress moves like no cloth does; the children hop
like sparrows; the barkeeper polishes his glass as if in a panic; the
barmaid's smile is frightening. All four are one fault: too many people
asked to *act* in one ten-second image-to-video clip. The model spends its
motion on five figures at once and gets none of them right, and faces are
where that shows first.

## What is asked for instead

One rule for both: **ask for less motion, and for motion that has no
beginning and no end.** Wind, light, smoke, water and fire loop by nature;
a gesture does not.

### The gate

- Start frame only, no end frame (an end frame is what made the motion die).
  Twelve to sixteen seconds if the model offers it, so that the cut can avoid
  the first second (the model's settling) and the last two.
- Prompt (English, for the image-to-video model):

  > Locked-off camera, no zoom, no pan. A steady, even breeze from the left
  > the whole time, never gusting and never stopping: leaves and petals
  > drift across the frame at a constant, gentle speed, new ones entering
  > from the left edge as others leave on the right. Tree crowns and grass
  > sway slowly and continuously. Clouds drift slowly to the right. The
  > runes on the gate glow with a slow, even pulse. Water glints. Nothing
  > else moves; no people, no birds landing, no change of light, no change
  > of composition. Painterly, the look of the still, kept exactly.

- Negative: `camera movement, zoom, gust, wind stopping, freeze, slow
  motion, time-lapse, flicker, morphing, new objects, text`.
- The loop is then closed on the *drifting* layer, not by a cross-fade of the
  whole picture: choose a cut where the frame difference is at its steady
  level on both sides (see "Acceptance"), cross-fade only as long as a leaf
  needs to cross a tenth of the frame, and check the wrap against its
  neighbours. If the model offers a seamless-loop option, take it and still
  measure.

### The tavern

- People are **still life, with small life**: nobody dances, nobody hops,
  nobody polishes. The room moves instead.
- If the start frame shows the dancer mid-step and children mid-jump, the
  still is changed first (an image edit of the owner's painting, same style):
  the dancer becomes a listener at a table or a fiddler holding a long note,
  the children sit on the floor with the cat or watch the card game, the
  barkeeper leans on the counter with a cloth over his shoulder, the barmaid
  carries a tray and looks down at it with a closed, calm mouth. No face
  looks into the camera and no mouth is open: a closed mouth cannot go wrong.
- Prompt (image-to-video, start frame only):

  > Locked-off camera, no zoom, no pan. A quiet evening in a warm tavern.
  > The fire in the hearth burns steadily, candle flames waver gently, light
  > flickers softly on the walls. Thin smoke and steam rise slowly. Dust
  > floats in the light. The people stay where they are and barely move:
  > slow breathing, a small nod, a hand that turns a card over once, a cat's
  > tail that swings slowly. No one walks, dances, jumps, laughs or talks;
  > faces stay calm, mouths closed. Cloth hangs still. The playing cards
  > keep their own frames and backs, with no text. Painterly, the look of
  > the still, kept exactly.

- Negative: `dancing, jumping, running, fast motion, laughing, open mouth,
  teeth, talking, camera movement, zoom, morphing, extra limbs, new people,
  text, logos`.
- If a moving figure is wanted after all, it gets a clip of its own later
  (one figure, one action, composited by mask), never five in one clip.

## Acceptance, before anything is cut or encoded

Measured on the untouched download, per clip:

1. **No dying motion.** The mean absolute difference between neighbouring
   frames, averaged over one-second windows, stays within ±25 % of its
   median over the span that is used. A span where it falls to half is not
   used.
2. **The wrap is a neighbour.** After closing, the PSNR of last frame to
   first lies inside the range of the neighbouring pairs of that clip (for
   the 18.11. gate: 38.8–42.6 dB), not below it.
3. **Faces.** Every face is looked at on a contact sheet of one frame per
   half second at full resolution. One frame with a wrong mouth or eye
   rejects the clip.
4. **The owner sees the source clip** before the 4K60 interpolation and the
   AVIF encode are spent on it.

The legal frame is unchanged (`docs/legal.md` §"New 4K vista loops"): the
owner's own paintings, the owner's own paid account, no stock catalogue, no
Wizards card frame, back, symbol or text.
