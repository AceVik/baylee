# Where the client is going

This is the design programme for the duel client: what it should look like,
how it should be operated, and what is still missing before it is a complete
Magic client. `docs/client.md` remains normative for what the client *is*;
this file is normative for what it is becoming, and the two are meant to be
read in that order.

It was written from three parallel read-only audits of the tree at `f0b0e81`
— visual language, interaction model, completeness — plus verification of the
claims that carry the most weight. Where a finding is quoted below it names the
file it rests on. Where a number was checked against the code rather than
against a document, it says so, because two of the numbers in `CLAUDE.md` were
wrong and one of them sent an audit down the wrong path.

Two facts bound every proposal here and were verified rather than assumed:
the workspace is **Bevy 0.19** (`Cargo.toml:97`), and the wasm build renders
through **WebGL2**, not WebGPU (`Cargo.toml:107` lists `webgl2`; there is no
`webgpu` feature and `index.html` overrides nothing). So every shader below
obeys what `card_common.wgsl` already obeys: uniforms only, no storage
buffers, no compute, compile-time loop bounds.

---

## 0. Three locks, before any of the design

A design programme that begins with palettes while the table freezes is a
programme that will not be believed. Three faults lock a real game today. Two
were found by audit and confirmed by reading the code; the third is the one I
had been chasing by hand.

### Lock A — a refused action removes the question

`flush_outbox` clears `duel.interaction` the moment it sends
(`crates/baylee-client/src/lib.rs:630`). If the engine then refuses the
action, nothing replaces it:

```rust
// crates/baylee-engine-server/src/lib.rs:241
// An illegal action is the seat's problem, not the game's: `act`
// already refused it and nothing moved.
let Ok(routed) = session.act(player, action) else {
    return Vec::new();
};
```

The reasoning in that comment is sound about the *game* and wrong about the
*client*: nothing moved on the table, but the client had already thrown the
question away. The seat is left with no prompt and no way to answer.

Locally that is a freeze. Over the network it is worse than a freeze: the
decision clock keeps running in the engine process and on expiry the house AI
answers for the seat (`crates/baylee-gamehost/src/session.rs:313-331`). A
refused click does not stall your seat — **it spends your decision.**

It is reachable from an ordinary click. `is_selectable` accepts any `ObjectId`
when `options` is empty (`interaction.rs:566`, the discard and bottom-of-library
case), and `activate_card` falls through to `toggle` for a permanent with no
abilities (`input.rs:79-90`). So tapping a land during a cleanup discard
selects it, Confirm sends it, the engine refuses it, and that seat is done for
the rest of the game. Offer-a-Draw is a second door to the same room: the
button is enabled unconditionally (`hud/overlay.rs:235`) while the engine
requires the offerer to hold priority (`engine/actions.rs:15-20`).

The existing test asserts that a rejection *surfaces* (`host.rs:380-392`). It
does not assert that the player can still answer.

**This is consistent with the fault I measured live yesterday, and not yet
confirmed as its cause** — a legal land under the cursor that no key and no
click would play, with `last_error` null and the phase advancing on its own.
The advancing phase fits: it is the house AI taking the seat. One thing does
not. That `/state` dump showed `interaction` *present*, with `lands`
containing the card; Lock A predicts `interaction: null`. Both can hold only
if the AI answered and a fresh `Pending` arrived between the key and the read,
which is plausible and unmeasured. The `outbox` / `mana_run` / `ability_menu`
fields added to `/state` today are what settle it: a queued-but-unsent action
or a mana run owning the keys are the two other shapes that look identical
from outside. Fix Lock A regardless — it is a real defect on its own evidence,
which is the `Vec::new()` at `engine-server/src/lib.rs:248`.

**Fixed.** The repair turned out to be smaller and better placed than the plan
above, which wanted the client to remember `last_pending` and rebuild the
`Interaction` itself. It does not have to: `Session::snapshot` already returns
the outstanding choice for a seat, already sends it *only* to the seat being
awaited, and is documented read-only precisely so a reconnect cannot pump an AI
seat's turn. So both hosts just ask it again —

- `LocalHost::submit` pushes the `Failed`, then feeds `snapshot(seat)` through
  the same `host_message` decoder its normal traffic uses.
- `EngineRunner::refused` sends an `Error` frame plus that snapshot, through
  the existing `route`, so an unattached seat is still filtered out.

Fixing it in the *host* rather than in `Duel` means the client keeps no new
state and the rule holds for anything else that ever speaks the protocol.
A `Phrase::WhyEngineRefused` on the prompt's second line is still worth having,
but it is now cosmetic rather than load-bearing.

### Lock B — questions whose options are drawn nowhere

`PlayerView::looking_at` is filled by the gamehost for library searches, scry,
dig, wish and reorder (`gamehost/view.rs:253-269`). I checked what reads it in
the client:

```
grep -rn 'looking_at' crates/baylee-client/src crates/baylee-client-core/src
→ table.rs:240    ….looking_at(look, Vec3::Y)      Transform, the camera
→ table.rs:623    ….looking_at(Vec3::ZERO, Vec3::Y) Transform, the camera
→ test_support.rs:97   looking_at: Vec::new(),      test scaffolding
```

Three hits, unfiltered: two are Bevy's `Transform::looking_at` and have
nothing to do with the field, and the third initialises it empty in a test.
**Nothing has ever rendered it.** So a
`ChooseCards` from a library search, and *every* `OrderObjects` (both are
library reorders, `resolve/mod.rs:512,904`), draws a headline and
`HintClickBoard` over a board containing nothing to click. The prompt bar shows
its OK button only when `can_confirm()` is true, so what a player gets is a
dead bar. The gamehost even names the missing consumer in a comment at
`view.rs:814`: "without `looking_at` the dialog is a row of blanks."

The same hole covers every target in a graveyard, in exile, or on the stack —
counterspells and reanimation cannot be aimed at all, because the stack panel
is `Pickable::IGNORE` throughout (`hud/stack.rs`) and graveyards reach the
board model only as counts (`board.rs:320-323`).

Fetchlands landed in `f0b0e81`. This is now in most decks.

### Why CI is green over both

`crates/baylee-client/tests/duel_flow.rs` answers `ChooseCards`,
`ChooseTargets` and `OrderObjects` by hand-building the `PlayerAction`. Those
are exactly the variants whose options can live off the board. A test that
constructs the action proves the engine accepts it, not that a player can
produce it — which is the reason `docs/client.md` already gives for the colour
case, applied there and nowhere else.

The repair that closes the class is an invariant, not a fix: **every option in
`Interaction::selectable()` must be drawn**, computed from the board model,
the zone-browser model and the chooser rows, never from the renderer. That test
fails on both locks and on every future one of the same shape.

### Two corrections to `CLAUDE.md`

- `VIEW_VERSION` is **9** (`crates/baylee-view/src/lib.rs:41`); the file said 7,
  then 8. Bumped to 9 by `PlayerView::priority_held`. One audit worked from
  the stale number, which is what a stale contract file costs.
