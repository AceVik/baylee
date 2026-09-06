---
name: game-visual-design
description: The visual half of a game client — palette, readability of colour identity, drawing text and numbers where there is no DOM, table and card materials, and how to prove a render claim by measurement. Use when choosing colours, drawing something on the 3D table or the HUD, adding a badge or a numeral, or when the screen looks wrong.
---

# Visual design for a game client

`game-ux` covers what a player understands and clicks. This covers what they
*see*: colour, contrast, materials, and the numbers on a card. The two are
easy to confuse and have different failure modes — a screen can be perfectly
usable and unreadable, or beautiful and impossible to act in.

## Colour carries meaning before it carries mood

In a card game, colour is already load-bearing before any designer touches
it: colour identity is a rule, not a theme. So the palette has to be chosen
around what must stay distinguishable, and everything else fits in the gaps.

- **Never light card art.** Scene lighting on artwork shifts hue, and a
  shifted hue is a card whose colour identity misreads. This repo draws the
  table `unlit` for exactly that reason, and the stage has no light in it at
  all.
- **Reserve a channel per question.** "Whose turn is it", "who is everyone
  waiting for", "what can I do with this card", "what is this card" are four
  questions. If two of them use the same visual channel — both a border glow,
  both a tint — the player has to guess which is speaking. This client
  separates them: a keyword sheath holds still, an invitation to act travels
  around the border, an armed action stops travelling.
- **Semantic colour is not the accent colour.** Good/warning/critical must
  survive a change of theme and must not collide with the accent.

## There is no text on a 3D table

A canvas has no DOM, so every glyph is a decision:

- Numerals on a card can be a small stencil (this repo uses 4×6) rather than
  a font. A stencil is legible at the size a card actually occupies and costs
  no font atlas.
- Symbols that have no single glyph have to be composed. A hybrid mana pip is
  one disc with two glyphs clipped to opposite halves, because no font ships
  a hybrid mark.
- A licensed symbol font gives you the **mark**, not the colour. The coloured
  disc behind it is yours to draw, which is also what keeps it legal.
- Anything that changes per frame per object belongs in the **material key**,
  not in a second pass. Packing three small numbers and two flags into one
  `u32` means a creature that took damage becomes a different material and
  redraws with nothing else to do.

## Geometry needs tests about geometry

A card that renders as a bright X is not a transform bug, and a test that
asserts the transform will pass the whole time. Mesh construction — corner
arcs, winding, UVs — needs its own assertions. This repo shipped a card quad
whose corner arcs each swept the neighbour's quarter turn; the "an untapped
card lies flat" test stayed green throughout.

The same applies to a bound that only goes one way. "Dark enough" let a felt
texture ship four times too dark. Bound both ends.

## Measure before you theorise

When something renders wrong, the cheapest first move is a reference that
uses none of your code. Set a clear colour and read the pixel back: a clear
colour touches no material, texture or shader, so a discrepancy there is the
pipeline and not your work. This repo found a whole-screen colour problem
that way — a red clear colour rendering `(234, 51, 35)` in stock Bevy and
`(62, 19, 21)` here.

Two more measurements worth the minute they cost:

- **A full-frame diff proves an animation claim.** Two frames, peak
  difference per channel, plus the counter-test with the animation off.
- **Composite the arithmetic offline first.** Working out a tint over a
  measured background in a scratch script beats rebuilding the client per
  attempt.

And the failure that no measurement of *materials* would ever have found: the
table shipped as a black screen because an opaque panel the width of the
canvas defaulted to open. Check what is in front before debugging what is
behind.

## Frame the scene against the part of the window it is seen through

HUD overlays — a tab strip, a hand bar, a phase rail — can cover a quarter of
the window. A camera aimed at the middle of the playing surface then puts the
most important part of it underneath the hand bar, on every screen. Compute
the rig from the content's extent *and* a description of what the HUD covers,
and reapply it as the content changes until the player takes the camera over.

## Generate ornament rather than borrowing it

Ornament is the easiest thing to copy by accident and the hardest to defend.
Arithmetic borrows nothing: a seeded value-noise fbm gives every player the
same felt grain with no asset, no licence question, and no download. Seeded,
because two players on the same table must see the same surface.

## Responsive means one function, not many breakpoints

Pick a frame — phone, tablet, desktop — from the width, and take *every*
size from it. Scattered breakpoints drift; one metrics object cannot. And on
a phone, a touch target is 44 logical pixels whatever else changes.

## One palette, in one place

A colour written at the call site is a colour nobody can change. Keep them in
one module (`palette` here) and name them for **what they mean** — the mood of
a seat, the standing of a player, an invitation to act — not for what they
look like. A constant called `GOLD` has to be renamed the day the accent moves;
one called `ACTIVATABLE` does not.

That naming is also what makes a contrast rule checkable: a test can assert
that two constants stay distinguishable, and it can assert bounds in **both**
directions. A one-sided "dark enough" is how the felt shipped four times too
dark.