- The copy limit was enforced **per row** in the gateway (`main.rs`) and **per
  card** in the client (`deckbuilder/builder.rs`). Two printings of one card
  therefore saved with eight copies — through the side that is supposed to be
  doing the enforcing, so it was a way to cheat rather than a display bug.
  **Fixed:** `parse_deck_lines` now accumulates per `CardIndex`.

  This one is not a `CLAUDE.md` correction, and calling it one was my own
  error. The file says "the copy limit stays on the card, as the gateway
  enforces it" — which was a true statement of the *intent* and a false
  statement about the code, because the gateway did not do it. The file is
  correct as of the fix; nothing in it needs changing. Worth keeping as a
  reminder that a doc which disagrees with an enforcing code path is a bug
  report about the code, not a typo in the prose.

  One question that fix deliberately does **not** answer: both sides count per
  *list*, so four in the deck and four in the sideboard is eight legal copies.
  Real tournament rules count the two together. Changing that means changing
  `DeckBuilder::add_print` in the same breath, because `CLAUDE.md`'s rule for
  the builder is that a live save button always saves — so it is its own
  change, with its own test, and not a thing to slip into a bug fix. Noted
  here so it is not lost.

---

## 1. The design language

### 1.1 The table, read numerically

The hearth ring is `HEARTH_SIZE` 34 with its band at 0.46–0.60 of the
half-size: a ring roughly 20 table units across on a 44-unit felt, with 24
ticks and two hairlines, and the medallion's two rings inside it. It is the
largest, brightest, most detailed object on screen, and the eye reads a
roulette wheel. Nothing in the frame says *cards are played here*.

The felt is not the problem — `CLOTH` (0.098, 0.165, 0.122) is a decent
billiard green — the `WARM` candle pool at α 0.12 sitting on top of it is,
which is why the middle reads olive and the corners read green.

First moves: `HEARTH_SIZE` 34 → 18, 24 ticks → 8 (a compass, not a clock), one
hairline, `WARM` α 0.12 → 0.06, medallion glows α 0.55 → 0.35. The measurable
bound: in a full-table shot the ring must be smaller than the nearest seat's
mat.

And the prerequisite that makes all of it moot until it is done: the local
seat's battlefield is underneath the hand bar.

**Done — both, and the second one turned out to be the whole of it.**

- Every number above is applied, and `HEARTH_TICKS`, `HEARTH_INNER` and
  `HEARTH_OUTER` are now constants in `tabletop.rs` rather than literals the
  renderer and the tests each wrote out separately (they disagreed: the
  renderer asked for a 0.46–0.60 band and every test asserted against
  0.55–0.72). `WASH_SIZE` is derived from the ring instead of written down,
  because a wash wider than the ring it washes reads as the table being that
  colour. The bound is a test:
  `table.rs::camera_tests::the_hearth_ring_is_no_bigger_than_a_seats_ground`
  (renamed, and corrected — see below: it was measuring the wrong edge).
- The hand bar was not a *rendering* problem. `CameraRig::default` was
  `distance: 20, target: (0,0)` — a hard-coded shot of the middle of the felt,
  taken against the **window**, while the tab strip, the hand bar and the phase
  rail are overlays covering about a quarter of it. On a 1728×1052 laptop the
  local seat's mat projected below the hand bar's top edge; on a phone it was
  worse. `CameraRig::home(layout, canvas)` computes the shot instead, from
  `TableLayout::extent` (every pod's box, rotated by its `facing` — a seat on
  your left plays *across* the table) and a `Canvas` naming what the HUD
  covers.
- The arithmetic is worth writing down because it is exact rather than tuned.
  With the eye at distance `D`, the lean `L`, and `C = 1/√(1+L²)`, a felt point
  `s` units from the look point along the screen-vertical has camera-space
  `depth = D + L·C·s` and `height = C·s` — the cross terms cancel. So
  `s = D·ground(q)` is **linear in `D`**, and the distance at which the far
  edge lands under the tab strip and the near edge above the hand bar is one
  division, not a search. `table.rs::camera_tests` projects the four corners of
  every pod forwards, at two to eight seats and on a phone-shaped window, and
  asserts each lands inside the visible band. The forward projection is written
  out again in the test on purpose: reusing the inverse would agree with it
  however wrong both were.
- Sideways the fit is measured at the table's **near** edge. A perspective
  camera sees less felt where the felt is closer, so the band under the front
  row is narrower than the one through the middle, and measuring in the middle
  put a four-seat table's outermost mat past the rail — caught by the test, not
  by a screenshot. With the far edge pinned the near edge's depth is linear in
  `D` too, so this is still one division.
- The eye distance is clamped **before** the look point is derived from it,
  and that ordering is load-bearing. Aiming for a camera the clamp then moves
  is the one way this arithmetic can put the table off screen while every
  formula above is still right: the far edge would be pinned for an eye that
  is not there, and land above the tab strip. Clamped first, a table too big
  for `MAX_DISTANCE` keeps its far edge pinned and overflows at the *bottom* —
  the graceful direction, and the one a player can pan out of. A four-seat
  table on a phone is that case, and it is a `Canvas` problem rather than a
  camera one: at the width it needs, the felt's own edge comes into frame. The
  fix is tranche 4 turning the vertical rail into a horizontal strip, and the
  phone test asserts the overflow as the deliberate gap it is.
- `frame_table` reapplies it when the seats, the focus or the window change,
  and stops as soon as the player has aimed the camera themselves — a rig equal
  to the last framing, or still `default()` (what the resource starts as and
  what `navigate_home` asks for), is the table's; anything else is theirs.

**Photographed, because a test proves the arithmetic and not the picture.** A
live duel driven through `dev-control`, with a temporary probe stamping a
`Plate::Fight { 2, 4, 1 }` and two chips onto every placement so lands carry a
corner, settled three things at once:

- The framing holds on a real table. Both mats sit whole inside the visible
  band, the local mat clears the hand bar with felt to spare, and the hearth
  ring is visibly smaller than the nearest mat.
- **The plate and the chips had never been rendered on a 3D card before**, only
  through `card_ui.wgsl` on a hand card five times the size. They draw
  correctly at table distance: `2/4` with the damage fill rising a quarter of
  the way, a blue `8` chip and a green three-pip chip stacked above it. At
  device pixels they are sharp — `number_cover`'s antialiasing needs no
  distance term. At 1× DPI the whole card is 64×90 logical pixels and the
  plate is a smear; nothing an `aa` term fixes, because the plate cannot grow
  without covering the art. That is what the hover preview and the detail
  panel are for, and it is the argument for finishing them.
- One defect the camera work did not cause and the test suite could not see:
  **playing the card under the pointer left its preview standing over the
  middle of the table.** The first reading of the photograph was that a hand
  card's node outlives the card, which is wrong — `sync_overlay` despawns the
  whole overlay and rebuilds it on every revision, so there is no stale node.
  What outlives the card is `duel.hovered`. `pointer_hover` clears a hover
  only on an `Out`, and Bevy fires no `Out` for an entity that has been
  despawned; the pointer has not moved, so nothing else speaks either. The
  ghost *was* the preview.

  The fix is in `pointer_hover`: the hover is now held against the **kind of
  entity that reported it** (`HoverSource::{Hand, Table, Elsewhere}`) and
  re-checked every frame, before the grace window, because the pointer's
  stillness is the whole problem. The source is what makes it answerable — a
  land goes on existing under the same `ObjectId` after it is played, so
  "does this object still exist" would have found it on the battlefield and
  kept the preview open. It is no longer *a hand card*, and that is the true
  sentence.

  The first cut of that fix reintroduced the very stall §"The pointer only
  speaks when it moves" is about, which is worth recording because the shape
  recurs: **a source is only usable if the code can tell when it is not the
  author.** `hovered` has four writers; the keyboard cursor walking off a
  permanent onto a hand card would have met a stale `Table` source and been
  cleared a frame later. So the system remembers the value it left and treats
  any difference as somebody else's, resetting to `Elsewhere` — whose union of
  hand and table is not a weakened version of the other two but the keyboard
  cursor's *own* invariant. A pointer hover holds while the pointer is over
  the entity that reported it; a keyboard cursor holds while its object is
  anywhere in `cursor_grid`, which spans the hand and every pod's lanes. An
  `ObjectId` survives a zone change, so a card played off the cursor is still
  in the grid one row down, and clearing it there would not remove a ghost —
  it would drop the player's cursor and send the next arrow key back to the
  first card in hand.

- And the *second* thing in that photograph, which the hover fix did not
  touch: **a closed own-board overlay was showing the tops of your own
  permanents.** One card top per permanent, clipped to its title bar and
  standing above the hand bar — which is exactly what a card left behind by a
  card you had just played would look like, and why the two were read as one
  defect for as long as they were. `animate_overlay` parks the closed panel
  at `window − HAND_BAR_H − 14`, so it is `KNOB_H` tall and should show its
  handle and nothing else; the lanes inside it were children of the panel with
  no clip. The number 14 was written out three times — the knob's height and
  the closed `top` in two places — which is how a panel could be shorter than
  the thing inside it without anything saying so.

  Two lines of the fix are worth keeping. The lanes went into their own box
  that clips on `y` only, because a row outgrowing the panel *sideways* is a
  different question and hiding its tail would be the lie rule 3 is about. And
  that box needs `min_height: 0`: a flex item's automatic minimum size is its
  content, so the first attempt grew the box to hold a full card row and then
  clipped nothing — it moved the picture by a few pixels and was photographed
  again before it was believed.

**And the felt itself, read once more with the numbers rather than the eye.**
Four photographs in, the hearth was still the loudest thing on the table and
its test still passed. Both are explained by one line: the bound compared the
ring against `SeatSlot::lane_width`, which is the mat's **long** edge. A
two-seat table at aspect 1.78 gives every seat `half_extent` `(9.40, 2.21)`,
so the mat is 18.8 across and **4.42 deep**, and `HEARTH_SIZE` 18 put a
10.8-unit ring on the felt — two and a half times the depth of a player's
whole board, filling 86% of the 12.58-unit gap between the two of them, and
comfortably under 18.8. The bound was wrong, not the picture.

`SeatSlot::mat_depth` exists now for exactly this: the dimension a mat is
smallest in, and therefore the one anything claiming to be smaller than a
seat's ground must be measured against. `HEARTH_SIZE` is 4.5, so the ring is
2.7, and the bound goes both ways — `lane_height < ring < mat_depth`, wider
than one row of cards and narrower than a mat. One-sided is how the felt's own
brightness shipped four times too dark (`docs/client.md`), and it is how this
shipped two and a half times too big.

The same photographs showed the local mat reading as **brass rather than
gilt-rimmed felt**, and that had a cause worth writing down because the code
denied it in a comment. `tabletop::seat_mat` wrote every pixel `[1, 1, 1]`
with the shape in the alpha channel — field at 0.095–0.150, rim at ~0.77 —
and the seat's colour arrived as the material's `base_color`, which multiplies
the *whole* texture. So the rim was never the part carrying the seat's colour;
the entire mat was that colour, and the rim merely more opaque. `seat_mat`
takes the accent now and crossfades toward it on its own curve — the same
distance from the edge that sets the opacity, but a shallower exponent, so the
hue reaches further in than the ink does. Tying the two together is the
obvious thing to write and it renders every rim off-white: a colour that only
arrives where the rim is already opaque arrives on a handful of pixels. The
material's tint is neutral brightness, and the glow underneath keeps the
accent because spilling it onto the felt is the glow's whole job. The cost is
one 512×256 texture per seat instead of one for the table, which is what
sharing an image was buying and what made the bug unavoidable.

Not a cause, though it looked like one: `zone_brightness` returned up to
`0.95 × 1.38 = 1.311` for the local seat, and an unlit material under
`Tonemapping::None` clips each channel independently, which would turn gold to
yellow. It did not happen — `to_linear()` runs before the multiply, and gilt
`srgb(0.78, 0.63, 0.33)` is linear `(0.571, 0.355, 0.089)`, so the brightest
channel reached 0.749. Worth stating because the srgb arithmetic is the
obvious arithmetic and it gives the wrong answer.

It became a cause the moment the fix above landed, which is the part worth
keeping. Taking the accent out of the tint makes that number a multiplier on
**white**, where 1.311 and `0.95 × 1.15 = 1.0925` both clip to 1.0 — a local
seat holding priority and a local seat merely taking its turn drawn as the
same flat white, which is the one distinction the mat exists to draw. So the
scale is bounded at 1.0 and `local` became a small lift rather than a
separate base, since which mat is mine is now genuinely answered by the rim.
Three tests state it. The general shape: **a value that was safe because of
what it was multiplied by is not safe once you change the multiplicand**, and
the two edits sat in different files with nothing connecting them.

Two more literals fell out of the same change. `MEDALLION_SIZE` was a bare
9.5, inside a ring of `18 × 0.46 = 8.28` only by coincidence of the number
above it; at `HEARTH_SIZE` 4.5 it would have drawn a 9.5-unit colour wheel
around a 2.7-unit ring, the ornament swallowing what it is inlaid in. It is
derived from the ring now at the proportion the table already had, as
`WASH_SIZE` already was — the rule being that anything positioned *inside*
the lamp is measured from the lamp.

**One clause the same probe settled about the chips.** The overflow chip drew
nothing when the probe asked for `more: 1` beside a single chip, which looked
like a lost tail. It is not: `Corner::of_parts` fills all three slots from a
sorted list before `more` can be anything but zero, so `more > 0` with an
empty slot is unreachable, and the reachable state draws at slot 3 exactly as
`packed()` lays it out. The "never hide the tail" rule holds; the probe asked
for a state the model cannot produce.

### 1.2 Tokens

One palette source. "Blue" is currently defined three times — `tabletop::PIE`,
`manaui::disc_color`, `face::frame_color`. `PIE` is already renderer-free and
becomes the single source; disc and frame derive from it and the derivation is
pinned by a test.

| token | hex | purpose |
|---|---|---|
| `felt.deep / cloth / worn` | `#090F0B` `#192A1F` `#243729` | the world's neutral |
| `pie.W U B R G` | `#F0E8CC #4A8CD4 #4D4557 #D45C47 #5CA86B` | colour identity — **no other token may sit on these hues** |
| `gilt` | `#C7A154` | *metal*: mine, my seat, the active rim. Never pulses |
| `candle` | `#FFDB9E` | *light*: the game is asking you. May breathe |
| `ember` / `cool` / `dusk` | `#E65738` `#6B94DB` `#8566B8` | combat / untap-draw / end washes |
| `steel` | `#5C6B80` | indestructible band |
| `jade` / `frost` | `#99E6BD` `#CCDBE6` | hexproof / shroud films, deliberately off-pie |
| `slate` | `#0F1214` α .80 | chrome; blue-black so it reads as *not cloth* |

Two decisions inside that table. **`palette::ACCENT` teal goes** — it is the
one hue on screen that comes from nowhere in this world, and "the game is
asking you" becomes `candle` while "mine" stays `gilt`; the two are told apart
by metaphor (light versus metal) and by behaviour (candle may breathe, gilt
never does). And **offers become one hue at two energies**: today playable is
gold and reachable is indigo, which spends a second hue on a distinction that
is really about certainty. Both become `candle` — the engine's yes at full
strength, the client's "I could tap for that" at half with a dashed perimeter.
`dusk` is then free to mean only end-of-turn.

Type: three faces are shipped and there is no fourth. A serif display face is
the cliché to avoid and fails at the 8–10 px this game sets card text at;
Beleren is not ours to imitate. Inter carries every role, separated by size,
weight and tracking rather than by family — and every number that changes gets
`tnum` (`TextFont::font_features`, `FontFeatureTag::TABULAR_FIGURES`,
verified in `bevy_text-0.19.1`), which nothing currently sets. Life totals
that jitter their neighbours are a legibility bug, not a nicety.

### 1.3 The keyword algebra

This is the question that was asked directly — how do several animated
keyword borders compose without becoming noise — and the answer is that they
must not compose at all. Colours that mix are the failure mode: six glows
summing to a grey halo, on a board of thirty such halos strobing at unrelated
rates, next to a card whose colour identity is now unreadable.

So instead: **eight channels, one keyword per channel, and hue is not one of
them.**

| | channel | capacity | nature |
|---|---|---|---|
| C1 | band **material** (inner 60% of the border) | one | continuous |
| C2 | band **film** (outer haze) | one | continuous |
| C3 | **perimeter light** (travelling) | one | transient — an offer |
| C4 | **face treatment** | one | this-turn state |
| C5 | **elevation** | one | geometric |
| C6 | **rail** marks | ordered list | discrete, countable |
| C7 | **corner plate** | numbers | P/T, damage, counters |
| C8 | **shadow tint** | one | selection, targeting |

Five rules hold it together:

1. One keyword, one channel — never two.
2. A continuous channel holds at most one claim; two claims are resolved by a
   **dominance table**, engine-truthfully, and the loser is not drawn.
3. A discrete channel shows *all* claims, shrinking slot width, never hiding —
   a tail nobody can see is a lie by omission.
4. **Hue is not a free channel.** The five pie hues are colour identity;
   films live off-pie (`jade`, `frost`, `steel`).
5. **Motion is not a free channel either.** Everything that says "is" breathes
   on one shared `BEAT` with per-effect phase offsets; only the *offer*
   travels, so a player can find it with peripheral vision because it is the
   only thing moving along an edge.

The ceiling is **three continuous claims legible at once** — one band, one face
treatment, one elevation — plus the transient offer. There is deliberately
nothing left to assign a fourth to.

**Dominance.** `glow_bits` already drops hexproof under shroud, which is
correct: a shrouded object cannot be targeted by anyone, so hexproof adds
nothing. Two more collapses of exactly the same kind are currently drawn as two
marks where one is a fact — **double strike ⊐ first strike** (CR 702.4: the
first-strike mark carries no information the double-strike mark does not) and
**unblockable ⊐ menace** (menace's only effect is a restriction on being
blocked). Flying does *not* collapse under either, because it also decides what
the creature can block. No other collapse: hexproof and indestructible are
different facts and both draw — which is what having C1 and C2 as separate
channels is *for*.

**Shroud + indestructible, the case that had to work.** Today `film_amount`
0.88 over a base buries the steel and a player sees a lavender rim. The fix is
sub-lanes: the base occupies the inner 60% of the band at full strength with
its specular line intact, the film becomes a soft outer haze peaking at the
card's edge, and the film's alpha is **capped at 0.55 whenever a base is
present**. Shroud's haze is already noise-modulated, and that noise is what
lets the specular through — mist over metal is the right picture. The
measurement: at 90 px card width the steel specular must be visible along the
top and bottom edges through the frost. Below α 0.40 the film stops reading as
a film; that is the floor.

**Flying is elevation, not colour.** It is the keyword a player scans a board
for, and its picture is geometric: the card floats, the contact shadow stays on
the table, and the *gap* is the cue. `FLYING_LIFT` 0.05 goes into the base lift
in `card_transform`, never into the hover/selected `+=` lines, or a hovered
flier would move 0.036 and read as nothing. The rail mark stays too — rule 3 —
exactly as a real player both sets the card on a die and reads the word.

All 20 keywords in `keyword_tests::ENFORCED` get an assignment. Three are not
battlefield facts and get nothing on a permanent (flash, uncounterable,
rebound); changeling is a type-line fact and belongs in the type line, not on
the border. Prowess and unblockable become new rail marks 12 and 13.

**Past six marks the rail wraps to two rows** rather than compressing below
7 px, because seven keywords on one creature is rare and the picture should
look rare. And `cardrail::badge_at` — written, and called by nothing — becomes
the hover and long-press tooltip, which is the honest fallback for a pip that
small: the pip says a keyword is there, the word says which.

**The prerequisite nobody had wired — done.** `Preferences::reduce_motion`
stopped `glide` and the camera and was ignored by every shader animation.
`motion: f32` in `CardParams`, multiplied into `t`, fixes all of them at once,
and it is free: the struct was already 48 bytes, because five `u32`s and two
`f32`s come to 28 and a `vec4` has to start at 32. The new field lands in
padding that was there all along. (The estimate above said 32 → 48; the struct
was never 32.)

The rule that made it one number rather than a second pipeline: **at
`motion = 0` every term must land somewhere it could have been**, so that a
still card is the moving one held still and not a different drawing. Stopping
the clock gives the strongest form of that — phase zero *is* the mean —
wherever the term is a pure `a + b·sin(t·ω)`. Where it also carries a spatial
phase the freeze is an honest frame instead: an indestructible border keeps
its catch-light at a fixed height (`sin(t·0.8 + uv.y·3.0)` rests at
`0.72 + 0.28·sin(3·uv.y)`), and a rail mark rests at its own slot's offset
(`t·BEAT + k·0.22`). Both are the picture stopped, which is the claim. Three
terms are neither, and each for a different reason:

- **The offer light** is a *position*, not a brightness. A stopped chase parks
  the head somewhere on the perimeter and leaves the rest dark, which reads as
  a defect rather than as a still version of anything. It degrades to the
  circuit's mean, which is an even ring: ∫₀¹ (1 − 2·min(h, 1−h))⁵ dh = 1/6, so
  `0.22 + 0.60/6 = 0.32`.
- **The will-tap pulse** carries a `− 0.9` phase offset, which is what puts the
  price a beat behind the deed it pays for. Phase zero is therefore two thirds
  of the way *down* the swing, not in the middle of it. That one scales its
  oscillation instead of its clock.
- **The UI foil's `tilt`** stands in for the view angle the table shader gets
  from geometry, and the glint is brightest where the angle is *zero* — a real
  foil catches the light edge-on. Freezing at zero would freeze a hand of
  foils at their most garish. It rests at the angle whose glint equals the
  moving mean instead (E[(1−|sin|)²] = 1.5 − 4/π).

Two things about the wiring that are not obvious. The clock does **not** go in
`CardLook`: that is the material cache's *key*, and a global preference in a
key keeps both answers alive forever and evicts neither. And a change rewrites
the cached materials **in place** rather than emptying the cache — a cleared
cache is only refilled by whatever draws the card next, and a table nobody is
touching draws nothing, so emptying it makes the switch appear to do nothing
until the game moves.

`CardParams` is now written out in three files with nothing between them, and
a wrong order there has no error and no crash: swapping the last two fields
feeds `tint`'s red channel in as the clock and the clock in as a colour.
`card_params_is_the_same_struct_in_all_three_files` reads both shaders and
compares them to each other and to the Rust order.

### 1.4 Counters, and the dice question

The dice idea was evaluated properly and rejected, and I agree with the
reasoning. A die on a slab at `CAMERA_LEAN` is about 6 px tall; nobody reads a
d8 from a d10 at 6 px, and nobody reads it at 60 px either without counting
faces. **Shape is not a number.** What a player actually reads off a real die
is the numeral on top — so a polyhedron costs a mesh, a material, a
light-dependent silhouette and a text label in order to deliver what the label
delivers alone.

But dice do carry one thing worth keeping: **pips**. One to six as dot patterns
are read pre-attentively, faster than numerals and with no glyph at all.

So a **chip, not a die**: flat, round, stamped, die pips up to 6, numerals from
7, in the reserved bottom-right fifth. A chip is also what a counter looks like
when it is *paper*, which is what Magic actually sells, and on a 2.5D table a
flat token sits honestly where a polyhedron does not. Up to three chips are
visible and a fourth kind collapses to "+N" with the tooltip listing them.
+1/+1 and −1/−1 annihilate in the engine (CR 704.5q), so only one of those ever
shows.

Lore counters are the exception that proves the rule: a saga chapter is a page,
not a token, so it gets a *square* parchment plate with a roman numeral.

**Loyalty is not a counter.** It is `PublicObject.loyalty`, its own field, and
it is a planeswalker's life total. It takes the P/T plate's place — same
corner, same plate, same numeral role — with a gilt rim. Explicitly **not a
shield**: a shield-shaped loyalty box in that corner is the WotC planeswalker
frame's own element, and "a plain shield nobody owns" is the argument every
borrowed frame element makes.

And the plainest gap of all, found by grep rather than by eye: **there is no
P/T, no damage and no counter drawn anywhere on a card showing art.** They exist
only on the constructed text face. Three rules facts, invisible on the normal
view of the board. That goes first.

**Done — the plate.** `baylee-client-core/src/cardplate.rs` says what the
corner holds and `plate_layer` in `card_common.wgsl` draws it, which is the
same split `cardrail` already had and for the same reason: the arithmetic
belongs where a test can reach it without a GPU. Four things the
implementation settled that the paragraph above did not.

- *One `u32`, not four.* Three ten-bit numbers and two kind bits is exactly
  thirty-two, so the whole plate is one more uniform beside `glow` — and it
  rides the **material key**, so a creature that is dealt three damage becomes
  a different material and the corner redraws with no second pass. That is the
  trick the glow has used since M1. A number too big to pack **clamps**, never
  wraps: a 40/40 drawn as a 1/1 is wrong and looks right, which is the worst
  thing a board can show. Power is packed with a bias because power is
  genuinely negative on a board.
- *Damage is a fill, not a third numeral.* The plate reads `2/4` and fills
  from the bottom to `damage / toughness`. What a player needs off a blocked
  creature is how close to lethal it is, not an arithmetic problem in two
  numbers — and damage is the one thing on that plate that is not printed on a
  real card, so drawing it unlike the printed numbers is the honest treatment.
- *Numerals are a stencil, drawn from a 4×6 bit grid.* There is no text on the
  3D table and Bevy has no 3D text, so the alternatives were projecting a UI
  numeral onto every card each frame — chasing its tap rotation, its hover
  lift and its place in a stack, which is exactly the desync `Motion`'s "one
  door" exists to prevent — or a rasterised atlas. A stencil is the same
  argument the felt and the eleven marks already make: ornament is the easiest
  thing to borrow by accident and arithmetic borrows nothing. The grid is
  sampled bilinearly rather than tested, so a one-cell stroke has soft sides
  at any size and no staircase at `CAMERA_LEAN`.
- *Loyalty beats power.* The one case is a planeswalker that is also a
  creature. Loyalty is what that permanent dies to and its power says nothing
  about how close it is, so the gilt plate wins and the P/T stays one held
  modifier away. It also had to go into `ObjectSummaryKey`: loyalty is drawn
  now, and two walkers of one name differ by exactly that, so a stack of them
  would have worn one number and lied about the other.

**Done — the chips, and the page.** The column above the plate is
`cardplate::ChipRow` and `chip_layer`, and the two are decided together as one
`Corner` because one of them can silence the other. Four things this half
settled.

- *Colour is the whole of the kind.* A chip has room for a count and nothing
  else, so which counter it is has one channel left, and it is the disc's own
  colour — `Chip::tint`, in the model, where a test can reach it. That is
  deliberately half an answer: it separates the chips on one card from each
  other, and the badge tooltip is what will name them. Pips to six, numerals
  from seven, and the count clamps at 999 rather than drawing its low three
  digits, for the reason the plate clamps.
- *Two words, not one.* Three chips and an overflow, each a tint and a count,
  do not fit in thirty-two bits without capping a count so low a proliferate
  deck would out-count it. The overflow is packed as a fourth chip with a
  reserved tint, so the shader's loop has one shape.
- *`+1/+1` and `-1/-1` keep their chips.* `board::badge_counters` used to drop
  the second on the grounds that the projected numbers already carry it, and
  nothing ever called it. By the time something did, the rule was wrong in both
  directions: a 3/3 and a 1/1 wearing two `+1/+1` plate identically, and so do
  a 3/3 and a 5/5 wearing two `-1/-1`. The chip is the only thing that tells
  them apart, so it is drawn and the function is gone.
- *A chapter is a page.* A saga has no body, so the corner is free — and the
  plate turns square, light and barely rounded, with the chapter in roman
  numerals in sepia. It is the one counter the chip column drops, because the
  page says the same number. Lore counters are only ever on sagas (CR 714), so
  no type line is needed to know one, which is the only reason this is
  expressible: a `CardGroup` carries counters and does not carry subtypes.
  Chapters past `V` fall back to arabic, because `VI` needs a composition rule
  this does not have and an honest number beats a pretty one.

The screenshot found one thing no test could: the roman `V` drawn as
`1001` five times over `0110` renders as a **U**. It needs the tip —
`0110` then `0100` — and `IV` reading as `IU` on a real card is exactly the
class of bug `docs/client.md`'s black-screen entry is about.

### 1.5 States that are not keywords

They take channels no keyword uses, so they cannot collide by construction.
Tapped is rotation; summoning sickness is the face treatment (retimed onto the
shared `BEAT`); targeted and selected are the shadow tint; phased-out is face
alpha.

Combat is **position**. An attacker slides toward the defender's side of its
lane; a blocker slides to meet it; a thin hairline pairs them when there are two
or more defenders. Not a glow, because combat is already a colour on the table
— the ember phase wash — and a red border on an attacker would sit inside a red
room. Position is the one channel nothing else uses during combat, and it is
what a player does with a physical card: slides it forward. `CombatView`
already carries everything needed; no view change.

### 1.6 Motion

Everything moves through `Motion` and `glide`'s `1 − e^(−rate·dt)`, so the
table below is rates, never fixed clocks — 95% settle is `3/rate`.

| event | rate | settle |
|---|---|---|
| enters battlefield, lane repack, tap | 16 | 190 ms |
| hover lift | 22 | 135 ms — a hover is a question, not an event |
| attack advance / block meet | 12 | 250 ms — deliberate; the two arrive together |
| **leaves the board** | 12 | 250 ms — new; today the card despawns on the frame it vanishes |
| camera | 24 | 125 ms |

Shader tempo is the same discipline: one `BEAT` for every "is" breath, half of
it for steel (metal is slow), double for the lethal flash. Six unrelated rates
is the fairground this section exists to prevent.

### 1.7 Day/night, sleeves, playmats

Day/night does not exist in the engine, the view, core or the DSL — verified by
grep. As a rules fact (CR 726) it is engine work, and it is listed in §4. What
belongs here is why it will not fight the phase tint when it arrives: different
*extent* (the phase wash is a lamp over the pool; day/night is a single scalar
multiplied into the felt everywhere) and different *timescale* (phases change
every few seconds; day/night changes once a turn at most and glides three times
slower). Night darkens felt, never cards.

The sleeve seam is three local facts and no protocol: a `SeatStyle` per seat
resolved from a resource that returns the default for everyone today; an
`edge_tint` on `CardParams` so the card's wall can be the sleeve's colour
rather than the scan's edge; and a **generated** back — a guilloché of summed
sines in polar coordinates, seeded, no `rand` — because ornament is the easiest
thing to borrow by accident and arithmetic borrows nothing. What a *remote*
seat's sleeve looks like has to travel in `GameStatic`, which is §4.

---

## 2. The interaction model

### 2.1 One pipeline

`input.rs` is 900 lines of Bevy events, resource reads and early returns in a
fixed system order, and it is where truth gets lost between `Interaction` and
the wire. Yesterday's land-drop fault is the evidence: I proved with a test
driving the real Bevy system that the chain works —

```
test input::tests::the_primary_key_plays_the_land_under_the_cursor ... ok
```

— which means the fault was never in the decision, only in the state the
decision reads. That is a structural problem and it gets a structural answer:

```
Bevy events ──► Gesture ──► resolve(&Gesture, &Snapshot) -> Vec<Intent> ──► apply
  input.rs                    core/gesture.rs (pure, headless)                input.rs
```

A keyboard cursor over a card and a pointer over the same card produce the same
`Hit`. Every HUD button dispatches into the same `resolve`, so a test that
fires `Act(Confirm)` and a test that taps the OK button become the same test —
and `Fired::of_actions`, which exists precisely to let buttons and touch enter
the key pipeline and is called only by tests, finally has a caller.

`Why` is an enum rather than a string and reaches three places: the prompt
bar's second line, `devctl /state`, and the tests. A refusal the engine still
makes after the client has checked is then a bug report with the gesture
attached.

I have already made `/state` report `outbox`, `mana_run` and `ability_menu`,
because all three answer silently and all three look exactly like "the key did
nothing".

### 2.2 The seventeen questions

`Interaction::mode_for` matches all 17 `Pending` variants with no wildcard arm
and `confirm()` builds an answer for every mode. **The model layer is
complete; the renderer is not.** The client makes exactly four things
clickable: battlefield cards, the own-board overlay lanes, the own hand and
command zone, and the player tabs.

Status by variant: eight are fine (`Mulligan`, `LegendChoice`, `ChooseColor`,
`ChoosePlayer`, `YesNo`, `ChooseSubtype` — the last only since yesterday —
`GameOver`, `Priority` in the ordinary case). Four are operable but **blind**:
`MulliganBottom` and `DiscardChoice` show no tally of what has been chosen,
`ChooseCastMode` renders "Mode 1 / Mode 2" with no text, and `ChooseNumber`
never prints its value anywhere while its arrow keys also pan the camera. Two
are **unreadable**: both combat prompts are fully operable and draw nothing,
because the renderer reads `selected()` — which is empty in combat modes —
while `is_selected()`, the method that knows about pairs, has no caller, and
`assignment()` and `focus_position()` have none either. Two **lock**:
`ChooseCards` with off-board options, and `OrderObjects` always.

The fix that covers the locked pair and most of the rest is one thing: a **zone
browser**. A model in `client-core` over `looking_at ∪ graveyards ∪ exile ∪
command ∪ stack`, and a tray panel that reuses `spawn_hand_card`'s node path —
**not a second card renderer**. It opens automatically when a pending's
options include anything outside battlefield and hand, and on demand from a
key or a tap on a pile chip at the mat's corner. It carries the filter box the
subtype picker already proved, and in order mode it answers `OrderObjects` by
click-click swap. One panel closes tutoring, scry, dig, wish, delve, reorder,
graveyard targets and counterspell targets.

### 2.3 The problems of scale

**A stack of thirty.** Consecutive entries with the same source, controller and
targets collapse into runs — Grapeshot's twelve copies become one row — decided
in the model, not the renderer. The top entry keeps its card because it is the
one being answered; everything else is a 24 px line in a scrollable column.
Two Bolts at two faces stay two rows, because the identity test is on the
targets.

**Forty tokens.** `CardGroup` groups by a key that includes damage and status,
which is correct for *drawing* — a damaged Soldier is no longer interchangeable
— and wrong for *declaring*, because one damaged token forks a group into forty
mid-declaration. A coarser `combat_key` for declaration, plus
`toggle_group(members, want)` and a count stepper, turns "attack with 30 of 40"
into one gesture. Which thirty is the engine's problem and no player cares.
`Lane.overflowing` is already computed and ignored; honouring it with a
collapse chip is the rule that a card never leaves the mat.

**A hand of eighteen.** The hand currently re-sorts by playability on every
priority change, which moves cards under the pointer. Draw order is the stable
answer, with playability expressed as a glow rather than a position, and an
explicit sort key for players who want one. A card a player is aiming at must
not move.

### 2.4 Stops, and the pass

The engine already has `PriorityHold::{Always, PassWhenNothingToDo,
UntilStackEmpty, UntilTopOfStack, UntilEndOfTurn}`, and `SetPriorityHold` is
journaled and legal at any time. The only sender today is the gateway replaying
standing answers. That is the whole F6 apparatus, unused.

Three layers: ~~**engine holds from the client**~~ (a key that sends
`UntilStackEmpty{depth}` — cancelled the moment anyone responds, which is
exactly F6 semantics), **the rail as the stop table** with an opt-in
"competitive stops" preset, and ~~**hold-this-turn** as a toggle rather than a
held key, because a held key cannot be a touch gesture~~.

**Done — the first and third, as `F6` and `F7`.** Two actions rather than one,
because "let this stack resolve" and "nothing more this turn" are genuinely
different orders and the engine has both. Three things the implementation
settled that the sketch above did not:

- *Both keys cancel.* Either key, pressed while any hold is running, sends
  `Always` rather than replacing the hold. A player who has stopped being
  asked should not have to remember which key did it.
- *A hold has no other symptom.* The prompt bar is empty because the seat is
  not being asked — which is exactly what an idle turn looks like. So the
  state is drawn (an accent chip beside the concede button) with the way out
  next to it. Without that, a hold set two turns ago and forgotten is a game
  playing itself with nothing on screen to blame.
- *The view carries it, as a bool.* `PlayerView::priority_held`,
  `VIEW_VERSION` 8 → 9, the viewing seat's own hold only — a hold is a
  statement about what its owner will respond to, and the whole table knowing
  it is a read a player is entitled to keep. `PassWhenNothingToDo` reports
  false through `PriorityHold::suppresses`, which `auto_answer` now shares, so
  the indicator cannot disagree with the engine.

**Done — the second, as `RailPreset`.** Two presets rather than one, because a
preset with no way back is a trap: competitive stops turn seventeen of the
twenty-four buttons red and clicking them green again one at a time is not an
undo. `Stop everywhere` is the default said out loud, and `Competitive stops`
keeps seven windows: both of your own main phases, the whole of combat on both
turns, and the end step of an opponent's.

The thing that decided which seven is not taste. **A red row is not a pass in a
declaration step** — `auto_answer` turns it into `DeclareNoAttackers` /
`DeclareNoBlockers`, which is a decision and not a skipped window. So the row
on whichever side actually asks this seat to declare has to stay green:
attackers on its own turn, blockers on an opponent's. A preset that got that
wrong would not stop asking, it would decline every block for the rest of the
game — and the test that holds it is written through `auto_answer` rather than
against the table, because the table is not the claim.

A preset is written as a button, not held as a mode. The chip is nonetheless
lit while the rail still matches, which is the only honest answer to "am I on
competitive stops right now"; the first hand correction puts both chips out.

One bug in that area, found in the audit: `Situation` carries no stack depth,
so a red rail row passes priority *with a spell on the stack*. Red should mean
"nothing to do here when nothing is happening", never "let their sorcery
resolve unanswered".

**The keymap.** This answers the open question I had been carrying. Today
`Primary` is Space and means "the hovered card if any, else pass" — a pass key
whose meaning depends on where the mouse happens to be resting. That is a
misclick generator, and it is the same precedence chain that made my land
mysteriously not play. **Space becomes `Confirm` (pass / OK) and Enter becomes
`Primary` (act on the cursor card)**, which is also the MTGO and Arena
convention. It is a swap of two rows in `Keymap::standard()` and no new action.

### 2.5 Undo, and its absence

There is no undo in the engine and there should not be one: a journaled,
deterministic kernel does not roll back. So the client's job is to make the
irreversible **two-stage**. ~~A tap on a spell *arms* it — the card lifts, its
mana plan is drawn as rings on the lands it would tap, and a second tap or
Enter sends it; Escape disarms with nothing on the wire.~~

~~**Mana abilities are the exception and stay one tap**, because floating mana
is the one cheap mistake: it empties at end of step and a wrong colour is fixed
by tapping another land. Exactly the entries in `legal.mana_abilities` — today
any lone ability goes through on the click that found it, including "Sacrifice
this:".~~ ~~Concede has no confirmation at all today; one misclick ends a
ranked game.~~

**Done.** `Duel::armed` holds an [`Armed`] — an
`ObjectId` and a `Deed` (`Play`, `Ability`, or a mana `Run`) — and the second
tap on the same card, the confirm keys, or the button in the prompt bar send
it. Everything is resolved against the *current* `LegalActions` at every one
of those points rather than trusted from the tap that armed it, which is the
rule `ManaRun` already followed step by step; a deed the engine has withdrawn
disarms instead of guessing. Cancel is first in the Escape order, because an
armed deed is the only state in the client with nothing on the wire behind it.

Two decisions worth writing down. The exemption is **not** `legal.mana_abilities`
after all: that list carries the CR 305.6 shortcut and granted abilities but
not a printed `{T}: Add {G}`, so a mana dork would have asked for a
confirmation a basic land does not. It is `AbilityOption::mana`, read off the
card's own `mana_ability` flag (CR 605.1) — and not off the source list either,
which reduces a permanent to the *one* tap it usually has: Yavimaya Coast
prints two mana abilities, and the second would have asked. And picking from the
**ability chooser arms** rather than sends — the chooser disambiguates, it does
not confirm, and a lone "Sacrifice this:" arming while the same ability among
three did not would be exactly the inconsistency a player trips on.

The *drawing* is two more bits in the `glow` word — `ARMED` and `WILL_TAP`,
bits 5 and 6, below `MARK_SHIFT` where three were free — and it is the same
sentence in two halves: the armed card says what will happen, the lands the
plan would spend say what it will cost. Drawn rather than written, because
"Tap 3, then cast" does not say *which* three and which three is a plan the
player never made.

Both ride in the border's outer register beside `ACTIVATABLE`, and the
grammar there is now three words rather than one. `ACTIVATABLE` travels — it
is an invitation, and a travelling light is what the eye finds across a whole
board. `ARMED` holds still and pulls in tight against the printed edge: the
invitation has been accepted, and a light that still moved would say it was
still a suggestion. `WILL_TAP` is cool where those two are warm, and a beat
behind the armed card's breath, because the price follows the verb. An armed
card is **not** also drawn activatable — `glow_of` drops the offer the arming
accepted, or the same border would carry a chase and a ring saying the same
thing twice.

The lift is the other half and costs no bit: an armed permanent takes the
`SELECTED_LIFT`, not the hover's, and keeps it when the pointer leaves,
because arming is a commitment the player has already made and being selected
is the same claim. In the hand the card stands eight pixels out of the row,
which is headroom the bar already had.

Two things about the group case. `Offer::on` takes a `CardGroup`'s **members**,
not its representative, because a plan taps one particular Forest and the card
drawn for it may be standing for four. And both bits are **any**, where
`CardGroup::activatable` is deliberately *all* — the reason for *all* is that
an offer invites a click and must not invite one that gets refused, and these
two invite nothing. A stack of three Forests two of which are about to tap is
better drawn lit than dark.

### 2.6 Touch

Touch never hovers, because hover is gated on `CursorMoved`. So the preview
becomes a long press, and the "hovered card" path — which is the primary
click's first precedence — is simply dead on a phone until gestures are unified
(§2.1). `Metrics::of`, which the lobby already has, moves into the duel HUD:
the rail becomes a horizontal strip because twelve 44 px rows do not fit a
phone's height, and `softkeys.rs` gains a filter field kind so the subtype and
browser boxes can raise a keyboard at all.

No drag-to-target. Tap-tap is the same gesture as the keyboard path, both are
tested by the same resolver, and a drag across a pinch-zoomed 3D table lands on
the wrong card. The one drag is the camera.

A four-player board does not fit a phone, so on a phone the 3D table shows only
the local mat and the seat whose turn it is; the others are their tabs, and the
text board is the real board view.

---

## 3. What a complete client still needs

Ordered by damage, not by area. Sizes are rough.

| | Item | Scope | Size |
|---|---|---|---|
| 1 | Lock A: refusal keeps the question (client + one engine-server function) | client + engine-server | 1–2 d |
| 2 | Lock B: the zone browser, and the "every offered option is drawn" invariant | client | 3–5 d |
| 3 | Stack and graveyard entries clickable as targets | client | 1–2 d |
| 4 | Reconnect — `NetworkHost::reconnect()` is called only from a test and `DuelReport::Failed` has no reader at all | client | 1 d |
| 5 | Disconnected-seat policy: an unattached seat stops the clock and stalls the table forever; `Playing` games are never reaped; engine death is silent to seats | engine-server + gateway | 3–4 d |
| 6 | Legal strings — the fan-content disclaimer and Scryfall attribution are required by `docs/legal.md` §2–§3 and appear nowhere in the client | client + gateway | 1 d |
| 7 | Combat drawn at all; P/T, damage and counters on cards showing art | client | 3–4 d |
| 8 | Commander end to end | core→engine→gateway→client, **bump** | 2–3 w |
| 9 | Opponents' command zones, graveyard/exile browsers, monarch badge, saga/level counters — all already in the view | client | 2–3 d |
| 10 | `ChooseNumber` visible; concede confirmation; Offer-a-Draw only with priority | client | 1 d |
| 11 | `layout` carried through `CardDef` — fixes adventure faces requesting a nonexistent back image, and the MDFC back that cannot be previewed from hand | dsl + codegen + gateway + client | 2–3 d |
| 12 | Decision clock on screen; game log; results and replays | view **bump** | 1–2 w |
| 13 | Accessibility: colourblind-safe seat and team palettes, UI scale, an aria-live headline on wasm | client | 3–4 d |

**Commander deserves its own note**, because the gap is much deeper than the
missing display. No commander ever enters `Zone::Command`: the gateway drops
`Deck.commander`, `SeatSpec` has no field for it, `compute_legal` never walks
the command zone, the tax counter is dead code, and every lobby game is
`Freeform` at 20 life — `FormatId` does not appear in the gateway's source at
all. The local command zone *is* drawn; there is simply never anything in it.

Two more that are cheap and embarrassing: **the deckbuilder cannot import or
export a decklist**, and there is **no sound and no "your turn" notification**,
which a slow multiplayer game needs more than it needs shaders.

---

## 4. The protocol batch

Everything here is outside the client and costs more. Grouped so it can land in
one `VIEW_VERSION` bump where possible — 9 has since been spent on
`priority_held` (§2.4), so this batch is a bump to **10**.

- **A refused action answered on the wire** — the other half of Lock A, and the
  only item here that blocks something in §0.
- **A per-seat event log.** No `GameEvent` leaves the engine process and
  `Session` exposes no `journal()`, so there is no in-game log, no post-game
  log, and no answer to "why did that die". Filtered per seat by the same rules
  as objects, because hidden information applies to a log too. Until then the
  client can diff successive views and be honest about what that can and cannot
  attribute.
- **Reveals reach other seats.** `GameEvent::Revealed` is journal-only, so a
  revealed card — public by CR 701.20a — is seen only by the asked seat.
- **The decision clock on the wire** as remaining milliseconds on
  `ChoiceRequest`, not an absolute time; clients share no clock.
- **`LossReason` per seat**, so a concession is not drawn as a death, and so a
  future rating can tell concede from timeout from disconnect.
- **Combat damage division.** The engine divides automatically in blocker
  order; under post-Foundations CR 510.1a that is a player's decision being made
  for them. A 6/6 blocked by a 2/2 and a 4/4 should be able to put all six into
  the 4/4.
- **Trigger ordering**, **split piles**, **modal min/max** (choose two, choose
  one or both), **`Pending: PartialEq`** so a re-offered question does not wipe
  a half-made selection, and **`Cancel` before costs are paid**.
- **Day/night**, and **remote seats' cosmetics** in `GameStatic`.

---

## 5. Sequencing

**First, because nothing else matters while a fetchland freezes the table.**
~~Lock A~~ **— done.** Both hosts now answer a refusal with the error *and* the
question again: `LocalHost::submit` re-reads `Session::snapshot`, and
`EngineRunner::refused` does the same over the wire. `snapshot` is read-only,
so re-asking cannot pump an AI seat's turn, and it sends the choice only to the
seat actually being awaited. Two tests hold it — one per host — and each ends
by playing the refused seat's *next* action, because asserting that a
`ChoiceRequest` came back does not prove the seat can still act.

~~Then: the zone browser and the pile chips; the "every offered option is
drawn" invariant; and `duel_flow.rs` rewritten to answer through the browser
model rather than by hand-building actions.~~ **Lock B — done.**
`baylee-client-core/src/browser.rs` is the model, and it has no `Hand` and no
`Battlefield` zone on purpose: `BoardModel` and `Browser` cover disjoint halves
of the table, which is what lets
`every_offered_object_is_drawn_somewhere` assert that each offered id is drawn
*exactly once* rather than at least once. The panel is `hud/tray.rs`, opened by
the pile chips or by `G`. The chips sit in the HUD rather than at the mat
corners as planned above, because the zone counts they replace are `TextSpan`s
inside one text entity, and a span has no layout node to click.

**Second, the common turn made fast and safe.** ~~The keymap swap~~; ~~the
`ChooseNumber` stepper and digit picks~~; ~~arm-then-act with mana abilities
exempt~~; ~~concede confirmation~~; ~~engine holds~~ ~~and the stop preset~~.
Left: the stack panel with runs, which is the one item on this list nothing
else needs and which is deliberately not being done ahead of tranche 3.

**Third, the board made legible.** ~~The hearth shrunk and the own battlefield
out from under the hand bar~~ (done — §1.1); ~~P/T, damage and counter chips on
art faces~~ (done — §1.4, plate and chips both); combat drawn — `is_selected`,
assignment lines, the focus pulse and the per-defender
summary; ~~`motion` wired to `reduce_motion`~~ (done — §1.3); the palette
unified; the keyword
dominance table and the off-pie films; flying as elevation.

**Fourth, the rest of the client.** Text board, detail panel, badge tooltips,
the diff log; touch; the constructed face with inline symbols; departures.

**Then the protocol batch**, in one bump.

---

## 6. Refused, deliberately

- **Polyhedral dice**, growing or otherwise. Shape is not a number.
- **Scene lights, PBR, tonemapping changes.** The stage is unlit on purpose so
  that a blue card at the far end of the table is the same blue as one near the
  camera. Gloss lives in the shader, not in the scene.
- **A fourth typeface**, and a serif display face in particular.
- **Hue-mixing two keyword films into one border**, or per-keyword colours for
  flying, reach and trample. The rail is the countable answer, and the careful
  composition thinking that was asked for concludes by refusing to compose
  colours at all.
- **Independent animation rates per keyword.**
- **Drag-and-drop targeting or ordering.**
- **A second card renderer** for the browser, tray or stack.
- **Client-side rules inference** beyond arithmetic on projected public
  numbers. The layer system stays in the engine.
- **Undo.** Two stages before the wire, and a `Cancel` from the engine where
  the rules allow one.
