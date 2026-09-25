# Game Client (M5)

Bevy 2.5D duel client. Three crates, split by what can be tested without a GPU.

| Crate | Contains | Depends on a renderer? |
|---|---|---|
| `baylee-view` | The wire view: projected characteristics, per-seat filtering | no — not even the rules kernel |
| `baylee-client-core` | Table layout, board model, interaction state machine, image policy | no |
| `baylee-client` | Bevy plugin: 3D stage, overlay, input, texture cache | yes |

## Table presentation (17 September 2026)

> The panels here are named as `docs/dictionary.md` names them — **bar** is
> the seat bar, **ledge** the merged hand-and-question bar, **drawer** what
> grows out of it, **tray** the put-down dialog. This section was written with
> a vocabulary of its own ("action dock", "action rail", "information
> drawer", "phase track") and none of those four was a panel anybody could
> look up. `frontal.rs` still spells its material handles `skirt` / `rail` /
> `drawer`, which is why the mapping is given where it matters rather than
> renamed out from under the code.
>
> **"Shelf" and "ledge" each name a feature that exists on two surfaces**, and
> that is deliberate rather than a collision: `tabletop::MAT_LEDGE` is the
> mat's shoulder, the shelf is the defined sum `MAT_MARGIN + MAT_LEDGE`, and
> `hud/ledge.rs` says in its own first paragraph that it took the name on
> purpose because the hand zone gets the same shoulder at its top. Renaming
> either would delete an analogy the code states twice. What the prose owes is
> the **qualifier**: *the mat's shelf* against *the hand's shelf*, *the mat
> ledge* against *the HUD ledge*, wherever both are in play. A sentence that
> needs its section heading to be read correctly is a sentence missing a word.
> Within one section about one surface the bare word is right, and mixing the
> two words for that one surface a line apart is the thing to avoid.

The current table uses warm golden-hour daylight, cool cloud shadows, and
an indigo night with moving aurora and stars. The sky's eased day/night value
also grades the glass. Rules day/night still overrides the ambient preference.
The old land-driven atmosphere renderer and its full-table blended shader have
been removed; atmosphere intensity now controls the sky's aurora and sparks.
The lobby backdrop remains separate from the duel.

`felt.wgsl` draws smoked glass above winding water and lava channels. Water
ripples and caustics travel faster than the molten crust; their confluence
cools into dark obsidian with a restrained steam veil. Fixed banks keep the
motion legible as flow. Two scales of domain-warped cellular seams form
connected trunks and fine capillaries, with spatially blended water and lava.
The pattern is vascular rather than spaced along golden-section curves. A
client-only OS-random seed chooses domain offset, orientation, and scale once
per duel; card updates, resizing, and reconnects keep that surface stable.
Everything stays in the existing opaque pass. This is a
stylised glass material, without screen-space refraction or a render target.
Table corners use 6.5% of the short axis. Wide duels use a 0.62 camera lean
and tighter framing; smaller windows and multiplayer rings retain the original
camera. Broad duel mats gain up to 1.1 world units of depth; their aspect
compensates for camera foreshortening so the extra space reaches the card lanes.
Hover lifts were lowered to keep picking stable at the steeper angle.

Large hands retain at least 65% of each card's width and scroll horizontally,
with page arrows and a position indicator. Hovered cards draw above their
neighbors for the duration of the hover. Mouse and trackpad scrolling use their
native units; keyboard navigation includes the spaces between groups.

Compact framed icon buttons in the actions bar offer draw order, mana value,
name, type, and color. Category
modes both sort and group, with one heading and count per contiguous group.
Artifact creatures stay in the creature group, multicolor has its own group,
and mana values above nine share a 10+ group. Narrow windows cycle the same
modes through a compact button. Ordering persists through view updates without
changing the engine's hand or reordering on playability changes. Group headings
use twelve additional pixels above the cards.

Cards sit 0.028 units above the glass with contact shadows, a narrow subdued
stock highlight, and minimal ambient sheen to retain printed contrast. The
summoning-sickness veil keeps its breathing/ring cues with only a slight cool
cast. The compass turn numeral uses bold, centered serif type.

Battlefield outlines enclose **only the three card lanes**. The reserved
centre-facing band is outside the outline: compact name/life/hand/counters
at the owner's left, and the step tiles beside it — the phase rail is gone
and `seatbar::tile` draws a turn's steps on each seat's own bar. Library, graveyard
and exile counts sit beyond their respective piles, icon above number. Life
uses a 24-point numeral with an 18-point icon. Poison and energy appear only
when nonzero; the wire view does not expose player experience or charge.
All these anchors follow the projected table, with upright text at every seat.

The creature lane includes a full `STAGE_STEP` of combat clearance, taken from
the spare spacing between the three rows. Advancing attackers stay inside the
battlefield and clear of the information band. Identity and phases have wider
separation, and wide duels frame the table more closely. A seat without any
designated commanders reclaims the left command strip; its right-hand piles
and the camera's footprint remain fixed.

The entire player summary accepts player targeting, including its life and
hand cells. Legal targets receive a gold outline and selected players remain
lit; an invalid target click cannot move the camera. Outside a target choice,
the existing seat-focus behavior remains available.

Graveyard and exile previews show the four newest cards with shallow tilt and
a small hover offset that keeps the card under the cursor. The library uses
the same bounded preview with backs; authorized search/scry cards continue to
appear through the choice interface. Command slots never fan. All movement
uses the existing card motion system and respects reduced motion.

Proximity groups related stats; type size establishes their hierarchy. Gold
marks the active turn, with a secondary cool accent for priority. Phase groups
keep their chronological positions, offer a full name on hover, and preserve
keyboard stop toggles. Phase lighting is anchored to the game step, so a new
snapshot cannot restart its transition. `tableicons` is the audited Mana /
Font Awesome map; tests check it against the bundled font files.

The central compass shows just the turn number. Its stones breathe, and its
ring advances one mechanical detent per turn over 1.15 seconds. A short gear
and catch sound is synthesised once and follows the sound preference. Joining
a game does not rotate or play it; priority changes do not move it. Reduced
motion settles the ring immediately and freezes the shader clocks.

The **drawer** shares the **ledge**'s material. Its bottom remains open, and
its measured width removes the ledge rail's top tooling across the join. Both
keep their normal outer edges. (`Cloth` in `frontal.rs` holds the three
handles this is about: `skirt` is the hand well, `rail` the strip at the
ledge's top, `drawer` the extension above it.)

This section supersedes the older descriptions of the cloth, weather overlay,
and information panels inside the mat below.

## The wire view

A client cannot run the layer system, so `PublicObject` carries **projected**
characteristics (name, P/T, types, colours, keywords), not the printed card.
An anthem, a clone, or an animated land arrives already resolved.

Hidden information is unrepresentable rather than merely omitted: library
contents have no field, another seat's hand is a count, and a face-down
permanent's `card` is `None` for anyone not entitled to look.

`GameStatic` (seats + print table) is sent once; `PlayerView` is a full
snapshot per change, which makes reconnects trivial.

## 2.5D

Cards are textured quads lying on a 3D table; everything a player *reads*
(prompt, hand, stack, life, threat lines) is a 2D overlay. Tapping is a
rotation, and focusing an opponent is a camera move rather than a re-layout.

The card is a rounded slab built by hand (`table.rs::rounded_card_mesh`): a
rounded rectangle carrying the printed face, with a thin wall around its edge
and no bottom (the camera never goes under the table). The wall borrows the
UV of the face vertex above it, so a card's edge is whatever colour its
border is — black for most cards, which is exactly right. `CARD_THICKNESS` is
far more than a card's real proportion, and deliberately: at any camera
distance a player uses, a true 1/50th would be a fraction of a pixel.

Under it is a **contact shadow** — one quad a little larger than the card,
carrying a painted halo that is dense under the card and gone by its own
edge. It is a child of the card, so it follows the tap rotation and the hover
lift with nothing to keep in step. It is a shape, not a cast shadow: a real
one would need the table to be lit, and everything down there is unlit on
purpose.

Neither reads at a purely top-down camera, which is why `CAMERA_LEAN` exists.
The table is read from above — a four-seat pod ring is laid out for a plan
view — but a camera exactly overhead throws away every cue that a card is an
object: the edge projects to nothing and the shadow hides underneath. About
14° off vertical is the compromise.

It was 22°, and what moved it is that a lean is paid for by the seat furthest
from the camera and collected by the seat nearest it — which is always the
player's own. Three boards laid out at exactly the same 12.0 units were drawn
450, 381 and 378 pixels wide, so the one board a player compares every other
against was the odd one out, and a free-for-all looked like a format it was
not. Halving the lean and halving `FOV` together — the same table framed the
same way, from twice as far off through half the angle — bring that spread
from 18.9% to 6.3%, and the seats that give nothing up are the opponents': it
is the player's own board that comes back to the size of theirs.
`every_seat_is_drawn_a_board_of_the_same_width` is the bound. The few per cent
left is the foreshortening of a board turned away from the camera, and the
only way to spend that is a lean of zero, which is a table of decals.

The mesh has five tests of its own, and it is worth knowing why. It shipped
once with every corner arc sweeping the quarter turn belonging to its
neighbour: the outline folded through the middle twice and every permanent on
the battlefield drew as a small bright X. Nothing caught it, because the test
that existed — `an_untapped_card_lies_flat_on_the_table` — asks whether the
*transform* is flat, and it always was. The mesh is now checked for what
actually went wrong: that the outline turns one way and closes exactly once,
that it covers the area a card of that size should, that every face triangle
faces the printed side, that the wall closes around the card facing outwards,
and that the face sits on top of the slab rather than level with it.

## The table itself

### September 2026 material direction

The current treatment is **midnight mineral cloth, aged champagne metal and
five-colour inlay**, replacing the casino-green material described in the
historical rationale below. The slab and play lanes retain their measured
footprints: a material redesign must not silently move the cards, pile
hitboxes or projected seat shelves.

`felt.wgsl` adds a restrained mineral vein, brushed rail, twin engraved
circuits, inset colour segments and tooling around the firewheel.
The inlay colours stay in fixed positions; only their illumination moves.
The phase lamp remains a separate semantic light. `mat.wgsl` adds recessed
corner shoulders and terminates lane rules short of the frame. Both resolve
fine details against the pixel footprint rather than assuming a resolution.

The sky is graded independently: open blue daylight with warm cloud edges by
day, blue-black air and distant drifting mist at night. Daylight also raises
the mineral table’s exposure; the existing night grade is preserved. It shares the table's cool shadows
and warm accents, but **there is no full-screen colour filter on card art**.
The unlit card pipeline and the existing day/night preference are unchanged.
All decorative shader clocks use the existing motion multiplier, so reduced
motion retains the material without the travel or shimmer.

The CPU generators remain references for base colours, lane boundaries,
contrast and texture—not pixel-identical renderings of the shader's added
metalwork. Shared constants are still checked by the client camera tests.

The ledge and seat furniture share `FrontalMaterial`: recessed
mineral-leather ground, thin champagne tooling and fixed five-colour inlays.
Eleven persistent handles cover the dock, its upward info extension and eight seats; the virtual clock and
reduced-motion preference govern decorative movement. Split seat bars place
a 220px identity/life plaque beside the timeline and zone/turn status, with
an honest 871×48px minimum footprint and unchanged compact fallbacks. The
library uses an original resident sleeve with matching geometric inlays;
opening a pile follows the existing motion targets, with a seven-card fan
limit and a shallower, more widely spaced silhouette.

Under the cards, everything is generated rather than shipped:
`baylee-client-core/src/tabletop.rs` computes the felt and a
seat's mat into plain RGBA8 buffers. Three reasons, and the first is the one
that decided it: `docs/legal.md` §2 rules out WotC assets, and a fantasy
table wants exactly the kind of ornament that is easiest to borrow by
accident — arithmetic borrows nothing. It also ships no bytes (a 1024² felt
is a megabyte and a half the wasm build does not have to carry), and being
pure functions over a pixel buffer it is all testable in the renderer-free
crate, with no GPU anywhere.

The noise is a hashed value-noise fbm with fixed seeds — no `rand`, no clock
— so every player at a table sees the same grain in the same place, which
starts to matter the moment anyone screenshots anything.

**Every seat gets a zone.** A shared battlefield with no divisions is a pile;
each seat now plays on its own mat, sized from its `SeatSlot` and rotated to
face it, with three bands across it for the three lanes so an opponent's rows
can be read without counting cards. The rim carries the seat's colour: the
viewing seat is gilt, matching the firewheel's rings, so "mine" is the one
edge a player never has to look for; the others take the colours of the pie
in ring order, which makes a four-way game four distinguishable places rather
than three anonymous opponents.

That rim is also the cheapest place to answer the two questions asked on
every priority pass. A seat's `Mood` is `local` plus a `Standing` — lost,
holding priority, active, waiting — and brightness follows it: the seat
everyone is waiting for is the brightest thing on the felt, and a seat that
has lost fades most of the way out, because its permanents are gone and its
zone should stop competing for attention. `Standing` is an ordered enum
rather than three booleans on purpose: holding priority and being the active
seat are true together nine times out of ten, and drawing both would mean
adding two brightnesses and hoping.

Being the local seat is a *lift* on that scale and never a rank, which is a
change the rim fix forced. While the tint was the accent scaled by the mood,
the scale could run past 1.0 harmlessly — an accent's linear channels are all
well under 1, so a local seat holding priority multiplied out to 0.749. With
the accent in the texture the tint is neutral, the number multiplies white,
and 1.0 is a ceiling: the old 1.311 and 1.0925 would have clipped to the same
flat white and drawn "holding everyone up" exactly like "taking my turn".
`zone_brightness` is bounded at 1.0 now, and three tests hold the shape — no
mood asks for more light than white, a dimmer standing is drawn dimmer, and a
standing always outranks being the local seat, so my own idle mat can never
outshine the opponent everyone is waiting for.

In the middle of the table the colour wheel burns. It is five flames in the
arrangement every player already has in their head — white at the top and
clockwise from there — so it is orientation as much as ornament, and it
stands on the one patch of felt no seat ever plays on. Around it,
`tabletop::hearth` paints a pool of lamplight with a ring of faint arcs
and tick marks inlaid in it — one texture, because they are one thing to look
at, and because a table with nothing between the seat mats reads as an
infinite green plane however good the grain is. The pool is candlelight and
the inlay is gilt; a test asserts neither ever goes cold, since a blue light
over a green table makes colour identity a guess.

### The centre inlay

Five faceted enamel stones replace the procedural flames. They sit in the
existing engraved compass and retain the eased mana-source strengths from
`baylee_client_core::firewheel`. A slow glint and a small coloured light pool
give them depth without moving their silhouettes. Black uses muted amethyst.
The shader no longer evaluates layered erosion noise per stone; ring wear
is also bounded to the centre instead of sampled across the entire table.

The sky adds sun rays, moonlit aurora curtains and distant hazy ridges.
Day/night transitions move the celestial bodies and carry the table's
lighting, dust and soft shafts through the same eased exposure. The two
speeds remain: slow ambient changes, faster rules-driven changes. Reduce
motion freezes decorative animation; atmosphere Off removes the air pass.

Seat information uses a large life total beside a two-line identity, four
quiet count cells, and a labelled phase timeline with the current step
written above it. All remain within each seat's reserved band, including
multiplayer layouts. Only the identity wears the cloth material, avoiding
different-sized panels writing competing dimensions to the same handle.

That the rim carries the colour is true as of the fix below, and was not
before it. `tabletop::seat_mat` used to write every pixel white with the mat's
shape in the alpha channel alone, and the seat colour arrived as the
material's `base_color` — which multiplies the whole texture, so the field got
the same hue as the rim and differed only in opacity. A gilt-rimmed board was
in fact a sheet of brass. `seat_mat` takes the accent now and crossfades to it
on a shallower curve than the opacity's — the hue has to reach further in than
the ink, or the colour only ever lands where the rim is already solid and
every seat's rim reads off-white — the material carries neutral brightness,
and the
glow beneath keeps the accent because putting the seat's colour on the felt is
what a glow is for. It costs a texture per seat rather than one per table;
sharing that image is precisely what made the separation impossible.

**The lamp is atmosphere, and it took three goes to make it behave like one.**
It shipped as a twenty-unit ring with twenty-four ticks on it — the largest,
brightest, most detailed object on screen, and what the eye read was a
roulette wheel. `HEARTH_TICKS` is eight (a compass, not a clock), there is one
hairline rather than two, the pool is half as strong and the wheel's own
light is dimmer.

The third go was the size, and it is the clearer lesson: `HEARTH_SIZE` went
34 → 18 and the ring still dominated four straight photographs while its test
went on passing, because the test compared the ring against the mat's **long**
edge. A seat's mat is 18.8 units across and 4.42 deep, so an 18-unit quad's
10.8-unit ring was two and a half times the depth of a player's whole board
and passed a bound of 18.8 without trouble. `SeatSlot::mat_depth` is the edge
that matters and the bound now names it, in both directions —
`lane_height < ring < mat_depth`, wider than a row of cards and narrower than
a mat — for the same reason the felt's brightness assertion goes both ways.
`HEARTH_SIZE` is 4.5.

**The camera frames the table against the window it is seen through, not
against the window.** The HUD is not beside the battlefield, it is on top of
it: the hand bar is an overlay on the same full-window camera. It used to be
far more than that — a strip of seat tabs and a phase rail under it took 110
logical pixels off the top of every window, in every game, and both of them
said what §"Every seat's bar" now says on the seat's own mat, so `Canvas::hud`
has a `top` of zero. The rig used to be a
hard-coded twenty units aimed at the middle of the felt, and the result was
that the local seat's own mat projected *below* the hand bar — a player could
not see their own creatures, which makes every other piece of board legibility
moot. `CameraRig::home` computes the shot from `TableLayout::extent` and a
`Canvas` that names what the HUD covers, and `table::frame_table` reapplies it
when the seats, the focus or the window change — stopping only while the
player is looking at one seat.

**A hand no longer moves this camera at all**, and that is three owner reports
in a row rather than one decision. The orbit went first, because the left
button is also the button that plays cards and every click that travelled a
pixel turned the table. The zoom went next, because the wheel argued with
every scrolling panel in the interface and the referee between them never
held. The pan went on 14.09.2026 with the rest — *„Generelles Camera Movement
kann weg (also nicht nur die Maus Controls, sondern auch die Keyboard
Controls)"* — and the report under it is the one that names the cost of a
keyboard route outside the keymap: the arrows drove the table while a text
field had the keyboard, because `input::camera_controls` read `KeyCode`
directly and every other duel key goes through `Fired::of` and stops at
`browser_keys`. So `input::camera_controls` is gone, and what is left is a rig
only `frame_table` and `navigate_to_player`/`navigate_home` ever write —
`table::framing_tests::nothing_a_hand_does_moves_the_table` writes every
gesture of all three generations into one frame and
`looking_at_one_seat_holds_the_camera` is its counter-test.

**The lean is not one number any more.** `CAMERA_LEAN` is still what a ring
of three or more seats is shot at, and what any table on a narrow window is
shot at; a **duel on a wide window** blends towards `DUEL_LEAN` — about 27°
off vertical — as the window grows past 800 logical pixels. The reason is
that the two shots answer different questions: a ring has to keep every seat
readable at once, so it stays near the plan view, while a duel has only two
sides and can spend the freed angle on the table's depth. It is written as a
blend and not a switch because the alternative is a camera that jumps as a
window is dragged across one pixel. `only_a_wide_duel_takes_the_more_oblique_shot`
is what holds the three cases apart, and **`CameraRig::lean` is the value
everything downstream reads** — the projection test writes it out forwards
rather than reusing the constant, which is what caught the fit when the
constant stopped being the whole answer.

The inversion is exact rather than tuned, which is why it is arithmetic and
not a magic number per screen size. With the eye at distance `D`, the lean
`L` = `CameraRig::lean` and `C = 1/√(1+L²)`, a felt point `s` units from the look
point along the screen-vertical has camera-space `depth = D + L·C·s` and
`height = C·s` — the cross terms cancel — so `s = D · ground(q)` is linear in
`D` and the fit is a division. `table.rs::camera_tests` projects the four
corners of every pod *forwards* (written out a second time on purpose: a test
that reused the inverse would agree with it however wrong both were), at two
through eight seats and on a phone-shaped window, and asserts each one lands
inside the band a player can actually see.

Sideways the measurement is taken at the table's **near** edge, not at the look
plane. A perspective camera sees less felt where the felt is closer, so the band
under the front row is narrower than the one through the middle — measuring in
the middle put a four-seat table's outermost mat past the rail. Once the far
edge is pinned the near edge's depth is linear in the eye distance as well, so
this stays one division.

The distance is clamped before the look point is computed from it, not after.
Aiming for a camera the clamp then moves is the one way this puts the table off
screen with every formula still right — the far edge gets pinned for an eye
that is not there. Clamped first, a table too big for `MAX_DISTANCE` keeps its
far edge at the top of the window and overflows in front of the local seat,
which a player can pan out of. A four-seat table on an upright phone is that
case, and no camera fixes it: at the width it needs the felt's own edge comes
into frame. It used to be said that this waited on the rail coming off the top
of the window and `Canvas` changing. That has happened, and it did not fix it:
the phone's problem is the ring's shape against a tall window, not the hundred
pixels the HUD was taking.
**The pool says which step it is.** The rim already answers *whose* turn it
is; where in the turn we are was only ever readable off the rail, in text, at
the far edge of the screen. `tabletop::phase_light` gives each of the twelve
steps a lamp — cool and low through the beginning steps, the pool's own
candlelight through a main phase, an ember rising into combat that peaks at
damage, dusk at the end — and `table::sync_phase` eases the middle of the
table towards it. Combat is the case it is for: a board going warm as
attackers are declared says "something is about to happen to you" faster than
a highlighted row.

Three things keep it from becoming noise, and all three are the kind of
mistake that is obvious only afterwards. It is a **wash**, blended over the
pool, not a multiplier into it — multiplying candlelight by a cold colour
gives grey, which is how a tint like this normally fails. It leaves the
**firewheel alone**: that is the colour wheel, the one thing on the table
that has to stay literally true, and a red cast over it would be lying about
colour identity. And it is sized against the *ring*, not against the pool's
quad — the first version was 42 units wide at alpha 0.34 and read as "the
table is red" rather than as a lamp over the middle of it. The colours
themselves are argued with in `tabletop`'s tests (combat peaks at damage, a
main phase barely washes at all, no step is a saturated filter) rather than
looked at in a screenshot.

Everything down here is `unlit`, and stays that way: card art must never be
tinted by scene lighting, because colour identity has to be readable at a
glance. The table gets its depth from shading painted into the textures
instead — so the stage has no light in it at all, and the camera carries
`Tonemapping::None`.

`Tonemapping::None` is belt and braces — Bevy attaches no tone mapper to a
camera by default — but it is worth saying: in an unlit scene every number
already *is* a display value, so a tone mapper reading them as radiance
would be wrong, and a future default doing it quietly would be very hard to
see.

Which is the lesson from how this table actually shipped. For a long time it
rendered as a black screen with two faint gold rings floating in it, and
every explanation offered for that was about colour: the felt is too dark,
the textures are being tone mapped, the sRGB is being decoded twice. All of
them were wrong. **The own-board overlay was an opaque panel the width of the
canvas, and it defaulted to open** — `palette::PANEL` is `srgba(0.05, 0.06,
0.08, 0.88)`, so the entire table, its mats, its cards and every animation
on them were behind a sheet of 88% black from the first frame. What finally
found it was measuring instead of reasoning: a red clear colour renders at
`(234, 51, 35)` in a stock Bevy app and at `(62, 19, 21)` in ours, and a
clear colour never touches a material, a texture or a shader.

It was made opt-in, and has since been **removed altogether**. It drew the
local seat's battlefield a second time, flat, in the half of the screen the
camera already frames that board in — so every permanent had two places to
be, two hover states and two sets of glows to keep in step, and the panel
kept the one power nothing else on screen has: covering the game. What went
with it is the whole sliding layer (`OwnBoardOverlay`, its knob, the `X`
action and `Duel::overlay_open`/`overlay_t`); `hud/overlay.rs` keeps its name
and its job, which is the retained HUD tree — tabs, prompt slip, stack.
`input::camera_controls` no longer had anything to refuse to run under, and
has since been deleted outright — see "A hand no longer moves this camera"
above.

The felt was too dark as well, and that was real: it was authored at about a
quarter of the brightness it needed, and
`the_felt_is_dark_enough_to_read_cards_against` passed every run because it
only ever bounded the bright end. It bounds both now.

### Two costs every generated surface pays

A redesign that was shown live and turned down — a granite slab with runes
breathing on `globals.time` — measured two things that outlive the look it
was measuring. Neither is visible from the arithmetic, and both apply to any
surface this client computes rather than loads:

- **Texture size is paid on the main thread, at every duel start.** The
  buffer is built where the frame is, so it is not a memory decision: a
  2048 × 1200 slab froze a debug build for ten seconds before the first card
  was drawn. The felt and the mats are the sizes they are because of that.
- **A grain whose period falls under about four pixels is aliasing, not
  stone.** Noise frequency and texture size are therefore one decision and
  not two — raising a surface's detail without growing its buffer buys
  shimmer at exactly the angle this table is seen from.

The rest of that branch is either landed or unwanted. What was structural in
it arrived by other routes — the slab cut from `TableLayout::extent` rather
than a fixed sheet, the layout built against the canvas the table is *seen*
through, and the pod depth taken from what three lanes of cards need — and
what is left is the look itself, which was the part that was turned down. The
code is at the tag `archive/table-redesign-wip` if it is ever wanted back.

## The air over the table

Six things can be in it — leaves, mist, embers, shafts of light, fog and
snow — and **the lands on the battlefield decide how much of each**. A board
of Forests gets a leaf fall; a board of Swamps gets a fog lying in the table;
Snow-Covered anything gets flakes. It is weather in the same sense the sky is
weather: it changes no legal action, nobody is meant to read it, and a player
who never notices it has lost nothing.

The split is the usual one and earns its keep twice here.
`baylee-client-core/src/atmosphere.rs` reads the board and answers a
`Weather` — six amplitudes and a three-component grade — which is arithmetic
a test can argue with. `baylee-client/src/atmosphere.rs` and
`shaders/atmosphere.wgsl` are what is left once that is taken away: one quad
and one fragment shader.

### Reading the board

Only lands are looked at, and only the five basic subtypes plus the snow
supertype. A land with *k* basic types contributes `1/k` to each of them, so
a Tundra is half a Plains and half an Island and is still one land; snow is
counted whole and separately, so a Snow-Covered Forest is a Forest **and** a
flake source. A land printing no basic type at all — most of them — says
nothing, which is why a deck of utility lands plays under still air rather
than under a grey average of everything.

Two functions of the counts decide an amplitude, and the second is the one
that matters:

- **saturation** `1 - e^(-c/3)` on the *total* count, so the air thickens
  quickly over the first few lands and then stops. The third Forest is worth
  more than ten times the twelfth, which is a test
  (`more_of_one_land_thickens_the_air_and_then_stops`).
- **share** `c² / Σc²` per kind, which sums to one by construction. So the
  six amplitudes together can never pass the player's budget, and a
  twenty-land board is not twenty times busier than a one-land board. That is
  held over every mixture the five basics and snow can make, up to four of
  each (`the_air_is_never_busier_than_its_budget`).

Squaring the share is what makes a mono-coloured board read as *one* kind of
weather and a five-colour board read as a light dusting of everything. A
linear share would give a Forest-heavy board a permanent haze of the other
four.

### The grade, and why it cannot dim the felt

Besides the six amplitudes a `Weather` answers a **grade**: a multiplier the
felt shader applies as `under_weather`, beside `under_sky` and `under_lamp`.
The stage has no lights in it and cannot have any, so every "light" on this
table is a multiply on a surface's own colour, and this is the third of them.

Each basic pulls that multiplier towards its own hue by at most
`GRADE_REACH` (0.25) — deep green for Forest, teal for Island, ember for
Mountain, pale gold for Plains, violet for Swamp, a pale chill for snow.
**Every row of the pull table has a Rec.709 luma of zero to within 0.01**,
the largest of them being about 4e-4, so however many are mixed and at
whatever amplitudes, the felt's brightness cannot move by anything a player
could see; only its hue can. That is the owner's *ohne das Spielen zu
beeinträchtigen* turned into arithmetic rather than into restraint, and two
tests hold it (`every_pull_is_a_hue_and_not_a_brightness`, which is where the
0.01 is written down, and `the_felt_keeps_its_brightness_under_any_weather`).

The grade is a separate uniform from the sky's tint rather than being folded
into it, because the sky's strength depends on the hour and the weather's
does not.

### One surface, under everything a card casts

Everything else is painted on a **single quad**, cut to the slab's own
racetrack by `table::flat_table_mesh` and laid at `table::ATMOSPHERE_LIFT` —
0.0035 of a unit above the felt. That is above every mark belonging to the
table (a seat's mat at `ZONE_LIFT`, the glow under it) and
below the contact shadow under a card at `CARD_LIFT * 0.5`, and therefore
below the card.

The promise that follows is geometric, not a matter of taste: the quad is
blended and the cards are opaque, so the cards write depth and the depth test
rejects the quad at every pixel a card occupies. **Not one card pixel can be
touched.** A future layer that wanted to be a post-process would have to give
that up explicitly, which is the point of putting it here.

A leaf is painted rather than flown. A billboard in the air would spend most
of its life in front of a card at a camera twenty degrees off vertical; the
quad is the other way round, and the shader is handed `fall`, the direction
on the table plane a falling thing appears to travel, recomputed as the
player orbits so the weather is not painted on the lens. The parallax is real
even so: a mark drawn as though it were high in the air crosses the table
faster than one near the felt, which is the whole of what sells depth here.

### The lift ladder is the draw order, which it had never been

`TableQuad::lift` used to carry a comment saying nothing on the table was
depth-sorted. It was never true. Bevy sorts the transparent phase by
`rangefinder.distance(mesh_center) + depth_bias`, ascending — so the order
was **distance to the camera**, and a seat's mat on the near half of the
table was painted over a card's contact shadow on the far half. Nothing had
noticed because every previous overlay was small enough to sit entirely
inside one lane.

`table::sort_bias(lift)` is the fix and is simply `lift * 400_000`: it turns
the lift ladder into the sort key, so the ladder finally decides what covers
what, wherever on the table the two things happen to be. It is applied to
every blended surface down there — the mats, the table quads,
the contact shadow and the air. `depth_bias` affects **only** the sort key
and never the depth buffer, which is what makes it safe to hand it a number
that large.

### A veil and a mark are priced differently

A **veil** — mist, fog, shafts — is paid for on every pixel of the table, so
it must be cheap and it must be broad: low-frequency fbm, no per-mark loop.
A **mark** — a leaf, a flake, an ember — covers almost nothing, so it may be
expensive where it lands. Marks are drawn out of a hashed cell grid with a
travel offset; amplitude buys *count* first (a gate on the cell's own keep
value) and brightness only second, so half as much weather is half as many
leaves rather than the same leaves at half opacity.

`SHAFT_PEAK` is the one constant here that is derived rather than chosen.
Shafts are the only layer that *lifts* the felt, and the felt already sits
near the top of what `the_felt_is_dark_enough_to_read_cards_against` allows,
which leaves about 0.047 of linear headroom and puts the shaft's peak alpha
at 0.031. It is 0.025. The consequence is worth stating plainly, because it
looks like a bug: **shafts are the quietest of the six**, and a Plains-heavy
board gets the most restrained weather on the table. That is the bound doing
its job.

### What looking at it changed

The numbers above were checked live through `dev-control`, one layer at a
time at full amplitude, against the same table with the air switched off.
Measured over the felt, the veils move it by about 5 of 255 (fog, over 99% of
the cloth), 7 (shafts, 93%) and 1 (mist, in blooms at the rim, 11%); the
marks cover a few tenths of a percent of the table and are strong where they
land. Leaves at their maximum are about twenty on the whole table.

One thing failed that reading and was changed. An ember was a disc: a flat
plateau of 187 red twenty pixels across, then 64 three pixels later, then a
halo nobody could see under it. On a green felt that is a counter somebody
left on the table, not something burning. The core falls off from its middle
now and meets the halo without a step, and the halo is worth seeing. It
costs the same handful of pixels and is an entirely different object.

### The setting

`Preferences::atmosphere` is `Off` / `Soft` / `Full`, a budget of 0, 0.5 and
1.0, and it lives in `Preferences` rather than in `ClientSettings` because it
is a taste and not a machine's capability: it travels with the account over
`GET`/`PUT /settings`, beside the sky. `Off` despawns the quad outright
rather than setting six amplitudes to zero — a slab-sized blended surface is
a near-fullscreen pass every frame whether or not anything is drawn on it,
and that is the difference that matters on a phone.

`reduce_motion` **freezes** the air rather than emptying it, on the same
`MOVING`/`STILL` clock the cards, the mats and the sky use. A player who
asked for a table that holds still asked for that, and a setting that quietly
removed the weather would look like a bug in the weather rather than like the
setting working.

Nothing here is shipped: no leaf sprite, no snowflake texture, no gradient
for a light shaft. `docs/legal.md` §2 decided that for the felt and the mats
and §5 decided it again for sound; ornament is the easiest thing to borrow by
accident, and arithmetic borrows nothing.

## Eight seats

- Seat information attaches to the **ledge inside** each seat's own rim, on
  the long edge nearest the middle of the table: name, life, priority and turn
  at the end of that band which projects leftmost, zone counts at the other
  end, and the twelve phase steps horizontally between them. These are three
  separately projected text panels sharing one band and one scale, not one
  full-width shelf toolbar. `hud/seatbar/attached.rs` owns their drawing; the
  smallest overview keeps the legacy `Mark` fallback. Historical shelf
  geometry below still controls card-lane reservations and fallback density,
  not desktop panel bounds.

  Where the fallback starts is worth knowing rather than discovering at a
  table. Photographed on a 1728×1052 window: **four** seats still get the
  panels, turned with their mats — a side seat's band runs up and down the
  screen and its ink is turned with it.
  **Eight** seats do not: every bar drops to `Mark`, which is a row of pips
  and a priority stroke, so nobody's life total is on the screen at all. That
  is the fallback behaving as designed and it is also the hole in it.
  `attached.rs` is written to give a **focused** seat its panels back, which
  is the way out; that path is not measured here, and the eight-seat table is
  where to measure it.
- Seats sit on a ring, local seat at the near edge, opponents clockwise **in
  turn order** — the player on your left acts after you. Allies share a side
  and face the same way; everybody else has a side to themselves.
- The ring is shaped to the canvas, so a wide screen is actually used — except
  where that would seat a **free-for-all** as something it is not. Three seats
  each playing for themselves sit on a circle at 0°, 120° and 240°, because on
  a canvas-shaped ellipse they come out side by side across the top, which is
  the silhouette a 2v1 draws. It is offered to any table with no allies at it
  and taken only when the camera can afford it (`layout::ROUND_COST`); four
  seats and up keep the canvas shape, and so does a canvas narrower than about
  4:5 — a phone held upright would pay more than twice the reach for a circle,
  so three seats there go on sitting where they fit. The desktop canvas only
  started affording it when the tab strip and the phase rail came off the top
  of the window: at an aspect of 2.25 the three-seat ring was an ellipse of
  12.95 × 4.99 and the circle was refused, at 1.97 it is 8.28 × 8.28 and costs
  9.3% of reach. That is why three is the one seat count whose boards got
  *smaller* when the window got taller, 34.9 → 32.0 pixels a table unit, and
  it is the circle being bought rather than anything going wrong.
- Every seat plays on a board of the same width, and focusing an opponent
  widens that one at the other opponents' expense, never at yours.
- Lanes fan when crowded and report overflow when even fanning stops being
  legible.
- A seat's tab carries a **second life total** when one applies. Twenty-one
  combat damage from a single commander ends a game at any life total
  (CR 903.10a), so a seat facing commanders is on two clocks and only one of
  them is the number beside the heart. `client-core/src/commanderdamage.rs`
  turns the tally into a track: the worst single source as a fill over a bar
  that always stands for twenty-one, every other source as a tick on the same
  scale, and the fill turning `DANGER` five short of lethal — the life
  total's own threshold in the same panel, so a player learns one rule and
  not two. The bar is a fixed width in every tab, because its job is to be
  compared across seats. What it deliberately does not do is add the sources
  up: twenty from one commander is lethal next hit and twenty spread over
  seven is a scratch, and a total would be a number Magic does not have.
  Which tick is which commander is the tooltip's answer — the same half this
  panel already gives for a counter chip's colour.

## Grouping and the token summary

Identical permanents draw as one card with a **count badge** hanging off a
corner of it, `×N` (#261; `cardplate::count_word` and `badge_rect`,
`badgemat.rs`, `badge.wgsl` and `badge_ui.wgsl`, drawn by
`card_common.wgsl`'s `count_badge`). It was a pill painted over the printed
cost until #274 took everything of ours off the print; the owner's placement
is "ganz oben links an der Ecke, leicht überragend mit elevation shadow". So
it is an object lying on the card, as the strip is: since #298 the plate's
figures, 0.12 card widths tall, over their own drop shadow (`BADGE_DROP`,
`BADGE_BLUR`) and **no plate behind them**; one quad per merged card, a
child of the card, not pickable and not a `CardShadow`; one material per
distinct count on the table. The `×` is load-bearing: a bare `12` in a
corner reads as twelve generic mana.

**It hangs off the card's top-left corner, outside it** (#298). Its right
end stands in the print's own black border (`BADGE_RIGHT`, 0.031: the
border's 0.045 less the shadow's reach), its top 0.008 under the card's top
edge, and it grows **left** with its digits, off the card. The bottom-left
stand-in it had while the frame's ledge carried the plate
(`BADGE_OFF_THE_PRINTS`) went with the ledge: without a frame it would have
stood under the card, on the row behind.

- A tapped card turns its badge with it, as any object lying on the card
  does; there is no longer a system holding it upright.
- **What it costs, accepted by the owner:** in a fanned row the overhang
  lies on the cards before it, which the row has already covered with the
  cards after them; in a row with less room between two cards than the
  badge overhangs, it lies on the top-right corner of the card before it,
  where that card's cost is.
- `a_badge_lies_only_on_a_card_its_own_card_lies_on` holds the rest: every
  badge, spawned as the scene spawns it, against every card on the table
  that is not one before it in its own row and that its own card does not
  already lie on, at a duel and a ring of eight, rows of 2 to 40 in all
  three lanes (reaching the 0.26 fan), untapped, tapped and every other one
  tapped, and with a creature staged beside a merged one.
- It grows to `×99` (`BADGE_W`) and no further; three digits are set smaller
  to fit (`badge_cap`).
- It lies at the strip's height, half a row step over its card's face, which
  also lays its overhang over the card before it in a fanned row, a whole
  step lower (`a_badge_lies_on_its_card_and_under_the_next_one`).
- Nothing of it reaches past the print's border onto the art
  (`nothing_of_the_badge_reaches_past_the_printed_border`).

Until #210 the count was the job of a function (`table::stack_badge`) that
nothing called: 54 Goblins drew as one Goblin, the slab under the card was
the only cue, and a pile's depth is capped. A pile never wears a count
(`Placement::badge` is zero for it); its size is the deck under it and the
seat bar's number. The hover preview wears the same badge, because it is the
table card held up larger and the count is the one thing its art cannot say:
a UI node hanging off the card's turning frame rather than off the face,
whose node clips to the card, so it turns with the front and hides at the
quarter turn. The panel is placed as though it were the badge's reach wider,
so a preview opened at the window's left edge keeps its count on the screen,
and the bubble's clip lets it out as far
(`a_preview_keeps_its_count_badge_on_the_screen`). `BoardModel::group` is
how it and `/state` find the card a pointer is on.

**A merged card is a stack** (#261: "wie ein Stapel gerendert"). It stands
on its deck: the top card at the deck's height (`stack_rise`, 0.006 a card
up to thirty) and one slab per card under it up to fourteen
(`stack_layers`), children of the card built by `table::sync_stack`. A slab
is a card with no print — nothing under the top card carries an image —
jogged right and left in turn, and further out the deeper it lies: from
half of `PILE_JOG` under the top card to all of it at the foot (0.045 card
widths, about four pixels on a table card), so each side is a staircase of
edges. And the slabs on a side alternate between the back and a lighter edge
(`slab_color`, `SLAB_EDGE_COLOR`, twenty-odd display levels over the back),
so the layers stripe. The slabs wore the top card's identity paper while
the frame had a paper to wear, and at 0.012 a card that paper was all that
showed; with the frame gone, twenty Forests read as one Forest on a dark
block until #298 widened and staggered the jog and striped the edges
(`a_pile_shows_its_layers`; two constant assertions under `PILE_JOG` hold
the jog between half a keyword mark and the air a card has in its lane
cell). The library stays backs all the way down, face down
(CR 401.2; `a_library_is_backs_all_the_way_down`).

The deck follows the count. `sync_stack` rebuilds the slabs and the
contact shadow when the count changes, and nothing when it does not — rebuilt
rather than moved, because `ground_the_shadows` remembers a flier's resting
shadow by entity. Until #274 the deck was built once, when the card was
spawned: a group of Treasures growing from two to twelve under the same top
card kept one slab under a card that had risen to stand on eleven
(`the_deck_follows_the_count`). **Accepted:** in a fanned lane the jog lies
under the neighbours on both sides, so only a roomy row shows it; the badge
and the deck's own walls still say "stack" there.

When they merge depends on what they are (`board::group_objects`). Tokens
merge from two, on any row: a token is made to be one of many, and a fan of
Treasures says nothing their `×N` does not. Cards merge only once the row
would have to fan them (`pack_lane(..).fanned`); two Forests on a roomy row
are two Forests, because a second one swallowing the first was
`docs/observed-faults.md` 19. A token is what `board::provenance_of` calls
one, so a token copy of a card merges like a token.

Two independent guards keep the merge honest:

- objects merge only when every property a decision reads matches
  (`PublicObject::summary_key`): name, card or token, controller and owner,
  status (tapped, face down, …), the projected types, colours and keywords,
  P/T and the printed P/T under it, damage, loyalty, counters, summoning
  sickness and any granted mana. So a tapped Soldier stands beside the
  untapped ones, and a Soldier with lifelink beside the plain ones;
- objects with individual identity never merge, however identical they look —
  attacking or blocking (sent), enchanted, equipped, or targeted by the stack.

**In a choice, a merged card is a pool** (#210). What the answer being built
proposes for a permanent — declared at a defender, blocking an attacker,
picked as a target (`board::Proposal`, from `crate::proposals`) — is part of
what it is merged on, so the proposal splits the card the way a sent
declaration does: three of twelve Soldiers declared at a seat are a `×3`
stepping forward beside a `×9`, two sent at a planeswalker a card of their
own again. `table::track_proposals` rebuilds the board the frame the answer
changes, whichever door changed it. A click on a merged card is
`Interaction::toggle_group`: on the undeclared card it adds the next member,
on a declared one it takes the newest back. It draws past each card's first
member (`pool_order`), because a card is drawn as its first member and
taking from the back handed the declared card a new one on every click —
measured on thirty Elves as 31, then 30, then 29, a card replaced each time.
Now both cards stay the entities they were. The first split re-centres the
row once (one card becoming two moved the `×29` from x 864 to 828 on a
duel's row), so the second click lands where the card now is, and every
later one where the second did. A declared card draws one arrow, from its
first member (`combatlines::wanted_lines` finds no card for the others);
its badge says how many it carries. `⇧`-click and `⇧E`
(`Action::ActivateGroup`, `input::activate` with `whole`) take the whole card:
`toggle_all` adds members until the choice refuses one (a full answer, a
blocker the focus cannot take), and on a card with nothing left to add it
takes them all back. Until #210 a click toggled the drawn member only, so
the second click on twelve Soldiers took back the first and the whole card
stepped forward for a declaration of one. Sent attackers still stand one
card each; merging them by defender is its own change, because blocking two
of a merged five needs a focus that walks to the next unblocked member.

The board model also builds a text chip row per seat (`board::TokenChip`,
`12× 1/1 Soldier · 3× Treasure`) and a one-line threat read
(`ThreatSummary`: power ready, blockers, open mana, cards in hand), meant for
an unfocused pod at eight seats. **Neither is drawn** — measured for #210,
nothing in `baylee-client` reads either.

## Which ability is on the stack (and how a client names it)

The engine is free of card text on purpose, but a client still has to be able
to say *"Ondu Cleric's rally trigger is resolving"* rather than *"an ability
is resolving"*. An ability on the stack is its own object with no card of its
own, so the view carries what it points at:

```rust
PublicObject.stack_item: Option<StackItem>
// StackItem::Spell
// StackItem::Ability {
//     source: ObjectId,
//     ability: Option<AbilityRef>,
//     text: Option<StackText { face: u8, line: u8, of: u8 }>,
// }
```

`AbilityRef { card: CardIndex, index: u32 }` is the stable handle. `index` is
the position in that card's `CardDef::abilities`, so a client that knows the
card pool can map it to text; the reserved indices (`SPELL`, `ENTERS`,
`ADDITIONAL_COST`, `MIRACLE`, `UPKEEP_COST`, `SYNTHETIC`, `COMMANDER_ZONE`,
all counting down from `u32::MAX`) name the questions that are not listed on
the card, and `AbilityRef::is_listed_ability` separates the two.

The same handle addresses a seat's standing answers
(`PlayerAction::SetAbilityPolicy`), which is why it deliberately says nothing
about a particular game: the client keeps *"always say yes to Ondu Cleric's
rally"* in the account's preferences and sends it into the next one.

**Getting the text is the client's job, and no text crosses the engine
boundary.** The source is codegen, and what it emits is an *index* rather than
the text: `crates/baylee-cards/src/generated_lines.rs` says, per card and per
**face**, which printed sentence each ability came from, and
`crates/baylee-cards/src/lines.rs` is the reader
(`lines::ability_line(card, face, index)`). A client then splits the printed
text of the printing and language its player chose and takes that sentence, so
a loyalty ability draws as what it does instead of as `+1`.

Three things about it are load-bearing. It is **generated**, because the
answer is read out of the English oracle text against the compiled ability
list and a running game holds neither — `xtask` is the one place that links
both, and `codegen --check` on a developer's machine is what keeps the table
from going stale (it cannot run in CI: a runner has no card-script
reference). The unit
is a **face**, because abilities are per face (`abilities_for_face`, and a
back face never inherits) while `AbilityRef` carries no face: whoever looks a
line up supplies it, and for a stack entry that is *not* the face the source
object is showing — see below. And the English sentence *count* travels beside the index
(`AbilityLine::of`), because the index is resolved against a text the host
did not count: the client's compiled Oracle, which `refresh-oracle` can move
without a view bump. `cardtext::sentence` refuses a face whose count differs
(`a_line_counted_against_other_text_draws_nothing`) — an index merely out of
range is caught by anyone, but one that is in range and off by one is shown to
the player as precise text, which is worse than `+1`. Only the stack can meet
the refusal, because only its coordinate is the host's; the ability sheet and
the cast chooser read this build's own line table. On a refusal the stack
places the ability by its `AbilityRef` in that same table (`stack_sentence`),
so the row still reads the ability's own sentence; an entry with no
`AbilityRef` keeps its source's name. A translation that
joins two lines is no longer this guard's case: `baylee_cardtext::align`
places the printed lines against the Oracle, and one that does not pair
draws the Oracle's line.

It does not reach every ability, and the misses are honest: 485 of the pool's
500 stack-capable abilities know their sentence, and every one of the fifteen
that do not is a **trigger** — an evoke or echo trigger printed as a keyword
line rather than as a sentence, a quoted sub-ability inside a copy sentence, a
land's "when this enters untapped" beside its own enter condition.
`cargo run -p xtask -- ability-lines` is the report that names them;
`baylee_cards::lines`' tests are the floor.

A keyword line **is** a sentence where the keyword is an activated ability,
and the card's own punctuation says which: `Equip {2}` and `Cycling {1}{B}`
print their cost on the line, and `Station (Tap another creature you control:
…)` prints only the word and spells the ability out in its reminder, colon and
all. A keyword that is a *bit* never does — `Flying (This creature can't be
blocked except by creatures with flying or reach.)` has no colon anywhere in
it. Before `lines::reminder_spells_out_an_ability` read that, the two
Spacecraft in the pool had no sentence for their only activated ability and
the sheet drew the row as "Ability 1".

**A mana ability is placed and is not counted.** It was neither: a mana
ability was `LineShape::Other` on both sides of `lines.rs`, so the table held
`None` for all 408 of them and the ability sheet composed a label of its own
("Tap for {G}") out of the `ManaSource` — which is what the owner asked to
stop seeing, since the card prints a better sentence than the client can
write, in the printing's own language. `LineShape::Mana` is that shape, said
on **both** sides for the reason Karakas gives: it prints two `{T}:` lines and
only one of them is the mana, so a half-applied exclusion let the bounce
ability fit both. What it changes is only which cell the table holds, never
the denominator — a mana ability does not use the stack (CR 605.1), and
`LineShape::stackable` is the one place that says so. Two lines with the same cost are separated by what they
*make* (`lines::mana_fits`, the same job `loyalty_head` does for a walker);
Yavimaya Coast's `{T}: Add {C}` and `{T}: Add {G} or {U}` are why. The one
tap printed nowhere keeps a composed label: the CR 305.6 shortcut, which a
Bayou's text does not mention. A granted ability is printed too, only not on
the land it is granted to, and it has its own paragraph below.

**The cost column is the sentence's own head** (#212). A row draws its cost on
the left and what the ability does beside it, and both halves come out of the
one printed sentence: `abilitysheet::cut` splits it at its cost colon
(`baylee_cardtext::split_cost`: the first `:` or `：` ahead of any reminder or
quotation, with no length cap), and the head is the column, in the card's
words and the player's language: `{1}, {T}, opfere dieses Artefakt`. The
ability's own cost licenses the cut and is never drawn beside a sentence.
`AbilityOption::cost` holds only its symbols (`{2}, {T}`, or a walker's
`{L+2}`). A head that prints a symbol the ability does not cost belongs to some
other sentence, and a head in words alone is licensed only by a cost with no
symbols, so a colon inside an effect never passes for a cost. A sentence with
no cost colon, which is every keyword line (`Equip {1}`, `Level up {1}`,
`Reconfigure {R}`, `Station`), is drawn whole with an empty column. Only a row
the card prints no sentence for draws the ability's symbols there.

**A granted ability is its grantor's sentence** (#212). The land under a
Chromatic Lantern prints nothing about the `{T}` it was given; the Lantern
prints it, in the sentence saying lands "have" it (CR 113.10). The view names
that sentence per grant (`PublicObject::grants`, view version 31,
docs/protocol.md §"Who granted it"), and the row draws the grantor's name
over it (`abilities::grant_words`): *Chromatische Laterne* over *Länder, die
du kontrollierst, haben „{T}: Erzeuge ein Mana beliebiger Farbe."*, through
`cardtext::said` like every other sentence, so it is the player's language
where the grantor's text has arrived and the compiled English before. Both
lines and not just the sentence, because the row is on the land and the
sentence is about lands: without the name, nothing on the row says which
permanent to look at. The cost column stays empty. The engine's cost is the
granted ability's and the sentence's head is the grantor's, so no head is cut
and the sentence is drawn whole. `AbilityOption::label` is the grantor
face's English name (the fallback line, the armed shelf, the redraw
fingerprint). A grantor the view names with no sentence (nothing on its card
wrote the grant by value) is its name alone. A grantor this seat may not see
(gone to a hand or a library, face down) is an entry with nothing in it, and
the row is "Granted ability", as every grant was before. A granted *mana*
ability whose output the view states (`PublicObject::granted_mana`) is mostly
not a row at all: `abilities::pour_out` turns it into pips, which draw a
colour and no words, so the Lantern's sentence is read on a land only where
the view could not reduce the grant to colours.

This replaced seventeen `Phrase::Cost*` wordings of `CostPart` (`Sacrifice
this`, where the card prints `Sacrifice this artifact`) and a cut at the first
`: ` under 48 bytes, which drew Gemstone Mine's fifty-byte German cost as an
effect. `every_written_row_draws_its_printed_cost_or_its_whole_sentence`
sweeps the pool offline against the compiled English Oracle: 1302 rows draw a
printed head, 19 cards draw whole (named in the test and held equal both
ways), and none is refused. The armed shelf draws the same head
(`abilities::printed_words`), and where a sentence has none, which is a
keyword line that is its own cost (`Equip {2}`, `Ausrüsten {2}`), the whole
line. The stack cuts only a walker's badge
(`abilitysheet::loyalty_cut`), because it has no cost to license any other
cut with.

**The text is asked for by card, and the English Oracle is under all of it**
(#212). `crates/baylee-client/src/cardtext.rs` asks `GET
/catalog/text?lang=…&oracle_ids=…` of the gateway the lobby is signed in
to, with that session and to no other host (`cardtext::TextGateway`, set by
`lobby::systems::text_follows_the_session`; the route answers a session
only, #270), about the cards the seat's view names
(`PlayerView::cards`: the hand, each public object's card and the card whose
rules it has, each stack ability's card), each card once per language, one
request out at a time, at most 500 ids in it. The key is the card and not the
printing because text belongs to the card (`docs/protocol.md` §"Card text"):
the gateway picks one printing per card and language
(`baylee_cardtext::pick`), and a copy's abilities are printed on the copied
card, which the copy's own printing says nothing about. `PlayerView::cards`
walks the same objects `prints` does, so a seat asks about no card it has not
been shown: a face-down permanent names none, and another seat's hand is a
count. Under English nothing is asked at all, because the compiled Oracle is
the English. A gateway that does not answer, or answers `400` because it
predates `oracle_ids`, is not asked again until the language or the game
changes.

A face or a row never waits on that request. `CardTexts::face` falls to
`cardtext::english` (the registry's name and cost, `pool::type_line`,
`oracle::face`), and `cardtext::sentence` draws the served `printed` line
where `baylee_cardtext::align` pairs it with the Oracle (placed once, when the
entry is filed) and the Oracle's own line where it does not. An offline duel
reads English on every card, and a translation that joins two lines costs
that card its German and not its words — "Fallback ist immer englisch".

The Scryfall door this replaced (`POST /cards/collection`, by printing id) is
gone: a deck names English printings, so all it could fetch was the English
the Oracle now compiles in. Its successor asks by card and language. While the
gateway does not answer (offline, signed in nowhere, or not running) and the
language is not English, `cardtext::ask_scryfall` searches `oracleid:… lang:…` for one card
the view names without text, then waits `PACE` (0.3 s: Scryfall asks for ten a
second at most, and a game is a guest there) before the next.
`scryfall::printings` sorts the answer in the catalog's `ORDER BY` and
`baylee_cardtext::card_entry` builds the entry, so it is the entry the gateway
would have served. A 404 is an answer (no printing in the language); a failure
closes this door until the next game as well. A gateway that answers from an
English-only catalog is not second-guessed. The deck builder sends the same
search (`scryfall::search`) and reads the answer through the same
`scryfall::entry`, so the lobby, the game and the gateway pick a printing by
one rule: where no printing in the language translates the rules text, the
most complete local printing (#239), which is why a name-only promo no longer
hides `Basisland — Wald`. The cache is one document per language, keyed by
card and written from the whole table; an entry written before the rekey names
a printing and no `oracle_id`, and is dropped when read. Panels redraw on
`CardTexts::generation`, which moves on every filing — a count of cards does
not, since a fresher entry replaces a cached one.

**The host does the lookup, and the face is the part that is easy to get
wrong.** `gamehost::view::stack_item` fills `StackText` in, which is why a
client needs neither the card registry nor the table — `baylee-client-core`
does not link `baylee-cards` and is not going to. The face it sends is
**not** the one the source object is showing: an ability on the stack is
independent of its source (CR 113.7a), so a Sheoldred who has turned back
over while her chapter ability waits would name the front face's list, the
chapter index would land *in range* on it, and a wrong sentence would be
drawn as precise text — which `of` cannot catch, both counts being English.
The right answer is already on the object for free: `own_abilities` is the
very `&'static [AbilityDef]` slice `abilities_for_face` returned, captured
when the ability was put on the stack (CR 608.2), so `ability_face`
identity-compares it against each face's list. A copy therefore answers
`None` — it carries the *copied* card's list while its object names the
physical card — and that refusal is the point: a Spark Double drawing the
double's own sentence would be a stranger's text on the stack.

The client indexes that same face's localized text, and borrows that same
face's picture, so the sentence and the art beside it are the two halves of
one card.

**The same table answers a question that is not about the stack at all.** A
*mode* and an *alternative cost* are printed sentences too, and neither is an
ability: the whole modal ability is one block of text and what a player picks
between are the sentences inside it, while an alternative cost is a field on
the face. The engine names them `CastModeKind::Mode(i)` and `Alternative(i)`
and says nothing else — there is no label in the protocol and there could not
be — so the cast chooser drew "Mode 2" while the ability sheet beside it had
been drawing the card's own sentence since it existed. `FaceLines` therefore
carries two more arrays, `modes` and `alternatives`, read by
`lines::mode_line` and `lines::alternative_line`, and they are two arrays
rather than one because they index two different lists exactly as the two
kinds do. Both are about **face 0**, which is a fact about `cast_wizard`
(it enumerates modes out of `abilities_for_face(0)` and alternative costs out
of `def.faces[0]`) rather than about the table, so the caller states it.

**A mode is not only a spell's**, and reading it as one was the whole of a
defect. `AbilityDef::ModalTriggered` is a triggered ability whose controller
picks a mode as it goes on the stack (CR 603.3c), it asks with the same
`Pending::ChooseCastMode` and the same `CastModeKind::Mode(i)`, and it carries
the same `&'static [SpellMode]` — and the walk that writes this table matched
`ModalSpell` alone, so every one of the pool's six modal triggers arrived with
an **empty** row and drew "Mode 1 / Mode 2" over a card that prints the
sentences. Nothing said so, because an empty row is also what a card with no
modal ability has. `lines::face_modes` is the one reading of both, shared by
the walk that writes the table and the tests that hold it against the pool,
because two spellings of it is how the first one went unnoticed.

It takes the **first** modal ability on the face, and that is a contract
rather than a shortcut: `Mode(i)` names a mode and not the ability it belongs
to (the pending carries an `ObjectId` and nothing else), so a face whose modal
abilities offered *different* modes could not be labelled at all, whatever
this table held. Derevi, Empyrial Tactician is the pool's face with two of
them — one printed sentence, two trigger conditions, two `modal_triggered!`
abilities over one set of modes — and `a_face_offers_one_set_of_modes` is what
keeps that true as cards are added. The trigger's face is assumed to be 0 for
the same reason the cast options' is, and with the same latency: every modal
trigger in the pool is on a front face.

The reader is `codegen::lines::map_modes` and `map_alternatives`, and both
obey the transcoder's honesty rule — one unread clause and the card is refused
whole, because a row carrying the card's own words on the *wrong* mode is
worse than the number it replaced. A modal card prints its modes one of three
ways: as **bullets** under a "Choose one —" header, where the bullet count has
to equal the mode count or nothing is claimed; as **overload**, where the
card prints its body and a keyword line and a mode with a `cost_override`
finds the line printing that cost; or under "**choose up to one** —", which
prints one bullet *fewer* than it has modes, because the mode a player
declines with is printed nowhere at all. That third one is Ertai Resurrected,
whose declining mode is `mode!(&[])`, and it was worth a rule rather than an
exception: read as an ordinary bullet count the card is one short, and
refusing it cost its two perfectly good sentences their rows as well. Both
halves are asked for — the printing says "choose up to" *and* exactly one mode
does nothing — because either alone is a guess.

What no printing of these three covers is a choice stated **inside** one
sentence: "you may tap or untap target permanent", "put your choice of a
+1/+1 counter or two charge counters". There neither mode *is* a sentence, so
the number is the honest label and the reader refuses. Derevi and Inspirit,
Flagship Vessel are the two, and they are named in a list
(`MODES_PRINTED_INLINE`) rather than tolerated as a count, so that the next
card reading as unknown stops a build instead of joining them in silence.

An alternative cost is printed either as a sentence — and the two
templates are themselves the discriminator, "without
paying its mana cost" being the cost of nothing and "rather than pay" one that
was substituted — or as a keyword line (`Evoke {2}{U}`,
`Evoke—Exile a white card from your hand.`), held to the single word in front
of it so that an ordinary sentence cannot fit a cost with no symbols. Every
non-mana part has to be named in the printed words (`parts_fit`), and a
sentence fitting two options, or two sentences fitting one, is refused rather
than guessed at. All 9 modal-*spell* modes and all 8 alternative costs in the
pool map, and `every_mode_and_alternative_cost_knows_its_printed_sentence`
holds both as an equality: an ability may honestly have no sentence, a mode
of a spell never can. Of the 15 modes the six modal *triggers* carry, 10 map
— the other five being Ertai's decline and the four Derevi and Inspirit print
inside a sentence — and those two shapes are asserted from the side that says
which, an effect-less mode having to know *no* sentence and the inline cards
being named, so the exception cannot quietly widen.

### Combat and table panels (September 2026)

Combat arrows use the projected life-value anchor from the seat identity,
including opposite seats and camera rotation. Object defenders still use the
actual card position. Their continuous warm core, soft halo and moving current
keep attack and block paths readable without hiding card faces.

`client-core::strike::between` reads damage-step edges, using the previous
battlefield so creatures killed by that damage still participate. A first
snapshot and repeated snapshots are silent. `combatfx` presents first strike
as a short gold slash and normal strike as an amber impact with a heavier
synthesised cue. The card's lunge is a temporary offset to its glide goal;
there is no accumulating transform or rules change. Live impact geometry is
capped at 48 entities. Reduced motion keeps the cards still and sound continues
to respect the sound preference.

The stack, zone browser and ability menu share the table's dark surface and
gold accents. The stack's plus/minus button animates a clipped, scrollable body;
its title and count remain visible. Its fold progress survives HUD rebuilds.
The zone browser opens at 900 × 738 logical pixels, fitted to its available
band, and its entrance does not restart when filtering or receiving artwork.
The ability panel has a 440-pixel maximum width and high-contrast keycaps.
Where the printed text has not arrived, a row draws its **cost** and its key
and no words at all — it never composes a category of its own. It used to, and
the owner's instruction is what removed it: *„es gibt keine sondercases,
überall steht der original skryfall text in der clientsprache — fallback auf
englisch."* Two labels for one card are two wordings of it, and only one of
them is on the cardboard.

Hand hover raises a card by 12 pixels through the existing touch spring and
keeps that card in front. Flying creatures gently bank through their motion
target, and inverse-parent shadow transforms keep their shadows flat on the
table while they bank and tap. Losing flying restores the original contact
shadow. Inspection and reduced motion stop the idle banking.

### The stack panel draws it

`hud::spawn_stack_panel` is where that stops being theory. Each entry is a
card, not a line of text: the spell's own picture, or — for an ability, which
has no card at all — the picture of the permanent it came from, borrowed
through `StackKind::Ability { source, text }`, at the face `text` names.

**The entries are not peers.** The next thing to resolve is a *full* row — a
72-pixel card, the name at 16 px over up to two lines, what kind of thing it
is and whose (`Ability · Llanowar Elves — *You*`, the seat in the slant
because it is the one word that names a person), then an arrow and a picture
of everything it points at. Everything under it is a *compact* row at about
two thirds of that: a 46-pixel card, one line of name cut to fit rather than
wrapped, smaller thumbnails, no arrow and no subtitle. The size ramp **is**
the depth cue, which is why there is no numeral beside the rows — position
already carries the order and the badge already carries the count.

The panel is capped at 62% of the window height. Its body scrolls through
all entries using fixed-height rows (164 px for the next entry, 82 px for
queued entries), a bounded rendering window and height-preserving spacers.
Scroll position survives hover, selection and language changes.

Selecting an entry marks a stopping point. “Resolve to selection” passes
priority until that entry reaches the top, then stops **before** it resolves.
The engine pauses standing yields at this boundary and the client also suspends
its phase autopilot until a manual action. Legal target selection takes precedence
over marking a stop. Removing the marked object also cancels the hold.

For the selected ability (or the top ability without a selection), independent
“Always pass” and Ask / Always yes / Always no controls apply to its exact
`AbilityRef`, across copies of that card. Preferences persist locally and through
account settings; individual rules or all rules can be cleared in automation
settings. Only optional, automatable yes/no prompts accept standing answers;
targets, payments and other choices remain manual. Rules are applied to the
engine as seat actions, which move no journal entry, and are included in
deterministic snapshots. The engine keeps a seat's automation for the whole
game, so the client sends each order once a game: the view its card first
shows up in (`PlayerView::cards`), or when the settings change it. An order
for a card the table has not shown is not sent at all, so the host learns
nothing about the account's other decks (#285). Explicit stops and
cancellation take precedence over standing yields.

Under the title sits one more line
the prompt slip cannot carry: whose answer the table is waiting for, from
`PlayerView::awaiting`, and nothing at all once the game is over. It read
`PlayerView::priority` until `VIEW_VERSION` 23, which answered a narrower
question than the line asks — a seat picking blockers or discarding to hand
size holds no priority, so the line went blank on exactly the questions
between two of an opponent's spells that it exists for.

**The panel is 352 px wide because 296 was not wide enough for the names.**
The name column of a queued row is the panel less the padding either side, the
row's own padding, the rail, the card and the gap, which came to 187 px —
measured against the advance widths of Inter, which the client shipped then,
58 of the 1475 faces in the pool (3.9%) did not fit that at the size they are
drawn, and a further two dozen were cut short by `fit`'s estimator although
they would have. (In the Faustina that replaced it only 3 of the 1475 overrun
that column, so the widening is not what the serif needed; it is what the
estimator needed, and that half of the argument is untouched.) The estimator is the part worth knowing about: it budgets characters at
a flat `0.52 × size`, and a real name runs anywhere between 0.41 and 0.70, so
it was wrong in **both** directions at once. Widening is what makes it safe
rather than merely luckier. At 352 the queued column is 267 px at 14 px, the
budget is 36 characters, and over all 1475 faces nothing clips and nothing is
cut that would have fitted — the longest, `Okina, Temple to the
Grandfathers`, is 245 px. The full row does not call the estimator at all: its
name *wraps*, capped at two lines by a `max_height` over a clipped node, which
is a bound rather than a claim about the pool — a printing this client has
never seen gets two lines and a clean edge instead of a third line that pushes
the queue out of the panel. If a name column is ever narrowed again, `fit` is
the thing to replace with a real measurement rather than to re-tune.

**The sentence *quotes* the card, so its mana symbols are marks and not
discs.** `{T}: Add {G}.` had been drawn with its braces, which is the one
place in this client a symbol was spelled out in letters, and the obvious fix
— `manaui::spawn_rich`, which every other surface uses — is the wrong one
here. That builds a wrapping flex row of words and discs; the name and the
subtitle directly above it are a real `Text` flow, so a third surface set by a
second typesetter would sit in the reader's eye every time the panel is open,
with looser word gaps, no kerning across a word boundary and a hand-rolled
ellipsis that no longer knows what four lines are. `manapip::inline` is the
other reader over the same brace scan: a symbol with one glyph becomes a bare
`Inline::Mark`, set as a `TextSpan` in the `mana` font at `STACK_MARK` of the
prose's size and in the **prose's own ink**, so the sentence stays one flow.
A hybrid has no glyph — it is one disc with two halves — and is quoted as the
two marks with a slash, the way the oracle text writes it.

The rule behind that is worth keeping, because it decides the next case too:
**showing a card is not quoting one.** The hover preview and the deckbuilder
*show* the card, and a printed disc belongs there — colour identity is
information a player reads off it. The stack sentence is a caption that quotes
the card's text, at twelve points in the row's muted ink, where the disc is
about ten pixels across and the mark inside it — the only part carrying "tap"
— is exactly what a disc that small takes the contrast from. Colouring the
bare mark instead would put the most saturated ink on the row in a register
that is otherwise muted, and a warm red glyph would land close enough to the
candle accent to be read as a claim about what the engine is offering — one
hue, one claim, which is the rule every other border and light in this client
already answers to. So the shape is the claim here and the grey stays. The
preview is not changed to match, and that is the point rather than an omission
— the two are almost never on screen together, and consistency *within* the
panel is the one a reader experiences.

Splitting comes before budgeting, which is the one thing easy to get backwards:
`{T}` is three characters of source and one mark on screen, so the old
`cut(block.text(), room)` charged a symbol three times over and dropped text
the row had the space for.

**A row arrives rather than appearing**: it lifts 14 pixels into place, grows
from 0.96, and its ink, its fills and its accent rail come up from nothing
over about a fifth of a second (`ARRIVE_RATE`, the same `1 - e^(-rate·dt)`
everything else in this client eases with). Three things about that are
load-bearing.

The progress cannot live in the row. The HUD is retained and rebuilt whenever
`HudRevision` changes, and *hover* is part of that gate, so a pointer twitch
mid-arrival despawns the row and spawns it again; progress kept in the entity
would restart on every one of those. It lives in the `StackMotion` resource,
keyed by `(ObjectId, is the row full)` — the shape is part of the key so that
a **promotion**, which is what a resolution looks like from the panel's side,
eases into the full row instead of cutting to it. Departure is not animated at
all, and deliberately: the object that resolved is gone from the view, and
drawing a ghost of it would be drawing something the view no longer carries.

**A promotion runs the arrival backwards, and that is the resolution
animation.** It was being drawn as an arrival — lifting into the slot from
above, which is the movement of a spell *landing* on the stack and the
opposite of what happened. A promoted row now starts `PROMOTE_LIFT` below its
place and at `PROMOTE_SCALE`, so it comes up out of the slot it was queued in
and grows, and its rail lands at `INK` and cools to `ACCENT` on the slower
`SETTLE_RATE` — still settling after the row has stopped moving, so the accent
mark is seen travelling one slot down the queue. That is "a spell resolved"
told as the movement it is, with no ghost of the object that left.
`a_promoted_row_comes_up_from_the_slot_it_was_in` asserts the *sign* of the
lift, which is the one thing that tells the two apart: the alpha ramp is
identical for both, and it carries its own counter-test — a spell that is
merely cast still drops in from above.

What is **not** there is a stagger when several rows land in one frame, and the
reason is not that the depth is out of reach: `spawn_stack_panel` walks the
stack in depth order and could bake a delay into the row on the way past, and
because the progress lives in `StackMotion` rather than in the row, re-baking
that delay onto a row already at rest would change nothing. It is the **fade**
that makes it expensive. A delay has to reach every node the arrival touches,
and `Arriving` is built at nineteen places in `hud/stack.rs` — or `StackMotion`
holds seconds instead of progress and every one of those nodes remaps it. The
panel's own fade and slide already cover a panel going from empty to populated,
so this is "not worth it today" and not "impossible".

The *other* direction of that key change is carried across instead of eased,
which is the case the panel is most often in. A spell landing on a stack that
already had one demotes yesterday's top to a queued row — a new key for an
object that has been there all along — and seeded like an arrival it would
fade in beside the newcomer, so the player would watch two spells land when
one did. `ease_the_stack_in` therefore seeds a new `Entry(id, false)` at
rest whenever `Entry(id, true)` was being tracked in the frame before;
`a_row_that_steps_down_does_not_announce_itself` holds it, and
`a_row_that_is_promoted_still_arrives` holds the asymmetry.

The card picture is faded by a **veil**, not by an alpha. The art is a
`MaterialNode` on a material shared with every card that looks the same
(`a_look_is_shared_by_exactly_what_looks_the_same`), so an alpha written there
would fade the player's hand along with it; an absolutely-positioned child the
colour of the empty slot fades exactly this one card and costs one node.

And every faded node says *which* colour it fades. `Node` requires a
`BackgroundColor` and a `BorderColor`, so a line of text carries a transparent
background whether it draws one or not — a system that wrote whichever colour
it found multiplied that transparent black's alpha back up and gave every
label in the panel an opaque plate. The first live shot of the panel was eight
of them where the words should be, while every arithmetic test passed;
`fading_a_label_does_not_give_it_a_plate` is the test that would have caught
it.

**A row is a card, so it answers a click like one.** This is the one place the
panel was unfinished rather than merely plain: a duel stopped dead on a
`ChooseTargets { options: [200], min: 1 }` whose only option was a spell on
the stack, and there was no way on either device to say it. The pointer found
nothing — every node in the panel carried `Pickable::IGNORE` except the
66-pixel picture inside the row — and the keyboard found nothing either,
because `cursor_grid` was built out of the *board* and the stack is not on the
board. The model had always allowed it: `Interaction::toggle` takes "a
permanent, a card in a zone, or a spell on the stack". Both halves are wired
now. The **row** carries `HandCardVisual`, so a click on the picture, the
name or the printed sentence goes through `activate_card` and lands on
`toggle_group`, which for anything but a merged table card is `toggle` —
nothing earlier in that chain is ever true of an object on the stack — and the picture inside it is `Pickable::IGNORE`, because a pickable
child would take the row's hover for itself and the row would never light.
`cursor_grid` gains the stack as its last row, which is the topmost, because
the panel is drawn highest. `a_click_on_a_stack_row_answers_the_question_it_was_asked`
goes through the real `pointer` system and the real message rather than
calling `toggle` by hand, and `the_card_cursor_reaches_a_spell_on_the_stack`
is the keyboard's half.

What a row then says about itself is the hand bar's grammar, unchanged. A
spell the pending question would accept wears the same `ACCENT` halo a card in
hand wears, at the same three weights — 0.70 offered, 0.85 offered and
hovered, 1.0 chosen — out of the same `hand::halo`, because "the rules accept
this as an answer" is one claim and a player must not have to learn it twice.
It goes on the **picture** and not on the row, which keeps the row's own teal
honest: the rail marks a *slot* and is a bar, the halo marks an *object* and
is a glow around a card, so a counterspell aimed at the top of the stack reads
as "this card, in this slot" rather than as a louder slot. Hover is the row's
own ground one step lighter — `PANEL_HOT` for the full row, alpha 0.62 for the
second, 0.30 for a row that rests at nothing — and it is built into the tree
rather than animated by a `Feel`, because `ease_the_stack_in` writes that same
`BackgroundColor` for as long as a row is arriving, and two writers with
different opinions about the alpha would fight for a quarter of a second every
time a spell is cast. The overlay is rebuilt on every hover change anyway.

`devctl`'s `/state.cards` reports the stack as a third zone beside `table` and
`hand`, read off the `StackRowCard` marker. It is not a convenience: a driver
could read `interaction.pending`, see a `ChooseTargets` naming object 200, and
have no way at all to find object 200 on the screen.

The resolution happens in `baylee-client-core`, not in the renderer:
`BoardModel::from_view` turns each `TargetRef` into a `StackTarget { what,
name, art }` by looking the object up through `PlayerView::object`, which is
the one place that has the view. A `TargetRef` alone is a handle; a panel that
wants to *show* what is being targeted needs a name and a face, and doing that
lookup once, testably, without a GPU, is the whole reason the split exists.
Player targets keep `name: None` deliberately — seat names live in
`GameStatic`, which the board model has never carried — so the renderer spells
those out and draws them as a chip rather than as a rectangle pretending to be
a card.

Two consequences worth knowing. A target's art is added to
`BoardModel::required_images`, because a spell can point at a card in a
graveyard that nothing else on screen is drawing. And an ability whose source
has already left the battlefield (CR 113.7a) has no picture to borrow: it
draws as a name with an empty plate, never as a missing entry.

### And the hover writes it out in full

The panel abbreviates on purpose — a queued row draws no sentence and a full
row cuts one to `STACK_SENTENCE_LINES` — so **the hover is where the whole
sentence is read**. `hud::slip` hangs a sheet of parchment under the preview's
card carrying that one sentence entire: the player's own printing, in the
player's own language, with `{T}` and the pips as marks and the reminder text
in the same quieter slant the row uses.

Four decisions in that, and each is easy to get backwards.

**It is the bubble and not the row.** §8.2 of the redesign already made the
preview "the only place rules text is read at size", and a second such place
inside the panel would be two answers to one question. It also has a concrete
failure: the overlay is rebuilt on every hover change, so a row that grew
under the pointer could push the pointer out of itself, which changes the
hover, which rebuilds the row collapsed — a two-frame oscillation that
`pointer_hover`'s grace window would not suppress, because the pointer really
has moved relative to the tree.

**It is the sentence and not the card.** A card prints several abilities and
the stack holds one; `stack_sentence` already resolves which. For a *spell* it
is the cast face's whole rules text, because on the stack the object **is**
its text about to resolve — the stack is the one zone where "what does this
card look like" is never the question. The exception is the text face: when
the player has asked for it (`FaceCtx::always` — the held modifier or
`prefer_text_view`), the face is the slip and the sheet is not drawn. Asked
about the *choice*, never about `preview_face` having returned something: a
constructed face is also what fills in while art is in flight, which is the
ordinary case offline, and is exactly the case where a card scan with English
rules text on it looks as though this already worked.

**Paragraphs are split before `split_blocks` is asked, not after.** A card
prints paragraphs and `card_face::split_blocks` does not say where they ended:
it splits on the newline and on a reminder's brackets alike and hands back one
flat list. Drawn flat, Aminatou's three loyalty abilities came out as
`…oben auf deine Bibliothek.−1: Schicke…`, with the second one starting in the
middle of the first one's last line — the first thing the live shot showed. So
the printing is cut on `\n` first and each paragraph handed to `split_blocks`
on its own, which is then exactly the reminder split a paragraph wants. Each
becomes a `Text` of its own, `SLIP_PARA_GAP` apart, and the wipe stays **one**
veil over the whole page: the ink is meant to arrive down the sheet in one
movement, not once per paragraph. An ability on the stack is one sentence and
therefore one paragraph by construction, which is why only the spell branch
splits.

**The progress is a resource.** `SlipWash`, for the same reason `StackMotion`
is one: hover is part of the HUD's rebuild gate, so the slip is despawned and
respawned by the very pointer movement that opened it. The nodes are spawned
**at rest** and `wash_the_slip_in` runs after `sync_overlay` to take the ink
back off — a rebuild after the wash has finished therefore draws nothing dim,
and one mid-wash re-attaches where it was. Walking down the panel re-inks the
sheet without rebuilding it: the ground restarts only when there was no slip a
frame ago.

The wash itself is three stages, and the middle one is the only thing in this
client that is not an exponential. The sheet comes out of the card's foot at
`ambience::FEEL_RATE`; the ink fades up at the same rate under a veil whose
edge travels down the page **linearly over a duration**, because an
exponential wipe rushes the first line and crawls through the last and reading
order is the whole reason the wipe exists; and the ink lands a quarter of the
way back towards the sheet and dries to its own colour on the stack panel's
own settle rate. Wet ink is lighter and browner. Legible at about 200 ms,
finished at about 450, and `reduce_motion` collapses all of it to a sheet that
is simply there, written, on the first frame.

Two smaller things the sheet costs. It is part of the bubble's *size*, so it
is resolved before `preview_art_size` is asked how large the picture may be —
a card sized to the window with a sheet hung under it would put the sheet off
the bottom of the screen. And the resize handle is measured from the card's
foot rather than the bubble's, or it lands in the middle of a sentence.

What is deliberately **not** here: a hint on the queued rows that there is
more to read. The hover is the invitation and the full row's ellipsis is the
honest sign; a glyph saying "more" on six rows at once would be the wall of
text the ramp exists to prevent.

## Images and memory

Keyed by printing id — no API call is needed to render a board.

**A token is the one thing on the table with no printing**, and for a long
time that meant it had no picture: `card` was `None`, so the board model
asked for nothing, the renderer fell back to drawing the card's own face, and
a Soldier was a flat white rectangle with its name across it. An `ImageKey`
therefore names an `ImageSource` — a `Print` or a `Token` — and a token's id
is the one the engine already stamps on the object (`PublicObject::token`,
whose doc says it exists for exactly this). What it resolves through is
`TokenDef::scryfall_id`, a printed token card chosen per token in
`baylee-cards/src/tokens.rs`, and the choice is deliberate: the picture
carries the token's printed text, so a 4/4 Angel with flying may not wear the
art of the 4/4 Angel with flying *and vigilance* that most sets print.

Two consequences of putting the lookup on `TokenDef` rather than in the
client. It is a `resolve` **parameter** and not a process-wide cell, because
`baylee-client-core` does not link the card registry (the `manaplan` /
`manasources` seam) and because a cell set on one path is already a known
fault — `ART_BASE` is that, and `docs/observed-faults.md` entry 2 is where it
is written down. And the same parameter shape then took the second source
that has no printing.

**A copy is drawn as the card it copies**, and the view says two things at
once for it. `PublicObject::card` is the *cardboard*: a copy effect assigns
characteristics and never a printing — CR 707.2 lists the copiable values,
CR 109.3 lists the characteristics, and art is in neither — so a Clone is a
Clone in every zone it visits. `PublicObject::name` is the *projection*, and
that is what the player is looking at. Reading the first under the second drew
a Clone with "Llanowar Elves" written beneath it, which is a card that does
not exist; a token some copy effect made was worse still, carrying neither a
print nor a registry token id, and drew as a coloured rectangle with a name
on it.

The handle both have is the name, so `board::Registry` is what a board model
is handed to turn one back into a picture — the second thing it cannot answer
for itself, beside the lane widths, and the same shape `resolve` already used
for `token_art` one paragraph up. Its `named` closure is
`baylee-client`'s `cardart::named`, which answers `board::Wears`:
`Card(index, face)` for a card in the pool, `Token(id)` for one of
`baylee_cards::tokens::ALL`, `None` for a name neither table prints.
Three details decide whether it is right:

- **The answer only counts when it disagrees.** A permanent copying nothing
  answers with its own card, so the disagreement between what the registry
  says the name is and what the object *is* is the test for a copy, and
  `board::art_of` asks it of every permanent rather than looking for a flag no
  view carries.
- **A registry token is asked about its own name, by id.** It is the one arm
  that cannot compare handles, and the paragraph on copies of tokens below is
  why.
- **The face travels with the index.** A copy of a transformed permanent takes
  the name the *back* is printed with, so the lookup answers `(index, face)`
  and a card the table is showing the back of is not drawn front-up.

What it does not do is pick the *printing*. There is none to pick: a printing
is not a characteristic, so codegen's reference printing is as true a Llanowar
Elves as any other. An exact printing would need the view to carry what the
permanent copies, which is a `VIEW_VERSION` change and gamehost's to make —
`docs/observed-faults.md` entry 16 is where that half stays open.

**And the same judgement says so on the card.** Drawing a copy correctly means
the board no longer admits that it *is* a copy, which is the fault the entry is
actually named for, so `board::worn` — the disagreement, factored out of
`art_of` — also answers `board::provenance_of`: `Printed`, `Token` or `Copy`,
one value of three, which is what makes the last two exclusive. A token some
copy effect made is a **token**: the chit is the whole truth about it and there
is no original to go and look at, so a copy mark would promise one. A
face-down permanent is `Printed` and wears nothing — its `card` is `None` for
the same reason a token's is and is not the same fact at all, and `is_token`,
which was that field, called every opponent's morph a token for as long as
nothing read it.

The mark is a **crest** at the strip's right end (#298): a square of paper,
verdigris for a token, violet for a copy, oxblood for a commander, with the
glyph printed on it — see "The card surface". It was the frame's own paper
while there was a frame (#274), and an **identity slip** under the printed
name before that: the Mana font's `ms-token` (a
squirrel), `ms-ability-copy` (two cards) and `ms-commander` on those papers,
sampled out of the same atlas the keyword marks come from. Before the slips it
was two **fixed rows** in a column in the right margin, and before those a
filled disc in the card's **top-left corner**. All three homes were on the
print. The paper is what carried the distinction even then, and it is the
whole of it at table size now; `client-core/src/cardcrest.rs` holds the
papers and the glyphs.

Proved on a running table rather than argued, in the slips' day: Llanowar
Elves, a Spark Double that entered as a copy of it and a Rite of Replication
token of it, drawn side by side — no mark, two cards, a squirrel.

The bits are `cardmat::glow::TOKEN` and `::COPY`, 20 and 21 of that word,
above the twelve bits the keyword rail rode until #274 (empty since; the
numbers stayed so the WGSL `GLOW_*` constants did not move). `glow_of` reaches the registry itself here
rather than being handed it — the opposite of the seam one crate down, and
deliberately: this crate links `baylee-cards`, and all three of its callers
would otherwise pass the same closure to get the same answer, which is three
chances for a card in the hand bar to disagree with the same card on the
table.

**And the copy's own card stands beside its preview.** The mark says *that* a
permanent is a copy; the little card at the preview's foot, captioned
`Phrase::CardUnderneath`, says of what. `board::original_of` asks
`board::art_of`'s answer rather than `worn`'s: the card underneath is offered
exactly when the picture being drawn is not the object's own. That is the same
set for every copy the registry can name, and it is the right answer for the
one it cannot — a copy of something no lookup resolves is drawn as its own
card, and a second copy of that same picture beside it would explain nothing.
A token has no cardboard to offer and is `None` on the first line.
`hud::hand::underneath_place` decides where it stands, a pure function for
`preview_place`'s reason: the preview is *already* against whichever window
edge had the room, so "beside it" is off the screen about half the time, and
on a phone there is no beside to be had at all and overlapping is the right
answer. The renderer resolves it from the view rather than reading
`CardGroup::original`, for `glow_of`'s reason one paragraph up; the field
exists so `required_images` holds the picture resident, because a hover has no
frame to spend on a fetch.

Photographed on a running table, like the mark before it: a Spark Double that
entered as a copy of Llanowar Elves, hovered — the preview is a Llanowar Elves
carrying the copy mark, and the Spark Double stands at its foot under one word.

The ask it half-answers reads, in the owner's own words, "carries the original
card as a symbol-sized card beside it, hoverable into the card preview", and
*beside it* has two readings this has not chosen between. Beside the permanent
**on the felt** is a second card entity in the scene, placed through
`sync_scene` and `Motion` like everything else down there, plus a hover path
that previews an `ImageKey` where today it previews an `ObjectId` — scene and
HUD work, no shader. **On** the card face is a second texture binding on
`CardMaterial` and both card shaders, which is a material change and the
fallback-closing kind of decision a commit has to state. Beside the *preview*
is the third reading and the one shipped: it is the cheapest, it answers the
question the ask is about, and it commits to neither of the others.

**And a copy of a token is a copy too.** The seam used to know one table. A
permanent copying a **token** — a Clone on a Soldier — projects a name no card
is printed with, so the lookup answered `None`, which is the same answer it
gives for a permanent copying nothing: the table drew a Clone, wearing no
mark, with "Soldier" written under it. `board::Registry` is the widened seam
and `board::Wears` is what it answers — `Card(index, face)` or `Token(id)` —
so one judgement covers a copy of either, and `cardart::named` asks the card
table first because a card printed with a token's name is what a player would
mean (`no_token_is_named_after_a_card` says the pool has no such card today).

The second lookup is the interesting one, and it is `tokenart::name`. The card
arm compares **indices**, because a name and an index are the same handle in a
pool where no two cards are printed alike — `7a7da76` is the test that says
so. Two *tokens* are printed alike: `ALL` holds a 1/1 and a 2/2 Shapeshifter,
a name answers with the first of them, and a token compared against that
answer would be judged a copy of its own twin and drawn as it. So a chit is
compared by the name its own **id** carries, which is the one asymmetry
between the two arms of `worn` and the reason `Registry` carries two closures
rather than one. What is left is the tie itself: a *copy* of either
Shapeshifter is drawn as the 1/1, because a projected name is the whole of
what a copy carries and two tokens with one name are one name.

Photographed on a running table: a Jasmine Dragon Tea Shop made a 1/1 white
Ally, a Spark Double entered as a copy of it, and the hover draws the Ally
token's own printing with the copy mark top-left and the Spark Double at its
foot under one word. One thing to read there and not mistake for a fault: the
type line says "Token Creature — Ally", because a preview of a copy draws the
copied *printing* and that printing is a token card. The permanent is not a
token, and the mark in the corner is what says so — the copy mark, not the
token disc.

**Where the bytes come from is one process-wide setting.** By default the
Scryfall CDN; when `GET /auth/config` says `art_cache`, the gateway's own
mirror instead (`images::use_art_base`, and `docs/protocol.md` §"Card art" for
the route). A setting rather than a parameter, deliberately: the places that
turn a printing into a picture include pure helpers in the deck builder and the
lobby preview that have no resource to read, and threading a base URL through
all of them would spread the knowledge instead of containing it. It is told
rather than guessed, because a gateway with the mirror off answers 404 for
every printing and a client that assumed wrong would draw a table of
constructed faces.

The gateway is asked at sign-in, so a client launched straight into a game with
a `SeatTicket` — `dev-table`, or a browser handed `?game=…&token=…` — never asks
and keeps the CDN. That is the correct fallback rather than a gap: the pictures
still arrive, they are simply not the gateway's copies.

**Both schemes have to be registered.** A scheme with no asset source does not
fail the way a missing file does: the request never leaves, the load never
settles, and `textures::Failure` never hears about it. The table draws
constructed faces on grey slabs and looks like a slow network. A native client
registers both through its own reader (below); a browser gets them from bevy's
`http` and `https` cargo features, which only the wasm build turns on.
`textures`' `a_card_picture_can_arrive_over_either_scheme` is what stops the
pair being trimmed to one — because a development gateway is
`http://127.0.0.1:28766`, so `https` alone means every picture disappears the
moment a client signs in and adopts the mirror, while an unsigned-in client on
the CDN goes on looking perfectly healthy. That asymmetry is what made it look
like a card bug for a week.

**A native client fetches art through its own reader (#250, `artreader.rs`).**
Bevy's `web_asset_cache` kept its cache in `.web-asset-cache` under the
*working directory*, and `save_to_cache(..).await?` failed the load after a
download that had succeeded (bevy_asset 0.19.1, `io/web.rs`). A macOS app
started from Finder or the Dock runs in `/`, so every card drew its text face
and the log said only `No such file or directory`. Measured: the same binary
and config drew no art started from `/` and all of it from any writable
directory. `ArtReaderPlugin` now registers the `http` and `https` sources
before `AssetPlugin`, and:

- caches in `$XDG_CACHE_HOME/baylee/art` when that is set, else
  `~/Library/Caches/baylee/art` (macOS, iOS), `%LOCALAPPDATA%\baylee\art`
  (Windows), `~/.cache/baylee/art` elsewhere, and the app's cache directory on
  Android. It never uses the working directory;
- treats the cache as optional: a read or write that fails is one `WARN` per
  operation and error kind, and the downloaded bytes still load;
- names a cached picture by the SHA-256 of its URL, and writes it to a `.part`
  that is synced and then renamed into place, so a crash cannot leave half a
  JPEG that fails to decode on every later launch;
- asks only under `https://cards.scryfall.io/`, `https://backs.scryfall.io/`
  and the art base in force (`images::art_base`, which is the gateway mirror
  after sign-in). Anything else, and any path containing `..`, is refused with
  an error naming the host before a request is sent;
- follows no redirect, and a non-success answer (a 3xx among them) is an error
  and never a picture, since a redirect could leave the allowlist;
- sends `baylee-client/<version>` as its User-Agent, gives up connecting after
  10 s and on the whole request after 30 s, reads at most 16 MB, and trusts
  certificates the way the system does (`platform-verifier`, as bevy's reader
  did).

A cache that bevy's reader left in `<repo>/.web-asset-cache` is simply no
longer read: pictures are fetched again, once, into the new place.


Board cards are fetched `small` (146×204); only the focused card is fetched
`normal`. That is the difference between ~36 MB and ~400 MB for a large table.
A byte-budgeted LRU (`TextureBudget`) decides evictions; the browser budget is
deliberately below the desktop one and the ordering is checked at compile time.

**Everything the board draws is one size, and the hover preview is the
exception.** The stack panel used to ask for `normal` — for a card it draws 66
logical pixels wide — which made a spell cast out of a hand the player was
already looking at fetch its art a second time and draw the constructed face
until it landed. `board.rs` now asks `small` everywhere; the only key at the
readable size is the one `hud::overlay` rewrites for the preview, which is also
the only place drawing a card big enough to need it.

That preview reads a size nothing else does, and it asks at the instant the
pointer arrives — too late to fetch. So `Preload` warms `normal` for the local
hand and the local command zone: the two zones the preview can point at that
the rules keep small (a hand is about seven cards, CR 514.1; a command zone at
most a pair). A battlefield has no such bound, and eighty permanents at 1.3 MB
each would spend the whole browser budget on a convenience and then thrash it —
so hovering a permanent still fetches on the spot.

**What is drawn during that fetch was the wrong thing.** `wants_face` answers
"is this key's art here yet", and with the answer "no" the preview drew the
*constructed face* — the text card, which reads "Rules text unavailable"
wherever the catalog is not wired up. So hovering an opponent's land produced
a paragraph of apology for as long as the download took, every single time,
which is a worse picture of a card than a slightly soft one. The preview now
falls back to the `small` art the board is already holding for that same
permanent: `stopgap` is taken only when the big key has *not* arrived and the
small one has, the full-size fetch is still started (asking is what starts
it, so a preview that settled would never sharpen), and `CardLook` is built
from whichever key is actually being drawn — a look naming one texture beside
a handle holding another would hand the same material two different images on
consecutive frames. The constructed face keeps its two real jobs: the
modifier held, and a card whose art is nowhere at all.

**Two things in that path could not recover, and both were invisible.** The
preload queue was built on the first frame a view existed — when the
battlefield is empty and every printing but the player's own deck is still a
hole in `GameStatic.prints` — and never again, so nothing a seat *earned*
during the game was ever warmed. And `CardTextures::failed` had no way out at
all: a printing asked for before its print entry arrived was recorded as failed
and stayed that way, so an opponent's land could draw as a blank rectangle for
the rest of the game. The set now records *why* (`Failure::Unresolved` vs
`Failure::Load`), and the two come back by different routes because different
things change their answer: a new print table forgives an unresolvable printing,
because a print table is the only thing that can resolve it; a failed fetch is
retried on a timer, three times, four seconds apart.

That second one was very nearly shipped as permanent, on the reasoning that a
URL which answered with nothing would answer the same way next time. **The
measurement says otherwise.** One offline game recorded two load failures while
a plain `curl` of all 194 printings of both decks returned 200 for every one of
them — the fetches were transient, and a transient failure believed the first
time is a card drawn blank for the rest of the game, which is what the player
reported. The attempt count is what keeps the other case honest: a printing
whose art really is gone costs three requests in a game, not one every four
seconds forever. The unresolvable path also logs now; it was the silent one,
which is the whole reason all of this hid.

## The card surface

Art is the texture; the *finish* is the shader. One material
(`cardmat::CardMaterial`, one WGSL file shipped inside the binary with
`embedded_asset!`) draws every card, because a board of three hundred
permanents can afford one pipeline. What the rules and this client say about
a card stands on objects of their own, each with a material of its own.

### The print fills the card

**Nothing this client *paints* lies on the print (#274), and since #298 the
print is the whole card.** It is the owner's rule and Scryfall's: their image
terms ask that a card image is not covered, cropped, blurred, tinted or
stamped, and the artist's name, the collector line and the © line run along
the print's bottom edge — exactly where the keyword rail and the
power/toughness plate used to lie, with the ward bands tinting the rest of
that strip (8.2:1 contrast down to 2.6:1 under shroud, measured in #270).
#274 answered with a frame: the print scaled into a window and our paper
round it. The owner did not want the frame (#298: "I really do not like the
new gray cards border. Maybe just remove it?"), so the print fills the quad
again, 1 × 1.397 card widths, edge to edge, and what the frame said stands
on objects of its own:

- **The strip** (`cardrail::Strip`, `marksmat.rs`, `label_strip`) lies on
  the art along the seam between the art and the type line, as a lifted
  object with its own shadow, and carries the card's **label** from the
  left: the plate with its chip, the sleep moon, the keyword marks and the
  identity crests ("The strip says what the card does in combat", below).
  The owner's okay for a wider strip carrying all of that is recorded in
  `docs/legal.md` §3.
- **The count badge** hangs off the card's top-left corner, outside it
  ("Grouping and the token summary").
- **The offer's light** lies on the felt round the card (`floormat.rs`,
  `floor.wgsl`, `floor_light`): a quad under the card, `floormat::REACH`
  (0.10 card widths) past it on every side, added to the felt at
  `table::FLOOR_RUNG`, over the contact shadows and under every card — so
  the card covers the light's middle and the next card of a fanned lane its
  edge, which the owner accepted. The amber chase, the indigo chase, the
  armed gold and the blue will-tap pulse are the frame rim's own colours and
  motion; the material key is the four offers (`cardmat::glow::OFFERS`).
- **Protection** is a mark on the strip (hexproof, indestructible and shroud
  were appended to `MARK_ORDER` as slots 12–14, the index being the wire and
  the atlas cell). The shells that are its glance — a brick wall for
  defender, domes for hexproof and shroud, a steel rim for indestructible —
  are #298's next step.

- Drawn on the print: its own **finish** (`print_finish`), because a foil is
  what that printing *is* — the one exception the owner accepted — and
  light that passes over the whole card and leaves nothing behind: the
  table's lamp pool, the arrival sweep and the zone-change doors. The
  brushed **coating** every card used to wear is gone, not moved: its floor
  lifted the print's blacks from 18 to 23–26 of 255 everywhere, the artist's
  line included, and the owner took it off entirely.
- `nothing_but_the_finish_is_drawn_on_the_print` reads both card fragments
  as text and fails if anything but the finish writes the print, anything
  but light and the corner touches the colour after it, or the card still
  reads a word the frame used to (`glow`, `plate`, `chips`). The live half,
  not yet run on #298's look, is a diff with the clock paused: the same card
  rendered with every state on and then off must be identical everywhere on the print but under the strip's
  rectangle (`cardrail::quad_rect`), the badge's and the felt round it —
  including the bottom 7% of the card, where the artist and © lines are.
- The mesh is rounded at the print's own corner radius and the sliver its
  edge antialiases through is inked the edge wall's colour, which also
  covers whatever a scan's corners were photographed against.
- A text face (a card drawn from our own text) fills the card the way a
  print does, and is laid out against it: see "The text face" below.

### The text face

A token has no print, a printing's art can still be on its way, and a player
can ask for text over art (`face::wants_face`). Until #259 the shader drew
that window as a flat dark tint under the card's metal light — the owner's
"dunkle Metall-Platte" — and a lane of text tokens read as a row of holes.
Now the window is laid out and drawn as a card is:
`baylee_client_core::textface` is the geometry, the shader's `text_face`
(`card_common.wgsl`) draws its parts, and the table's `Text2d` lines stand
on them.

- A dark border (`BORDER`, 0.040 card widths) inside the card, then a name
  bar, a pinline, an art box, a type bar and a text box, in a card's order.
  The foot under the text box stays empty: a print has its collector line
  there, and there is none to write.
- A bar is one line of its text plus `BAR_PAD` above and below (`bar`):
  Alegreya Sans has no line gap and its ascender and descender add up to
  1.2 em, which is also the height bevy sets a line at. The type bar's top is
  pinned on the keyword strip's seam (`cardrail::strip_bottom`), so the strip
  lies on the art box and never on the type line
  (`the_strip_lies_on_the_art_box_and_never_on_the_type_line`); the art box
  takes what the name bar leaves.
- No power and toughness on the table's face: a card showing its text face
  always has its plate on the strip (`Corner::shows_plate`), which is the
  P/T box. The one exception is `face::world_stats`'s — an animated
  planeswalker's body, which the plate (showing loyalty) cannot say — at the
  text box's foot, right, where a print has its P/T. The interface's faces
  (hand, preview) have no strip over them and write their own numbers.

**The table sets its text large, so its bars are deep.** A table card is
about 94 pixels wide, where a print's own name is five pixels tall and
unread. The table's name is `NAME_EM` (0.11 card widths, `Text2d`'s 11 px at
`PX_PER_UNIT` 100) and its type line and cost `SMALL_EM` (0.10), so its name
bar is 0.152 deep where a print's is about 0.08. Over the pool's 2715 names
at 11 px only 71% fit the 0.770 of a line, so a name steps down once and then
breaks (`textface::fit_name`): 11 px on one line; else 10 px on one line
(83%); else 10 px on two lines broken after the last word that fits, and the
name bar takes its two-line depth (0.260). Never a third line and never
smaller: a second line that still runs over is cut with an ellipsis. The cost
is not beside the name — there is no room — but on the art box's first line,
right, where it lies on the dark art as a print's cost lies on its field.

The type line is one line always, so the text box never moves (`fit_type`):
10 px, else 9 px, and past that it gives up words from the **front**. On the
table the subtypes are the half that says something — a Soldier matters to a
tribe, and "Creature" is already said by the plate and the frame — so first
the supertypes go ("Legendary Creature — Elf Druid" → "Creature — Elf
Druid"), then the card types and the dash ("Elf Druid"), and only then are
subtypes dropped from the end, whole, with an ellipsis. A line with no
subtypes keeps its card types. Over the pool's 2745 type lines: 71.1% fit at
10 px, 5.8% at 9 px, 1.9% give up only their supertypes, 20.1% are their
subtypes alone, and 1.1% (29, "Land — Mountain Plains Swamp" → "Mountain
Plains…") lose a subtype; cutting subtypes from the end first had cost 23%
of them one, "Creature — Human Soldier" the commonest. The preview says the
whole line. A supertype is known by the English word the engine prints
(`SupertypeSet::from_word`); a translated line has none the table can tell
apart and goes from the whole line straight to its subtypes.

**The widths are read off the font, before anything is laid out.** A
two-line name bar is a different face, so the fitting has to be decided when
the card's material is — not a frame later, when bevy has laid the text out
and the card would change under the player's eyes. `face::Widths` sums the
shipped Alegreya Sans Regular's own advances out of `Assets<Font>` (with
`swash`, which the mark atlas already links): exact for German names as for
English ones, and a hair long rather than short since kerning is left out. A
character the font lacks counts a full em. Until the font has arrived — an
HTTP fetch on the web — the width is `textface::average_width`, 0.420 em a
character, the Regular's mean over the pool's names, which answers the
one-line-or-two question as the font does for all but 179 of them. A face
fitted by the average is fitted again when the font arrives
(`table::ShownFace::is`,
`a_face_fitted_by_the_average_is_fitted_again_when_the_font_arrives`).

**The material draws the parts, keyed by one word.** `textface::face_word`
packs what the shader needs into `CardParams::face`: the face is on, three
hues — the bars' and the art box's two halves — and the depths of the two
bars, a byte each in 512ths of a card width (`textface::Depths`). The depth
is the one number both halves read: `Regions` places the text by it and the
shader draws the bar by it, rounded up so a bar always holds its lines. The
overlay sizes its bars by its own em, so its depths are not the table's two;
the word carries whatever they are. The material and the text come from one
fit (`table::face_now`), so when the font's arrival moves a name onto its
other number of lines the material is keyed again with it
(`the_font_s_arrival_re_keys_the_material_with_the_text`). One material per
word and state: a mono-green board is still one material.

- **Colour is the card's colours now**, as the rules have it. One colour is
  that colour; two have gold bars and an art box running from one to the
  other, left to right in the order the colours are written; three or more
  are gold; none is grey.
- **A land's bars are grey** whatever it makes, as on a printed land, and its
  art box takes the colour of its one basic land type — the type is what taps
  for the colour (CR 305.6) — or grey with none or several.
- **The bars are light and the text box lighter**: the bars' colour mixed
  0.65 of the way to a warm white, as a printed text box is, so a row of text
  tokens beside prints is not a row of holes. The art box is the same hue
  dark (0.35 at its top, 0.20 at its foot) with a faint cloth over it, the
  same weave on every card. The bars are bevelled into raised plates; there
  is no ornament, no frame art and no symbol, only geometry (`docs/legal.md`
  §2).
- **The text is dark ink** (`textface::INK`, sRGB 0.09 0.10 0.12): at least
  4.7:1 on every bar (black's, which was lifted three hundredths for it) and
  11:1 on every paper (`the_ink_reads_on_every_bar_and_every_paper`). A
  lethal number is a dark red for the same paper. The cost stands on the art
  box, which is too dark for the dark ink on a black card and too light for a
  light one on a white card (2.3:1), so it takes whichever stands further off
  the colour under it (`textface::cost_ink`). On the mid-tones the better ink
  stands between 3.4:1 (grey) and 3.9:1 (gold): over WCAG's 3:1 for large
  text, under its 4.5:1 for body text, and the cost is 10 px.
- `the_text_face_is_the_same_face_in_both_languages` holds the shader's
  numbers to `textface`'s, the hues to `Hue::tone`, and the word's shifts to
  `face_word`'s.

**The overlay draws the same face at its own em.** The hand, the preview,
the stack and the tray draw a card with no art through `CardUiMaterial` and
the same `text_face`, and lay their text out by the same rule
(`face::UiFace::lay`, then `face::spawn_ui`). Only the sizes differ:
`textface::Sizes::overlay` takes each as a share of the card's width in
pixels held between two sizes in pixels (`UI_NAME`, `UI_TYPE`, `UI_BODY`),
so a 92-pixel hand card and a 308-pixel preview keep a print's proportions
without a second table of constants, and the word carries the bars' depths
that fit makes. The overlay is near, so its cost stands where a print's
does, at the name bar's right end, and the name gives up the room the pips
take (`Sizes::name_room`). The text is placed by `Regions` from the same
depths the shader draws by; every node is `Pickable::IGNORE`, so the face
takes no hover from the card it is drawn on.

- **Rules text steps down, then scrolls.** It starts at its own size (15 px
  in a preview at the default scale) and steps a pixel at a time down to
  `BODY_FLOOR_PX`, 10 px, where a sentence stops being read
  (`textface::fit_body`). Past the floor the text box scrolls, with a thin
  bar in the bars' colour in its right margin (`face::FaceScrollbar`); the
  margin is kept whether the bar shows or not, so the text never reflows
  when it appears (`SCROLL_MARGIN`). Laid out by bevy over the pool's 2716
  cards in English, the text still runs over at 10 px on 3.2% of them in a
  308-pixel preview, on 26.8% at a preview scale of 0.75 (231 pixels), and
  on one card at 372 (`how_much_of_the_pool_runs_over_at_the_floor`,
  ignored; run it by name). German runs longer.
- **The fit is a model of the layout, held to the layout.** It cannot wait
  for bevy to lay the text out, because the material is keyed before
  anything is spawned. So `manaui::rich_depth` models how `rich` sets a
  line: a flex row that wraps, whose items are runs of words and marks, so
  a run that will not fit beside a mark takes a line of its own, whole; and
  bevy measures a run up to the whole pixel. The body is drawn by
  `manaui::spawn_rich_in` in the face's own font at exactly the fitted
  size. Drawn through `hud::tf`, as it first was, it came out 1.2 times
  larger and a weight up, and a text fitted to its box ran over it
  (`the_rules_text_is_drawn_as_it_was_fitted`).
  `a_face_fitted_to_its_box_fits_it_in_bevy_s_layout` lays the forty pool
  cards hardest to model out in a headless bevy with the shipped fonts, at
  three widths, and none the fit says fits shows its bar. Over the whole
  pool none does either.
- **The wheel scrolls what is under the pointer, and on the table a card's
  readable surface is its preview.** The preview is a tooltip that follows
  the hovered card and is never under the pointer, so a wheel over the
  hovered table card scrolls its preview's text (`hud::scrolls`). The offset
  belongs to that preview (`hud::PreviewScroll`): it survives the overlay
  rebuilding the preview (`keep_the_preview_scrolled`) and starts at the top
  again when the hover moves to another card or off every card
  (`follow_the_hover`). A card whose text fits leaves the wheel inert; it
  nudges nothing else. The hand keeps its wheel
  (§"Table presentation", `hud::scroll`).
- **Known limit:** a hovered *hand* card whose text still runs over at 10 px
  shows its scrollbar in the preview and cannot be scrolled there, because
  the wheel over the hand scrolls the hand. The ways out are a larger preview
  (`preview_scale` in the settings) or the same card on the table.
- **A card in hand draws the short face** (`Detail::Compact`): name, cost
  and type line, no rules text. At the hand's 92 pixels that text would be
  six pixels, under the ten-pixel floor, and hovering the card opens the
  preview, which is where it is read (`a_card_in_hand_draws_no_rules_text`).
- **Not yet:** dragging the scrollbar's thumb; the bar only shows how much is
  hidden and where the box stands. And the overlay fits a face when it
  rebuilds, not when the font arrives: a face drawn before Alegreya Sans
  loaded (a web client's first frames) keeps the average width's fit until
  the next rebuild. The table re-fits on the font's arrival
  (`table::ShownFace::is`); the overlay does not yet.

Foil and etched finishes share `print_finish` in `card_common.wgsl` (#198).
The existing artwork sample supplies luminance, pigment and screen-space
contours, with no extra texture fetches. Foil uses a restrained, pigment-shifted
iridescence; etched foil emphasizes fine metallic contours and antialiased grain.
Both protect dark ink and bright rules boxes and use bounded screen blending.
The same treatment serves table cards and UI previews, retaining the existing
view-angle and reduced-motion clocks.

Materials are shared on a `CardLook` — art, finish, glow — which is exactly
what the shader draws differently and nothing more. Forty plain Islands stay
one material; a foil Island is a second; an Island the rules have made
indestructible is a third until it stops being one.

**The finish comes from the print table, never from the card.** `GameStatic`'s
print table is per seat, so a printing a seat has not earned resolves to
`None` and is drawn plain — a hole rather than a foil. Reading the finish off
the card instead would be a hidden-information leak with no game object to
hide behind.

**The marks and the glow word come from `PublicObject.keywords`**, which is
already projected — the layer system has run, so a creature that gained
indestructible this turn wears it this turn. `cardmat::glow_of` is the one gatherer for a `PublicObject`,
wherever it is drawn — battlefield, stack, command zone, tray or preview;
inside it `glow_bits` narrows the engine's `u128` to the bits the shader
reads, and a test pins each one against `KeywordSet`, because that numbering
is generated and a card glowing for the wrong keyword would be a rules lie a
player would believe.

The hand bar does not call it at all: a card in a hand is its print and its
finish and nothing else of ours (#298). What the frame said there — the armed
ring, a commander's paper — went with the frame; the hand's own halo says what
can be done with a card, and an armed card stands up out of the row. The
keywords never reached it: they say what is protected *on the battlefield*,
and a hand that wore them would be claiming something that is not yet so.

What a card *is*, what can be *done* with it and its *numbers* were three
registers of the frame #274 drew round the print — its paper, its rim and its
ledge. #298 took the frame away and gave what it said to two objects that lie
off the print: the **strip** over the art says what the card is, in marks and
numbers, and the **light on the felt** round the card says what is on offer.

- **The felt says what is on offer.** `glow::ACTIVATABLE` rides in the glow
  word but is deliberately *not* in `KEYWORD_BITS`: it comes from
  `LegalActions` rather than from the card, and is drawn as a warm light
  travelling round the card on the cloth (`floor_light`, see "The print
  fills the card") rather than as anything on the card, for exactly that
  reason (see "Tapping lands for a spell"). `glow::REACHABLE` is its twin
  for a card lying in a pile: the same chase in the hand's indigo, because
  it is this client's offer to tap lands first rather than the engine's yes
  (see "A card in a pile is reached for too"). `glow_of` draws one or the
  other, never both, and the engine's wins.
- **And the felt also says what has been decided.** Two more bits share
  that register, and the difference between them and `ACTIVATABLE` is motion.
  `glow::ARMED` is the card an armed deed is waiting on (see
  `docs/keyboard-map.md` §Arming): a bright light pulled in tight against the
  card's edge, breathing in place and **not** travelling, because the offer
  has already been accepted and a light that still moved would say it was
  still a suggestion. `glow::WILL_TAP` is what that deed would spend — the
  sources of an armed mana `Run` — cool where the other two are warm, and a
  beat behind the armed card, because the price follows the verb. `glow_of`
  drops `ACTIVATABLE` on an armed card rather than drawing both: one light
  carrying a chase *and* a ring would be saying the same thing twice with
  nothing left to read the difference from. `Offer::on` answers both from a
  `CardGroup`'s **members** rather than its representative — a plan taps one
  particular Forest and the card drawn for it may stand for four — and both
  are *any* where `CardGroup::activatable` is *all*, because that rule exists
  to stop an offer inviting a click that gets refused and these two invite
  nothing.
- **The strip says what the card does in combat.** Fifteen keywords —
  flying, first and double strike, deathtouch, haste, lifelink, menace,
  reach, trample, vigilance, defender, prowess, and since #298 hexproof,
  indestructible and shroud — are marks in a row, one place each, always in
  the same order (`client-core/src/cardrail.rs`). They
  are marks and not more paint because paint cannot *count*: a creature can
  carry six of these at once, and six colours mixed into one border is one
  colour that says nothing. **The mark is the Mana font's own ability
  glyph**, baked to a distance field at startup by `markatlas.rs` and sampled
  out of one atlas row — twelve procedural pictograms drawn in WGSL until
  September 2026, and replaced not because they were bad but because a player
  arriving here has already learned Magic's icons somewhere else and no
  drawing of ours can be the picture they already know.
  `cardrail::MARK_GLYPHS` is one of the three doors those codepoints come
  through, and `docs/legal.md` §2a is what makes that a rule rather than
  tidiness.

  Hexproof, indestructible and shroud were absent until #298, because the
  frame's paper said them — steel for indestructible, green wisps for
  hexproof, a colder, denser haze for shroud — and a mark repeating that
  would have been the same claim twice. With the frame gone they are marks
  12–14, appended rather than inserted, because a slot is a GPU bit and an
  atlas cell. A card with both hexproof and shroud wears both marks; the
  glow word lets shroud swallow hexproof (`glow_bits`, CR 702.18a against
  702.11b) for the shells that will be their glance, #298's next step.

  The row ran along the card's bottom edge as a *rail* until #274, over the
  artist's line. It is an **object** now (`marksmat.rs`, `marks.wgsl`,
  `marks_ui.wgsl`; the drawing is `card_common.wgsl`'s `label_strip`): a
  dark plate with its own contact shadow, one quad per card with anything to
  say, a child of the card so it follows every glide, tap and lift, not
  pickable and not a `CardShadow`. Its bottom edge stands on the seam where a
  modern frame's art meets its type line — `cardrail::M15_SEAM`, measured on
  36 scans, row 378 of 680 — at the card's left, where a fanned lane leaves
  every card's edge in sight, and its shadow falls left, right and up on to
  the art, never down on to the type line. Marks never shrink (0.085 card
  widths, eight pixels on the felt); a seventh opens a row **above** the
  first, so the row a creature already wears never moves and the strip's
  foot stays on the seam. The card's own material has no dimension for
  them any more: the strip's material is keyed on `cardrail::Strip`'s four
  words alone — the marks, the plate, the swing and the label — so a table
  has one strip material per distinct label on it.

  It lies half a row step over its card's face, not a card's thickness as
  first planned: a lane's whole rise is `LANE_RISE` (0.004) shared out over
  its cards, and a strip lifted 0.055 was nearer the camera than the card
  laid over it and drew on that card's art
  (`a_strip_lies_on_its_card_and_under_the_next_one`). The shadow is what
  makes it read as lying on the card. **Accepted:** a tapped creature in a
  fanned lane turns its strip out of the exposed edge — its attack is
  declared, and the preview names its marks. The
  preview draws the same strip as a UI node over the art at the same place.
- **The moon says a creature is asleep.** A creature with summoning
  sickness (CR 302.6) wears a crescent after its plate (`label::MOON`,
  `moon_sdf`), and its plate writes in the moon's grey. It was night
  falling on the frame's paper under #274, and a dimmed, desaturated print
  before that — which is the very thing Scryfall's terms name.
  `board::asleep` sets it only for creatures, because summoning sickness is
  visible on nothing else (`only_a_creature_is_modelled_asleep`).
- **The crests say what the card *is*.** A token, a copy and a commander
  each wear a **crest** at the strip's end: a square of paper — verdigris,
  violet, oxblood — with the Mana font's `ms-token`, `ms-ability-copy` or
  `ms-commander` printed on it (`cardcrest`, `cardrail::Item::Crest`),
  provenance first, then the commander. The answer was a crown on the top
  edge, a column in the right margin and slips under the name before #274,
  and all three were on the print; #274 made it the frame's paper, and #298
  cut that paper down to the crest. At table size the paper is the answer
  and the glyph is not relied on; in the preview it names it. The ink holds
  4.5:1 on all three papers (`the_ink_reads_on_every_paper`; the slips' ink
  measured 3.5:1 on the oxblood).
- **The plate says what the card *is* in numbers.** The strip's first item
  is a plate: a creature's power and toughness, or a planeswalker's loyalty
  behind a gilt rim. It was the bottom-right fifth of the print until #274,
  which the rail left empty for it, then the left end of the frame's ledge;
  since #298 it leads the strip (`cardplate::PLATE_W`, 0.196 wide), because a
  lane fans with each card's own left edge exposed —
  `the_plate_and_the_first_mark_survive_the_tightest_fan`. And it is there
  only when it says something the print cannot (`Corner::shows_plate`): a
  vanilla 2/2 under nothing has its own P/T box, and a plate beside it
  saying 2/2 is noise on every creature of the board. A body the layers
  changed, marked damage, a card drawn without its print, a print whose box
  the next card of a fanned lane covers, and every loyalty and chapter
  count, are written. `client-core/src/cardplate.rs` decides what it says
  and packs it into one `u32` — three ten-bit numbers and two kind bits —
  that rides the strip's material key, so a creature dealt three damage
  becomes a different strip and the plate redraws with no second pass.
  Marked damage is the plate **filling from the bottom** to
  `damage / toughness` rather than a third numeral: what a player needs off
  a blocked creature is how close to lethal it is.
- **The numerals are type, and were a stencil.** Each was a 4×6 bitmap mask
  sampled bilinearly, and two complaints came off it that turned out to be
  one fault. It looked **blurry**, because a mask that coarse smoothed up to
  eleven physical pixels is a blur with no edge to sharpen. And it looked
  **off-centre**, because every glyph was given the same four cells: `1` drew
  its flag in the left two and `/` ran corner to corner, so the ink inside a
  fixed box sat wherever the picture put it. A distance field has an edge at
  any size and an *advance* is what centring a line of type means, so the
  corner now sets `AlegreyaSans-Bold` — already shipped, already the
  interface's face — out of the same atlas `markatlas` bakes the keyword
  marks into. Two features are asked for at bake time and both are load-bearing:
  `lnum`, because this face's **default** figures are oldstyle and `3`, `4`,
  `5`, `7` and `9` would hang below the baseline; and `tnum`, so that a
  creature growing from `9/9` to `10/10` does not shunt its own slash
  sideways. `PLATE_PAD` went 0.014 → 0.020 in the same change, because a
  stencil's ink stopped short of its own box and a typeface's does not — at
  the old padding the digits and the plate's rim ran together. It went back
  to 0.012 for the frame's ledge, which was 0.125 deep, and the strip kept
  it: the figures kept their 0.075 (`PLATE_CAP`, seven physical pixels on
  the felt) and the margin paid. A sleeping creature's plate writes in
  moon-grey, the moon beside it.
- **Deathtouch greens the power, and only the power.** That is the half of
  the body the keyword acts through: a 1/1 deathtoucher trades with anything,
  and what does the trading is the 1 on the left. A colour on the number
  rather than one more mark on the strip, because the
  number *is* what the keyword changes the meaning of. `cardplate::Tone`
  reads it off the strip's own badges rather than off the raw keyword word, so
  the mark and the colour cannot disagree. Toxic is written into the enum and
  reaches nothing: `board::keyword_bits` has no toxic bit yet.
- **And the counters stand beside it, on a chip.** The net power and
  toughness a permanent's ±1/±1 counters add, on a chip 0.100 wide one gap
  right of the plate: green stock when it grew, violet when it shrank, the
  plate's dark body for ink. At table size the chip is nine pixels wide and
  the **stock** is the reading; the figures are the preview's. A symmetric
  swing — every `+1/+1` and `-1/-1` counter there is — is written once
  (`+2`), and a lopsided one in full, shrinking to fit. It stood *above* the
  plate, one size down, until #274; the ledge had no room above a plate, and
  the strip keeps it beside. What
  stood there before that was a column of stamped **chips**, pips to six and a colour per
  kind of counter, and the owner read it as saying nothing: a green disc with
  three pips on it is a rebus for `+3/+3`, and the plate two millimetres
  below was already writing the answer in figures. The cost is named rather
  than hidden — charge, time, level, keyword and loyalty-on-a-non-planeswalker
  counters had a chip each and now have none on the table; the badge tooltip
  names them in full, which is where the chips' colour code always had to be
  decoded anyway. Drawn large (`BASE_AA`, the same test the damage rules
  use), the chip also writes the **printed body** when it is not the body on
  the plate — under the swing when there is one — which used to hang under
  the plate because the plate covered the print's own P/T box. A **saga** is
  the exception that takes the plate itself: a square parchment page with
  the chapter in roman numerals, at the same left edge. `Corner::of` decides
  the plate and the chip together, and `Corner::of_object` does the
  same for the hover preview — which drew the *printed* numbers until it did,
  so a 2/2 under an anthem was a 3/3 on the table and a 2/2 in its own
  preview. A card drawn as text on the table (`face::spawn_world`) no longer
  writes the body under its type line when the plate already says it
  (`face::world_stats`): it said `3/3` twice. An animated planeswalker keeps
  its line, because its plate is its loyalty.
The pictograms live in a third shader file, `card_common.wgsl`, together with
the printed corner both shaders cut at: it is everything the table and the
overlay have to agree about. It contains no bindings and no bevy syntax at all
— every shader-global it needs, the time and the colour underneath, arrives as
a parameter — which is what lets bevy compile it as an imported module, what
keeps it clear of the two different bind groups the two shaders read `globals`
from, and what lets the naga test parse it on its own. Which item of the strip a
fragment is inside is found by laying the label's items out in order
(`LABEL_ITEMS`, nineteen: the plate, the moon, fifteen marks, two crests), as
`cardrail::Strip::layout` does: a loop bound at compile time, no dynamic indexing, and inside the GL budget like
everything else here. *How many* marks there are is counted in that same
loop, and that is not stylistic: `countOneBits` is what WGSL offers, naga
lowers it to GLSL's `bitCount` with no version check, and `bitCount` arrived
in ES 3.10 while WebGL2 compiles ES 3.00. The browser would have rejected the
shader and the card pipeline with it, so the table would have drawn no cards
at all — a whole-client failure from one builtin, in the one target
`cargo check --target wasm32` cannot see, because WGSL is not lowered to GLSL
until the pipeline is built in the browser.

The browser build renders through **WebGPU** now (`Cargo.toml` lists
`webgpu`, not `webgl2`), which lowers nothing to GLSL and would accept
`countOneBits` — so that trap is disarmed and the loop stays anyway. It is
the same loop either way, it costs nothing, and it is what keeps `webgl2` a
one-word swap for a browser that has no WebGPU. The rule is therefore about
*this* file rather than about the backend: nothing in the shaders reaches
past the older budget until a commit says it is spending the fallback to get
something.

The card's material draws nothing past the print: the mesh is exactly the
card, and what stands past its edge — the count badge, the offer's light — is
an object with a quad of its own, so no layout in the client has to leave
room round a card for it.

**The corners are cut twice, at the printed radius, in two different ways.** A
Scryfall scan is a rectangle: the card's rounded corner is in the file as
white paper, and drawn untouched it is the single most obvious way for a card
to look like a photograph of a card. The frame #274 drew hid them; since #298
the scan fills the card again, and what is cut below is its corner. On the table the mesh is already rounded
(`table::CARD_CORNER`), so the shader only inks the sliver the mesh edge
antialiases through; in the overlay a UI node has no mesh, so `card_ui.wgsl`
cuts the corner in alpha — and that is the one the player was actually looking
at, since hand, preview and printing picker all drew the scan square. Both cut with the same `corner_sdf` at the same `PRINTED_CORNER`
(4.76%, which is 3 mm on a 63 mm card, and lives in `card_common.wgsl` with
the rail), and `hud::card_radius` is the same number again, because that
wrapper node clips the card and carries its shadow.
All of them used to be 10%, which took the white away by taking a tenth of the
card with it, and made every permanent read as a token.

The whole thing stays inside the WebGL2 budget: uniforms only, no storage
buffers, no texture arrays. Animation reads `globals.time` from the view bind
group, so nothing is written per frame — a material is created once and never touched again while
it is on screen.

The 2D overlay draws cards through the same surface (`CardUiMaterial`, one
shader file, one set of constants), so a foil in a player's hand looks like
the foil that will land on the table. The one difference it cannot avoid is
that a UI node has no world position and no normal, so there is no view angle
to drive the sheen with; time does it instead, and the sweep runs on its own
rather than answering the camera. A card in hand carries the finish and
nothing else of ours: the strip tells a player what is protected *on the
battlefield*, and a hand that wore it would be saying something that is not
yet true. It is the same print through the same `print_finish`, so a card
picked up off the table is the same card.

Both material stores reach their systems as `Option`. A headless test has no
render plugins and therefore no `Assets<CardUiMaterial>`, so every drawing
function falls back to a plain `ImageNode` rather than growing a second code
path — which is what keeps the overlay tests free of a GPU *and* of the
network.

The printing picker uses a second, tiny cache keyed on CDN url and finish: the
cardboard a player is choosing between is not in any game, so it has no
`PrintRef` and no print table to look one up in.

`cardmat::tests` parses and validates both shaders with naga, the same front
end wgpu uses. Without that a WGSL error would surface only when a real
pipeline is built, which on the web is the one environment that cannot be
debugged by looking at a filesystem. It caught a reserved keyword on its first
run.

## Motion

Nothing on the table is positioned directly. `sync_scene` writes a `Motion`
target and `glide` moves the card towards it, so every source of movement — a
lane repacking, a tap, a hover, a card entering play — arrives through one
door and animates without knowing it is being animated. It also cannot
desynchronise from the board model: there is nothing to keep in step, because
the target is recomputed from the model every frame.

The interpolation is exponential (`1 - e^(-rate·dt)`) rather than a fixed
duration, for two reasons. It is frame-rate independent, where the naive
`lerp(0.2)` per frame makes the whole table twice as fast on a better machine.
And the thing being animated is a *correction*: a card whose lane repacked by
half a millimetre and a card that just entered the battlefield are the same
code path, and the first must not take as long as the second.

A card appears above its mark and drops onto it. Direction-agnostic on
purpose — a card could fly in from its owner's hand, and at four seats around
a ring that means four directions and a card that crosses two other players'
boards to get home.

### Where a card came from, and where it went

The question that makes any of this possible is one a view does not answer
directly: a creature that died and a creature that was exiled both simply stop
being in `view.battlefield`, and nothing about the battlefield alone tells them
apart.

`baylee-client-core/src/zones.rs` is where the difference is read. An object
keeps its `ObjectId` across a zone change — the engine bumps a version and
clears the projection (CR 400.7) but does not renumber the handle — so two
consecutive views name the same card in two different lists, and the
difference between them *is* the event. `zones::Tracker` remembers where
everything was and answers each new view with the moves that touched the
battlefield, in object order: by id and never by discovery, because discovery
is a `HashMap` walk and two clients watching the same game would build
different scenes from it. A view it has already read answers with nothing,
which is what stops a renderer running at sixty frames a second from
reporting the same death until the next question is asked.

What it cannot answer, it says so about. A card put on the bottom of a
library, or bounced to an opponent's hand, is in a zone this seat sees as a
*count*: there is no object to find, so the move is reported as going nowhere
and the table draws the neutral exit. The alternative — watching that seat's
hand count go up in the same view — is a guess that reads exactly like a fact,
and one wrong frame of it would show a card flying to a hand it never reached.
Answering it properly means the view saying where a card went, which is a
change to the engine's side of the wire and is not this branch's to make.

**A `Place` carries its seat, and that is not decoration.** A graveyard is a
pile standing beside a particular chair, so `Place::Graveyard(PlayerId)` is
what lets a renderer answer the only question it actually has: where on the
table to send the card. Told merely "a graveyard", it would know every fact
about the move except that one.

**Most permanents that leave the battlefield are never despawned at all**, and
this is the fact to hold on to before reading anything else here. A pile's top
card *is* a placement — `placements()` pushes one for each pile whose top is an
object — so a creature that dies stays in `SceneIndex::cards` under the same
`ObjectId` and *glides* off its lane and onto the graveyard through the
update-in-place branch, inheriting the material cache, the hover lift and the
arming glow along with it. The stale branch below is reached only by a card
going somewhere with no pile drawn for it, or by a card that reached a pile and
is not its top.

So the exits are three, and which one is taken is decided by the destination's
pile first. A card bound for a pile this table draws glides *to that pile* and
slides `PILE_TUCK` under the card standing on it — the same point the top card
is gliding to on the same frame. That is the whole reason `pile_stand` exists:
two creatures dying together must not be treated differently for the accident
of which of them sorts on top, and a version that sank the second one through
the felt at its own lane would do exactly that. A bounce is the entrance run
backwards, higher and smaller, because the hand bar is an overlay and there is
no place on the felt to send it to. And a card this seat cannot follow shrinks
in place, turning nowhere and going nowhere, so it reads as neither of the
other two. A pile place whose stand cannot be resolved — a layout that has not
arrived — falls back to that same neutral exit rather than inventing a
destination.

Coming back is the exit reversed: a card returning from a pile starts *on* that
pile, at full size, because it is the card coming back and not a card being
made. This is the branch a *buried* card takes; one that was its pile's top was
already on the table and glides home through the update-in-place branch, from
the same point — which is why the two are one call to `pile_stand`. Every other
arrival is the one this table has always drawn: a creature cast from hand
arrives from the *stack*, a token arrives from nowhere at all, and both of them
belong dropping onto their mark.

A card that does leave is not despawned at once; it loses `CardVisual`, leaves
`SceneIndex::cards` and is marked `Departing`. Losing the component is what
matters — it is the thing every reader finds a permanent by, so a hover, a
preview and a combat line all stop following something on its way out of the
game — while the entity itself stays for `EXIT_LIFE` and glides to its exit
pose through the same one door as everything else. `retire` counts it down
rather than asking `glide` whether it has arrived, because one exit ends at a
scale of nearly zero and one ends behind another card: "has it arrived" is the
wrong question for a card whose destination is nowhere.

**A stale id with no move behind it is despawned on the spot**, as it always
was, and that arm is load-bearing. It is the graveyard's old top card, covered
by the one that landed on it this frame, or a group that re-keyed when its
lowest-id member went — nothing about the table changed where either stands, so
an exit played for one of them would be a card visibly sliding out from under a
pile it never left.

One case is drawn thin and is worth knowing about. `SceneIndex::cards` is
keyed by a group's *representative*, so four Islands are one entity: an Island
that leaves is not a departure at all, it is the count going from four to
three. And if the one that leaves happens to be the representative — the
lowest id, since `group_objects` sorts by name and then id — the group re-keys
to the next member, so one card plays the exit and another is spawned in its
place. Neither is wrong on screen; a mass token death is where it reads
thinnest.

**A move that both ends of are visible is a door**, and the card is dressed in
it on the way through. `zones::Passage` is the taxonomy — bounce, exiled,
flickered, destroyed, returned — read off a `Move` by `Move::passage()`, which
is `None` for an ordinary arrival and `None` again for either end the seat
cannot see, so the paragraph above about counts holds unchanged: no door is
invented for a card whose destination is a number. Three of the five are
departures (`Passage::is_departure`) and two are the same doors opening the
other way, which is the point of naming them in pairs. Exile and flicker are
one ring drawn at `mix(REACH, 0, phase)` and `mix(0, REACH, phase)`; a
screenshot of each at the two phases that should mirror came back byte for
byte identical, which is the only proof worth having that a reversal is one
figure run twice and not two figures that happen to look alike.

The wiring is split by where the card is at the moment it needs dressing, and
there is no third option. An **arrival** is still in the board model, so
`Sheen::usher` stamps the door on to the sweep `Sheen::observe` started on that
same frame. A **departure** is already gone — it has left `SceneIndex::cards`,
lost `CardVisual` and will never be handed a `CardLook` again — so
`table::dress_the_exit` writes the door on to the material the card is already
wearing, on the one frame that is possible. Both go through `cardmat::wear`,
which is where `reduce_motion` is answered once instead of at each end.

That split is what makes one ordering edge load-bearing: a view's batch of
moves exists exactly once, and `watch_for_arrivals` has to stamp it before
`sync_scene` drains it. Read in the other order the tracker answers the second
reader with nothing, because that view has already been read, and every
arrival door stops being drawn with no error anywhere.
`schedule_order_tests` asserts the edge for that reason, with a counter-test,
because nothing else in the suite would notice it going.

The three constants in `card_common.wgsl` are worth their doc comments,
because all three were first written wrong in the same direction — too much of
everything. `DOOR_WIDTH` at 11 spread the band over the whole card and read as
a permanent turned amber rather than as something crossing it; `DOOR_GLOSS` at
0.85 saturated every channel it touched; `DOOR_REACH` at 1.15 kept the ring
off the card for the first third of its phase, so the exile door began by
doing nothing. 40 / 0.60 / 0.95 is what a forced door through hot shader
reload looked right at, and the ring carries `DOOR_WIDTH * 2.0` of its own
because a circle crosses a pixel twice where a line crosses it once.

The camera follows its rig the same way, but faster: a drag that lags behind
the pointer feels broken where a card that snaps feels cheap. Yaw interpolates
the short way around, or focusing the seat on your left would spin the table
three-quarters of the way to reach it. `ShownRig` is a second copy rather than
smoothing `CameraRig` in place, because the rig is *input* and everything that
writes it wants to be able to say "there".

`Preferences::reduce_motion` turns all of it off, and it travels with the
account for the same reason the keys do: a player who cannot read a moving
board cannot read one on any machine.

It reaches the *shaders* through the material, as `CardParams::motion` — the
clock every animated term on a card is multiplied by, zero when the player
has asked for stillness. One number rather than a second pipeline, because a
board of three hundred permanents cannot afford a variant and because two
drawings of the same card drift apart the first time either is edited. The
discipline that makes one number enough is that **zero has to leave each term
somewhere it could have been**: a still card is the moving card held still,
not a different picture. A pure `a + b·sin(t·ω)` gives the strongest version
of that for free — phase zero *is* the mean — which is the armed ring. Where the term also carries
a *spatial* phase the freeze is a real frame rather than the average, and that
is still what is wanted: indestructible steel rests with its catch-light
at a fixed height (`sin(t·0.8 + uv.y·3.0)`), and a rail mark rests at whatever
its own slot offset gives it (`t·BEAT + k·0.22`). Both are the moving picture
stopped, which is the whole claim. The three that need more are the
travelling offer light (a stopped chase is a
parked hot spot, so it degrades to the ring it averages to), the will-tap
pulse (its phase offset puts zero near the bottom of the swing, so the
oscillation is scaled rather than the clock) and the hand's foil, whose
stand-in for the view angle is brightest at zero and would freeze a hand of
foils at their most garish.

The setting is not part of `CardLook`. That is the cache key, and a key
carrying a global preference holds both answers at once and evicts neither;
the caches keep the setting beside themselves and rewrite what they hold
**in place** when it changes, so the cards on the felt catch up on the frame
the switch moves rather than whenever something next redraws them.

## Hosts

The renderer never touches a socket. It talks to a `DuelHost`:

- `LocalHost` runs an engine in-process (solo play, embedded duels, tests) and
  goes through the same protobuf envelopes a socket would carry;
- `NetworkHost` (`src/net.rs`) is a websocket to the gateway's
  `/games/{id}/ws`, drained into the same `HostMessage` stream.

Both decode with the same function, so solo play is a real test of the wire
format rather than a shortcut around it. The binary picks between them on
whether it was handed a `SeatTicket`: `BAYLEE_GAME` + `BAYLEE_SEAT_TOKEN` in
the environment, or `?game=…&token=…` in the page URL in a browser. A ticket
that will not connect is a hard stop, not a quiet fall back to solo play —
somebody is waiting at that table.

### When the socket goes away

`NetworkHost::redial` re-dials and queues a `ResumeGame` naming the last
sequence the seat saw; the gateway answers with a full snapshot when the
client is behind and with nothing at all when it is not. That has worked since
the host was written and **nothing ever called it**. A dropped socket pushed
`Failed("the connection to the table was lost")`, which set the prompt bar's
sticky error and reported the duel as broken — while the table was still
there, and the seat still resumable.

The mechanism was never the missing part. The policy was, and the trait was
why nobody could supply it: `InstalledHost` is a `Box<dyn DuelHost>`, so
`NetworkHost::is_open` was not reachable from the application at all.

Three pieces now:

- **`DuelHost::link()`** returns a `LinkState`. Four states rather than three,
  and `Connecting` is the one that earns its place: after a dial is started the
  host is not `Down` any more, or the system would start another dial on the
  very next frame and keep doing it for as long as a socket takes to open. A
  `WsEvent::Error` arriving *while dialling* also falls to `Down` rather than
  becoming a message, because a failed dial is not reliably followed by a
  `Closed` on every platform, and a host stuck in `Connecting` forever is the
  freeze this whole path exists to prevent. `Local` is the default, so a host
  with no socket never enters the schedule — an in-process engine would
  otherwise be "reconnected" to twelve times and then declared unreachable.
  A fifth, `Refused{table}` (#271), is a table that refused this client's
  protocol: final, never dialled again, and the bar says which side is behind
  (`docs/protocol.md` §"Which side checks the protocol (#271)").
- **`Retry`** (`baylee-client-core/src/reconnect.rs`) is the schedule and
  nothing else: 0.5 s, doubling to a 15 s cap, twelve dials, then `exhausted`.
  Renderer-free and transport-free for the same reason the lobby's decisions
  are — a schedule that can only be exercised by disconnecting a real gateway
  is a schedule that is never tested. It is allowed to back off at all because
  the engine's *decision* clock does not run for a seat with no socket
  (`docs/protocol.md`), so nobody is losing a game on time while it waits.
  That is the right reason for the back-off and was, for a while, also given
  as the reason the banner could promise the seat was untouched. It is the
  wrong clock for that — see below.
- **`keep_the_table_connected`** is the wiring, and runs only in `Opening` and
  `Playing`. Not `Finished`: a table whose game has ended closes its socket in
  the ordinary course of things, and a client that redialled then would spend
  two minutes trying to rejoin a game it just watched end.

The banner is drawn from `link()` every frame, **not** through
`Duel::last_error` — that field clears in `Duel::submit`, a call a
disconnected player cannot make, so the words would have outlived the
disconnection they described. It is a `Phrase` on `Duel`, so the decision
stays where a test can read it and the words stay in the overlay, which is the
only thing that knows the language.

#### What the banner may claim, and why it cannot count

The schedule outlives the thing it was reassuring the player about. Twelve
dials land at t = 0.5, 1.5, 3.5, 7.5, 15.5 and then every fifteen seconds to
**120.5 s**. `HouseRules::reconnect_window_secs` is shorter than that on all
four presets — 30 s or 60 s — though not on every table that can be opened,
since a room may ask for anything up to an hour. When it does expire,
`Session::stand_in` gives the chair to the house, and for the rest of that
second minute the client was saying "reconnecting…" over a seat somebody else
was answering for.

Dialling on is right and did not change: `SeatAttached` runs `hand_back`, so
the twelfth dial still returns the chair. What changed is that there are two
sentences now, turning at `Retry::brief`.

**The client still cannot say when the handover happens, and now it knows
whether one is coming.** The half that cannot change is that **the client is
disconnected for exactly the window it would be counting down**, so the moment
of the handover is not observable from here however much the client is told.
That is why the second sentence is in the future tense — *"the house will
answer for your seat until you are back"* — rather than the indicative.

The half that did change is the window itself. `reconnect_window_secs` is not
60 — 60 is the `casual` preset, `blitz` is 30, and
`gateway/src/clock.rs::resolve` accepts anything from `MIN_RECONNECT_SECS`
(10) to `MAX_SECS` (3600) from whoever opened the room. The client used to be
told none of it and guessed, and the guess was a constant: `PATIENCE` was 8 s
*because* 8 is under the gateway's floor of 10, so the second sentence was
always early. `GameStatic::reconnect_secs` (`VIEW_VERSION` 26) is that number
on the wire, and `reconnect::Window` is what a client has been told about it.

**The guess was wrong in two directions and neither was the one it was written
to prevent.** The floor of ten is the *gateway's*, enforced on a room; the
engine takes any window at all, including zero — and zero there is not *at
once*, it is *never*. `EngineRunner::clock` returns no deadline for it and
`a_table_that_never_gives_up_a_chair_never_takes_one` pins that, so at a table
seated by a harness with a zero window the bar promised a handover that was
never coming, for the whole of a two-minute outage. And at a window of one to
nine seconds — which a room cannot ask for and a harness can — it promised it
late, after the chair had already gone.

Both fall to one rule: **the wording turns at whichever comes first, the cap
or the window, and does not turn at all without one.** `Window` is three
states rather than an `Option<u32>` because *nobody told me* and *this table
waits forever* are different facts that happen to want the same sentence, and
flattening them would make the agreement look like an accident. `PATIENCE`
survives as a readability cap — at an hour-long window the second sentence
would otherwise wait an hour — and stops being a claim about a constant in a
process this crate does not link. The `const` assertion that pinned that claim
is gone with it; what replaced it is a test over a range of windows that
straddles the cap, because a bound written where nothing can check it is the
`RAIL_WIDTH` defect in a different file.

Two things a `Retry` unit test cannot see, both now covered in
`reconnect_tests.rs`. The schedule **does not advance during a dial in
flight** — `tick` must not run there or a slow socket is dialled underneath
itself — so how long the player has been gone is a second accumulator
(`stayed_down`), fed from both arms; measured by the schedule alone, one
slow-failing socket would have held the short sentence up for as long as it
took to fail. And `Connecting` and `Down` **alternate** for the whole of an
outage, so both arms take their sentence from one `link_note` helper: the
`Connecting` arm named `LinkLost` outright, which on a capped schedule would
have flipped the bar between two accounts of one outage every fifteen seconds.

### The clock a player is on

A seat was given no clock at all: on the default preset a player saw nothing
for ten minutes and then lost a decision in silence. `PlayerView`
`decision_remaining_ms` is the number (`VIEW_VERSION` 25), **relative**
milliseconds from the moment its view was built, so the client counts down
from it and takes the next view as the correction — an absolute deadline
would make this machine's clock a rules question.

The owner's question on #69 was not "how many seconds" but **"does the player
ever see the clock, and from when"**: a countdown visible the whole time turns
every decision into a timed test, one that appears at the end is a warning.
This is the warning. `DecisionClock::SHOW_AT` is 60 s, flat rather than a
fraction of the table's limit — which is what makes `blitz` (30 s to decide)
right by construction rather than an edge, because at that table every
question *is* the last minute and the number is on from the moment it
arrives. `Cue::ClockLow` sounds at 60 s and again at 10 s.

Three decisions worth keeping.

**The sound is latched per question, and a question is told from a correction
by size.** The view carries no question identity and `Pending` has no
equality, so `DecisionClock::RESTART` is the rule: a rise of more than a
second is a new question, because a correction is this client's count against
the engine's and differs by a network hop, while a new question restarts at
the table's limit, which `clock::resolve` will not let below ten seconds.
Without it `blitz` would ring on every view of the same question — and the
acting seat is re-sent its own question every time anybody at the table says
anything. That floor is the **gateway's**: a room cannot ask for less, a
local harness can, and at a one-second table seated by `dev-table` the sound
becomes a tick. Named rather than guarded, because the alternative is a
question identity the wire does not carry — and because a limit enforced in
one layer says nothing about the layer beneath it, which is the mistake
`PATIENCE` one section up was *making* until the table's own window reached
the client. This one stays named: `decision_secs` is on the wire too, but a
sound that fires at a fraction of the table's limit is a different decision
from a sound that fires at a minute, and nobody has asked for it.

**The number is drawn for every seat and rung only for this one.** The view
publishes the awaited seat's remainder to the whole table deliberately, so
that a long pause reads as a clock rather than as rudeness. A *sound* every
time an opponent thinks for a minute would be a metronome, landing exactly
when this player is reading the board — the same rule that makes
`Cue::YourMove` a flank rather than a state.

**The cell's presence is gated on the revision; its value never is.** A
`LedgeRevision` field holding the seconds would rebuild the whole shelf once a
second for the last minute of every question, taking every `Feel` on it back
to rest; so the revision carries a `bool`, the tree changes twice per question,
and `count_down_the_decision` writes the digits in place. It touches no
`Node` — the cell is given `clock_width()` when it is spawned, reserved for
the widest number it can hold, because a cell that resized as the digits
changed would shove the sentence sideways once a second. The write is guarded
on the string having moved: assigning an equal `Text` still marks it changed,
and `bevy_text` re-lays every glyph of a component it is told moved.

**The seconds stand in the button the clock will press** (#258). The owner:
*„Der countdown/timeout gehört in den Button Text von dem Button der
‚gedrückt' bzw. als choice akzeptiert wird, wenn der Countdown abläuft."* The
clock answers a timed-out seat with the answer that does nothing where the
question has one (`baylee_engine::choice::timeout_answer`,
docs/protocol.md §"What the clock answers"), and the client reads the same
function: `ledge::clock_answer` turns it into the button that sends it.
Pass priority is the priority row's Confirm ("Pass 12"), an empty
declaration is "None", keeping the hand is "Keep", and a declined "may" is
"No". The ledge then builds its `DecisionClockLabel` inside that button,
after its words and in their ink, at `LABEL_PT` and two digits wide
(`button_clock`), instead of the cell beside the question. It is still one
label, so `count_down_the_decision` writes whichever was built. The cell
remains for everything else: another seat's clock (no answers on this
shelf), a question the house answers (a discard, targets), an armed deed,
and the client's own cast chooser, which takes the answers off the row.
`clock_answer`'s test sends every such button through `Interaction`, so a
number on a button is an answer that button really sends.

What is **not** here is the other half of #69 — stating the limit once, in the
room and on the seat sheet. That needs the limit to reach a client at all, and
it does not: see the section above for why a client cannot be told a window
while it is inside one. It is gateway's #98, at `VIEW_VERSION` 26.

Running out reports `DuelReport::Unreachable`, once rather than once a frame.
Its own variant, because the gateway's `Error` envelope carries the engine's
refusal of a *single action* through `DuelReport::Failed` — a shell that
returned to the lobby on every `Failed` would eject a player for a misclick.
Nothing reads `DuelReport` yet; `Unreachable` exists so that whatever does can
match on it rather than on prose.

### Everyone decides their opening hand at once (#257)

Before turn 1 every seat is asked its keep-or-mulligan, and then its bottom
cards, at the same time, and turn 1 begins when the last seat has kept (a
house rule over CR 103.5's turn order; the engine's mulligan says so). The
view carries it from `VIEW_VERSION` 32: `PlayerView::deciding` is the seats
still deciding, empty from turn 1 on, and while it is not empty `awaiting` is
per view — this seat while it decides, nobody once it has kept — and
`decision_remaining_ms` is this seat's own remainder or nothing
(docs/protocol.md §"How long a seat has left"). Three readers follow.

- **Who the table waits on is one predicate.** `board::is_awaited(view,
  player)` reads `deciding` while it is not empty and `awaiting` otherwise.
  The mat's rim (`Standing::Asked`, through `SeatPod::is_awaited`) and the
  seat bar's caret both call it, so before turn 1 every seat still deciding
  glows and wears the caret at once. Read off `awaiting` alone, a seat that
  had kept saw a table waiting on nobody.
- **A seat that has kept is told who it waits on.** A host sends each seat
  only its own question and `flush_outbox` drops the interaction once the
  answer is sent, so from its keep until turn 1 a seat holds no question and
  the shelf stood empty over a hand it could not play yet.
  `Prompt::after_keeping` reads the view instead: "Waiting for {name}" for
  one seat, "Waiting for {n} players" for several (the carets say which).
  `Duel::headline` is the one chain — the cast chooser, else the question,
  else this — that the ledge draws and the overlay rebuilds on.
- **The clock needed nothing.** `DecisionClock::sync` already counts only
  when `awaiting` is this seat, which in the window is this seat's own
  question; a seat that has kept is shown no other seat's clock, because with
  several running a bare number would not say whose it is.

Two readers of `awaiting` are left alone on purpose: the stack panel's
"waiting for" line, because the panel is only drawn over a non-empty stack
and there is none before turn 1, and `compute_owed_plan`, because no payment
window opens before turn 1 either. Against the house offline none of this is
visible: the house keeps for its chair before the first view is sent, so the
wait only appears at a table of two or more players.

### The table opens for everyone at once (#256)

A networked table does not start until every seat has drawn it
(docs/protocol.md §"The curtain"). The client's half:

- **It says so.** `poll_host` calls `DuelHost::ready` the first time a view has
  been built (`Duel::ready_sent`). It says it again after a fresh `Static`
  while the table is still closed, because that is a new attach the engine
  may not have heard from. For now "drawn" means the first view is built;
  #256(b) moves the call to the moment the table is drawn, art included.
  `ready` has no default body: a host without it would hold the table for the
  engine's whole wait, and only a missing method makes the compiler say so.
- **It holds, and does not drop.** Before `HostMessage::Curtain` the engine
  reads nothing a seat sends, so `flush_outbox` keeps the outbox until the
  curtain arrives (`Duel::curtain_up`) and sends it on that frame. The outbox
  is not empty that early: `run_autopilot` sends the standing ability orders
  as soon as there is a view. No question arrives before the curtain, so a
  player has nothing to answer until then.
- **It never closes again.** `curtain_up` is cleared only by the
  `Duel::default()` a new game starts from. A reconnect or a lag resync
  leaves it set.

`LocalHost` sends `Curtain` last in its first batch: one seat, nobody else
loading, no clock.

## The lobby

Without a ticket the binary adds `LobbyPlugin` (`src/lobby.rs`) instead of
opening a duel, and the client produces its own ticket: register or sign in,
save a deck, open a table or join one. On a granted seat it builds the same
`SeatTicket`, connects the same `NetworkHost`, and sends `DuelCommand::Open` —
from there nothing above the host can tell this game from one the command line
handed it.

The split is the same one the duel uses. `baylee_client_core::lobby::Lobby` is
the whole state machine — screens, form fields, one request in flight at a
time — and answers input with a `LobbyRequest` rather than performing it; the
plugin turns that into an `ehttp` call and feeds the outcome back as a
`LobbyEvent`. So the flow is tested without a window, and the mapping onto the
gateway's routes is tested without a gateway.

**Which gateway.** An address typed into the gateway form is not saved as
typed. The client asks it `GET /info` first (`docs/protocol.md` §"Which gateway
is this? (`GET /info`)"), and saves it only if a gateway answered: one that
answers `/info`, or one from before that route that still answers
`GET /auth/config`. If nothing answered, the address stays in the field with a
refusal under it. Every saved address is asked again at startup and when it is
chosen, and its row shows the operator's name over the address and the version.
The version is green when this client's protocol and view versions match,
red with a warning mark when either differs, and amber with a mark when the
gateway is too old to say or not answering. Pointing at the mark shows why, in
a hint drawn outside the retained tree like the card preview (`lobby/hint.rs`).
The verdict is a colour and never a refusal: an incompatible gateway is still
saved, and whether a game opens is decided by the view check on its first
frame, as before. Everything a gateway sends here is untrusted, so
`client_core::lobby::gateway_info` drops control and bidi characters and caps
the name and the version before anything is drawn.

The front door's colophon ends on the source line (AGPL §13, #270): the
address the chosen gateway gave in `/info`, else this build's repository.
Since #299 it is a link (underlined, `Press::OpenSource`) that opens the
address in the player's browser through `webbrowser` (its `hardened`
feature: http(s) only), and on a tablet or a desktop the same address is a
QR code under it (`lobby::source`, two logical pixels a module, nearest
sampling, the standard's quiet zone, near-black on paper). A phone draws no
code, since it cannot read itself. Both are drawn, and the address opened,
only when it passes `gateway_info::web_address` again at the door
(`source::keep_the_code`, `source::open`), because a hostile gateway's
answer is the thing being drawn; otherwise the line stays plain text.
`docs/legal.md` §6 is the reason for the line.

**Playing as a guest** (#269; `docs/protocol.md` §"Playing as a guest").
When the chosen gateway's `/auth/config` says `guests_enabled`, the sign-in
face draws the guest's way in first, above the tabs and ruled off from them:
a box for the name to play under (empty is the gateway's `Guest`) with
"Play as guest" beside it, the gateway form's address row in shape and
behaviour, Enter in the box pressing the button. A gateway that has said
nothing about guests is offered none: it may have no route for them.

A guest is its session, so the client keeps it: `ClientSettings::guests` holds
the token and handle per gateway address, and the entry then reads "Continue
as Guest#1a2b" and goes straight to the tables with that session, without
asking the gateway first; if the session has ended, the deck list that
follows is a `401`, the kept guest is dropped and the player told
(`Lobby::session_ended`). That makes the settings file a credential store,
so it is written `0600` on unix, new or over an old one
(`settings::store::write_at`), and `KeptGuest`'s `Debug` prints no token.
In a browser the token sits in `localStorage` under `baylee:client-settings`
with the rest of the settings, readable by any script on the client's origin
and with no mode to narrow; it is never logged, there or natively.
Removing a gateway from the list leaves its guest kept, as it leaves an
account's decks on the gateway.

For the whole of a guest's visit the tables screen says what a guest is:
deleted with its decks about thirty days after its last visit (29 to 30:
the gateway renews a guest's session at most once a day, on any call made
with its token, `docs/protocol.md` §"Playing as a guest"). Signing a guest
out asks first, because nothing signs in as a guest again; on yes the kept
guest is dropped and the session ended on the gateway, which deletes the
guest. Every sign-out now ends its session on the gateway
(`LobbyRequest::LogOut`, fire and forget: the lobby has already forgotten
the token, and a gateway that did not hear it lets the session lapse). The client has no sleeve or mat
upload yet; the gateway refuses one from a guest, and a control for it has
to be hidden from a guest (`Lobby::guest`).

**Deleting the account** (#292; `docs/protocol.md` §"Deleting an account
(#292)"). The settings screen's header carries "Delete account" while an
account or a guest is signed in. It opens a confirmation over that screen,
drawn from the lobby's own state (`Lobby::deleting_account`,
`lobby::confirm::draw_deletion`) and not from `Destructive`, because it
types into a field and answers the gateway. A registered account types its
password again, into the confirmation's own box (`Field::AccountPassword`,
never the sign-in form's), because a session left signed in on somebody
else's machine is not enough; nothing is sent without it. A guest has no
password and confirms on its session alone. The request is
`DELETE /account` signed with the session and a JSON body, `{"password":…}`
or `{}`, since the gateway refuses an empty one. A `204` forgets the session
here as a sign-out does, drops a kept guest, and says "account deleted" at
the front door. A wrong password (`403`) or too many tries (`429`) leaves the
session good: the confirmation stays up with the gateway's words under the
box, and what was typed is cleared. A `401` is the session already gone,
as everywhere, and closes it. Enter sends and Escape cancels, which gives
the caret back where it was.

**Rooms.** The table screen lists every room the gateway knows and draws each
one seat by seat: who is sitting there, whether they are a person or the AI,
at what difficulty, what they brought, and whether that chair is ready. A host
opens a room by picking a size (2 to 8) instead of pressing one "host" button,
and from then on every chair is a row of controls — the host's rows switch a
chair between a person and the AI, pick the AI's difficulty, and hand the room
to anyone else sitting at it; everyone else's row is read-only except for the
one chair that is theirs, where the only control is which deck to bring.
Sitting down is a tap on an open chair; standing up is a tap on your own, and
it no longer takes the table with it when the person standing up is the host.

Two buttons on the table's own row, because they are two different claims:
**Ready** is this player saying so and every player has one, **Start** is the
host's and is greyed until the listing says `startable`.

**The list arrives by itself.** `lobby/feed.rs` holds a websocket to
`/lobby/ws` carrying the page this client is reading, re-sent whenever anything
in the lobby moves — a chair taken, a room started, a game over. It is opened
for the *query*, not just the account, so typing in the search box or stepping
a page closes it and dials again; that is also why the URL is built from the
same `GameQuery` the HTTP route uses. A socket that could not be opened is
retried every four seconds, and `Feed::live()` is what the old two-second poll
now waits on: it runs only while nothing is pushing.

A panel takes its height from its content with the screen as a floor
(`align_self: Start` plus `min_height: 100%`). Stretched to the row instead —
what a flex item does unasked — it is exactly one screen tall while its rows
carry on past the bottom, so a scrolled list leaves its own panel behind and is
drawn straight onto the backdrop. Nine tables was the first time anything was
long enough to show it.

**SEARCH** matches a table's name and its host's, and **‹ Back / More ›**
appear only when there is more than one page — a lobby with four tables in it
should not have to explain what page it is on. Both are sent to the gateway
rather than filtered here: the client holds one page, not the lobby.

One box, two uses: **ROOM PASSWORD** locks a room as it is opened and is what
a locked one is joined with. Never two boxes — they are never both wanted at
once — and it is spent on the next open or join and then cleared, because a
password left lying in a text box is the next room's password by accident.

Both boxes on this screen are typed into for the first time: text entry used to
be the sign-in form's alone, so the room password box could be focused and not
filled. Tab rings between the two, Enter runs the search, and `typing_here()`
is what stops a keystroke landing in a field the screen on show does not draw —
the caret survives a change of screen, and without that guard a password ends
up half-typed into a search box.

The seat rows are drawn from the listing verbatim, which is why they carry no
account ids — the client is shown display names and a `you` flag, and has
nothing else to leak.

Hosting an open table is the one asymmetric case. A game against the house and
a join are both playable the moment the gateway answers; an open table holds a
seat whose game does not exist yet, so the lobby keeps the table screen up with
a banner until somebody sits down opposite — which reaches it on the feed, the
table turning `"playing"` being a lobby change like any other. Opening the seat
socket earlier would connect and close again with nothing on it.

The deck list offers *new*, *edit* and *delete*, all of which open or act on
the builder below. Two buttons stay because nothing else does their job: "add
the starter deck", which posts the acceptance file's `Allytifact` rows in one
tap, and "play the house AI offline", which installs a `LocalHost` and needs
no account at all. A finished game gets a "back to the lobby" button, which
closes the duel and drops the host with it.

That is also where an **offline table stops existing**. Offline the table *is*
the host in this process, so the game ending ends the table — and nothing used
to say so. The room went on being listed as `yours` and `"playing"`, so the
next listing made `Lobby::reclaim_a_seat` ask for the ticket to that chair,
and the offline performer, which has nobody to ask, refused it in words. A
player who had done nothing but finish a game came back to a red line in the
corner beside a table still described as running. `came_back` closes it now,
and `reclaim_a_seat` asks nothing at all offline: it exists to recover a
ticket that died while the table lived on at a gateway, and offline a seat
cannot outlive the table it belongs to.

And the end of a game is **not a refusal**. `Tone` is the only thing that
tells the two apart and the lobby draws a refusal in red, so leaving a seat
has two doors — `Lobby::unseat` for a table that could not be reached, and
`Lobby::stand_up` for one whose game is over. Every finished duel used to put
its own ending up there in the colour that means somebody has to do something.

The lobby is `DuelPhase::Closed` only, and brings its own 2D camera — the duel
brings its own and the two never coexist.

### A wait is not a veil

`lobby/systems::waiting` says what the lobby is waiting for on every frame it
is waiting — signing in, talking to the gateway, working offline, taking a
seat — and `loading::raise` decides whether that is worth a screen. It is not,
for the first quarter of a second (`loading::GRACE`), and offline that covers
every wait there is: a request is answered in this process and its reply is
read on the next frame, so *play offline*, *edit* and every other button
raised the veil for one or two frames. Thirty milliseconds of "One moment"
over a screen that had already finished drawing is not information about a
wait, it is a flash — and a flash on every click is what the offline lobby's
flicker was. There is deliberately no minimum time to leave the veil up once
it is raised, because that would be the same lie in the other direction; a
duel taking the screen still drops it on the frame it does (`teardown`).

## The deck builder

A screen of its own (`Screen::Build`), and the same split again: every
decision is in `baylee_client_core::deckbuilder::DeckBuilder`, tested as
arithmetic, and the plugin only draws it and forwards the taps. The drawing
is `buildui.rs`, for the same reason `settingsui.rs` exists: it is a screen,
not a lobby, and the two together were four thousand lines with no seam in
the middle. It borrows the lobby's `Metrics`, `Press` and widget helpers, so
a deck row looks like a lobby row without a second copy of either.

Two things decide its shape.

**The pool is what this build can play.** It is `GET /pool` — the compiled
card registry, not the 118k-printing catalog — because a builder offering
catalog cards would be offering cards the engine cannot put on a table. Every
row carries its `Coverage`, and "playable only" is on by default: it hides the
stubs — cards the registry knows and the engine does nothing with. Partial
cards stay, because they do play, and are marked *partial* with their author's
note in the card panel; turning the switch off brings the stubs back, marked
*stub*, which is a different thing from pretending they are fine. The whole
pool arrives once per session, asked with it, and is forgotten at sign-out
(`DeckBuilder::forget_pool`, #270), since `/pool` answers a session and the
next one may be at another gateway. Every
filter — text, colour identity, type, mana value, sort — runs locally, so
search answers at keystroke latency and never at the gateway's.

It was on by default in three comments and in `DeckBuilder::new`, and **off
in the client**, which is issue #63. `new` is called by nothing outside the
tests: `Lobby` derives `Default` and is the only thing that ever holds a
builder, so the flag shipped `false` and the builder offered every stub. The
flag is stored inverted now — `show_unplayable`, whose `Default` *is* the
intention — so the derive cannot disagree with a constructor again, and the
test that holds it asks a fresh `Lobby` rather than a `DeckBuilder` built by
hand. Asking the hand-built one is what let the whole file be green about a
configuration the client never had, and one existing test had quietly come to
depend on the wrong default: `PoolCard::default()` is a stub, so a test about
finding a card by its German name was only finding it because stubs were
shown.

**The builder filters twice, and now says so once.** Four chips and a query
box, and the filter panel edits the box alone — so a player who opens the
gear, types a condition and cannot find their card is reading one of two
truths. `DeckBuilder::chips_in_force` reports what else is narrowing the list
and `filterui::build` draws it as one line under the rows, in the same `note`
shape the unanswerable-condition warning uses and after it, because that one
is about a row on the panel and this one about something off it entirely.

The chips stayed chips rather than becoming query terms, which was decided
before the line was built: three of the four map cleanly (`id<=wu`, `t:x`,
`is:playable`) and the curve bar does not — it excludes lands, so it is
`mv:3 -t:land`, and a chip that writes two terms is no longer one chip when
it is cleared.

Two details are load-bearing. The line names `playable_only` although
`DeckBuilder::filtered` leaves it out, and the two are right about different
questions — `filtered` answers "is there anything for Clear to clear", which
a standing preference is not, while this answers "is something hiding cards",
where that switch is the largest of the four. And the model says *what*
filters while `buildui` says what it is called, because a colour's and a
type's word live in `buildui`'s own tables and the type's key stays English on
purpose: it is matched against a printed type line. It matters most where it
is least visible — `chips_shown` is `!phone || filters_open`, so on a phone a
chip narrows the list while being drawn nowhere at all.

**One row per card, in every language, and on both faces.** The pool sends
the card, not its printings, and each row carries `alt_names` — every name
that card is printed under, anywhere. So a German player types "Blitzschlag"
and finds the row a deck stores as "Lightning Bolt", and finds it *once*: a
list that repeated the card for each of the forty sets it appeared in would
be answering a question nobody asked.

A two-faced card carries one more, its whole `A // B` spelling, and because
the search is a substring match that answers three things a player might
type: the front face, the back face, and the joined spelling a deck site
exports. The back face had no answer at all before — somebody who knew
Agadeem's Awakening as the land it becomes could not find it by that name.
Unlike the translations it needs no catalog, so offline play has it too. A
search may find a card by a name its *back* prints where a deck row may not:
a row has to resolve to one card and `Demonic Tutor` must stay Demonic Tutor,
while a search offers candidates and showing every card that prints a name is
what a search is for.

Which piece of cardboard is the other question, and it gets its own dialog.
`◈` on a pool row opens the **printing picker**: `DeckBuilder::open_picker`
fires `GET /printings?card=<index>`, the dialog opens immediately on the
printing the row already names, and the answer fills a carousel — art from the
Scryfall CDN, the set and collector number underneath, language chips, and
finish chips for plain / foil / etched. A finish the printing was never sold
in is drawn dead rather than hidden, because which finishes exist is part of
what is being chosen between; and moving the carousel onto a printing that was
only ever sold plain takes the finish back to plain, so a row can never name
cardboard that does not exist.

The pick becomes a `baylee_core::deckrow::PrintChoice`, which is why an
`Entry` is `(slot, count, print)`: two printings of one card are two rows in
the list, addressed by row (`Press::RemoveRow`) rather than by card. The copy
limit is not fooled by that — it counts every printing of the card, which is
the rule `POST /decks` enforces.

The rule that keeps old decks clean: **a choice that changes nothing writes
nothing.** Picking the default printing leaves `4 Lightning Bolt` exactly as
it was, so a deck built before any of this existed can be re-saved without
gaining a single character.

**Saving must not surprise.** `DeckBuilder::problems` is a mirror of what
`POST /decks` enforces, split into blocking and advisory. Blocking is the
gateway's own list (a name, a non-empty deck, 250 lines and 250 cards *per
list*, no card the pool has lost) and it is what greys the save button out;
advisory — 60 cards, a sideboard of 15, a land count that fits the curve,
cards that are not fully implemented — is written in the panel and never stops
anything. If the button is live, the deck saves.

The rest is a deck list: two zones, a mana curve whose bars are also the mana
value filter, the coloured pips the main deck asks for, and `+`/`−` on every
row rather than "click to remove" — a list is read far more often than it is
edited. Reading a card is its own target (`?` on the row, closed with `×`),
because a touch screen has no hover to read one with and the row itself has to
stay the fast way to add.

**Hover shows the card, and it is not part of the tree.** A pointer resting on
a row — in the pool or in either deck list — draws that printing's art beside
it, at the size a card is actually read at, flipped to the other side of the
pointer when there is no room and clamped so a row near the bottom does not
push it off screen. It lives on its own entity behind its own epoch counter
(`Hovered` → `CardPreview`), spawned and despawned by a system of its own:
routing it through the retained tree would mean tearing down two hundred rows
to show one picture, on every pointer move. The row works out its own URL when
it is spawned rather than when it is hovered, because a row already knows which
printing it is showing and a hover that had to go looking would be doing it on
the pointer's schedule. A preview is `Pickable::IGNORE` — it must never eat the
click that would add the card underneath it.

**A card with no picture previews as its text face (#259).** Where a row has
no printing to fetch, or the player reads text rather than art
(`prefer_text_view`, the setting the table reads too), the preview draws the
card's text face: the same `face::UiFace` the duel's overlay draws, with its
rules text (`Detail::Full`), in the printing's finish. The face is built when
the row is hovered, from the registry index the row carries (`HoverCard::index`
→ `face::of_pool`), not for each of the rows the list spawns. Its words are the
row's — the name, type line and rules text the gateway served in the player's
language — on the printed card's body out of the registry. A gateway with no
catalog serves no rules text, and the face then carries the English Oracle
(`generated_oracle::ORACLE`), never an empty box. The rules text steps down to
10 px as it does on the table and shows its scrollbar past that
(`face::show_scrollbars` runs in the lobby too), but does not scroll: the
preview follows the pointer and is never under it, and the wheel over the list
scrolls the list. The back stays a picture, the printing's or the card back.
The held modifier that turns a table card to its text is not read here. A
preview whose picture is still on its way waits for it, as before, rather than
drawing the text face in the meantime.

**The card panel is where a card is moved, not just read.** `?` opens a menu
over the card: add it to the deck or to the sideboard, move the copy that is
already there from one to the other, remove it, or set it as the commander.
Moving keeps the printing — the whole point of having chosen one — and the
panel says what is where ("2 in the deck, 1 in the sideboard") so the buttons
are not the only way to find out. A card that cannot lead a deck is refused as
a commander rather than offered and then rejected on save; naming one that is
not in the deck yet seats a copy, because a commander that is not in its own
deck is not a legal deck and the builder should not need to be told twice.

**Mana costs are symbols.** `crates/baylee-client-core/src/manapip.rs` turns a
`ManaCost` into a list of pips — renderer-free and tested as a table — and
`manaui.rs` draws them with the OFL-licensed Mana font (`docs/legal.md` §2a,
which is where the licence and the trademark are kept apart — the font file is
not a WotC asset, the symbol it draws is a WotC mark used on sufferance, and
the glyphs that policy names outright are not used at all). The font gives a
monochrome mark only, so the coloured
disc behind it is the client's, which is also what makes hybrids drawable: a
hybrid has no single glyph, so the pip is one disc with two glyphs clipped to
opposite halves. Generic costs run out of glyphs at 20 and fall back to digits
rather than drawing the wrong number.

**A loyalty cost is a badge, and the badge is the font's too** since
18.09.2026. `manapip::loyalty_glyph` is the fourth door those codepoints come
through — `E627` for a cost that adds, `E625` for one that takes, `E626` for
a zero — and it is a door for the reason §2a gives the other three. What it
replaced was a *construction*: a rounded slab with a square turned 45° behind
it, which `bevy_ui`'s paint order made into a pentagon with no clip at all.
The trick was sound and it had one shape it could not make. A zero is a flat
lozenge on a printed card and is not a slab with a point on it, so the flat
tick was given the **upward** badge and a comment conceding that no card does
that. The owner read it off a live table and said so.

Two measurements hold the replacement, and both are read off the shipped font
rather than chosen. The badge's number does not sit in the middle of its box —
a point at one end is ink that carries no digits — so
`manapip::loyalty_numeral_centre` is the ink centroid of each glyph, 0.567,
0.437 and 0.497 at 200 px, and the padding on the badge is the difference
between that and the centre a flex box would use. And the three glyphs are
1.000 em wide apiece but 0.705, 0.680 and 0.585 em **tall**, so
`LOYALTY_BOX` is the tallest of the three and the other two sit centred in it:
a card is happy to draw a zero shallower than a plus and a column of rows is
not, because a badge that changed height between rows would move the sentence
beside it.

The one thing the glyph does worse than the construction is a two-digit cost.
A `−12` is printed and a glyph is the width the font drew it at, so the badge
cannot widen the way the card's does — the digits give way instead
(`WIDE_NUMERAL`). That is the trade, and it is named here because it is the
half a reader would otherwise find by looking at Jace.

Leaving a deck with unsaved changes takes two
presses — the first turns the back button into *Leave without saving*, and
anything else answers the question — because a deck is half an hour of work
and the way out sits in the busiest corner of the screen.

The frame decides the shape twice. A desktop and a tablet show the pool and
the deck side by side; a phone shows one at a time behind a switch that names
what is in the other, because the count is the whole reason to look. And a
phone folds the filter chips away behind a *Filters* button — three wrapped
rows of them is most of a phone screen, and what is under them is the point —
keeping *sort* and *clear* outside the fold. Both text boxes go through the
same `softkeys.rs` path as the sign-in form, so a phone raises a real keyboard,
and Enter in the search box adds the first hit.

Every list scrolls, and that took wiring: Bevy's `Overflow::scroll_y` only
clips, so `Scrollable` + `ScrollPosition` and a wheel-and-swipe handler
are what make sixty rows reachable — and a swipe that ends over a card is a
scroll, not a tap. Where each list was left is kept in `Scrolled`, a resource
deliberately outside `LobbyState`: the tree is rebuilt whenever *that* changes,
so adding a card would otherwise throw the list back to the top, and keeping
the offsets inside it would rebuild sixty rows on every notch of the wheel. A
new search does start at the top, because it is a different list.

## What the search box understands

`baylee_client_core::cardquery` is the language both search boxes read — the
deck builder's and the zone browser's. It is **Scryfall's**, or the part of it
this client has the facts to answer, and that is not a stylistic choice: it is
the language every player of this game already knows, from the site they look
their cards up on, and a second one invented here would have to be learned for
no gain.

What it covers: a loose word, `name:`, `!exact`, `o:` for rules text (with `~`
for the card's own name), `t:` for the type line, `c:` and `id:` for colours
and colour identity, `m:` for the symbols of a mana cost, `mv`/`cmc`,
`pow`/`tou`/`loy`, and `is:`/`not:` for the yes-or-no properties — the pool
answers `playable`, `partial`, `stub`, `commander`, `basic` and `dfc`, and a
zone answers the three a projection carries: `token`, `basic` and `commander`.
Every comparison Scryfall writes (`=`, `!=`, `<`, `<=`, `>`, `>=`), negation
with `-`, `or`, and brackets.

Four decisions in it are worth knowing, and three of them were paid for.

**The bare colon is not a synonym.** `c:rg` is *at least* red and green and
`id:rg` is *at most* — which is what makes `id:c t:land` the lands a colourless
commander may play rather than every land there is — while on a number it is
`=` and on a mana cost it is "contains". So `Op::Colon` is kept as its own
operator rather than rewritten at the door, because rewriting it would write
`c>=rg` back into a box a player typed `c:rg` into.

**A loose word looks further here than it does on Scryfall**, where it is a
name and nothing else. This box has always searched the type line and the
rules text beside the name, and the reason it keeps doing so is that it is the
*only* box there is: Scryfall has a page of controls under its own, and a
player here who types `instant` and is told there is no card by that name has
nowhere else to go. `name:` is the narrow reading and has a prefix because it
is the one asked for on purpose.

**A parse never fails and never drops a word.** `frobnicate:yes` keeps its
place as an unknown key and is written back out exactly as it came in; it
simply matches nothing. That is not leniency — it is the property the filter
dialog stands on. A dialog that takes a string apart into controls and puts it
back together has to be able to carry the part it cannot draw, or opening it
would silently delete half of what a player typed. The same property is why
`render` is held against `parse` rather than against a string:
`parse(render(q)) == q` for every query in the test corpus, which is one line
per shape the grammar makes.

**A term the surface cannot answer matches nothing, and neither does its
negation.** The two boxes know different things, and the line between them is
`baylee_view::PublicObject`'s own fields. A zone row is a **projection**, so
every characteristic is there and is the current one — an animated Dryad Arbor
answers `t:creature` and a bear under two anthems answers `pow>=3` — while the
card's *prose* and the pool's bookkeeping are not: no rules text, no printed
mana cost (only its value), no colour identity, no coverage. So evaluation is
three-valued and `Unknown` survives a `-`. `-o:draw` in a graveyard matches
nothing, which is the honest answer; the alternative is a panel claiming every
card in the pile lacks a word it never had the text to look for.

One consequence of the projection is worth saying out loud, because it looks
like a bug and is not: `t:` answers in English in a zone and in both languages
in the pool. A pool row carries the *printed* type line beside the English type
words, so `t:kreatur` finds a creature; a zone row's only type line is built
from the projection, which the engine keeps in its one language. Translating it
would need the catalog's type dictionary, and using the catalog's printed line
instead would be worse than untranslated — that is the card's type line, and
the object on the table may no longer be that card.

The one place it deliberately says *no* rather than guessing is the colour
nicknames. Scryfall knows seventy of them and this does not, and what matters
is how it does not: read letter by letter, `esper` is `{R}` and `boros` is
`{B}{R}` — wrong answers wearing a right one's clothes. So a colour value that
is neither a colour name nor made of nothing but `wubrg` letters is refused,
and is still written back out so a player can see what was refused.

## Taking a filter string apart

The gear inside the search box opens a builder, and the builder has one
property the whole thing is designed around:

```text
FilterForm::of(&query).query() == query
```

for **every** query, not only the ones the controls understand. A dialog that
decomposes a string and recomposes it differently rewrites what the player
typed the moment it is opened — and the first thing anyone does with a builder
is open it to look.

`baylee_client_core::filterdialog::FilterForm` is a **list of parts in written
order**, and that is what buys the property. Each top-level part is either a
`Row` — a term with the minus that may stand in front of it, which a control
can draw — or an `Opaque`, a branch that has no on/off reading and is carried
whole. `t:creature (c:r or c:g) mv:3` opens as control, chip, control, and
closing it gives back that exact line.

Two shapes were rejected on the way. **A struct of named fields** — one
`name`, one `mv`, one colour rule — loses a query a player may perfectly well
have written: `mv>=2 mv<=4` is a range and `t:creature t:goblin` a pair of
types, and a single field would drop one of each on the way in. And **taking
the terms out and appending what is left** is how most builders are written,
and is why most builders reorder the line they were opened on; keeping each
part in its place means the rebuilt query is the one that came in.

What a control may stand for is the top-level conjunction and nothing deeper.
A control says *this term is on*, which is a reading a term only has where
everything beside it must also hold — `a or b` minus `a` is `b`, a different
question — so a term inside a bracket is never lifted into a ticked box. That
is the same line `FilterPart` draws, said from the other side.

`Control::of` maps a key to the widget it wants (text, colour pips, a
comparison and a number, a tri-state tick, a mana cost), and a key nothing
knows gets a plain text box rather than being hidden: `frobnicate:yes` is
still something a player can read and edit. `FilterForm::unanswerable` names
the rows the surface the dialog was opened from cannot answer — it is a
warning beside the control and never a refusal, because a term this surface
cannot answer hides every row, which reads exactly like a search that found
nothing, and this is the one place the client can say so before it happens.

### Opening it writes nothing back

`FilterPanel::written` answers `None` until a row has actually been changed,
and that is the sharpest edge in the feature. What is proved above is about
*queries*, and a box holds a **string**: `Key::render` picks one spelling out
of the several Scryfall accepts, so a builder that wrote its own reading back
on opening would turn `color:red cmc:3` into `c:r mv:3` — the same search, and
a box rewritten by nothing but being looked at.

A fresh row is written into the box straight away, though, which means every
one of them has to be a term that **narrows nothing**. Two of the five word
keys read the other way round until the builder began opening rows — `o:""`
and `!""` matched no card at all — and two were worse than wrong: `""` and
`!""` were read back as `Query::Anything`, so a row a player had just added
vanished out of the box that was holding it. `wrote_nothing` is the
distinction the parser was missing, and it is only visible before `unquote`:
`""` is a value somebody wrote and a bare nothing is somebody mid-word.
`cardquery::tests::a_value_nobody_has_typed_into_narrows_nothing` is the
guard, and it asserts the parse beside the match because the corpus round trip
cannot see this one — an `Anything` written and read again is an `Anything`.

### One caret, and the buttons carry the meanings

The builder is a **mode of the search box**, not a window: the box keeps
showing the string, the gear inside it stays lit, and a tap in the box shuts
the builder and takes the caret. At most one row of the builder holds a caret
at a time, and while it does, the whole chord set goes there — Backspace,
Delete, the arrows by character, word and line, shift-extended selection, ⌘A.
What gets typed is put through `cardquery::value_of`, the parser's own reader,
so a row arrives at exactly the value the same letters typed into the box
would have made. That is also what reaches the things a keyboard of six mana
symbols cannot spell: `{W/U}`, `{R/P}`, `{X}`.

Every button in the panel carries a `filterdialog::Act`, and
`crate::filterui` — the one drawing, in either register — calls no method on
the panel at all. The owner reads the component and hands the `Act` over. A
control added to the builder is therefore wired by being drawn, which is the
failure this crate has shipped before: `Interaction::activate` sat written and
reachable from no button, and a test that calls the method cannot tell.

The two exceptions carry a marker instead of an `Act`, and both for the same
reason: the gear and the panel's *Done* are about whether the panel is on
screen, which is its container's business and not the model's. A model that
could close itself would be a model that knew it was drawn.

### A row is drawn by what it holds

`FilterPanel::control` and not `Control::of(&term.key)`, because one key
spells more than one kind of value: `c:r` is a set of colours and `c>=2` is a
count of them, `is:token` is a flag and `is:permanent` is a word this language
does not know. Drawn by the key alone, the second of each pair was a row of
unlit buttons — saying nothing about what it held, and one press away from
overwriting it. What no control can draw is drawn as a text box, which draws
whatever was written.

Three things had to move with it, and all three are the same mistake in
different clothes: a control that knows the key but not the value. The stepper
read `Value::Number` alone, so `+` on a `c>=2` row read nought and wrote
`c>=1`; `SetNumber` *wrote* a `Value::Number` whatever the key was, which
under `c` renders the same string and parses back as a `ColorCount`, so the
box was right and the form held something the box could never have produced;
and the `gerade`/`ungerade` pair is offered only where the key itself reads a
number, because `c:even` goes through the same reader every value does and
comes back an unreadable word. The number a button writes now goes through
`cardquery::value_of`, which is the same door the caret types through.

The caret is the exception that proves it. Typing re-reads the value on every
keystroke, so its kind changes mid-word — `toke` is a word and `token` is a
flag — and a row that swapped controls on the `n` would take the box out from
under the letters while the caret still named that row. A row being typed into
is a text box until the caret is given back.

All of this was found by **photographing** the dialog, not by a test: the
round trip was green throughout, because the form was carrying the values
faithfully and only the drawing could not show them.

### One drawing, two registers

`filterui::Register` is six colours — a ground, a control's ground, a line,
two inks, and one colour that means *on*. The zone browser passes
`Register::TRAY` (candle on dark oak) and the deck builder `Register::LOBBY`
(teal on slate), and nothing else about the panel differs. That both surfaces
draw from one function is the test of whether the split was made in the right
place; a second drawing would be a second set of buttons to keep in step with
the model.

The two differ in one thing that is not a colour: the `Surface` they hand it.
A zone row is a projection and a pool row is a printing, so the flags an `is:`
row offers and the rows the panel warns about are not the same on both.

## A payment window, and the sentence it did not have

A CR 605.3a payment window is **deliberately shaped like nothing**: an
ordinary `Pending::Priority` offering mana abilities and no plays. That shape
is what lets every client draw it and every agent answer it without a new
question — and it is exactly why neither could tell it from a quiet pass over
untapped lands, which is most windows most turns. The house agent said yes to
ward's tax, was handed the window, found nothing castable, passed, and its own
spell was countered. `PlayerView::owed` (`VIEW_VERSION` 24) is the answer, and
it is a field rather than an inference on purpose: reading "I owe something"
off an offer of mana abilities with nothing castable would tap lands in every
other quiet window too.

**The paying was never missing. Only the telling was.** A player in that
window can already tap lands one at a time and watch the mana float — mana
abilities are the documented exemption from the arming contract (one tap,
CR 605.1, because floating mana is the cheap mistake), and a payment window is
made of nothing else. So this is not the arming path pointed at a new cost. It
is three things that say what is happening:

- **The sentence.** `Prompt::headline` takes an `owing` flag and answers
  `Phrase::PayOrPass` instead of "Your move", on either player's turn — a
  payment window on an opponent's turn is still a payment window, so `owing`
  beats `Turn` rather than sitting inside it. It says what the window *is* and
  not what is owed, and stops short of what declining costs, because whether
  it is a countered spell or an unpaid tax is the engine's sentence.
- **The number**, beside the mana pool rather than in the shelf's middle
  column. `Owed {2}{G}` and `have {G}` then stand in one register at one
  scale, drawn by the same `manapip`, so a player subtracts them by looking
  instead of converting first. The strip is hidden while nothing floats, and
  the first frame of a payment window is exactly that case, so `owed` is a
  fourth conjunct on its `empty` gate — otherwise the row saying what is owed
  would unfold only after the player had worked it out.
- **The lands**, through the planner unchanged. `manaplan::plan` takes a cost
  it did not derive and spends the pool first by its own contract, so passing
  `owed` — which is the *total*, not the remainder — needs no arithmetic here
  at all. `glow::WILL_TAP` lights what it names.

`Duel::proposing` is what keeps those lands honest. It is an enum —
`Nothing` / `Armed` / `Owed` — and not an `Option<&Armed>` beside an
`Option<&Plan>`, because two options are four states of which two are
nonsense, and the third source this grows in a year would arrive as a third
option every reader could go on ignoring. An armed deed wins over an open
window: it is a commitment the player made, and a window opening underneath
it does not relight the board. Nothing is ever *armed* by a window, which is
its own test — arming here would be the contract growing an exception for the
one case it was written to exclude.

The plan is cached on `Duel` and refreshed on **both** edges, because it needs
both: the view carries `owed` and the pool, the pending carries the mana
abilities that could pay it, and the engine sends them in either order. A
refresh on one edge only fails silently — the lands simply never light — so
the order is asserted rather than trusted.

`a_quiet_priority_window_over_untapped_lands_lights_nothing` is the test this
whole field exists for, and it is the house-AI bug written from the client's
side. If it is ever green for the wrong reason the feature is a permanent glow
on every land a player owns, all game.

## Tapping lands for a spell

The engine offers a spell as castable only when the mana is **already
floating** — `casting::can_cast` checks the printed cost against the pool, and
the pool is empty until something is tapped. That is the correct rules answer
and a miserable one to play against: a hand of spells and five untapped lands
looks, to a client, like a hand with nothing in it.

`baylee-client-core/src/manaplan.rs` is the other half of that question. Given
a cost, what is floating, and the sources the engine *itself* listed as
tappable, it returns the taps that make the spell castable — or `None`, which
is a real answer too: it is what leaves the card unlit.

The matching is Kuhn's algorithm on a bipartite graph of demands against
available mana, not a greedy sweep, because greedy produces the classic
misplay: it pays the generic pip with the only land that makes black and then
cannot pay `{B}`. Two orderings turn "a matching" into the matching a player
would make — demands are taken most-constrained first, and each one reaches
for floating mana before any tap, then for a **clean** tap before a priced
one, then for the *least* flexible source that fits. So the Forest pays the
green pip, the Command Tower is still untapped afterwards, and Ancient Tomb's
two damage are taken only where no free land fits.

Price comes before breadth, and one case decides it: Ancient Tomb beside a
Command Tower paying `{1}`. By breadth alone the Tomb goes first (one colour
against five), and the player takes two damage with a free land untapped.
Payability does not move, since Kuhn finds a matching whenever one exists
whatever the order; only which taps it picks moves. But an order is per pip
and a plan is per permanent, so price first on its own would tap the Tower
*and* the Tomb for `{2}`, where the Tomb alone pays it for the same two
damage. So the matching is followed by `consolidate`, which gives back any
tapped source whose pips fit on mana the plan already has: floating, or the
unused half of a permanent it taps anyway. A priced source is tried first,
then the roomiest. The same pass stopped a Forest listed before a Sol Ring
from paying half of `{2}` beside it, which predates any price.

Four rules keep it honest, and they are the reason to read the module before
changing it:

1. **Every step is an action the engine offered.** A `Source` is built from
   `LegalActions` — `mana_abilities` for the CR 305.6 shortcut, `abilities`
   for a printed one — never from the client's own idea of what a land does.
   The run in `ManaRun` re-checks each step against the *current*
   `LegalActions` before sending it, so a plan that has gone stale stops and
   hands the turn back rather than pushing an action that would bounce.
2. **It never spends what a player would want to decide.** Phyrexian mana is
   read as its colour and never as two life; `{X}` and `{S}` are refused
   outright; restricted mana (Cavern of Souls) is not counted, because what it
   may be spent on is a rules question this side of the wire cannot answer.
3. **It under-counts rather than over-counts.** A source that makes two mana
   *of one chosen colour* is worth one mana here, because two units that must
   match are not two independent units, and pretending otherwise builds a plan
   the engine rejects halfway through — with the land already tapped. The cost
   of being wrong in this direction is one extra land.
4. **A card with more than one way to be cast is asked about *before* it is
   paid for.** This one is newest and is the subject of the next section: a
   planner that floats a cost has already chosen which cost, and choosing is
   not a planner's to do.

Knowing that ability 2 of a Command Tower makes mana takes the compiled card
registry, which `baylee-client-core` deliberately does not link, so that half
lives in `baylee-client/src/manasources.rs`. It refuses an ability that costs
mana to activate, because the plan would have to recurse.

The list an ability index points into is read where the engine reads it
(`manasources::printed_abilities`): the card the object's abilities are
printed on, else the **token definition** behind `PublicObject::token`. A
token has no card and so no `rules`, and until #210 nothing on the client read
a token's abilities: a Treasure was never a source, and seventeen of them left
a four-mana spell unreachable with the lands tapped. A Treasure now plans and
clicks like Lotus Petal — a mana ability, so one tap and a colour, never armed.
It makes any colour and costs the token, so the matcher reaches for it after
every clean source that fits. A free five-colour source (a Command Tower, a
land under Chromatic Lantern) used to tie with it and fell to object order,
because `priced` ranked only the modes of one permanent. It is a field on
`manaplan::Source` now, and the matcher reads it across permanents.

An ability that does something *besides* make mana it accepts, and **ranks**
— which is the same policy reached a different way. A tap with a price beyond
the tap sorts behind every clean tap the permanent has, and the surviving
entry carries `Source::priced` into the matcher, which puts it behind every
clean tap on the board. So the priced mode is reached only where nothing else
can pay: the case in which the player would have tapped it by hand anyway. A
price is two things written in two places and `manasources::priced` weighs
both — in the cost (Havenwood Battleground's sacrifice, a Vivid land's charge
counter, Spire of Industry's life) and in the effects (Adarkar Wastes deals
you a damage and charges nothing to tap). An effect counts as a rider only if
it is not mana: counting effects made a Karoo's second `AddMana` a price. Before that ranking existed the planner read amount
and colours alone and so *always* took the expensive mode on the 25 faces that
print both: it sacrificed Havenwood every time it tapped it.

The consequence is worth knowing before it surprises you at a table: **the
expensive mode of a permanent offering two is out of reach.** Havenwood is
never planned for `{G}{G}`, Vivid Crag is never planned for blue, and Adarkar
Wastes is never planned for white — one sentence with two causes. Reaching it
needs `manaplan::Source` to carry modes and the matching to pick one per
permanent; `the_expensive_mode_is_out_of_reach_and_that_is_the_bargain` is the
test that goes red the day it does. The trade is right because the two
failures are not symmetric: a shy planner costs a player some clicks, an
over-eager one strands a half-tapped board mid-cast with no way back.

What the accepting half buys is narrower than it sounds, and for the same
reason. A card with a clean tap beside a ridered one is planned as the clean
one and gains nothing. The whole yield is the eleven faces whose *only* mana
ability carries a rider, which were not sources at all — Ancient Tomb printing
`{T}: Add {C}{C}. This land deals 2 damage to you.` counted for nothing in
every plan, and a player tapped it by hand each time.

In the hand, this is a third state and it is drawn as one:
`BoardModel::from_view` takes an `Openings { playable, reachable, activatable }`
rather than one set, because they are different claims — gold is the engine
saying yes, indigo is this client offering to tap lands first. Clicking either
casts; the difference is what happens in between.

#### An offer has to be one the engine will accept

Indigo is a promise this client makes on the engine's behalf, and the promise
is expensive to break: taking it taps the lands **first**, and the engine then
refuses the cast with the turn's mana already spent and the card still in
hand. So every rule the engine will apply to that cast has to be applied here
first, and the two that were missing cost a turn each. The timing rule is
`baylee-client-core/src/timing.rs` — a sorcery was lit on an opponent's turn
and over an unresolved stack — and the target rule is
`baylee-client/src/targeting.rs`, because a spell with no legal target cannot
be cast at all (CR 601.2c) and Swords to Plowshares lit up with every creature
already exiled.

The two are written to **opposite defaults**, and that is deliberate rather
than untidy. `timing` errs towards offering, because the two effects it cannot
see are rare and a sorcery withheld for one of them is only a hand the player
taps themselves. `targeting` errs towards offering *too*, but it gets there
from the other side: it withholds **only when it can positively prove that no
legal target exists**, and offers whenever it cannot tell. Written as a
symmetric check — refuse whatever cannot be read — it would darken a card for
a reason the player cannot see, which is the fault this client has already
been reported for; written as a proof, the worst it can do is fail to catch a
case.

`castmodes::parts_payable` takes the genuinely opposite default and is the
useful contrast: anything it cannot answer from the view is refused, because a
refused *alternative* cost costs nothing at all — the card falls back to the
price printed on it. What decides the direction each time is the consequence
of being wrong, not a house style.

`targeting::matches` is a mirror of `baylee_engine::eval::matches_projected`
held to the stricter rule every mirror here is held to: the engine may answer
from the `GameState`, and what the **view** cannot answer returns `None` and
abandons the proof. Three details carry their own reasons. One unreadable
candidate abandons the whole count, because the claim is a negative and "none
of these matches" is only true if every one of them was looked at. A permanent
this seat may not look at abandons it too, since a face-down permanent is a
2/2 creature and a perfectly legal target (CR 708.2) while its view fields say
almost nothing. And `Filter::IsToken` reads `PublicObject::token` where the
engine reads `card.is_none()`: in a view that second test is also true of a
permanent nobody is entitled to see, which is a different fact wearing the
same shape.

The population is measured in `targeting`'s own test rather than written down
here, because three frozen numbers stood in this paragraph until a card batch
moved the pool under them inside a week. Two sentences differ by exactly one
condition and only the second is the feature: *this filter is readable* asks
whether every arm of `matches` can read the filter tree, while *this spell can
be proved targetable or not* asks that **and** that `legal_targets` has an arm
for the spec. Reanimate sat in the first set and outside the second for as
long as `CardInGraveyard` fell to the wildcard — `Filter::CREATURE` is
trivially readable and the spec had no arm at all — which is how "the
population is closed" survived being written down. Every target-requiring
spell ability is provable, never-targetless (a spec naming a player, or naming
something the spell already holds, refused by construction rather than by a
gap), or blind; a blind one is offered and never withheld, and the test puts a
floor under the population and a ceiling over the blind bucket.

#### A card in a pile is reached for too (#242)

The hand was the first place `reachable` read, and the command zone the
second. The third is the seat's **own graveyard**, for a card it may flash
back: the engine offers such a card in `castable` only once its cost is
floating, exactly as it does a hand card, so Snapcaster Mage's Opt ended the
turn in the graveyard beside the untapped Island that could have paid for it.
The view says which cards and at what price, `PublicObject::flashback`, and
the price is that and never the card's own — the two agree for a grant
(Snapcaster, Past in Flames) and differ for a card that prints flashback
(Think Twice: `{1}{U}` from the hand, `{2}{U}` from the graveyard).
`mana_for` reads the same three places, since a card in one and not the other
lights up and then does nothing when it is clicked. Timing and targets are
asked exactly as for a hand card; `targeting::provably_targetless` takes the
card's identity rather than a hand object for that reason.

A pile card is lit wherever it is drawn, by one predicate,
`Duel::reach_of`: `Reach::Offered` when the engine will play, cast or activate
it with what is floating, `Reach::Taps` when this client would tap for it
first. On the felt that is the pile's top card and the cards a hover fans out
(`glow::ACTIVATABLE` and `glow::REACHABLE`); in the zone browser it is the
hand's halo on the row's picture, gold and indigo, in all three views. The
browser's rebuild gate holds the lit list itself rather than trusting `seq`,
because the sets it reads come from the interaction as well as the view.

Two consequences worth knowing. A commander standing in the command zone with
the lands to pay for it was reachable and drawn dark; it is lit now, by the
same line. And a tap on a lit pile top is the cast, not the pile: it arms the
run as a tap on a hand card does, where it used to fall through to opening the
graveyard. The pile is still opened from the tray button and `G`, and by a tap
on any top card nothing is offered for.

### Which way to cast it is asked before anything is tapped

`Engine::cast_options` counts a spell's ways against the mana that is
**already floating**, which is right and unavoidable: the engine has no
planner and cannot know which lands are about to be tapped. The planner does
have one — and it used to float *exactly the printed cost* and then send
`CastSpell`. By the time the engine counted there was one way left, so it
asked nothing, and the client had chosen for the player in silence.

The silence ran both ways. An alternative cost **dearer** than the printed one
could never be picked by clicking: Reveillark's evoke is `{5}{W}` against a
printed `{4}{W}`, so seven open mana cast it the cheap way with three lands
still untapped. And a **free** alternative was taken just as quietly in the
other direction — Solitude with an empty pool is already in
`LegalActions.castable`, for its evoke, so the click exiled a white card and
the printed `{3}{W}{W}` was unreachable for a player who would rather keep it.

So the question moved in front of the floating. `Duel::cast_menu` is the same
chooser the engine's own `Pending::ChooseCastMode` opens — it *is* a
`Prompt::CastMode`, drawn by `choices::options` like any other, with the same
rows, the same costs and the same `ChoiceButton` — built one step earlier, out
of `baylee-client/src/castmodes.rs`. A row **arms** rather than sends, the way
the ability sheet's rows do, because there is no undo and a spell on the stack
is the least undoable thing in the game. The answer is then remembered in
`Duel::cast_answer` and spent when the engine finally asks, which is several
round trips later: the taps, the cast, and only then the question.

Three things about it are load-bearing.

It is remembered as a **`CastModeKind`, never an index**. The engine numbers
its options by position in a list it rebuilds against the pool of the moment,
and the pool of the moment is exactly what the run in between has changed.

It is **not carried on the `ManaRun`**. `advance_mana_run` clears the run on
the frame it sends `CastSpell`, so the `ChooseCastMode` arrives to no run at
all — and the free-alternative case has no run in the first place. It lives on
`Duel` and is written off when the seat gets priority back with nothing armed
and no run going, which is the shape of "the spell is on the stack and the
engine had only one way to offer".

It **refuses what it cannot read**. A row offered here that the engine will
not offer is a row that lies, so a way is listed only when every part of its
cost can be evaluated from the view: the mana through `manaplan` as always, a
life payment against the seat's own total, and a pitch (`ExileFromHand` with a
colour filter — Force of Will's blue card, Solitude's white one) against the
hand, where the card being cast is never a candidate for its own pitch.
Anything else takes the way off the list. So does everything but `Normal` and
`Alternative`: a modal spell's modes need `casting::mode_has_a_legal_target`,
which takes the `GameState` this client does not have and must not
approximate, so Damn's overload is still cast the cheap way when the click
finds one. That is a hole in *this* list and not in the chooser: a modal spell
the engine asks about — Sheoldred's Edict, which is affordable at three
different modes — opens `Pending::ChooseCastMode` and is answered through the
same rows.

One way is not a question, and the click then does exactly what it always did.
`fire_armed` had to learn one thing for this: a chosen way with taps left to
make goes to the run and **never** to the engine's standing offer, because
that offer is the wrong answer by construction — Solitude is `castable` the
whole time, for the way the player just declined.

While it stands it takes the prompt bar **whole**, answers included. Every
other row there reads `interaction.pending`, which is the ordinary priority
window the chooser was opened inside, so the bar drew "Choose how it is cast"
as its headline with a gilt "Pass priority" underneath — two primary buttons
on one sheet saying opposite things, and the one the keyboard would press was
not the one drawn as primary. The engine's own `ChooseCastMode` draws no
answer row either, a row there *being* the answer, so this is the same chooser
in the same clothes rather than a special case. `Esc` is the way back out.

The **rows** left the bar in AX step 6c and stand on the card's own parchment
now, beside the card in hand — §"A question about a card is a sheet" below,
where the two models and the three things that tell them apart are set out.
What stayed on the shelf is the sentence and the answers it takes away.

And a tap on **another card** puts it away, which is the change of mind that
already disarms. Only `activate_card` opens either menu, and it now closes the
other before it takes any branch — without that line a card with one way, a
land, or a permanent with abilities each left the previous card's question
standing over the new card's deed, and a row press then armed the card the
player had stopped looking at. The chosen way goes with it, and with an `Esc`
that disarms: `cast_answer` outlives the run that spends it by design, so
every gesture that *is* a player leaving a card has to say so, or the engine's
next `ChooseCastMode` about that card is answered by a decision that was
abandoned.

**A row says what the card says.** `Mode(i)` and `Alternative(i)` are the
whole of what the engine tells anyone about a mode or an alternative cost, so
the chooser drew "Mode 2" and "Alternative cost" — a player picking between
numbers on a card they may never have seen, while the ability sheet two
inches away had been drawing printed sentences since it existed. Both kinds
now read their sentence out of the generated line table (§"Which ability is on
the stack"), in the printing and language the player chose: Sheoldred's Edict
offers its three bullets, Solitude offers "Evoke—Exile a white card from your
hand". The same chooser asks a modal **trigger** — `Pending::ChooseCastMode`
is the one question for both — so Charming Prince and Aether Channeler read
theirs too, once the table stopped being silent about a trigger's modes.
Where it cannot, the number stays and is the right answer: Derevi prints one
sentence for two modes, and no row could draw words that tell them apart.
Reminder text is dropped — it is the card explaining itself, which is
worth a line on a card and is not what a button says — and the bullet a modal
card lists its modes under goes with it, the row already being one of several.
Four marks, and they are read off the catalog rather than guessed: `•` in
English, Portuguese and Chinese — which leaves no space after it — `*` in
German, French, Spanish and Italian, and `・` in Japanese. Only the drawing
is affected; the index is computed against Scryfall's English and every one
of those printings splits into the same number of lines.
The cost stays drawn beside the words even where the sentence prints it too,
because that is what the ability sheet does with "Cycling {B}" and the two
choosers are meant to be indistinguishable. A printing that does not pair,
or no text at all, reads the compiled English Oracle (`cardtext::sentence`).

**The normal way is its cost and no words** (#212). It said "Printed cost",
which is the client describing a button rather than the card saying
anything; the row now draws the face's mana cost as pips beside an empty
label, and a spell cast for nothing draws `{0}` rather than an empty row. An
alternative cost whose card the view cannot name has no words either (it
said "Alternative cost"). A **mode** with no sentence keeps its number,
"Mode 2", because there the number is the only thing that tells two rows
apart: Derevi, Inspirit and Tireless Provisioner state their choice inside
one sentence (`lines::MODES_PRINTED_INLINE`), a mode that declines prints
nothing, and a trigger's mode costs nothing to draw instead.

**A prepared cast is the spell it casts** (#212). Emeritus of Woe offers a
copy of Demonic Tutor under the reserved `PREPARED_CAST`, which is no
ability on the card, and the row said "Cast the prepared spell". It now
draws the linked card (`AbilityDef::Prepared { card }`,
`abilities::prepared_of`) the way a card is drawn: its name over its whole
text, in the player's language where it has arrived and in English before
(`abilities::prepared_words` over `CardTexts::face`), with the spell's
printed mana cost in the cost column. The linked card is asked for with the
cards the view names (`cardtext::wanted`), since no view ever names it; the
link is printed on the permanent, so asking tells the gateway nothing new. A
permanent's printed row with neither sentence nor cost symbols has no label
either: it said "Ability 4".

Drawing a sentence where two words used to go is what found the slip's other
half. A row was a button with no width of its own, and Force of Will's
alternative cost is 143 characters in German: it came out as one unbroken line
**914 logical pixels** wide, hanging 147 px past each edge of the 620-wide
parchment it was drawn on. `max_width` binds a node's own box and not its
children's, so the sheet stayed 620 and the answer walked out of it. An answer
is now capped at `SLIP_INNER_W` — the slip's content box, edge to edge — with
a `min_width` of zero under it, and both are needed: without the floor a flex
item's automatic minimum size is its own content, so it refuses to shrink and
the cap only moves the overflow. The line inside wraps on its own once
something narrower than the sentence tells it where, `manaui::rich` being a
wrapping row of words and marks. Measured: 914 × 31 before, 574 × 49.5 after,
two lines on one sheet, and a press on the second line still answers.

`activatable` is the board's half of the same idea, and it is the engine's own
answer: every source named in `LegalActions.mana_abilities` or `.abilities`.
It reaches the shader as a fourth glow bit, and is drawn as a *moving* warm
light running round the border rather than as a steady sheath — the keyword
glows say what a card **is**, this one says what a player **could do**, and
two different kinds of claim must not read as the same light. It rides on
`CardLook` like everything else, so a Forest that becomes tappable becomes a
different material and stops being one the moment priority moves on.

`CardGroup::activatable` is true only when *every* permanent the card stands
for can act — all, not any. A card standing for three identical creatures that
lit up because one of them could be tapped would be inviting a click that is
refused.

Manual activation is the other half, and the half that did not exist at all:
`Interaction::activate` was written and nothing called it, so a Forest, a mana
dork and a planeswalker were equally inert under the pointer.
`baylee-client/src/abilities.rs` is the list — built only from `LegalActions`,
in a stable order, each entry labelled from the registry, because "Ability 2"
is a label a player has to guess at and "Tap for {G}", "+1" and
"{T}, Sacrifice this, Pay 1 life" are not — a printed ability with neither a
colour nor a loyalty cost is named by what it costs to activate, which is the
half of it a player is actually deciding about. One
option activates on the click that found it; several open the **sheet** below.
Either way the answer goes out by *position*: the list is rebuilt from the
current `LegalActions` when the row is pressed, so a sheet drawn a frame ago
cannot send an ability the engine has since withdrawn.

### A question about a card is a sheet

It was a row of buttons in the prompt bar, which sat at the far side of the
screen from the card it was about and could only say "Ability 2" about the
ones it had no words for. It is now a piece of parchment anchored beside the
card itself: **cost, sentence, key** across each row — the cost drawn as
pips on the left, the ability's own printed sentence in the player's language
in the middle, and a numbered keycap on the right. That is the order the row
is *used* in: what a player checks first, then what they are choosing between,
then what they press. Both ends hold a fixed width, which leaves the sentence
one straight left edge down the whole sheet.
`docs/keyboard-map.md` §"The ability sheet" is normative on the keys;
`baylee_client_core::abilitysheet` is the arithmetic and
`baylee-client/src/hud/sheet.rs` draws it.

#### One renderer, two models

The sheet draws **both** choosers a card can open: what a permanent can do,
and the ways a card in hand can be cast. AX §5 had put every indexed chooser
in the drawer and left the ability one beside the card, and the sentence §5
gave for that exception — *it belongs to the card, not to the question* — is
just as true of the other one. So the owner's answer of 14.09.2026 is that
they are the same piece of paper (AX step 6c), and the drawer keeps the
questions with **no card to stand beside**: a colour, a seat, a target, the
number stepper, the subtype filter, combat's two lines. A colour is the case
that fixes the line — it comes off a mana ability that is already resolving,
so there is nothing on the table to hang parchment beside.

`sheet::Opening` is the seam. Two readers — `ability_opening` and
`cast_opening` — produce one struct of precomputed `SheetRow`s (an answer
index, the printed blocks, the one fallback line, the cost) and everything
below them draws rows without knowing which model it got. That direction is
deliberate: the renderer used to take an `abilities::AbilityOption` and reach
back into the `Duel` for the card's text, which is a shape only one model can
satisfy.

What a `Source` decides is exactly three things, and each of them is a control
that would otherwise be wrong rather than a style choice.

**Which button a row wears.** An ability row arms and is then sent, and is an
`AbilityButton`. A cast row answers by index and is one row of an indexed
choice, which is what `hud::ChoiceButton` is everywhere else in this client —
so a row that moved onto this sheet kept its click path (`input::pick_choice`
already routed both cast sources), its keys and its `/state` shape, and a
reader asking for *abilities* rightly does not see it. It is not a field on
`AbilityButton`: nothing downstream wanted to tell them apart.

**Whether there is a pager.** `abilitysheet::paged` counts against
`Duel::ability_page`, which `turn_the_page` writes and `ability_menu_keys`
turns with `0`. A cast list has no counter of its own, so a pager there would
claim there is more to see and then refuse to show it. Nothing in this pool
reaches ten ways to cast a card; the day one does, this is where it asks for a
counter rather than quietly losing the tenth row.

**Whether the head carries its cross.** Every door out of this sheet — the
cross, `Esc`, a press outside it, the row itself — works by clearing the menu
that opened it. A `Prompt::CastMode` the **engine** asked is a question the
table is waiting on and nothing on the paper can withdraw it, so the cross is
refused there. The name and the hairline stay: those say whose list this is,
which is as true of a question that must be answered as of one that can be put
down.

Two things about that last route are easy to get backwards, and both were
found by building it. `cast_opening` reads the engine's `Prompt::CastMode`
**as well as** `Duel::cast_menu` — the drawer dropped both, so a sheet that
read only this client's own would draw the engine's question *nowhere*. And
the head's name needs two lookups: `PlayerView::object` is the one zone-blind
spot in the view and does not look in the **hand**, which is the whole case a
cast chooser is about, so it falls back to `face::face_name`, which searches
the hand first. Without it the rows stood under a blank line.

**A permanent that makes mana gets a header of pips**, one per colour it can
pour, centred above the written rows and carrying no digit —
`abilities::Split` is where a sheet's pips end and its numbered rows begin,
and the digits count only the rows, so `1` is always the first sentence and
never the first disc. Where there are no rows left the header *is* the sheet,
which is the mana bubble a Plains or a Tundra opens.

Which taps the header stands for is the whole of the judgement, and it is
`manaplan::pours`: **a permanent taps once**, so the pips are a colour-wise
union over its taps rather than a list of abilities, and a colour offered
twice is one pip. It goes to the tap a player can foresee — an `Offer` carries
whether what one press pours is a number at all, and a pip promises one mana
of the colour pressed — and then to the tap that will not stop to ask. Harabaz
Druid under a Great Divide Guide is the card that settled it: the grant makes
one mana of any colour and the Druid's own ability makes X, where X is a count
of the battlefield nothing this side of the engine reads.

And **a tap the header does not stand for keeps its sentence**. That is the
partition in `abilities::pour_out`, and it replaced an all-or-nothing rule
that dropped every mana row once the pips were built. Two ways a mana row can
fail to be under a pip, and both are real cards: it is no offer at all
(Jasmine Dragon Tea Shop's any-colour tap is restricted to Allies, and a pip
can say "white" but not "white, and only on Ally spells"), or it lost every
colour to another tap (the Druid above, whose own ability had simply vanished
from the sheet). Either way the row is written out and the digits count it.

**A written mana row that is a number nobody here can count is a step, not a
send.** Pressing the Druid's row would otherwise tap the card and hand the
colour to the engine's own chooser; what it does instead is step *into* that
tap — `Duel::ability_tap`, read through `asking_tap`, so a value that outlives
its sheet is inert — and `abilities::options_for` then answers with that one
tap's pips and nothing else. The sheet is a bubble, the card is untapped, and
the press that answers is what taps it, which is the bargain every bubble has
always struck. `Esc` is one step back to the sheet.

Such a bubble carries a **prefix**, and by one rule rather than a special
case: *a bubble whose pips all stand for one tap whose pour is not a number
says how many as well as which colour.* `abilities::bubble_prefix` is the
rule, and it covers the Druid alone — whose bubble opens straight off a click,
and which is the same question — as well as the Druid under a Guide. It is
`X×` and not the number: counting the Allies means evaluating a `Filter`
against a `PlayerView`, and no such evaluator exists this side of the wire,
while a wrong count drawn as a fact would be worse than the letter the card
itself prints.

The sheet is answerable without a pointer, which the old chooser was not: it
opened, took the keyboard hostage and let go of it only for `Esc`, while
confirm reached `Interaction::confirm` — which during priority means *pass* —
and the cursor keys walked the table behind it. It owns the keyboard while it
stands, the row the keyboard is on is drawn as the chosen one so the two ways
of answering are visibly the same sheet, and the cursor is **two-dimensional**
because this sheet is the only one with two directions on it: `W`/`S` walk the
whole column, `A`/`D` are the pip strip and arrive on it from a written row in
one press.

**It is anchored to the card, and a card stands in two different kinds of
place.** A permanent is a pose in the 3D scene and has to be *projected*
(`table::card_box` through a `Lens`); a card in the player's own hand is
`bevy_ui`, and the layout has already put its node somewhere — in physical
pixels, which `ComputedNode::inverse_scale_factor` is the way back from. Both
ends of `place_ability_sheet` reduce to the same pair, a centre and a box in
logical pixels, and `corner_for` takes it from there, so there is one rule
about where a sheet goes rather than two. The camera is asked for *inside* the
table's half: a sheet standing beside a card in the hand has no business
waiting for a rig that has not settled.

The hand half was a defect before it was groundwork.
`abilities::options_for` reads `LegalActions::abilities`, which names a source
and an index and says **nothing about the zone** — so a card in hand offering
two activated abilities could always open this sheet, and the placer looked for
it among the table's cards, found nothing, and hid it. What the player saw was
a sheet that never appeared. The query is `HandRowCard` and not the wider
`HandCardVisual`, which the stack panel puts on every row: a spell on the stack
is not a card in the hand.

**Anything that grows out of the hand is a sibling in the HUD root, never a
child of it.** `spawn_hand_zone` stands on `Overflow::clip()`, and a child
whose bottom edge sits a pixel under the parent's *top* edge and grows upward
is clipped to exactly that pixel — nothing about it looks misconfigured, the
panel is simply *gone*. That measurement is what moved the drawer out of the
zone in AX step 6a, and the chooser that stands beside a hand card is the same
trap a second time.

### The pool, and the land that always tapped for the wrong thing

Rule 2 above — the planner refuses restricted mana — is correct and was, for
one card, the entire user-visible bug. Jasmine Dragon Tea Shop prints *two*
mana abilities: `{T}: Add {C}`, and `{T}: Add one mana of any color`
spendable only on Allies. `manasources` reduces a permanent to the one tap it
can read, `simple_mana` refuses the restricted one on purpose, so the land
resolved to `{C}` — every time, whatever the player wanted, and the Ally
spell the land is in the deck *for* stayed unlit.

Three things were wrong and only one of them was the planner:

- The restricted ability **was** in the chooser and was labelled `{T}`, which
  is its activation cost and says nothing about what it makes. Beside a "Tap
  for C" that is two offers a player cannot tell apart.
  `baylee_cards_dsl::mana_made` is `simple_mana` without the restriction
  filter, and that filter is the whole difference between the two questions: a
  *planner* must refuse restricted mana, a *label* must not. Both readings
  stay in `manaread.rs`, because two copies of the rule would be two answers.
- The mana pool was **drawn nowhere**. It is the one zone with no card in it,
  which is why it had no place on screen — and with no place on screen there
  was no evidence a tap had done anything at all. It is now a bar at the
  prompt bar's height on the opposite side, one pip and a numeral per kind,
  visible whenever the seat has something to answer so it is a place a player
  learns rather than a badge that comes and goes.
- `ManaPoolView.restricted` was **one uncoloured total**, so even once drawn,
  naming white off a Cavern of Souls produced a number with no colour on it.
  `VIEW_VERSION` 14 makes it `[u16; 6]`, indexed as `ManaColor::index` indexes
  — the engine's `RestrictedMana` carried the colour all along. *What* the
  restriction permits stays an engine question, answered at the payment; which
  colour is under it is a fact the player chose a moment ago and is owed back.
  It is drawn as a rim round the pip rather than as a different symbol,
  because the mana *is* white and simply cannot pay for everything white pays
  for.

The count is always a numeral beside the pip, never a row of repeated pips:
colour alone must not carry meaning, and five discs is a number the player has
to stop and count. `baylee-client-core/src/manapool.rs` decides the row and is
tested without a renderer, like everything else in that crate.

None of this changes what the planner does, and that is the point — the player
can now do by hand the thing the planner is right to refuse, and watch it
happen.

## A choice you answer by picking, not by clicking the table

Four of the engine's seventeen pending choices name something that is not on
the table: a colour, a seat, one of several ways to cast a spell, and a
creature type. The prompt bar drew the headline for all four and nothing else
— no button, no hint — and `Interaction::choose_index`, which is how all four
are answered, was never called by anything in the client. A tapped Underground
Sea therefore printed **Choose a colour** and the game stopped there. For
good — in any deck with a dual land, the first time one was tapped for its
mana.

`crates/baylee-client/src/choices.rs` is the list, and it is a sibling of
`abilities.rs` for the same reason: the *label* needs mana symbols and seat
names, which `baylee-client-core` does not carry. It reads `Prompt`, never
`Pending` — the prompt already carries every option the engine offered, in the
engine's order, and **that order is the answer**, so nothing there may sort. A
colour is drawn as its mana pip and no word beside it, because a `{U}` disc
says "blue" in every language there is.

Picking sends. There is no second "OK" for these, because there is nothing to
combine — one colour, one seat, one way to cast — and the list is rebuilt from
the *current* prompt when the button is pressed, so a bar drawn a frame ago
answers nothing rather than the wrong thing. That is the same rule the ability
chooser follows, and for the same reason.

A creature type is the same list with a filter in front of it, because the
engine offers all three hundred and fifty and three hundred and fifty buttons
is not a chooser. A box takes what is typed, twelve matching rows are drawn,
the cursor walks them and Confirm takes the highlighted one. Cavern of Souls
is why this is not a nicety: it asks its question **as it enters**, so the
lock it caused was a land drop rather than a deliberate tap, and the card is
`Coverage::Implemented`, so the deckbuilder offers it.

The filter forces one thing that is easy to get wrong and silent when you do:
**a row's position stops being its answer.** Twelve rows out of three hundred
and fifty are on screen, so `ChoiceOption` carries the engine's own `index`,
the button is keyed on that, and both the overlay and the click handler get
their rows from the same `choices::options` call. A chooser that sent the
position would name the wrong creature type — only for players who typed, and
without ever erroring.

What is typed lives on `Duel`, not on the `Interaction`. The interaction is
rebuilt from scratch on every `HostMessage::Choice` (`lib.rs`), and a re-sent
snapshot — a print table earned, a seat reattaching — would empty the box under
the player's fingers. It is cleared when an action is sent and when a choice
arrives that is not asking for a type.

The keyboard is swallowed whole while the box is up, and that is not caution:
letters *are* chords. `W` walks the cursor and `E` activates a card, so a
player spelling "Elemental" would otherwise play half their turn. Only the
keys that mean something to a list survive. The gate also sits **before** the
`Fired::quiet()` early return, because a letter typed into a filter is usually
bound to no action at all — `Fired` is empty for exactly the keys the box cares
about most.

The half of this that has nothing to do with buttons: a choice answered by
**clicking** has to say so. "Discard 1 card(s)" stood alone at every cleanup —
no button, because nothing is submittable until a card is picked, and no hint,
because none existed. A player who did not already know to click their hand
had no way to find out. That sentence is twice history: the bracket in it is
gone as well, and §"The interface's own words" says why a counted sentence is
written twice instead. There is a line under the headline now, and the cards
the choice would accept glow, from `Interaction::is_selectable` rather than
from the board model: `selectable()` is empty for a discard *by design* (the
engine does not enumerate a seat's own hand, which is already private), so the
per-card question is the only one with an answer.

The reason this survived a green test suite is worth writing down.
`tests/duel_flow.rs` plays whole games through the client's own path and it
answered a colour with `PlayerAction::ChooseColor(options[0])` — built by
hand, in the test. So the suite proved that the *engine* accepts an answer,
never that anybody could give one. Those arms go through
`choices::options` → `choose_index` → `confirm` now, and
`a_dual_land_can_be_tapped_and_the_colour_answered` seats an Underground Sea,
taps it, and answers the question the way a click does.

What the tests do not cover is the row on screen, and no live game reached it
either: `manaplan` pays a dual's pip without ever asking, so the only route to
that question is tapping the land by hand. The row is `abilities.rs`'s chooser
node for node on the same prompt bar, and the test walks the identical path up
to the spawn — which is the claim, and all of it.

## Before the pointer speaks, the table has to be pickable

Bevy's picking has one pointer and several backends, and only the **UI** one
is installed by default. The hand bar, the prompt bar and every button are UI
nodes, so they answered; the felt is `Mesh3d`, and for it `Pointer<Over>` and
`Pointer<Click>` never fired at all. Not rarely — never. The consequence was
not a missing feature but three finished ones that nothing could reach:
`Interaction::activate` was written and wired to `input.rs` and still left a
Forest inert under the cursor, a permanent had no hover preview, and the piles
beside a mat could not be opened. All three came back with one line,
`MeshPickingPlugin`, and the `mesh_picking` feature it needs.

That line is easy to lose again, so it is worth knowing what holds it: the
path names the module the cargo feature gates, so a later
`default-features = false` audit that drops the feature breaks the build
rather than the table.

The other half of "pickable" is what a button is made of. A `Text` is a
`Node`, so a label inside a button is a pickable child standing in front of
it, and `PickingInteraction` stops at the letters: the button lit up under
the pointer in its padding and went dead across the middle. Every label
inside a control is `Pickable::IGNORE` — the lobby's `button` and `chip`
always did it, the phase rail and the prompt slip did not, and the seat tabs,
the menu buttons, the armed row and both choosers were caught by the same
sweep a milestone later. It is worth
measuring rather than reading, because the symptom is indistinguishable from
an animation that was never wired: the diff over the word was 0/0/0 and the
diff over the same button's padding was 155/156/148.

## The overlay lines up with itself

Every panel that floats over the table now measures from the same two
numbers, `hud::EDGE` and `hud::ABOVE_HAND`. Before them the tab bar held its
tabs 8 from the window's edge, the phase rail under it held its steps 10, and
the mana chip and the prompt slip stood 12 — three left edges down one side
of one screen. Vertically the chip cleared the hand bar by 10 and the slip
beside it by 12. Each was defensible on its own and wrong beside the others:
the eye reads a shared margin as a decision and four near-misses as an
accident.

Three of the pass's findings are worth keeping, because each is a shape that
recurs rather than a pixel that was off.

**A state-dependent border moves the row it is in.** `bevy_ui` adds a border
to an auto-sized node's box, so the seat tab's rim — one pixel at rest, two
when active or focused — made the tab whose turn it was two pixels taller and
wider than its neighbours, and the whole strip shifted sideways as the turn
passed. The rim is two pixels always and says what it has to say in *colour*:
`ACTIVE` for the turn, the seat's own identity colour for the focus, that
same colour at 40% alpha at rest. The priority caret is the same rule from
the other side — it is drawn whatever happens and merely goes `Color::NONE`,
because a marker that appeared and vanished shoved every seat beside it. It
stands in its own column rather than inline, which is what lets the tab's two
lines share a left edge; inline, the caret indented the name by its own
advance and left the counts hanging seventeen pixels to its left.

**The strip has to state its own height.** The tab bar was auto-sized, so a
tab that grew simply grew *under* the phase rail: measured on the running
client, the active tab's gold bottom border read (72, 55, 31) where its top
read (214, 163, 79) — the same gold seen through 88% black. `hand::TAB_H`
states it now, and the number has **no slack in it**: a tab is 2 + 2 of
border, 4 + 4 of padding and two line boxes of 16.8 and 13.2 with 2 between
them, which is 44, and the strip's own 6 above and below make exactly 56.
Confirmed on screen — the tab's top border is drawn at logical y 6 and the
bottom of its bottom border at 50.

That exactness is the point, and it has a consequence worth stating on its
own: **a third line in a seat tab does not fit, and does not fail loudly
either.** The commander-damage track (CR 903.10a) was one, and it appears
only once a commander has connected — so no ordinary game showed it. Forced
on and measured, the tab stood 58.5 tall, its top border cut off by the
window's edge and its bottom border drawn over the phase rail. It sits beside
the life total now, on the same row, where it costs width instead — the
cheaper of the two on a strip that states its height and does not state its
width, and where a *second life total* belongs anyway. That is not free
either: the bar neither wraps nor clips, so eight seats all carrying a track
widen every tab by about ninety pixels, which is a thing to measure when a
table that size is playable. `nothing_new_is_stacked_into_a_seat_tab` counts
the calls that stack a row into a tab and expects two, because the layout
that would prove it exists only inside a running renderer.

**`Feel` owns `BackgroundColor` every frame, so a fill it does not know about
is a fill that lasts one frame.** That is why the answer chooser's brass
highlight belongs in `HudRevision`: `chosen_index` was the one input the
revision never compared, so the highlight followed the pointer (which rebuilds
the tree) and not the keyboard (which does not). `every_field_of_the_revision_
is_both_compared_and_assigned` reads the struct's field names out of
`hud.rs` and checks each against both halves of `overlay.rs`, so the next
field added cannot be forgotten the same way.

The fill under a control that is *off* is `palette::SLIP_GHOST` and never
`Color::NONE`, which is not "no fill" on a node carrying a drop shadow — it
is a hole with the shadow visible through it.

## Every seat's bar

A seat used to be described twice from the top of the window: a tab in a strip
saying who it is, how much life it has and what is in its zones, and a rail
under that strip saying where the turn had got to. Between them they took 110
logical pixels off every window in every game to describe seats that were
already drawn on the table, each on its own mat. Both are gone. What they said
is written on the seat's own ground.

**The mat grew a shelf to write it on.** `tabletop::MAT_LEDGE` is a fourth
band along one long edge of the mat, 0.95 table units, *added* to
`layout::POD_DEPTH` rather than taken out of it — three lanes stay exactly a
card tall each, because a lane is where a card stands and the ledge is
furniture. Two compile-time assertions fence it: wider than four rims (or the
bar is written on its own border) and narrower than three quarters of a card
(or it is a fourth row). It is veiled at `MAT_LEDGE_VALUE = 0.0060`, one shade
*below* the quietest lane, so the ink on it is the brightest thing on a seat's
ground; its own boundary is a seam 1.5 times a lane seam, because that is
where a seat's ground stops being a place cards stand and becomes a shelf they
are described on. Measured through the local mat on a running client: rim
(164, 145, 110) → ledge (26, 55, 39) → ledge seam (64, 78, 70) → creature lane
(34, 58, 44). The composited contrast is 8.7:1 for the parchment ink and 4.5:1
for its glyph edges — WCAG's 7.0 for text and 3.0 for graphical objects, and
`the_ledge_is_dark_enough_to_read_ink_against` bounds it against a *measured*
felt of (21, 63, 40) rather than against the linear `FELT_*` constant, because
the shader's lamp and the sky's tint are multiplies `tabletop` cannot see.

**Which of the two long edges is one question with one answer.**

`layout::LEDGE_IS_OUTER` is `false`, at every seat: a seat's ink sits on the
edge of its own battlefield nearest the middle of the table — the edge *that
seat* reads as above its board. On this screen that means the local bar is at
the top of the local mat and an opponent's is at the **bottom** of theirs,
below their creatures, and the two face each other across the hearth.

That is the owner's decision and it is the mirror of what the table did
before. Which edge used to be *two* questions with `is_local` as the seam.
Every other seat's was the viewer's: `ledge_is_outer` fell through to
`facing.cos() < -SIDE_SEAT_TILT`, whose whole content is that a bar is drawn
above the board it describes *on the one screen there is* — so a seat across
the table took its outer edge, behind its land row. The local seat's was
pinned to its near edge, the bottom of the screen, on the argument that what
a player reads about themselves belongs between their board and their hand.
What the two have in common is where they put an opponent: beyond their far
rim, at the very top of the window, as far from their own creatures as the
mat allows. The seat it describes has to read it upside-down and across a
whole board; the viewer reads it nowhere near what it is about.

Before those two it *was* the centre-facing edge for every seat, and it was
given up because two bars ended up back-to-back across the middle of the
table with an opponent's sitting under their creatures. Both halves of that
have since stopped being true, and neither by accident. The ink is no longer
in the gap between the mats — it is on the ledge strip inside each seat's own
rim, which `lane_center` keeps clear of cards — and the gap it vacated is the
firewheel's, which is the one thing at this table that wants the middle.

`every_bar_is_written_between_its_own_board_and_the_hearth` measures it rather
than restating it: at every seat of every ring from two to eight the shelf is
forward of all three lane centres, and in a duel the two shelves are between
the two boards instead of outside them.
`no_card_reaches_the_band_its_seat_writes_on` is the other half and the one
that could quietly go wrong — the band takes `MAT_LEDGE` out of the mat's
depth and `lane_center` has to spend it at the *same* end, which it did not
have to while the ink was floating past the rim rather than standing on it.

**The lanes keep their order; only where they start follows the shelf.** The
three run from the centre-facing edge outwards at every seat — creatures
nearest the middle of the table, lands at the back — because that is where
the cards stand and a card does not turn round because the ink did. So
`MatParams::ledge_outer` is a flag rather than a flipped `uv.y`: flipping the
uv would move the lane veils with the shelf and put the brightest of the
three behind the lands. What the flag *does* change is `lane_center`'s
`front`: a shelf on the centre-facing edge starts the lanes a `MAT_LEDGE`
past it, and one on the near edge starts them at the edge itself and gives
the near strip up instead. Either way a lane is exactly as tall as it was
before the bar existed, and no card is drawn where the ink is. The flag now
has one value at every seat and the shader keeps both arms, because the band
is a real thing whose end the model chooses; what has gone is the choosing.
`seat_mat` takes the same flag and
`a_flipped_shelf_takes_the_other_end_and_leaves_the_lanes_alone` reads both
mats — it has to read the shelf's *fence* rather than its veil, because a
`Texture` is eight bits a channel and the shelf's 0.0060 and the quietest
lane's 0.0080 are the same 2/255.

**Three panels on one band, and one scale for all three.** `hud::seatbar`'s
`attached` module writes identity at the end of the band that projects
leftmost, public counts at the other, and the twelve phase steps between
them; `pose_on` is the whole of the arithmetic and takes nothing but the
projected `ledge_corners` and which panel is being asked about. The scale is
chosen once from the band and handed to all three, because each panel is
placed by its own call and a panel that sized itself would grow into its
neighbours at exactly the width where it matters — the middle one is the only
one that can meet another, and it can meet two.

The steps are **horizontal** and in the middle of that band. They used to be
a vertical column of twelve beside the command-zone rim, which is the one
place on the mat nothing else wants; the owner asked for them at the top and
in the middle, and the middle of the *table* is not available — a track laid
across the gap between two mats would be drawn over the firewheel. The middle
of a seat's own band is, and it puts the phase a seat is in on the same line
as its name and its life.

Both bounds are measured against the band as a **trapezoid** and not as the
rectangle its mean makes it look like: one end of a mat is further from the
camera than the other, and `every_panel_stays_on_its_own_seats_band` projects
each table for real and asks how far past its own band each panel corner is,
on the side it is on. It is tight rather than a formality — the closest any
corner comes is 13.3 px at a duel, 4.9 at three seats, 3.6 at six and 2.4 at
eight, all of them the inset the end panels are placed with, shrunk with the
scale.

**The bar cannot ask the camera where the shelf is.** `bevy_ui` orders
`UiSystems::Layout` `.before(TransformSystems::Propagate)`, so a system that
reads a camera's propagated `GlobalTransform` in `PostUpdate` and writes a
`Node` position is writing one the layout has already read past — a bar a
frame behind the camera, forever. `table::Lens` is the answer: it builds
`clip_from_world` from the same `CameraRig::eye` the camera itself is set
from, and `hud::measure_shelves` projects each mat's four `ledge_corners`
through it. `the_lens_and_the_written_out_projection_agree` checks it against
the hand-derived closed form already in `camera_tests`, under half a pixel at
every ledge corner of a four-seat table.

**A shelf is measured along its own axis, not by its bounding box.** A side
seat at a four-player table is turned ninety degrees to the camera, so the box
round its ledge reported 60 pixels of usable length where the ledge itself has
430. `hud::Shelf::of` takes the four corners and reads `along`, `depth` and
`tilt` off them, and the bar is rotated to match with
`UiTransform::from_rotation`. Rotation carries picking: Bevy 0.19's UI backend
tests the cursor with `UiGlobalTransform::inverse().transform_point2`.

**Four densities, and the fourth is not decoration.** The same shelf projects
1141 pixels at a duel and 151 on an eight-player ring.
`seatbar::Density::for_length` takes the densest form that fits — Full (924),
Compact (788), Pip (348), Mark (143) — where "fits" allows a quarter of
overhang for everything below Full, because a bar may hang a little past a mat
it belongs to but must not be wider than the seat beside it. Three densities
would have put a 348-pixel bar on a 310-pixel shelf at five seats. `Mark` is a
caret, the seat's colour and twelve 8×10 pips: where the game is, and whose
seat, and nothing else. What a density drops, the seat sheet carries on hover.
Two invariants hold the set together and are tested: a thinner form never
carries a cell a denser one drops (four sizes of one bar, not four designs),
and no form ever drops the caret, the colour or the steps.

**A fifth form, chosen on the other measurement.** `Density::Split` writes the
bar on two rows — the twelve steps alone along the mat's top edge, spread
across its whole width, and the seat's identity beneath them — which is what
the owner asked the phase line to be. It is not a rung of that ladder and
could not be: it asks for a *shorter* shelf than the full bar (585 against
948, because stacked rows are as long as the longer of them) and a *deeper*
one than any single-row form (34 of ink against 28). So `Density::for_shelf`
takes both numbers, and `Shelf::depth` — which the ladder never consulted and
`camera_tests` only asserted about — is finally what decides something.
Three consequences worth knowing. Its tiles **grow**: they have the row to
themselves, so `Density::tile_width_on` widens them to `tile_width_max` and
`SpaceBetween` puts the slack past that into the phase gaps, which is the only
ink on any bar that is not a fixed number of pixels — and it is allowed because
they grow *together*, so nothing on the row moves relative to anything else.
Its box is therefore measured from the **shelf** rather than from the form
(`Shelf::box_size`), and `SeatBar::placed` carries that width so a camera
dollying straight in — which changes a ledge's length without moving its
middle — does not leave the bar at the width it was born with. And the box's
*top* is the shelf's outer edge at every seat: `ledge_corners` winds the
rectangle from that edge inwards, and `Shelf::of`'s half-turn fold flips
exactly the seats that needed flipping, so the steps are the row furthest
from the board at a near seat, a far seat and a side seat alike.
It reaches a duel and stops there, and what stops it is **length**.
`a_duel_is_written_on_two_rows` is the test, and it reports both shelves
rather than the first, because the number that decides the mat's shelf is
the *shallower* of the two. Those two are about a tenth apart (55.0 against 61.1
at 1728), so there is a band of window sizes — roughly 1280 to 1366 at this
aspect — where a duel writes its local bar on two rows and its opponent's on
one. It was 1150 to 1250 until the identity row grew from 14 px to 18 and took
the form's ink from 34 to 40: fourteen made that row a *caption*, cutting the
name and the life total to 12 pt to keep their descenders inside, so the half
of the bar that says **who** was drawn smaller than the half that says *when*,
with no gap between them. Eleven more pixels of row cost four hundred pixels
of window, and `camera_tests::DUEL_BARS` carries the measurement either side
of it. That band is the per-seat answer the ladder already gives a four-seat
table,
where a side seat and the seat across get different forms; a duel showing two
is the same rule and not an exception to it.

Three seats and up have shelves that are deep enough and far too short: at
1728 they project 372×46, 337×44 and 337×44 against the 585 px the two-row
bar is wide, so the length ladder takes over untouched. Both halves of that
sentence used to read the other way — the depth was the thing that ran out —
and that was the shelf being measured a printed border short of the one the
mat draws. See "Where a bar is measured from", below.

### The twelve tiles stand in five phases

A turn has five phases and they have three, none, five, none and two steps
(CR 500.1, and CR 501.1 / CR 506.1 / CR 512.1 for the three that have any).
Twelve tiles at one spacing says the opposite — twelve equal parts — and on a
duel's 1165-pixel shelf, where the tiles reach their cap long before the ends,
the leftover went into eleven equal gaps and the row read as twelve scattered
pills. `automation::RailPhase` is the grouping, and the row is drawn as five
groups: `Density::tile_gap` inside a phase, the wider `Density::phase_gap`
between, and the slack goes into the four phase gaps and never into the seven
tight ones.

Four things fell out of that and each is worth its own sentence.

**A main phase is `MAIN_SPAN` step-widths wide.** It has no steps at all — it
is a whole phase standing where a step stands, and it is where every land,
every sorcery and most of the spells of a turn happen. It is also the one
thing the grouping could not fix on its own: four of the five groups are runs
of two, three and five tiles, and the two main phases are groups of one, which
at a step's width read as exactly the stranded pills the grouping was meant to
end. Only the split form makes the claim — on the ladder the strip's length
*is* the thing being fitted, and two double-width tiles would push every rung
up by four or five tile widths.

**The tile width is a division, not a `flex_grow`.** Growth with a cap works
for twelve siblings in one row and stops working the moment they stand in
groups: flex hands each *group* its share, so a phase of one tile and a phase
of five end up with tiles of different widths and the short groups keep a
pocket of dead space once their tiles hit the cap. Twelve tiles that no longer
agree on their width is the one thing this form promised not to do, so
`Density::tile_width_on` divides once and `phase_gap_on` says where the
remainder went. `a_split_row_spans_its_shelf_and_keeps_its_two_gaps_apart`
checks both at every length the form is chosen at.

**The hinge closes the identity row.** It used to head the steps row, which is
where a hinge belongs on a bar written on one line; on two it made both rows
worse — the tiles began a turn-number's width in from the shelf's edge, and
the identity row ran out after the counts with four fifths of itself empty.
The turn number now sits at the far end of the row beneath the tiles, which
anchors that row at both ends and gives the twelve tiles the whole ledge —
and the four zone counts travel with it, so the row is a **nameplate at one
end and a tally at the other**: caret, colour, name and life on the left,
hand, library, graveyard, exile and the turn number on the right, with the
empty stretch in the middle where the eye passes over it. The strut that used
to sit in front of the hinge alone now sits in front of the first count.
The two rows are not aligned to one another's grid and must not be: the
tiles' widths follow the shelf while these cells are fixed by rule, and a
life total standing under "combat" reads as being *about* combat.

**The tiles follow the shelf, and the gap between phases has a floor.** A bar
is rebuilt when its *density* changes and re-placed every frame, which is
right for a box and was wrong for the one ink inside it whose width is not a
fixed number of pixels: the twelve tiles kept the width the mat's shelf
projected at the moment the tree was built, and a camera still easing towards its home —
every duel, for the first second of it — then left the box on the whole shelf
and the tiles a tenth short of it, with `SpaceBetween` quietly spending the
difference on the phase gaps. Photographed at 1728×1052: 66 px tiles and 45 px
phase gaps where the model says 72 and 24.5, on a shelf the bar had already
been told was 1127 long. Nothing looked broken — the bar still spanned its
ledge, in the wrong proportions — which is why it took a row profile through
the tiles to find. `hud::stretch_step_tiles` is the other half of
`place_seat_bars` and is guarded the same way, on the width already in the
node. The same photograph then showed the *far* seat, whose shorter shelf
(1069 against 1127) leaves four pixels over once its tiles reach their cap:
its phases stood ten pixels apart against three inside them, which is a
division a viewer has to look for. `PHASE_GAP` for the split form is five
times the tile gap now rather than three — a floor only this form reaches,
and one the near seat never notices, since its slack put it at 24.5 either
way.

**A step tile is ink on the shelf, not a chip on it**, and the bar shipped
with that hierarchy upside down. A *skip* is what most steps are, and the
skip wore `DANGER`, so the alarm colour was painted on the ordinary case
while the deliberate one got a quiet parchment frame; every live tile was
framed and filled either way, which is twelve stadiums across a 1127 px
shelf. It also made the bar opaque, and the mat's shelf is crossed by things
the table draws — a combat line to the far seat, a card lifting under the
pointer, a permanent falling in from `ENTRANCE_RISE`. Twelve solid chips
floating over all of them is most of what "it floats over stuff" was.

So the rare state is the marked one:

- **The ground is the standing order.** A stop is `PARCHMENT` at 0.14 with
  its glyph at full ink (6.99:1 on that ground); a skip is ink at half alpha
  on bare cloth (3.75:1 over the measured shelf, which is above the 3:1 a
  graphical object needs and below what a paragraph wants — right for a label
  nobody is being asked to read). A *luminance* difference rather than a hue
  one, which is what a green felt makes of any attempt to say go/stop in
  colour.
- **A solid fill is "here, now"**, on the active seat's bar only, with
  `PARCHMENT_INK` on it — the same solid-warm-with-dark-ink the prompt slip's
  own button uses. The two-ring halo went with it: the rings existed because
  a shadow drawn through a ten-per-cent fill lit the whole tile, and a solid
  fill has no such problem. `HALO_OUT` stays as the offset of the *under-tick*
  that marks the game standing in a step that is not a control.
- **A frame is keyboard focus and nothing else**, so it appears exactly when
  a control is being operated.
- **A dead step** (untap, cleanup — `RailRow::grants_priority`, CR 502.4 and
  CR 514.3a) is the dimmest ink there is, keeps no ground at all and is
  `Pickable::IGNORE`. Its 4% ground was added when it was the only bare word
  on a row of chips; the whole row is bare words now.
- The tile's corner is 3 px rather than `btn_radius`'s 6: on a 16 px tile six
  is a stadium, and a stadium is the browser chip the row was being read as.

**Time as a fourth channel is dropped.** Position already carries it — the row
reads left to right at every seat, and the gold tile says where the game is,
so "behind" is "left of the gold one". Keeping it collided with the skip
alpha: a skipped step ahead and a stop behind would both have been half-lit.

`Feel` had to learn a second colour for this. It mixes towards white and
*keeps the alpha*, so a tile resting at `Color::NONE` was lifted to a
brighter nothing and never answered the pointer at all; `Feel::rising_to`
states the hot end, and what a skipped tile's hover shows is the ground a
click would give it.

The bar is its **own retained tree**, `SeatBarRoot`, a sibling of `HudRoot`
with its own `BarRevision`. `HudRevision` carries `hovered`, so the overlay
tree is rebuilt on every pointer move; a bar that rebuilt with it would be
rebuilt some hundreds of times a turn. `place_seat_bars` is guarded the same
way `apply_camera_rig` is — it stores the corner and tilt it last wrote and
skips the write when they have not changed, because a `Mut<Node>` marks the
node changed on any write and taffy would relay out every bar every frame
while the camera stood still.

Being a sibling root is also what makes its depth a `GlobalZIndex(-1)` rather
than a `ZIndex(0)`, and the distinction is not pedantry: **`ZIndex` orders a
node only among its own parent's children**, so a number on one tree's root
and a number on another's are never compared at all — two roots both at zero
are tied, and the tie is broken by whichever was rebuilt last. The bar carried
`ZIndex(0)` with a comment naming the overlay's own 1/2/3/10 as what stands
over it, and every one of those numbers is inside `HudRoot`, a different
stacking context. The zone browser is what found it: the local seat's phase
tiles were being drawn straight through an opaque dialog.

**Both rows of standing orders are seen at once in the settings screen**, and
that is where they belong. A seat bar carries the twelve steps of a turn but
only the row that turn belongs to — an order about opponents' turns is
invisible on your own bar until an opponent is taking one — and a player
arranging stops wants the whole arrangement in front of them. `PhaseOrders` is
keyed by `RailSide` and not by seat, so one tile on one seat's bar sets an
order every other seat's bar then draws.

### Who is answering for a chair

A seat is one of three things, and until 19.09.2026 no table could tell them
apart: a player answering for themselves, the house playing that chair by
arrangement, and a player's chair the house is **holding** because nobody is on
the other end of it. `SeatIdentity` keeps `is_ai` and `away` as two fields for
a reason worth restating — a chair held for thirty seconds must not rename
itself to the house, or it would still be saying so after the player came back
— and its own rule is that the two are never both set.

`board::SeatRole` is those three as one enum, which is the collapse and not a
convenience: a pair of bools on a `SeatPod` is a shape with a fourth state
nothing can produce and every reader has to decide what to do about.
`SeatRole::of` asks them as an ordered pair rather than matching on both, so a
roster that broke its own rule gives one of the three answers — `Away`, which
is the more urgent and the only one that can stop being true. It comes off the
**roster** and not the view, because a held chair still has its player's life,
their hand and their name, and it lives on the pod because three surfaces ask
it: the bar, the mat's rim, and the seat sheet after them.

**The role is said as a mark in the name cell, and that is arithmetic rather
than taste.** `docs/game-log-design.md` §"The seat that stepped away" asked for
a `MUTED` role line under the name, written for a strip of seat tabs that no
longer exists. The identity plaque that replaced it is `HEADER_H` = 60 logical
pixels and its two rows already spend 56 of them, and the plaque's size is what
`Panel::Identity` is fitted to the mat's band with — so a third row is a taller
plaque is a band fit that two tests hold. A mark costs nothing instead: the
name cell is a `cell_node`, a fixed width that does not shrink, so everything
inside it spends the name's room and never the bar's. That is the caret rule
one level down.

The two states are drawn as two different things, because they are different
in kind. A chair the house plays is **named** the house, in the player's own
language — off the flag and never by matching the string, which is safe for a
plain reason: neither `LocalHost` nor the gateway has any other name for such
a chair, both write the literal `"House AI"`, so there is no host-chosen name
to hide and only an English word a German player was being shown. A chair that
is merely held keeps its player's name and wears a **clock**, which says "for
a while" without any words — the caveat the design's sentence carried. At
`Density::Mark` there is no name cell at all, and the rim is the whole answer;
that density's own contract is that framing the seat widens it back to a bar
that can talk.

Two things decide whether any of it is ever seen, and both were wrong before.
A flex item's `min-width` is `auto`, which for a `NoWrap` text resolves to the
whole string — so a name long enough to fill the cell refuses to shrink and
anything after it is pushed past the `clip_x` edge. The mark had been in this
tree since the bar was written and was invisible for exactly the names that
matter; the name now carries `min_width: 0` and the mark `flex_shrink: 0`.
That claim is asserted on the two `Node`s rather than on a measured layout,
because nothing in this repository runs `bevy_ui`'s layout in a test — every
other `ComputedNode` here is hand-written by the test that reads it.

And `BarRevision` gained the roles, because a roster is **its own payload**.
The host marks every seat's roster stale when a chair changes hands and the
fresh `GameStatic` arrives as a separate message, so a bar gated on the view's
`seq` alone is built from whichever roster was in hand and goes on saying so.
The direction that matters is the second one: a clock arriving late is a
nuisance, and a clock still there after the player has returned is the bar
telling the table something untrue. The revision is now compared and assigned
**whole** rather than field by field, which is what makes the next field
added to it a compile error instead of a gate that quietly stops watching one
of seven things.

#### The rim says it where the bar cannot

`Density::Mark` is 151 projected pixels of shelf and has no name cell in it, so
the mark above has nowhere to stand — and that is the density an eight-seat
ring puts every chair at, which is exactly the table where a player is most
likely to have gone. The mat's rim is the surface left. `MatParams::held` is
the flag, `Mood::held` carries it, and `SeatRole::Away` is what sets it:
`House` does not, because an AI chair was always an AI chair and there is
nothing provisional about it.

**It is a dash and not a dimmer**, and that is forced rather than chosen.
`table::zone_brightness` already spends the whole range 0.22 to 1.0 on what a
seat is *doing* — 0.22 for a seat that has lost, 0.62 for one waiting, 1.0 for
the seat being asked — so a held chair drawn quieter would land inside that
ladder and be read as a seat losing interest. `tabletop::rim_dash` is a gain
of mean **exactly** one instead: `1 + 0.55·cos(24·2π·turn)`, a whole number of
periods so the pattern closes across the seam `atan2` leaves at the mat's
right-hand edge, and a cosine rather than a square wave because a hard edge 17
pixels away aliases and crawls as the camera settles. The exactness is what
picking a cosine bought — the mean needs no tolerance argued about, and the
test is an integral rather than a threshold. Twenty-four is a length: the mats
this table draws run about 2800 screen pixels round at a duel and about 400 on
an eight-seat ring, so a dash every 117 px at the near end and every 17 at the
far one.

**The gain multiplies the whole rim signal — `border·RIM_LIGHT + running` —
and not the border alone**, which is a measurement and not a preference. Both
were built, and the second draws *nothing*. `value` is clamped at 1, and for a
seat on turn **and** being asked the undashed rim already reaches 1.98 at its
outer edge and is still at 1.16 half way in. Modelled over the rim's width at
the swell's crest, the contrast a gain on the border alone produces is
**0.000 across three quarters of it** — the crest and the trough both flatten
against the same ceiling — against 0.10 at the outer edge rising to 0.62
deeper in for the whole signal. A held chair is often exactly that chair,
because the house answers the moment it is asked. Taking the running light
down with the seat's own rim also says the right thing: a held chair's turn
and a held chair's question are the house's, so the whole rim is provisional.
The **hue** is untouched, so the mat still says which seat it is.

Two consequences of that ceiling are worth having in writing, because neither
is visible from the tests. Where the rim saturates the gain has **no crests**,
only troughs: the mark is read entirely as gaps cut into a rim that is already
at full, and the "mean exactly one" property is what the *base* rim gets —
which is all it has to be, since what that property is for is keeping the mark
out of the brightness ladder. And where there is no turn light at all — a held
seat simply waiting — the two applications are **identical**, at 0.68 contrast.
That is precisely the case `tabletop::seat_mat` models and the pixel tests
measure, so the suite is silent on the one configuration the choice was made
for. It was settled by a photograph instead, and the photograph is below.

##### What the shader actually draws

Taken through `dev-control` against an offline duel, with a throwaway probe
holding the far chair — once idle, once forced on turn **and** being asked so
the rim saturates. Read as the **periodic component at the dash's own
frequency** and not as a row's peak-to-peak, which is the trap here: the felt
under a mat carries lava veins that swing any row's min and max by more than
the mark does, so a naive contrast number measures the table rather than the
rim. Six dash periods fall across the sampled span of the far mat's edge.

- an **unheld** rim is flat at *every* frequency — amplitude 0.00, which is a
  zero that could have been otherwise and is what makes the rest a comparison;
- **held and idle**: k=6, amplitude 19.6 levels on a mean of 89, 3.2× the next
  strongest component;
- **held, on turn and asked**: the same k=6, amplitude 7.0 on a mean of 148,
  2.2× the next.

Two things the picture settled that the arithmetic only predicted. The first
is that it survives the size it is for, and that was **rendered rather than
resampled** — the first answer here was a Lanczos downsample of the duel, and
a filter over finished pixels is the one thing that cannot answer "does a
high-frequency pattern survive being made small", because the GPU's own
minification is what is being asked about. So: an offline table for eight,
every shelf reporting `Density::Mark` at 160–178 logical pixels, photographed
twice at the same seat with one variable changed —

| seat 4's rim, 380 px | mean | strongest period | ratio to background |
|---|---|---|---|
| shipped code, no chair held | 72.3 | k=4 at **0.67** | 1.07 |
| the same seat, chair held | 71.0 | k=7 at **16.50** | **4.13** |

which says three things at once. The dash survives real minification with a
quarter of its brightness as modulation. An unheld rim of the *same* mat at
the *same* brightness has no periodic structure at all — 0.67 against a
background of 0.62 is nothing, and that is the counter-test the duel could not
give, because there the only unheld mat was the local seat's and its rim is
clipped at its ceiling, which would read flat whatever was done to it. And the
**mean survives**: 72.3 against 71.0, 1.8% apart, which is the mean-of-one
property holding in the shader at the density it is for rather than only in
the generator a test can measure.

And the mark is
**weakest at the rim's outer contour** — 33 levels across the outermost bright
row against about 62 two rows in — so where the rim saturates it reads as a
scalloping of the glow rather than as breaks in the silhouette, and the
silhouette is what is read from across the table.

The cause of that is the **clamp and not `MAT_RIM_FALL`**, which was the wrong
diagnosis offered before the photograph: at the outer edge the crest is cut
off at 1 while the trough is not, so only half the gain survives there.

It is left as it is, and that is a deferral rather than a settlement — **#95**
holds it. Seven levels of modulation is under the twenty a still mark on this
table already swings from its own ink, so in the saturated case the contour is
at or below the noise floor and only the coherence of the pattern carries it.
Whether that is good enough is a thing to be *looked at* by whoever is sitting
at an eight-seat table, not decided from a duel's screenshot. Deepening the gain would clip the *base* rim too and cost the
mean-one property, which is the thing keeping the mark out of the brightness
ladder; dashing `running` harder than `border` is the change that would buy
the contour back, and it is a second constant nobody has needed yet. The
saturated case is also the rarer one for a held chair, because the house
answers the moment it is asked.

Measured on a 512 × 196 mat: along a straight run of rim an ordinary mat is
flat to the last bit, and a held one swings 0.141 to 0.475 about the 0.310 the
plain one holds. Over the whole mat the net comes to 1.3% of the light the
gain moves about rather than 0%, and that is geometry — `turn` is an angle
normalised by the mat's aspect, not an arc length, so the corners weigh a
little differently from the sides. `tabletop::rim_dash` is the arithmetic in
Rust and `mat.wgsl` spells the same one; `camera_tests::the_shader_and_the_
generator_agree_about_the_mat` carries both constants, and
`the_mat_shader_compiles` is what stops a typo in it becoming a mat that
simply does not draw.

`held` rides on `Mood` and is not passed beside it, which is the half that
would have been easy to lose. `sync_zones` skips a seat whose `mood` and
`accent` are both unchanged, so a flag outside that struct is written once
when the mat is built and never again — a chair going to the house would keep
a solid rim until something else about the seat happened to move. That is
`BarRevision`'s finding one layer down, and it is asserted as two moods that
differ rather than taken on the reading.

One trap found on the way, and it is #91: `PlayerView::object` answers for the
battlefield, the stack, graveyards, exile, the command zone and `looking_at` —
and **not** the hand, which is a `Vec<HandObject>` of a different type
entirely. It is a type split rather than a forgotten zone, so it will be found
again by the next panel that asks a question about a card in hand;
`face::face_name` is the door that searches both.

### Where a bar is measured from

A mat is drawn larger than the board it carries: `tabletop::MAT_MARGIN` past
the playing extent on all four sides, a printed border like the one round a
real playmat. That constant used to be `table::ZONE_MARGIN` and lived in the
renderer, where nothing else could see it — and everything else about a mat
is a *fraction* of its depth. So the shelf and the three lanes were laid out
over `layout::POD_DEPTH` and painted over `POD_DEPTH + 2·MAT_MARGIN`: every
band stretched by 18.5%, the shelf 0.46 units from where the geometry had
reserved it, the two lane seams 0.06 and 0.25 units off the rows of cards
they fence.

Nothing failed. The bar followed `SeatSlot::ledge_corners` to the pixel and
`ledge_corners` was not describing the band on screen, which is a defect no
screenshot can name and no assertion about the drawing can catch. What found
it was `/state.shelves` against a photograph: the model reported `mid_y` 605.9
over a 49.0 px shelf, the picture put the drawn ledge at 560..611 and the ink
at 591..620. Ink where the model says, model where the mat does not — so the
projection was the half that was wrong, and the seat's identity row was
standing on its own creature lane.

There is one rectangle now. `MAT_MARGIN` is in `client-core::tabletop`,
`LEDGE_FRAC`, `LANE_FRAC` and `MARGIN_FRAC` are fractions of
`MAT_DRAWN_DEPTH`, and they sum to 1 under a `const _` — the shelf, three
lanes and the border beyond the last of them are the whole mat, so a band
left over is a band in the wrong place. The border at the shelf's own end is
part of the shelf: nothing stands on either, they are contiguous, and the only
other reading paints a stripe of creature lane outside the shelf at the very
edge of the mat, which says a card could stand there. `ledge_corners` returns
that same rectangle, `MAT_MARGIN + MAT_LEDGE` deep. Its *length* still stops
at the playing extent, deliberately — the border is a margin for the ink to
stop inside, and a bar written out to the corner would be written across the
rim that carries the seat's colour.

`tabletop::the_mat_fences_its_bands_where_the_layout_put_them` is what would
have caught it, and it reads the fences out of the **texture** rather than out
of the constants the texture was built from: a number written down twice is
the whole of the fault, so asking one copy whether it equals itself proves
nothing. Its mutant is the old lane arithmetic — split what the shelf leaves
in three instead of taking `SeatSlot::lane_height` — which moves the first
seam 0.06 units and fails it.

It sweeps **both** mats, because the shelf changes ends and the lanes do not:
a near seat spends a border *and* a shelf before its creature row starts, a
far seat spends only the border and meets its shelf at the other end. Sampling
one of them is what let a second mutant through — reading the lanes' start as
a plain `LEDGE_FRAC`-or-zero, which is algebraically the same thing on a near
mat and half a card out on a far one. The far mat's branch exists in the
shader too, where no unit test reaches it, so it was read off the running
client instead: `/state.shelves` puts the opponent's shelf at 105.4..162.2
logical, and a column through the photograph finds the mat's rim at 106 and
the ledge fence at 162, the two lane seams following at 224 and 286 against
222.5 and 283 predicted from the ledge's own scale.

The bar the shelf can hold changed with it, and in the direction the owner
asked for: a duel's local shelf projects 61.1 px deep at 1728 where it read
40.3, so the two-row bar now reaches any laptop rather than only a wide
window. `MAT_LEDGE` was moved from 0.95 to 1.00 to buy that bar under the old
reading and would probably not have needed to be; it is left where it is,
because the tables were tuned there and a shelf's depth is a look rather than
an arithmetic.

## The bar's hinge says which turn and what the game is

The day/night designation (CR 731) is drawn beside the turn number, on the
hinge between a seat's counts and its steps. It is not drawn
at all when the game has neither designation, which is every game with no
daybound card in it, and no slot is held for one: a game that has become day
or night has exactly one of the two from that point forward (CR 730.1), so
the block appears once and its arrival *is* the announcement — which is why
the hinge *widens* when it comes rather than reserving room for it. The caret
rule above — draw it always and let it go `Color::NONE` — is for a marker that
toggles, and this one never does.

The two shapes below were photographed while this stood in the phase rail's
head. The rail is gone; the lessons are about drawing two things side by side
and about a fill needing something to be a fill against, and they moved with
the block.

Two shapes came out of photographing it, and both are the same lesson from
opposite sides.

**Two things side by side have to be the same size, and content will not make
them so.** Built as a column — the glyph over its word — the block stood 30.5
logical tall next to a turn number of 19.5, so the head read as two objects
rather than one line. Laid out as a row both are `rail::HEAD_H`, measured
identical at logical 73.0 to 92.5. The turn number carries a one-pixel
`Color::NONE` border it has no use for, because the designation needs one for
its flash to write into and a border is layout: without it the two would
differ by exactly two pixels forever.

**A fill is only a fill against something.** Night was drawn on
`palette::PANEL` to sit a shade below the turn number, which is true — but
the rail underneath is `PANEL` too, so the pill measured (13, 15, 21) on a
(12, 14, 20) strip and was not there: a pill by day, a glyph floating beside
one by night. The two states differ by what the block *says* — a sun in
`PARCHMENT`, a moon in `INK`, from `seatbar::designation_of`. Neither is
`ACTIVE`, which lights the current step a few cells to the right; a
designation wearing it would read as a step the game was in. The untap step
gave up the sun for a rotate-back arrow on the way past, because two suns on
one bar would have said the untap step is the daytime, and
`the_designation_does_not_borrow_a_step_glyph` reads both files to hold it.

The change is marked by a flash on the block's border and shadow, and it is
anchored to the **change** rather than to the entity. `PhaseNow` can ease
from zero at spawn because the tree is rebuilt when the step changes; the
overlay tree is rebuilt on every hover, so a light born with the block would
fire whenever the pointer crossed a card. `rail::DesignationFlash` keeps the
last designation it saw beside the clock, and
`a_rebuild_does_not_restart_the_flash` despawns and respawns the block
mid-decay to prove it.

Both of those systems are **waiting** as this is written. `light_the_current_
step` and `flash_the_designation` outlived the rail because what they know is
not the rail's shape; nothing spawns a `PhaseNow` or a `Designation` between
the commit that retired it and the one that gives the bar its baton and its
hinge-light, so both run over an empty query. Their tests spawn the markers by
hand and still hold what the systems are for.

## Two families

The interface is set in **Alegreya Sans** and what a card *says* is set in
**Faustina**, and the split is the point: one face separated by size gave a
card's printed text and the button beside it the same voice, and a card's
rules text is a **quotation**. `hud::UiFonts` carries seven files — five
static Alegreya Sans cuts (Regular, Medium, Bold, and the italic of the first
two) and Faustina upright and italic, variable on `wght` 300–800 — beside the
icon and mana faces. This **overrides `docs/design.md` §1.2**, which shipped
three cuts of Inter and said there was no fourth; the override is recorded
there.

The Bold is the newest of them and it draws one line: **a word a player can
press is set in Bold, and a word a player can only read is not.** So the
label on a button, a chip, a tray tab, an answer on the prompt slip, a phase
tile and the digit in an ability row's keycap are bold; the ability's own
printed sentence beside that digit, a card's name in a browser row and the
slip's prose are not — a sentence *quoted* inside a control is still a
quotation, which is why the ability chooser's `{T}: Add {G}` stays as it was
while the menu button's "Play {0}" beside it does not. `hud::tf_bold` is the
one door, and two more exist for exactly the reason it does:
`manaui::spawn_rich_label` and the tray's `dialog_label`, each a second
function rather than a `bool` on an existing one, because a flag in that
position is a thing to get backwards and both of those functions have callers
on **both** sides of the line.

It has to be a file. `TextFont::weight` reaches a variable font and these
cuts are static, so a bold label is a bold `.ttf` or it is nothing — and
Medium was already spent on the small sizes, where it is the reading weight
rather than emphasis. Three tests hold the asset itself
(`hud::tests::faces`): every face `setup_fonts` names is a file in the tree,
the bold cut's `OS/2.usWeightClass` really is 700, and nothing sits under
`assets/fonts/` that the client does not name — the last because
`AssetServer::load` is lazy and infallible, so a renamed `.ttf` is an
interface drawn in nothing and no test that says so.

No width estimate moved with it. Bold measures **0.4610** of the em on lower
case against the Regular's 0.4453 — 3.5% wider, read out of the shipped files
— and `stack::CHAR_WIDTH` budgets card *names*, which are never a control's
label.

Two scales carry the whole change, and the reason they are scales is the
reason they exist at all. Every size in this client was chosen against
Inter's x-height of 0.546 em. Alegreya Sans is authored at 0.458 and Faustina
at 0.494, so at the same nominal number the interface would read about two
steps smaller — `hud::UI_SCALE` (1.2) and `hud::SERIF_SCALE` (1.1) multiply a
caller's nominal size on the way into `TextFont`, and three hundred call
sites keep the numbers they had. 0.494 × 1.1 is 0.543, which is Inter's own
to three places.

The second effect is what makes it safe. `stack::CHAR_WIDTH` budgets a
character at 0.52 of the nominal size, measured on Inter's 0.531 mean
lower-case advance. Alegreya Sans measures 0.445 of the size it is *rendered*
at and Faustina 0.473 — 0.534 and 0.520 against the nominal, which brackets
the same 0.52. Every width estimate in the client therefore survived the
change of family untouched.

What did **not** survive is every number a font produced, and those were
re-measured one at a time rather than assumed: the verdict sheet's longest
line (585 px, was 598 claimed against Inter's real 561), the stack panel's
widest name (`Okina, Temple to the Grandfathers`, 202 px against 245), the
tray's zone badge (`Kommandozone`, 70.0 px against 68.8 — one clipped letter
if it had been left), and the mana mark's 0.72, which lands within a pixel of
the prose's cap in both faces by luck and is documented as luck so a third
face is measured rather than assumed to inherit it.

Three smaller decisions are in `hud.rs` and are easy to lose:

- **`SMALL_TEXT` (14 px) switches cut, not weight.** Alegreya Sans has a
  light Regular whose stems go grey under a 12 px raster, so anything below
  14 asks for the Medium *file*. `TextFont::weight` cannot do this job — it
  only reaches a variable font, and these four cuts are static.
- **Lining figures, always.** Alegreya Sans defaults to old-style figures,
  whose 3, 4, 7 and 9 hang below the baseline. That is right in a paragraph
  and wrong in a life total, a mana value, a turn number and a power. Every
  `tf` sets `lnum`.
- **`hud::bleed` is the ink halo and it answers `None` under 12 px.** A pen
  set down on parchment spreads, and the halo is that spread — but below 12
  px a second coloured copy of a stem *is* the stem, so the halo would be a
  blur rather than a bleed. Faustina's `wght` 500 (`hud::INK_WEIGHT`) is the
  other half of the same idea, and it works because Faustina is variable
  where the sans is not.

## The prompt slip is a sheet, and a sheet is a child

`hud::sheet()` inserted on a panel paints that panel's **content box**, so
any panel with padding drew parchment in the middle and flat
`palette::PARCHMENT` in a ring around it, with the sheet's own corners cut
inside the panel's. `hud::sheet_surface()` is the parchment as an
absolutely-positioned first child instead — an absolute child is measured
against its parent's *padding* box, which is exactly the ring that was
missing — carrying `Pickable::IGNORE` and a radius one pixel tighter than
the panel's so the two curves are concentric. The prompt slip and the ability
sheet go through it; the zone browser did too, until §1.3 made it a panel.

The slip's prose is set in **Faustina Italic**, which is a second font file
and has to be: Faustina is variable on `wght` alone, `TextFont` in Bevy 0.19
does carry a `style` field, and nothing synthesises an oblique from an
upright — so a slant this client cannot ask for is a slant it has to ship.
(It was Inter Italic for the same reason, one file down.) Bracketed
asides are greyed, and which stretches those are is
`baylee_client_core::prose::bracketed` — in the model, with a test, and
deliberately refusing to grey an **unclosed** bracket, because one stray
character must not drain the rest of a line.

The answers underneath share the sheet's width: `flex_grow: 1.0` with a
`flex_basis` of **zero**, since grow alone divides only the slack left over
after the labels and three answers with three different words would still
come out three different widths.

Four decisions make the slip's prose what it is, and only one of them is
about the slip. Ink with a little parchment showing through, the faint warm
shadow a letter lying on a sheet casts, and a grey for whatever the line says
in brackets are all about the *parchment*; the slant is the slip's own voice,
which is that of a question being asked. `slip_line` is
`slip_text(.., italic)` and only the slip is italic, so there is one
treatment rather than two that drift apart the first time either is adjusted.

Brass does not appear as **text** on parchment anywhere: measured, it carries
1.9:1 against `PARCHMENT` (`SLIP_ASIDE` carries 4.9, `PARCHMENT_INK` 13.7),
which is why the zone browser's current tab read fainter than the ones beside
it back when the browser was parchment too. It keeps its job as a *light* —
the card glow, the ordering badge — where it sits on its own fill.
`the_parchment_writes_no_letters_in_brass` holds it, over both parchment
surfaces.

## The end of a game is a sheet, not a line in the bar

A finished game used to be said the way everything else is said: one line in
the prompt bar at thirteen pixels, with the board still being drawn
underneath and two buttons floating over it sixty-four pixels from the top of
the window. But a result is not a prompt — there is nothing to answer — so
the bar now goes quiet the moment `Duel::ending()` has something in it, and
the verdict gets the sheet the prompt slip grows into.

`hud::finish` is **spawned once, on `OnEnter(DuelPhase::Finished)`**, and that
is the one thing about it that is different from every other tree here.
Everything else on this overlay is retained behind a revision because
everything else changes — a hover, a card drawn, a life total.
`Pending::GameOver` is the last thing the engine ever says at this table, so
there is nothing to synchronise: it is built on the edge, taken down on the
way out, and never touched in between.

**Won, lost and drawn are the same sheet.** Every hue in this palette already
carries a claim — `INK_DANGER` is damage, `HEAL` is life gained, `BRASS` is a
thing already taken, `CANDLE` is an offer — so a red loss would read as
damage and a gold win as something taken, and `docs/redesign-proposal.md` §1
retires hues rather than handing out new ones. Only the sentence differs.
Size carries the feeling instead: **forty pixels**, against the twenty this
overlay had never gone above.

The width is measured rather than chosen. `Das Spiel endet unentschieden` is
the longest verdict there is, and it sets **585 px** in the shipped
`Faustina.ttf` at `VERDICT_PT` times `SERIF_SCALE` on the `wght` 600 axis —
so the sheet is 720, whose inner 638 leaves it fifty pixels of air. (Inter
set the same line at 561 px. The serif is the wider face here by 4.4%,
because it is drawn 10% larger to match Inter's x-height and does not give
all of that back in the advances — a cost of the change, recorded as one.) A headline that shrank to fit its own sentence would
be saying that sentence matters less.

### The verdict says who, the line under it says how

`interaction::verdict` was already there and already read the winner off the
prompt; `interaction::ending_reason` is its second half and answers *how* the
game was decided — one line per `EndReason`, and `None` for a draw, whose
only available reason is the verdict's own sentence written twice.

It takes **no seat**. The winner and everyone who lost read the same second
line, so "every opponent has left the game" would be true from exactly one
chair at the table and false from the others; the reason is a fact about the
table.

What a player actually wants after a loss — zero life, an empty library, ten
poison — is **not in that line**, because it is not about the table: it is
one seat's, and the view carries it per seat (`SeatView::loss`, with
`SeatView::house_answered` telling a loss to the clock from one played out;
#83). Deriving it from the last view's life totals instead would be the
client deciding a rules fact, which is the line `CLAUDE.md` draws.

### Under both, why each seat went out (#83)

`interaction::loss_lines` says one seat's loss: a line per `LossCause`, in
the second person to the seat that lost ("Your life fell to 0 or less") and
naming it to anyone else ("Bob#1a2b conceded"). Where the view records that
the house answered the seat's last decision it adds a second line: "Your
time ran out, and the house answered your last decision" for
`HouseAnswer::Clock`, "You were not connected, …" for `StandIn`. Together
the two are a game lost to the clock, the reading `docs/protocol.md` §"Why a
seat lost, and who answered for it (#83)" gives: `house_answered` freezes at
the loss, so it names the last decision before the seat went out. A seat
still in the game gets nothing, whoever answered for it.

Every line is in the past tense and claims only what the view records: the
rule that took the seat out, and who answered its last decision. Not how
the life was lost, not how many decisions the clock took, not whether the
player was really away (`StandIn` is a missing socket, so it says "not
connected").

`interaction::table_losses` puts the reading seat's lines first and then
every other seat's that is out, in seat order, named through `seat_name`.
Every seat's and not only the reader's, because a loss is public: the winner
of a duel has as much reason to read "Bob#1a2b conceded" as the loser has to
read their own line. The sheet sets them under the reason, upright at
`LOSS_PT` (15) in the reason's soft ink, and a draw, which has no reason
line, still gets them. The lobby's note after the game (#155) appends the
reading seat's own lines to the verdict and the reason; the others' stay on
the sheet, which has a line apiece for them.

### Two plugins, one composition

The verdict is the *duel's* to say and the way out is the *shell's*, and
`DuelPlugin` is meant to be embeddable in an application that already has a
front door — it cannot know whether there is a lobby behind it. So the sheet
leaves one row marked `hud::FinishExits`, and `lobby::ui::spawn_leave_button`
puts its buttons in it.

That seam is a **marker and nothing else**: no call, no shared resource, no
ordering between the two plugins. Which is why the lobby's system had to move
to `Update`. `spawn_finish` runs on the same state edge, its `Commands` are
applied at the end of that schedule, and a system merely ordered *after* it
would query a row that does not exist yet — while an explicit sync point
between two plugins that do not know each other is exactly the coupling the
marker exists to avoid. It runs every frame the game is over and stops the
moment its buttons are standing. The row is held at a fixed 32 px — what
`answer_node` renders at, confirmed against a live prompt slip — so the sheet
does not reflow as they land.

The exits are the slip's own answers, from the same `hud::answer_button`, so
they obey the slip's own rule: the first answer is what the sheet is *for*
and is the only one in brass. A lone "back to the lobby" is therefore a lead
answer, which is right — there is nothing left for it to be quieter than.

Two things fall out of that, and both were found by putting the buttons where
someone would look at them. A launch handed a `SeatTicket` adds no
`LobbyPlugin` at all, so its row simply stays empty — honest, because that
client has nowhere to go either. And *play again* asked
`matches!(screen, Screen::Seated(_))` while its own comment said "only for a
game reached through the gateway": an offline duel is seated too, so playing
the house offered a rematch that would have asked a gateway for another of a
table it has never heard of.

### It settles on the veil's own number

There is no second fade. `dim_the_table` already eases `Veil::lit` towards
whatever wants the table dark, at rate 9.0, taking the whole step at once
under `reduce_motion` — and a finished game is now its second reason to
darken a table, the stronger of the two: a dialog holds the whole answer, and
a game that is over has no answer left anywhere. `settle_the_sheet` reads
that number and paints the sheet with it, and lowers the sheet the last
twelve pixels onto the table. One rate, one resting point, one
`reduce_motion`, and nothing had to be told a transition is happening — the
shape `sky::table_light` already uses for the felt.

The colours have to be *remembered* rather than read back off the node, which
is what `finish::Settling` is: one frame into a fade the node is carrying the
faded value, and taking that as the base is how a thing fades to nothing and
stays there. The exits are deliberately left out of it — they carry
`ambience::Feel`, which owns their `BackgroundColor` from their first frame,
and two systems writing one component would be two answers to what colour a
button is.

Measured live at `0.08` timescale, over four frames of one concession: the
patch where the headline sits goes (44, 70, 58) → (69, 73, 57) → (106, 98,
80) → (103, 96, 78) while the felt beside it goes (33, 56, 46) → (19, 47, 37)
→ (10, 27, 23) → (10, 26, 23). One number, read twice.

Those four frames were taken while the sheet was still *under* the veil (the
next section), so the values it rises through are the veiled ones and the
patch now rests at (215, 200, 163) instead of (103, 96, 78). What they were
measuring is unaffected: the sheet and the felt move together, on one clock,
in opposite directions.

### The sheet shipped on the wrong side of its own veil

The `Z` ladder in `hud` says what the order is for — "a surface that is
answering a question stands over a surface that is merely showing one. The
veil is the hinge: what is below it goes dark, what is above it stays lit" —
and the end sheet shipped **below** it. The veil is its sibling under
`FinishRoot`, carrying `Z_VEIL`; the sheet carried no `ZIndex` at all, which
is a zero against that three, so the veil was painted over the whole screen
and the verdict was read through it.

It did not look like a bug, which is the point of recording the arithmetic.
Parchment (224, 212, 176) reached the window as **(125, 116, 94)** and the
lobby button's brass (201, 162, 39) as **(117, 93, 24)** — each within a
unit of what `TABLE_VEIL` at 0.70 predicts when the compositing is done in
linear light, and both a perfectly plausible *choice* of colour for a muted
end screen. What gave it away was that the numbers were predictable at all.
The sheet now carries `ZIndex(Z_SHEET)` and the ladder is asserted rather
than merely written down. Re-photographed after the fix, the same two
patches read **(215, 200, 163)** and **(201, 162, 39)** — brass to the unit,
and parchment a few counts under its own constant because the grain lies
over it — while the felt beside the sheet stayed at (14, 18, 22). The veil
is doing exactly what it did; it is doing it to the table only.

### The bar stops whole

`Duel::ending()` silenced the *question*, and the prompt slip is drawn for a
question **or** a refusal **or** a word about the connection — so two ways
back onto the screen were left open, and both of them are answers to a game
that is still being played. A refusal is the engine turning down an action
and there are no actions left, `DuelSet::Input` not running in `Finished`; a
word about the connection is a table waiting for you, and the gateway drops
the socket after `GameEnded`, so a red "the connection to the table was lost"
under "You won" would be reporting a loss that cost the player nothing.

The draw and concede pills were the same mistake one level up. They are the
two controls that belong to no seat, they are both *ways to end a game*, and
they went on being drawn over the end screen at full strength — lit,
hovering under the pointer through `ambience::Feel`, which runs ungated —
answering nothing. `overlay::spawn_menu_row` existed so that "not once the
game is over" was one `if` at the call site rather than an indent around
seventy lines. Measured at the same corner across the same concession: the
pill patch read (54, 61, 68) while the game was on and (18, 20, 29) once it
was over, which is the veiled night sky with nothing in front of it.

The pair is no longer in that corner. AX §4.3 put both of them in the shelf's
right-hand column (`ledge::ways_out`), where the rule is the same sentence
and one line shorter: after `GameOver` the column is **empty**. The corner is
the stack panel's own now, and `MENU_BAND` — the one inset in this client
that was not `EDGE`, stated rather than measured because a stack panel that
discovered the pills by overlapping them would only ever have done so in the
games that have a stack — retired with them.

## The sounds are decided before anything can play them

Every moment worth hearing is named, decided on the edge it happens on,
deduplicated against what the eye is already being shown, and handed to a
sink. `baylee-client-core/src/cue.rs` is the deciding half and
`crates/baylee-client/src/sound.rs` is the sink; between them there is a queue
and no audio API anywhere, which is what lets the whole model be tested
without a device and proved by a **read** of `/state` rather than by somebody
listening at the right moment.

### Nothing that plays is a file

`docs/legal.md` §5 is the rule, and it is clause 2's reasoning applied to the
ear: ornament is the easiest thing to borrow by accident, and arithmetic
borrows nothing. The felt, the seat mats and the lobby's backdrop were given
that answer and so is this — `sound::render` writes samples and `sound::wav`
writes a RIFF header in front of them, so the client ships no audio assets and
there is no `sounds/` directory to audit. A player's own pack stays reachable
because a `Cue` is a named moment and not a file name.

`bevy_audio` and `wav` are the two bevy features that cost: rodio and cpal,
which is CoreAudio, ALSA, WASAPI and — in a browser — an `AudioContext` that
does not start until the player has clicked something. They add eighteen
crates, all permissive and all already on `deny.toml`'s allowlist.

**One instrument, struck twelve ways.** All of them are a struck rosewood bar
over a resonator tube: five modes of the bar and five of the contact, under a
force pulse whose *duration* is the whole of what a mallet is, and over an air
column that takes 27 ms at D4 to bloom. Metal was the other candidate and is
wrong for the reason it is wrong in a room — a bell's upper partials outlive
its fundamental, so two overlapping is a chord nobody asked for, and the clamp
in `sound::modes` is what holds the module to that. What separates the twelve
is pitch, gesture, level and distance, never timbre:

| cue | peak | pitch | gesture |
|---|---|---|---|
| `MyLifeLost` | 0.700 | D4 → B3 | two strikes 110 ms apart, falling a minor third |
| `MyLifeGained` | 0.560 | D4 → F♯4 | the same at 100 ms, rising a major third |
| `TheirLifeGained` | 0.224 | A3 → C♯4 | as above, a fourth lower and across the room |
| `TheirLifeLost` | 0.280 | A3 → F♯3 | likewise — the *same* mallet, never a gentler one |
| `CardDrawn` | 0.140 | D5 | one tap per card, up to seven, accelerating |
| `CreatureGrew` | 0.196 | A4 → C♯5 | a 45 ms flam, up to five, the life gesture made small |
| `CreatureShrank` | 0.238 | A4 → F♯4 | the same falling, and the louder of the two |
| `YourMove` | 0.154 | D3 | one yarn touch, six variants |
| `Refused` | 0.350 | — | two knuckles on the leather rail; no bar and no pitch |
| `GameWon` | 0.630 | D4 → A4 | two strikes 380 ms apart, a fifth up, let ring |
| `GameLost` | 0.560 | D4 → A3 | a fourth down at 460 ms, softer and longer |
| `GameDrawn` | 0.504 | A3, A3 | a unison at 420 ms — two different blows, not one twice |

Two intervals rather than one mirrored one, because mirroring would make a
gain sound minor; together they outline a triad, so a lifelink trade —
`MyLifeLost` and `TheirLifeGained` on one frame — is a chord and not an
argument. "Elsewhere" is a **room and a level**, never a gentler blow: a
softer stick would say somebody was hit more gently, which is not what a seat
across the table means, so the mallet is the same and what changes is a dulled
direct sound and a reflection that arrives almost with it. The three endings
are the only sounds longer than a blink and all three peak **below** a life
cue; all three come to rest on A and never on the tonic, because a cadence
left open is what keeps a win from being a fanfare. `docs/design.md` retires
hues rather than handing out new ones and this is the same restraint.

**Three of them count, and that is a change to the model.** The owner asked
for a draw and a counter to be audible *as an amount* — "so that when several
cards are drawn, you hear that too" — and the module had written down that
this could not be done, because a `Cue` is a flat variant with no payload. The
reasoning was right about where the change had to happen and wrong about what
it was: a twelve-point hit is not a louder one-point hit, it is one event
whose number is on the bar, but drawing three cards is three events. So
`cue::Beat` is a cue *and a count*, `Cue::most` says how far each can count
(seven for a draw, which is a hand; five for counters, which are gestures and
are counted less far), and the sink builds one buffer per count. A burst is
the same blow struck N times at a gap that **accelerates** — an even run is a
metronome, and a dealer's flick is not — with every blow taking its own row of
a ladder of spots, detunes and gains, so three is three blows and never one
buffer played three times.

What decides the counts is `cue::Tally`, a second view-differ beside
`lifeflash::Ledger` and deliberately without its clock. **A draw of three
arrives in one view, not three**: `gamehost::Session::pump` runs the engine
until a seat that answers over a socket has a question and only then builds a
view, so everything between two questions is one difference — `MERGE` exists
because combat damage puts a question between its hits and a resolving
*Divination* cannot. Two rules make the reading honest. A draw is counted off
the **library**, as the smaller of "cards new to the hand" and "cards gone
from the library", because a bounce and a *Regrowth* arrive in the same field
of the same view and only the library says which happened — so milling five
and drawing one is one, and a bounce with no draw is none. The same rule makes
a **tutor to hand a draw**, and that is its one deliberate imprecision: a
*Recruiter of the Guard* takes a card off the library and puts it in the hand,
which from two views is exactly what a draw is, and nothing in a `PlayerView`
says why a card moved. It was heard at the table before it was noticed here,
and it reads as right rather than wrong — the eye sees a card leave the library
for the hand and the ear says so — but it is an acceptance, not a distinction
the counter is able to make. And counters count
**objects, not counters**: three creatures taking one +1/+1 each is three, one
creature taking four is one, and a creature that *enters* with counters on it
is a creature arriving rather than counters being placed.

The frame budget moved with it. `MASTER` was chosen when two cues were the
worst case an ordinary frame could hold, and a *Sign in Blood* is a life loss
and a draw while a *Fathom Mage* is a counter and a draw. `sound::audible`
therefore fills a frame greedily by `sound::rank` — a refusal first, then
life, then the table's texture, then the draw, then the nudge — and **drops**
what will not fit rather than ducking it, which is the choice the rest of the
module already makes: there is no mixer and a sound either happens or does
not. An ending is still played alone.

**`YourMove` is the one that could ruin it.** It fires on every priority
grant. It is the lowest sound in the set, and the quietest but for the draw, one touch rather than
a gesture, and one of six variants cycled in a fixed order — detuned within
±16 cents, with the second partial struck at a different spot along the bar,
so no two consecutive firings are the same sound. What is **not** built is the
policy half, and it is the larger one: a grant that follows the player's own
action tells them nothing, so a debounce, a suppression window after the seat
sends anything, a refractory period and a louder cue when the window is in the
background would turn two hundred grants into a few dozen touches. That wants
a clock in `Cues`, which it has no field for. Until then the answer is the
setting.

**Three steps, not a slider.** `cue::Loudness` is `Full`, `Half` and `Off`,
one chip each on the settings screen beside the sky's, and it multiplies at
*playback* — the balance between them is baked into the buffers, so there
is nothing for a player to tune. `Off` stops the device and nothing else:
cues are still decided, drained and reported, which keeps "is it silent" and
"is it deciding" two separate questions.

Everything is computed once, on the frame the app opens, from a xorshift32
seeded with a constant — so the table sounds the same on every machine, and
`a_cue_renders_the_same_bytes_twice` is what says so. The one thing a
generated sound has no other audit surface for is what it *sounds like*:
`every_cue_written_out` is `#[ignore]`d and writes all thirty-seven to a
directory for somebody to listen to — which is where the counted three have
to be judged, because "seven taps at these gaps can be counted" and "a 45 ms
flam reads as a direction" are claims about ears and no test can hold them.

### One event, one cue

The rule the model is built on: *what the eye is shown and what the ear is
told are the same event, answered by the same arithmetic.* A triple block
sends three views a frame apart and `lifeflash` merges them into one `−7`;
`Ledger::note` says whether it started a flash or joined the one standing,
`Change` carries that as `started`, and the ear is told once. Two opponents
losing life on the same view are two numbers on two bars and **one** sound,
because two copies of a sound on one frame are not two sounds — they are one
sound played louder, which is why `Cues::push` deduplicates within the frame.

The same discipline decides how a game ending sounds. `interaction::Outcome`
is the five *sentences* the end sheet can write, `Cue::of_outcome` collapses
them into the three *sounds* through `Outcome::won`, and `verdict` is that
same `Outcome` under a language. A table that chimed differently for team 1
and team 2 would be saying something the game does not mean — and a second
reading of `GameResult` is how the ear and the sheet would come to different
conclusions about who lost.

An ending is also the one cue that is played **alone**. Deduplicating within
the frame bounds a frame at one copy of each sound, not at one sound, and the
frame a game ends on is the only one that can carry three: the lethal hit is a
life loss here, a life loss there, and the ending. Those three peaks sum past
what a loudspeaker can do — 0.70 + 0.28 + 0.63 — so `sound::audible` keeps the
ending and drops what shares its frame, and `MASTER` is left answering the
two-cue case it was chosen for. Dropping rather than ducking is the same
choice the rest of the sink makes: there is no mixer here and there is not
going to be one. `Cue::ends_the_game` is the predicate, and it lives in the
model beside `of_outcome` because which moment outranks which is a reading of
the game; the shell only makes the noise. Nothing about this changes what is
**drained** — `/state` reports the whole frame and `YourMove`'s variant
counter moves as it would have, so "is it silent" and "is it deciding" stay
two questions.

### A cue is a flank, and it can be taken back

Nothing in the model is asked "is it my turn"; it is told "it has become my
turn", once. `Cues` remembers one bit — whether the last question was this
seat's — because the acting seat is re-sent its own question every time
anybody at the table says anything, and a client that chimed on each of those
would be a metronome. That is the backlog's AC1, "keine Frage → Frage".

`Cues::retract` is the other half of the same idea and is why the schedule is
shaped the way it is. The standing orders and the autopilot answer in the
same half-frame that installs a question — `poll_host` → `run_autopilot`,
both in `DuelSet::Sync` — and a question the player never saw is not a
question. Every action goes out through one door, so `Duel::submit` withdraws
the chime; the drain is `sound::play_the_cues` in `DuelSet::Present`, after
everything that could decide on a cue and everything that could answer one.
A player answering a question they *did* hear reaches `submit` frames later,
when the queue no longer holds it, and the call is the no-op it should be.

The refusal cue is gated the way the prompt bar's refusal line is: `!over`. A
game that has ended keeps none of the things that answer a question, and a
refusal chiming over the end screen would be the client objecting to
something nobody can still do.

### What `/state` says

`last_cue` is a flat string — `"MyLifeLost"`, `"YourMove"`, `"GameLost"`,
`null` before anything has happened — written from `Cue::name`, which is
spelled out rather than derived from `Debug` because a `Debug` rendering is
allowed to change and a harness reads this. Conceding an offline duel and
reading `/state.last_cue` is the end-to-end proof that the whole chain is
wired, and it needs no speakers.

`last_count` beside it is the half a name cannot carry: `1` for every cue with
no amount in it, `0` before anything has been heard, and for the counted three
the number the sink was actually given. Drawing three cards and drawing one
are the same `last_cue` and two different sounds, so a harness reading only
the name could not tell a burst from a tap — which is precisely the thing the
counted cues exist to do.

### The front door has music, and the table never does

The gateway, sign-in and registration faces play a tune (#296), and it is
arithmetic like everything above: `baylee-client-core/src/music.rs` writes it
a sample at a time, in the manner of a tracker module and from no module. A
square lead with its own echo, an arpeggio on a pulse whose width sweeps, a
triangle bass and a noise drum, 32 bars of 6/8 in D minor at a dotted quarter
of 76. Where it comes from is `docs/legal.md` §5.

**It is streamed, never rendered ahead.** `Tune` is an endless iterator, and
`crates/baylee-client/src/music.rs` makes it a `Decodable` asset whose decoder
is the tune itself, so rodio pulls it a buffer at a time on the audio thread.
No frame computes a sample, nothing is held but the tune's few voices, and
startup waits for none of it. A browser has no audio thread, so the pull
there runs between frames on the one thread it has. Both are cheap: a
release build makes the tune about 300× faster than it plays on an M1 and
220× faster on a Pixel 11 Pro XL (`music::tests::record` prints the factor).

**It loops without a seam.** The first four bars are an introduction and are
heard once; `LOOP` returns to `RESTART`, bar 5. Every note restarts its
oscillator's phase and the drum's noise is reseeded at `RESTART`, so the
second pass is the first one sample for sample, and a player who leaves the
face open for an hour hears no drift. `the_loop_plays_on_without_a_seam`
holds the join to no larger a step than the tune takes inside itself.

**It is heard at the front door and nowhere else.** `music::heard` is
`Screen::SignIn` while `DuelPhase::Closed`. The music fades in over 2.5 s and
out over 0.8 s, on signing in, on a game opening over the face, and on a
mute. At most one player exists; one that has faded out is despawned, so a
table synthesises nothing, and the next front door starts again at bar 1
(`the_player_comes_and_goes_with_the_front_door`).

**The level is the device's.** `ClientSettings::music` is a `MusicLevel`: a
volume, heard as its square because a linear slider does all its work in its
first quarter, and a mute that keeps the volume for when it is lifted. It is
not in the account's settings, for `last_username`'s reason: it plays before
anybody has signed in. The fields are private because serde_json writes a NaN
`f32` as `null`, and a `null` there would refuse the whole settings file,
gateways and guest sessions included; the setters ignore a non-number and
clamp. Its controls are the settings gear's and a speaker beside it on the
faces it plays on, because music that starts by itself has to be stoppable
where it plays (WCAG 1.4.2). The system reads the level every frame, so a
slider is heard as it moves.

**A browser keeps it quiet until someone presses something.** Browsers start
an `AudioContext` suspended until the page has had a gesture, and cpal asks
its context to resume once, at startup, before any gesture can have
happened. `index.html` wraps `AudioContext` before the wasm loads and resumes
every context it made on each press, key or touch until it runs. That is
what makes the table's cues audible in a browser too.

## The zone browser is a dialog, which is a different material

`docs/redesign-proposal.md` §1.3 draws the line and §6 applies it:
**parchment is a sheet you read from, a panel is a place you work in.** The
browser was parchment, and a grid of ten card columns, on the argument that a
graveyard is something a player *reads*. It is not — §6 puts a checkbox, a
tally and a Confirm on it, and that is work — so it is a dark panel in its
own warm near-black (`palette::DIALOG` and the four inks beside it; the HUD's
older `PANEL` is a cool near-black and is the one surface in this client that
was never on a candlelit table).

What is in it is a **list by default, and a grid on request**. A row is a
checkbox, a thumbnail, the name, the cost in pips, the type line and a badge
saying which pile it is in. The chosen row goes **candle** — a wash rather
than a fill, because a chosen row is still a row being read — and never the
teal §1 retires; `the_dialog_says_nothing_in_teal` reads the file back.

The list is the default because **choosing is reading**: a fetchland offers
the whole library, and which of ninety lands is answered in the type line and
the cost, which are words. A grid answers "show me more at once" by growing
sideways, which buys nothing for those three facts — they fit in one measure
and everything past it is blank — while a list grows down, which is where a
hundred cards are. That argument used to end with the grid deleted, and it was
one step short: *not every opening of this panel is a choice*. A player
tapping their own graveyard to see what is in it is browsing, and browsing a
pile of cards is what a grid is for. So `browser::ViewMode` is three shapes
for the same rows — `Detailed`, `Large`, `Grid` — chosen by three icon
segments at the right end of the controls row, and the sideways axis is a
click rather than a refusal.

Three things fix those three shapes, and none of them is a taste. **The
picture has one size.** Card art is fetched at `ArtSize::Small`, 146 × 204
device pixels, so a 73-logical-pixel card is one texel to one pixel at
scale 2 and the next size up costs eleven times the texture — a hundred-card
library at `ArtSize::Normal` is 133 MB against a 96 MB budget on a phone. So
the large row's thumbnail *is* 73 and a grid tile grows to at most 100, and
no view asks for a different image than the row it replaces: switching shape
costs no fetch and no VRAM at all.
**The grid packs at that floor and grows into the gaps**, never into the
picture (`browser::grid_across`, the same rule `seatbar::Density::for_length`
uses on a mat): as many tiles as fit at 73, then shared out to fill the
measure. What is shared out is the **tile** and not the picture in it, which
is the one piece of this arithmetic that has already been wrong once: a tile
is `TRAY_TILE_CHROME` wider than its art — the focus rail on both sides and
the air that keeps the picture off it — so packing pictures and then drawing
each of them eight pixels wider puts every full row over its measure by a
whole tile, which `bevy_ui` answers by wrapping the last one onto a line of
its own. A zone holding two cards cannot show that and a graveyard always
would. The sheet's own `MIN_W` guarantees three columns, and three is exactly
the count at which the 100 cap can still be reached: the grid draws its
widest pictures in a ten-pixel window around a 380-pixel sheet and packs at
the art's own floor at every width above it. The two tests say which of them
checks which — the tray counts the widths where the cap binds, and the core
test holds `grid_across` at a measure narrow enough that it binds outright.
And **the sheet's arithmetic stays the detailed row's**: `TRAY_ROWS`,
`DEFAULT_H` and `MIN_H` describe the panel a player opens, so changing view
changes the flow inside it and never the rectangle it stands in.

What each shape drops is the interesting half. The **large** list loses the
type line and nothing else — it was the widest fixed column, and at 73 pixels
a frame's colour and a creature's silhouette are legible off the art itself —
and puts the name (one step larger, with the cost beside it) over the pile
badge on a second line. The badge stays because a merged multi-zone list is
the whole reason it exists. The **grid** writes nothing at all: a name under a
73-pixel column clips on most of the pool, and the hover preview already says
which card this is in full. What a tile still has to carry is the two things
a picture cannot say for itself, and they are the same colour at two
geometries — chosen is the art's own edge going candle, focus is a rail
standing outside the tile — so a tile that is both reads as two concentric
rings. An ordering's place number is the list's own candle disc moved into the
tile's corner, because a player who changes view mid-ordering must not have to
learn the mark again. Tiles run in one flow under a single ticked tab and in
headed runs when several are ticked, which needs no regrouping: `BrowseZone`'s
`Ord` *is* the tab order and `Browser::rows` already emits zone by zone in it.

An **arrangement** (`Pending::Arrange` — "put them back in any order", "the
rest on the bottom in any order") is answered by *moving* cards rather than
naming them, and `client-core/src/arrange.rs` is the whole model. It starts
as an answer — every card in the first pile that can take them all, in the
order they were offered — so a player who is content sends at once, and no
card can go missing from the answer on the way. A tap takes a card up (the
candle edge, because held is what `is_selected` means here); the next tap
puts it down in front of the card tapped; the same tap again lets go of it
where it is. The number in each tile's corner is its place in its pile, top
first, and the rows stand in that order, because a number and a position
that disagreed would shuffle under a player's pointer. The keyboard has the
same three gestures and one more: the focus keys walk the cards, the tick
takes one up and puts it down, and while a card is held the cursor keys
nudge it one place along its pile or to the end of the pile above or below —
the move a pointer makes by tapping the neighbour. `Esc` lets go of a held
card before it undoes anything, and only a second one puts every card back.
A choice is re-sent whole with every view, so `Interaction::new_keeping`
keeps a half-built arrangement when the same seat is asked the same cards
into the same piles again.

The sheet draws an arrangement **pile by pile** in every view — a heading
naming where the cards go and how many are there ("Unter die Bibliothek
(1)"), the pile's cards, then the pile's **end**. The end is a control only
while a card is held that could go there (`Arrangement::can_place`), and a
tap on it puts that card last in the pile: a tap on a card puts the held one
*in front of* it, so without the end the last place of a pile, and every
place in an empty one, were reachable from the keyboard alone. With nothing
held an empty pile still shows its end as a quiet "Leer", so a scry's bottom
reads as somewhere to put a card before anything has been put there. The
place numbers restart in each pile, the graveyard's pile carries none (its
order is not the player's), and the sort control is not drawn: the order on
the sheet *is* the answer and `Browser::rows` sorts it by nothing else.
`TrayRevision` holds the arrangement itself, because neither `selected` (an
arrangement keeps no picks) nor `aim` (a position a nudged card can keep)
sees every move.

The mode lives in `ClientSettings` beside the sheet's rectangle, and its
reader is hand-written for the reason `Keymap`'s is: the store is
`from_str(…).ok().unwrap_or_default()`, so a view mode retired in a later
build would refuse the whole file and take that player's sheet placement,
their language and their remembered username with it. An unknown name reads as
the default instead.

The footer is two buttons and both of them send what `PromptAction::Confirm`
sends. **Confirm is lit only when the answer is complete, and Cancel is drawn
only when the minimum is zero**, which is `Interaction::bounds` answering both
questions: there is no cancel action on the wire, so a question that will take
an empty answer is answered by *sending* one and a question that will not has
no way out to offer. Cancel is not Confirm with different words, though —
`Interaction::cancel` clears the answer first, because a player who ticked a
card and then changed their mind must not have that card sent under the word
"Cancel".

Two sizes are derived from all of that rather than chosen. `TRAY_PANEL_W` is
**one row** — the fixed columns plus a measure for the name and one for the
type line — and `Placement::DEFAULT_H` is the chrome plus eight rows **and a
half**, the half being what says the list continues without a scrollbar. The
grid before it was cut to four *whole* rows for the opposite reason: most of a
fifth row of cards was space nothing could ever be put in. `MIN_W` and `MIN_H`
are the same arithmetic at the floor — ten characters of name, and two whole
rows.

**The dialog is drawn on a revision of its own**, and that is the answer to
the owner's second report about it: *„Das Zonen-Dialog ist noch sehr instabil!
Beim Hover flackert alles"*. `HudRevision` carries `hovered`, so the overlay
tree is torn down and written again on every pointer move that changes which
object is under the cursor — and the dialog is a hundred rows that the pointer
moves *across*. What a player sees is the row they are reaching for going out
as they reach it: the replacement is a new entity whose `Feel` starts at
`warmth: 0`, and picking needs a frame to send `Over` to something that did
not exist when it last looked. `tray::TrayRevision` counts what the dialog
actually draws from — the `BrowserGate`, the snapshot, what is ticked, where
the keyboard is standing, arrivals, the text-face latch, the window — and no
hover at all, because the dialog reads none: `Feel` lights a row through
picking frame by frame with nothing rebuilt, and a row's focus ring is
`Interaction::aim`, the keyboard's row rather than the pointer's. The
placement is deliberately not in the gate either; `input::tray_drag` writes
the panel's `Node` directly so that dragging the sheet does not rebuild it.

**A gate field may not be a projection that throws state away.** Naming every
field closed the hole the sort buttons fell into; it did not close the one a
field's *type* can open. `BrowserGate::filter` was the box's `String`, and the
box draws more than its string — `tray::filter_runs` asks
`TextBuffer::segments` for head, selection and tail and puts the caret bar
between two of them — so every caret move was invisible to the comparison and
the box stood still until the text changed. Measured in the running client on
14.09.2026: `abcdef`, then five `ArrowLeft`s, three of them holding shift,
moved the box by **zero** pixels, and the `Backspace` after them deleted the
**b**, which is the model saying the caret had been standing at 2 the whole
time. The gate holds the `TextBuffer` now, whose own `PartialEq` carries the
caret and the anchor; the type change is its own counter-test, because the
expression that dropped them no longer compiles.

It is the **sixth** retained tree and it is *not* a sixth root: the veil
stands at `Z_VEIL` and the panel at `Z_SHEET` with the shelf's `Z_LEDGE`
between them, and a `ZIndex` orders a node only among its own parent's
children — so the two are two children of `HudRoot` which `sync_overlay`
passes over by marker, the same bargain the shelf and the drawer already have.
Wrapping them in one node to make a single root would put the shelf behind the
veil, and the shelf is where the question the dialog is answering is written.
The marker on the outer one is `TrayBand` rather than `TrayPanel`: the sheet
is the rectangle a drag writes and has to stay the inner node, and a sweep
told to keep the inner one despawns the band and takes the sheet with it.

**The sheet is minimised into a tray, and is not closed at all.** The owner
asked for the strip on 19.09.2026 — *"Baue den Tray ein. Die Actions Bar ist
voll, aber an der Actions Bar hängt manchmal so ein Info text. Auf eine
ähnliche Art und Weise ist der Tray so ein Attachement an die Actions Bar nur
rechtsbündig und kleiner von der Höhe her"*, and then the rule that gives it
its job: *"Er wird aber nicht mehr geschlossen sondern in den Tray minimiert.
Demnach ist der Button im Tray immer sichtbar und öffnet beim Klick den Zonen
Dialog."* `hud::ledge::tray` is that strip — the **third** retained
attachment on this ledge after the drawer and the mana pool, hanging off the
same top edge with the same one pixel of overlap, right-aligned where the
drawer is centred, and shorter than the shelf by the four pixels the shelf
spends on breathing room around its own button row.

What changed to make "minimised" true is **nothing about the state**.
`Browser::close` has always kept the ticks, the filter and the placement; the
sheet has always come back exactly as it was left. What it never had was
anything that said so. A window with an `✕` and no taskbar entry has been
dismissed, and the same window with a button still standing on the shelf has
been put down — so the button is the whole difference, and
`Browser::toggle_by_hand` is one method rather than two because there is no
third state to name.

**Four doors, and two of them had been lying.** `G`, `Escape`, the button on
the sheet's own head and now the tray's each decided for themselves whether
the sheet could be put away, and the head's button and `Escape` decided it
wrong: on a sheet a *question* opened they closed it, `Browser::follow`
re-opened it on the very next frame, and both were indistinguishable from
controls nobody had wired — which is what they had looked like for as long as
they had existed. `Browser::may_be_put_away` is the one predicate now. The
head's button is drawn only on a hand-opened sheet, exactly as the resize
corner beside it already was; the tray's is drawn always and **held** on a
question, because a button that is there and does nothing is worse than one
that is visibly not now; and `Escape` falls through to clearing a half-built
selection, which is what a player pressing it in front of a question means.

**The strip is over the sheet, and that rung is a requirement.** A sheet is
placed in a band that stops `EDGE` — twelve pixels — above the hand zone,
which was exactly enough while the shelf was the only thing on that edge. The
tray is seventeen pixels taller than that gap, so a **maximised** sheet covers
more than half of its button, and *"Demnach ist der Button im Tray immer
sichtbar"* was asked in the same breath as the strip itself. So `Z_TRAY` is a
new rung between `Z_SHEET` and `Z_PREVIEW`: the strip is lifted over the
sheet, rather than the band being shortened for every sheet by the height of a
strip that stands at one end of it. It stays under the preview because a
preview describes what is under the pointer and the button is something a
pointer can be over.

**Putting the sheet down is a movement, and so is bringing it back.** The
owner asked for it with the strip — *"Mit minimize Animation (und reverse
Animation)"* — and the shape it needed is the one `sync_drawer` and
`zoom_the_drawer` already use one panel along: `sync_tray` does not despawn a
sheet whose browser has shut, it sets `TrayReveal::closing` and returns, and
`reveal_tray` flies the sheet into the tray and takes it off the tree at the
end. Two details are load-bearing. The gate is on `showing = drawn &&
!closing` rather than on `drawn`, or a standing sheet beside a shut browser
fails the early return on every frame and the body despawns the thing that is
still moving. And the despawn is in `reveal_tray` and not in `sync_tray`,
`.after` it, because a sheet reopened on the frame its `t` reaches 1 would
otherwise be despawned by one system and rebuilt by the other in the same
frame.

What travels is a `UiTransform`, and where it travels to is arithmetic rather
than a guess: `ledge::tray::zones_button_centre` derives the button's middle
from `root_node`'s own numbers, the panel's middle is read off its `Node`, and
the scale falls to 8% — not the button's own 2.4%, at which the last third of
the movement is a dot sliding along the shelf rather than a sheet arriving.
`the_sheet_is_aimed_at_the_button_the_layout_actually_draws` reads the layout
back out of the node instead of restating it, because nothing in a running
client would notice the two disagreeing.

The veil goes with it, and that is what made `TrayVeil` necessary. `TableVeil`
is worn by two surfaces — this dialog's and the end screen's — painted by one
system on purpose, and the dialog's teardown used to despawn every one of
them. It could not tell them apart, so a game that ended while anything about
the dialog changed lost its darkening and `dim_the_table` had no node left to
paint. Now the dialog owns a marked veil, tears down only that, and the veil
outlives the browser being shut so that it can fade while the sheet is in the
air.

**The maximise button moved into the head, and it is a move rather than a new
control.** *"Der maximieren Button wandert neben den minimieren Button"*, and
the gesture it names already existed: a press and a release on the resize
corner that travelled less than four pixels between them. It could not be a
`Pointer<Click>` — a resize *ends* over the corner, because the corner travels
under the hand, so every drag would have fired one — and it existed at all
because the corner drew a `⤢`, which is an argument from a mark rather than
from a control. The mark and the gesture both moved; the corner now wears a
double-headed diagonal arrow and does one thing. The button joined
`tray_drag`'s press-exclusion list beside the minimise button and the zone
tabs, which is the third time that list has grown with the header.

It is animated as well — *"Das Maximiere und reverse soll auch schön animiert
sein"* — and what travels there is the **rectangle**, not a transform.
`Placement::lerp` is written into the `Node` on every frame by
`input::glide_the_sheet`, so the sheet is re-laid-out the whole way across and
its rows stay rows; a `UiTransform` stretched from 1020×738 to the band's
shape would carry the type column, the thumbnails and the row heights with it
and arrive as a distorted picture that snapped straight at the end. The store
holds the **target** from the first frame, which is the same split
`input::tray_drag` already keeps and the reason `write_placement` exists: a
rebuild mid-flight has to land where the sheet is going.

**The sheet opens wider, and the height is what did not move.** *"Der Zonen
Dialog bleibt wie er ist, er soll nur etwas größer werden"*, and the
arithmetic decided which axis. Measured at the 1728 × 1052 window this was
asked on, the band is 1728 wide and 837 tall: the sheet was taking 52% of the
one and 88% of the other, and a list has no size between two halves of a row
— the next stop up is a ninth and a half at 808, which is 96% of that band and
the dialog-reads-as-a-screen that `DEFAULT_H` has refused twice. So
`DEFAULT_W` went 900 → 1020 and `DEFAULT_H` stayed, with all 120 new pixels
reaching the two columns that carry words.

Reading those two constants side by side found three sentences that had gone
stale without a test noticing, which is worth recording because each was
invisible in the same way. `DEFAULT_W`'s doc opened "One row wide — what
`TRAY_PANEL_W` in the renderer computes", and the sheet had been 198 px wider
than a row since it went 702 → 900 with nothing written down; the seam's test
is a **floor**, and 900 passed it by not being the number the prose claimed.
`DEFAULT_H`'s said "seven rows and a half" and so did `TRAY_ROWS`' own doc,
with `8.5` written directly underneath it — the zone tabs moving into the
title row handed 30 px of chrome back and the sheet spent them on the row it
had just given up, which is the opposite of what both paragraphs said. In
every case the *constant* was right and the test read the constant.

**The mana pool is the same strip on the other side.** *"Der Manavorrat soll
auch ein repositioneng bekommen. Es soll symetrisch zum Tray aussehen nur auf
der linken Seite"* (19.09.2026), so `ledge::strip_node` is now one function
and both attachments spawn it — the four numbers that decide a strip's shape
(height, the pixel of overlap with the lip, the corner and the inset) live one
level up from either, and the whole of the difference is which side is
`EDGE` and which is `Val::Auto`. A copy in the second file would have been
symmetric on the day it was typed and only then; a strip one pixel taller than
its twin is visible at a glance, which is exactly the kind of drift a shared
constant is cheap insurance against.

What moved is *which* sweep it has to survive, and that is the half worth
writing down. The pool has always been retained — a mana that arrives is drawn
arriving, and the shelf's own children are despawned on every sentence, so an
entry that came and went with the question could never be seen to pop. It was
a retained child **of the shelf**, exempted from `sync_ledge`'s rebuild by
marker; it is now a child of `HudRoot`, exempted from `sync_overlay`'s sweep
by marker, which is the bargain the drawer and the tray already had. The
argument did not change and the node it hangs from did.

It stands at `Z_TRAY` with the tray rather than at `Z_LEDGE` with the drawer.
That is a judgment by symmetry with an argument of its own: a maximised sheet
reaches across the whole band, and the mana a player is holding is most worth
reading exactly while they are spending it — which is when a zone dialog may
well be open in front of them.

**It grows out of the shelf and folds back into it**, rather than being put
on the screen and taken off it. `StripZoom` is the tray's own pair one level
down — a `t` and which way it is going — and `grow_the_pool` is the only
thing that touches the strip's `Visibility`: it shows it on the first frame of
an arrival and hides it on the *last* frame of a fold, so the strip is never
taken away around a movement of its own that has not finished. The anchor is
the one number that is not the drawer's: `motion::from_bottom_left`, because
a node pinned at the left margin that shrinks toward its own middle slides
right as it grows, and the pool would arrive sliding out from under the
sorting buttons instead of out of the shelf.

Photographed with the clock stopped (`/pause`, then `/step` a counted few
frames at a time), the arrival is 52 physical pixels tall on its first frame
against 60 at rest, and 210 wide against 239 — 0.867 and 0.879 against the
0.88 `ZOOM_FROM` asks for. The two numbers that do **not** move are the point:
the left edge is at 24 and the bottom at 1701 in every frame of both
movements, where the drawer's `from_bottom` would have started the strip 14 px
to the right of where it ends.

That movement is also where the harness's still clock stopped being free.
`bar_of` runs with no time in it, which is right for everything that is a
layout and wrong for anything with two movements in series: the strip cannot
start folding until the last pip has finished fading, and a pip fades on
`delta`. Written without advancing `Time` the fold test read as a test,
passed, and asserted nothing — it survived a `grow_the_pool` that hid the
strip the instant the pool emptied, which is the exact fault it was written
for. It walks the two movements through in order now, each with the clock
pushed past its own span.

**The hand's sorting buttons took the shelf's left column, and got smaller.**
*"Dafür verschiebe die Hand Sorting Buttons in der Actions-Bar ganz nach links
und mache sie etwas kleiner"*, which is the other half of the same request:
the pool's 365 px of reservation left the shelf with it, the tools moved from
`LEFT_RESERVED` to `EDGE`, and `TOOL_H` is `BUTTON_H` less the eight pixels
that make a row of standing preferences read as quieter than a row of answers.
`LEFT_RESERVED` is gone and `tools_reserved(window_w)` is what `arrange` is
fed — 296 at a window wide enough for all five buttons and 128 at one that
cycles through them, against 675 and 510 before. The left neighbour is a third
of what it was, and every pixel of that goes to the question in the middle.

The reservation stayed a reservation and stopped being a refusal. §2.3 will
not centre the question between its neighbours, so that it does not move when
a mana pip arrives; the pool's contents changed *inside* one question and a
reservation that followed them would have undone that one level down. The
tools' width changes with the interface language and with whether the window
is wide enough for five buttons, neither of which can move while a question
stands — so reserving the wider language is caution here rather than a
requirement.

**The two ways out became a burger, and its panel is the fourth retained
attachment.** *"Aus den zwei Buttons rechts wird ein Burger Menü. Es geht auf
und dort ist ein schönes Menü (animationen zum auf und zu gehen etc.), hier
kommen noch mehr Buttons rein, aber erst Mal die Zwei nur und eine
Versionsanzeige+build des aktuellen Clients"* (19.09.2026).
`hud::ledge::menu` is the panel; the burger is a `BUTTON_H` square standing in
the shelf's **right column**, where the two labels used to be.

That column and not the tray's strip, which was the one decision in it. The
shelf's left column already holds the hand's sorting buttons and the two
strips hold the game's own state — so a column is where a *player's* controls
live, a strip is a door to something the game is holding, and the middle is
the engine's question. Hanging the burger off a strip would have made the two
strips asymmetric in kind on the day they were made symmetric in shape, and
it is the further of the two readings of *"die zwei Buttons rechts"*.

What it is worth is `RIGHT_RESERVED`, **222 px down to 40**. The old number
was "Remis anbieten" beside "Aufgeben" — measured, and paid on every window at
every question, because §2.3 reserves what the neighbours take whether or not
they are drawn. An armed concession is wider than both buttons together and
was drawn alone for exactly that reason; that knot is not smaller now, it is
**gone**, because a 28-pixel square is the same width whatever it is about to
say. The corresponding rung in `client-core`'s own `arrange` tests keeps 222
as a historical worst case, the way `LEFT` kept the mana pool's 365.

**It grows out of the corner its own button is in**, which is `grow_the_menu`
and `motion::from_bottom_right` — the mirror that file predicted when the
pool needed `from_bottom_left`, written out under its own name rather than as
a signed parameter, because a caller gets a sign backwards and does not get a
name backwards. `MenuZoom` is `StripZoom` one level down again and `sync_menu`
never despawns the panel: it writes `closing` and lets the fold finish.

The panel is retained for an argument **neither strip has**. The concession
takes two presses, the arming press changes `LedgeRevision` and rebuilds the
hand's shelf and its columns — so a panel that lived among them would be despawned between
the two presses of the one decision in this client that has no undo. The
second press would land on a button built half a frame earlier, at a position
nothing guaranteed was the same.

The version line is `baylee_build::short()`, the same string the lobby draws,
and it is what fixes `MENU_W`. The first draft was sized against the widest
row label and was six pixels too narrow: a label is bounded by the phrase
table and a build string is not — `0.1.0+build.1042 (d17af60d08)` grows a
digit per thousand builds and seven characters when the tree is dirty — so
the test bounds the panel over the worst shape that string can take, and then
asserts the real one is no longer than that shape.

`Esc` gained a rung between the preview and the zone browser, and a press
anywhere outside puts the menu away (`close_the_menu_on_a_press_outside_it`).
That system spares the panel's lineage *and* the burger, and the burger is
why it needs a list at all: a press and a click land on different frames, so
the press would shut the panel and `menu_click` would re-open it a frame
later, leaving a button that looked busy and did nothing.

Photographed with the clock stopped, three frames into the arrival the panel
is **552 physical pixels wide and 239 tall**, against **544 by 236** at rest —
and its right edge is at 3431 and its bottom at 1700 in both. So the corner
its button is in holds still to the pixel while the panel comes out of it, and
544 is exactly `2 · MENU_W`, which is what says the thing measured is the
panel. The 1.0147 it is drawn at there is the **overshoot**, not an
unfinished arrival: `opening(3/60 / 0.16)` is 1.0142, and `ZOOM_BACK`'s whole
point is that the curve passes 1 and comes back.

Two things about measuring it are worth more than the numbers. The first
attempt matched on the panel's ground colour over a loose window and got 871
wide with its right edge at 3395 — the hand's shelf has that same ground, so
what came back was the shelf, and it was committed before anyone held it
against `2 · MENU_W` or against `2 · (1728 − EDGE)`. Either check would have
refused it in one line, and one of them was already contradicted out loud: the
same reading said 0.986 while the narration beside it said the overshoot was
visible. And a **whole-window** diff cannot stand in for the tight crop:
`/pause` stops `Time<Virtual>`, which every movement here reads, but a
shader's `globals.time` runs off the render clock — measured, the felt away
from the panel differs by up to 52 levels between two frames separated by a
`/step` — so a diff of the window answers "everything moved", and a threshold
under that floor quietly reads the noise as the thing.

**Writing it found that the sweep's exemptions were held by nothing.**
`sync_overlay` spares six markers under `HudRoot`, and taking the tray's, the
pool's or the menu's condition out left all 27 tests in `hud::overlay`
passing. The reason is that the sweep leaves no hole behind it: a strip
despawned there is spawned again by the branch that rebuilds the root, on the
same frame, so the count is one either way and the picture is right on the
next frame. What is actually lost is the `StripZoom` or `MenuZoom` that was on
the **old entity** — a movement stopping dead mid-flight, not a node
disappearing, which is the one symptom a count can never see. The test that
reads as though it covered this checked the shelf and the drawer by identity
and the three strips not at all; it is
`the_shelf_and_its_attachments_outlive_a_rebuild_and_nothing_else_does` now,
checks all five, and fails for each of the three exemptions taken out.

The strip's icon is an archive box and deliberately not the layer-group a
seat bar draws for a library: two identical icons on one screen meaning two
things is worse than a less obvious one meaning its own, and what this dialog
shows is the cards a game has put *away* — graveyards, exile, the command
zone, the stack. The head's button draws a window's bottom rule rather than
the `-` the drawer's stepper spends on arithmetic. Both codepoints were read
out of the shipped icon font's own cmap and rasterised before they were
written down.

**A place you work in is a place the keyboard is already in.** The filter box
takes the keyboard as the panel opens (`input::browser_takes_the_keyboard`)
and gives it back on `Esc` or `Enter`, after which the sheet can stand open
through a turn with the letters at the table again. It was the other way
round — the box took the keyboard only on a click — and the owner found the
cost the first time they used the search: fifteen of the twenty-six bare
letters are bound actions, so a term typed into a freshly opened panel was
fifteen keystrokes fired at the game. One of them is `T`, which latches the
text view on and **persists it**, which is why the report came back as *"Alle
Karten sind falsch rum"* a session later rather than as anything about
searching. `K`/`B` and `Y`/`N` are worse and quieter: they reach the engine,
and there is no undo.

Three details hold it. The keystroke that *opened* the panel is not a
keystroke for the box — `G` opens the sheet on a frame where nothing reads the
message queue and a `KeyboardInput` outlives its frame, so `keyboard` advances
its reader past whatever is standing on the frame the box gains the keyboard,
or the panel opens with `g` already typed into it. The shell refuses to take
the keyboard at all where the platform owns its own typing (a phone's
`<input>` is raised by a tap and by nothing else) or while an ordering is
being made, which draws no filter box. And `Browser::start_typing` promotes
only a sheet that was **shut**: writing `Opening::ByHand` over one a question
opened would set `answers_here()` false and kill that question's own keys on a
dialog still standing, which is what clicking the filter box did to a search
prompt for as long as the box could be clicked.

**A zone tab is a box to tick, and the boxes are the title bar.** The owner
asked for both on 14.09.2026 — a checkbox at the start of each zone's name so
that ticking several *merges* them, and the chips moved up into the title row
with the word "Zonen" dropped, because the chips say what the panel is better
than a label repeating the name of the thing that was just opened. So
`Browser::tabs` is a `BTreeSet` and **the empty set is what "Alle" means**:
there is no fourth state to remember, unticking the last pile lands back on
everything instead of on a panel showing nothing, and `Browser::shows` is the
one place that meaning lives. It is a `BTreeSet` and not a `HashSet` because
`BrowseZone`'s `Ord` *is* the tab order, and a set that iterated differently
each run would be a second opinion about it. Two rules keep a tick honest: a
tap on a pile and a reveal **replace** the ticks rather than joining them — a
reveal merged into a graveyard is a reveal nobody can find — and a tick does
not outlive its pile, so a graveyard that empties takes its own tick with it.
Tabs in the title row means the row is also the drag grip, which is settled
the way the minimise button already settles it: `input::tray_drag` lets the
specific control claim the press before the row it stands on.

Every zone tab says how many cards are in it, in brackets, which is not
decoration: `bracketed` is what greys a run, so `Graveyard (12)` draws as a
name with a grey aside and reads as one. It is also the only thing the
deleted pile-chip strip said that nothing else on the sheet did — and the
dialog keeps the rule in its own two inks (`dialog_text`), rather than
borrowing `slip_text`, whose aside grey and warm letter-shadow are both about
lifting ink off parchment.

**The sheet moves and resizes, and remembers where it was put.**
`browser::Placement` is a rectangle inside the *band* — everything between the
window's top edge and the hand bar; it used to start under the phase rail, a
hundred and ten pixels down — in logical pixels rather than fractions of it,
because the grid inside is cards at a fixed size and a sheet that scaled with
the window would show a different number of columns on every screen. `fit`
shrinks before it moves (a sheet moved first can be pushed off the far edge
by its own width) and never writes itself back, so a window briefly dragged
narrow does not overwrite where the player put it. The geometry lives in
`ClientSettings` and not `Preferences`: it is a fact about this screen, not
about the account.

**A sheet a question opened is not furniture, and reads none of that.**
`Opening::ForChoice` is the client's own doing — it arrives with the question
and is taken away with it — so `Browser::placement` centres it on whatever
window it meets and `input::tray_drag` refuses to move it. Both halves are
needed and the writing half is the one easy to miss: the remembered rectangle
that sent the dialog to the left third of a window was 854 wide at `left:
437`, which is *centred* in a 1728-pixel band and 164 pixels left of centre
in the 2056-pixel one it was drawn in. `fit` clamps a rectangle inside a band
and has no opinion about the middle of it, so a sheet placed by one window
kept that place in the next. A drag that still wrote to the store would put
the panel back there a rebuild later, and would leave the *hand-opened*
sheet standing somewhere nobody chose.

The same opening pins the tab. `Browser::locked` is beside `tabs` rather than
inside it because the two answer different questions — `tabs` is which piles
are showing, `locked` is whether the player may change that — and it is
decided by
the **offer** and never by the prompt kind: every id in
`Interaction::selectable` living in one `BrowseZone` pins that zone, and an
id on the battlefield or in hand pins nothing, which is right, because a
question answerable by clicking a permanent must not lock the sheet to a
pile. A pinned tab is drawn without `Button` or `Feel` and with
`Pickable::IGNORE`: a control that lights under the pointer and then refuses
the click is worse than one that never invited it.

A `ForChoice` sheet also draws **no resize corner**. The drag is refused
there, so the handle would be a control that lights under the pointer and
then does nothing — the same rule the pinned tabs obey, applied to the one
piece of furniture left inviting a gesture nobody can make.

**The table goes dark behind a dialog that holds the whole answer**, and
behind no other. `Browser::dims_the_table` is `locked().is_some()` and
deliberately not `for_choice()`: a question opens this sheet whenever *any*
of its answers is somewhere the table cannot show, which is not the same as
every answer being in here. A `ChooseCards` spanning the cards being revealed
and the player's own hand opens the sheet and locks no tab, and a veil over
that question would be darkening the hand the player has to click. A reveal
with no choice attached falls out the same way and is right for the same
reason: cards being shown are not a question.

The veil is one full-window node, `Pickable::IGNORE`, painting
`palette::TABLE_VEIL`. Darkening is the whole of the ask — a veil that also
swallowed clicks would be making a claim the model does not make — so a click
on it falls through to `input::pointer`'s "nothing interactive" branch, which
clears the preview, which is what a click on empty felt has always done. Its
colour is **cold**, and that is an argument rather than a taste: the dialog
is srgb8 (28, 25, 19) and the veiled cloth is (13, 20, 25) — `TABLE_VEIL`
over `tabletop::FELT_CLOTH` at the veil's own alpha — so the two stand
within a dozen levels of each other on every channel and separate on
temperature rather than on brightness. The second number was (15, 29, 26)
while the cloth was green, and the argument it was written for survived the
redesign because it was never about the hue the cloth happened to be: a warm
panel over a cold table reads as two things whatever the table is made of.

The veil used to be the same blue-black the hand bar's own
ground was, and that is no longer true of the ground: the owner asked for a
container on 14.09.2026, and the hand zone is now `frontal`'s cloth in
`palette::DIALOG` — the dialog's own colour, with nothing cool about it. The
veil is over the *table* and the container is the dialog's register, so the
two no longer have to match. The alpha was measured on screen either side of one
`Confirm`, because a `BackgroundColor` composites in **linear** space where
an alpha buys far less darkening than sRGB arithmetic predicts: 0.70 takes
the felt (29, 53, 43) → (15, 29, 26) and a seat bar's ink 173 → 100 — a
little over half everywhere, and everything still legible. 0.60 read as
weather rather than as a table that had been put down.

How far the fade has risen lives in the `hud::Veil` **resource** and not on
the node, and that is load-bearing: the overlay is a retained tree, every
tick of a checkbox rebuilds it, and a fade held on the node would start again
at each of them — a table that flickered while a player chose a card. It
rises and never falls on screen, because the dialog is torn down the instant
it is answered and the veil goes with it; the number still eases back down
with nothing to draw, which is what makes the next question fade in from
nothing rather than snap from wherever the last one stopped.

**One question gets one Confirm, and the dialog's footer is the one that
stays.** `Browser::answers_here` is the single predicate both surfaces read:
the sheet draws its tally and its footer exactly when it is true, and the
prompt slip draws no answer row and no pick hint exactly then. Without it a
player ticking a fetchland's target was shown "Bestätigen" twice on one
screen — once under the rows the answer is made of, once out on the slip —
and had to work out whether the two meant the same thing. They do; §6 of *A
Table You Want To Sit At* says a dialog is a place you **work**, so the
button belongs under the work. The slip's pick hint goes with it for the
same reason from the other side: for a `ChooseCards` it reads "click a card
on the board", and the board is behind a veil with nothing on it to click.

The predicate is `for_choice()` and **not** `dims_the_table()` — a choice
spanning this sheet and the hand is still sent from here, there being nowhere
else to send it from, even while the table behind it stays lit. And it is
narrower than the interaction's own `bounds()`, which is what the footer read
before: a graveyard opened *by hand* while the engine asks about the
battlefield is not that question's dialog, and grew a tally and a Confirm for
a question none of its rows could answer. Nothing else on the sheet narrows
with it — a row is drawn as selected because it *is*, whoever draws the send.
The keyboard is unaffected either way: `Action::Confirm` reaches
`Interaction::confirm` in `input::answer_the_question` and has never been
gated on a drawn button.

`hud::Z_STACK` through `hud::Z_PREVIEW` are the six numbers that order is
written in, in one place, for the reason the seat bars gave: a `ZIndex`
orders a node only among its own parent's children, so these mean something
only against each other. The prompt slip stands *above* the veil — dimming
the sentence that states the question would be the veil contradicting itself
— and the hover preview above the dialog, because a card held up to the
light is held over whatever raised it. They order nothing outside `HudRoot`:
bevy sorts root nodes by `(GlobalZIndex, ZIndex)` and only then walks each
subtree, so the seat bars are wholly below all six and the departing card,
the ability sheet and a life flash wholly above them.

Two things about the mechanics are worth knowing before touching them. The
drag is a `Pointer<Press>` that records *what* is held plus a per-frame read
of `Window::cursor_position`, **not** `Pointer<Drag>` — the duel HUD is a
retained tree rebuilt on every snapshot, hover and selection, so a drag chain
bound to the header entity dies when that entity is despawned mid-gesture
(the lobby can use the gesture because its tree is not rebuilt per hover).
And the geometry is kept *out* of `HudRevision` for the same reason from the
other side: a rebuild per pixel of a drag would make the sheet unusable. The
system writes the sheet's own `Node` and the in-memory settings; only the
release touches disk.

`HudRevision` did gain the window's logical size, which nothing in it
followed before — a HUD built for one size simply stayed that way through a
resize, latent everywhere the overlay reads `windows` and not latent at all
once the sheet is placed from a band whose height is the window's.

A drag cannot be proved through `dev-control`: `/pointer` presses and
releases in one call, so there is no frame in the middle with the cursor
somewhere else. `input::dragging` proves it headlessly on the sheet's own
`Node` instead. That test also found the first reason a test must not reach
the settings store: releasing the pointer wrote the developer's real
`~/.config/baylee/client-settings.json`, moving the sheet in their own client
by the delta the test had invented. The store is now shut in every test
process (§"In the browser", "Only the player's client opens that store").

The diagnosis is the part worth keeping. Reading the HUD for a full-screen
node that might be swallowing the ray found nothing — `HudRoot` has carried
`Pickable::IGNORE` since it was written. A temporary probe over `PointerHits`
answered it in one build instead:

```text
order=0.5  [772v25/ui3456x2104/d=0.00]   the full-window HUD node — reported, but
                                         neither hoverable nor blocking
order=0    [384v8/mesh-card/d=29.85]     the mesh backend, once it exists
```

Three readings, one build: whether the mesh backend produces hits, whether
anything above it blocks them, and whether `pointer_hover` consumes what
arrives. Before the plugin the second line was simply absent.

A card carries no `Pickable` of its own. That was tried and made no
difference — `MeshPickingSettings::require_markers` is `false` by default, so
every visible mesh with an `Aabb` is a target already. What the table carries
is the opposite marker: the felt slab, the generated table quads, the zone
recesses, the face-down pile backs, the counted-stack backs and every contact
shadow are `Pickable::IGNORE`, so nothing under a card can answer a click
meant for the card.

## The pointer only speaks when it moves

`Pointer<Over>` fires when the *card* moves under the pointer exactly as
readily as when the pointer moves over the card — and on this table the cards
are always moving: a repacked lane, a tap, a hover lift, a permanent arriving.
A pointer left resting anywhere near the board therefore re-pinned
`Duel::hovered` every few frames, and the keyboard cursor could not walk at
all. Measured at a live table: twelve `KeyD` presses with the pointer over a
hand card moved the cursor once; with the pointer parked over empty felt the
same twelve cycled the hand cleanly. A game that had been played that way for
half an hour then stalled outright — four legal land drops on offer and the
cursor pinned to a permanent that was not one of them.

So `pointer_hover` accepts `Over` and `Out` only in the few frames following a
real `CursorMoved`, and drains them otherwise. The keyboard wins a tie, the
mouse takes the cursor back the moment it actually moves, and the grace of a
few frames covers the gap between the cursor event and the picking pass that
follows it. This is what the keymap's commitment — every choice answerable
without a pointer — costs in practice: not a key that was missing, but a
pointer that would not stay quiet.

That silence has a cost of its own, and it took a photograph of a live game to
see it: a card can *leave* while the pointer rests on it. Playing the card
under the cursor is the ordinary way — the overlay is rebuilt whole, the node
the pointer was over is despawned, and Bevy fires no `Out` for an entity that
no longer exists. Nothing moved, so nothing spoke, and the preview of the card
you had just played stood over the middle of the table until you happened to
hover something else.

So the hover is held against the **kind of entity that reported it**
(`HoverSource::{Hand, Table, Elsewhere}`), re-checked every frame *before* the
grace window — the pointer's stillness is the problem, not a reason to stay
quiet. The source is what makes the check possible: a land goes on existing
under the same `ObjectId` once it is played, so "does this object still
exist" finds it on the battlefield and keeps the preview open. The true
sentence is that it is no longer *a hand card*.

The source is only sound because the system also notices when it is **not**
the author. `Duel::hovered` has four writers — this system, `move_cursor`, the
click on nothing, and Cancel — so `pointer_hover` remembers the value it left
behind and reads any difference as somebody else's write, which resets the
source to `Elsewhere`. Without that, the keyboard cursor walking off a
permanent and onto a hand card would meet a stale `Table` source, fail to find
itself on the table and be cleared on the very next frame: the same stall as
above, through a different door. `Elsewhere` clears only when the object has
left the hand *and* the table, and that union is the keyboard cursor's own
invariant rather than a weakening of the two above. The two kinds of hover
are valid for different reasons: a *pointer* hover holds while the pointer is
over the entity that reported it, a *keyboard* cursor while its object is
anywhere in `cursor_grid` — which spans the hand and every pod's lanes. An
`ObjectId` survives a zone change, so a card played off the cursor is still in
the grid, one row down, and `move_cursor` keeps navigating from it. Clearing
it there would drop the player's cursor, not a ghost.

**And the grace window is rearmed by the pointer, which is what let the
flicker back in.** `*grace = 3` on every `CursorMoved`, so a player *moving*
the mouse across their own lands holds it permanently open. What blinked there
was reported as "sometimes it flickers strangely", and the cause was not in
this file at all: **the hover preview was pickable, and a board card's preview
opens centred on the window — exactly where a player's own lands sit.** The
panel therefore landed on the pointer that opened it. `Out`, preview closed,
`Over`, preview back, for as long as the pointer kept the grace window alive.
The tooltip frame did carry `Pickable::IGNORE`; the card face inside it did
not, and `Pickable` does not inherit.

Standing still is the worse half of the same bug, and the one that made it
legible. With the pointer at rest the grace window expires mid-oscillation,
and the `Out` it discards is the only one there will ever be — the card has
left the hover map, so every later `Out` names the panel and matches no
`CardVisual`. The hover latches, and `/state` reports a card the pointer left
minutes ago.

Reading the picking code did not find this; photographing the table did.
`/state` said `hovered` never cleared, which reads as a picking failure and
sent two sessions after one. What settled it was a screenshot: an opponent's
permanent cleared perfectly and one of the player's own never did, and the
difference between them was not in their entities but in where each one's
preview panel opens. The rule that follows is worth more than the fix — **a
description of a thing must not be able to take the pointer from it**, which
is now one recursive `Pickable::IGNORE` in `hud/overlay.rs` rather than a
property each face is trusted to remember.

**That fix did not close the report, because the report had two causes.** The
owner played again and said the flickering remained, and the second one was
not a hover at all: `card_ui.wgsl` coated every card in a metallic sweep on a
continuous six-second loop, so the whole hand bar changed a little on every
frame with nothing hovered, nothing moving and nobody's turn. A 24-by-16 grid
of mean absolute pixel difference over two frames of a still table read 18–30
per cell across the hand and 0 across the felt, which is the shape of the
answer: a repositioning bug moves cards, and the cards had not moved. The
sweep is one-shot now (`sheen.rs`, `f8e4350`) and the same grid reads a
maximum of 2.4 with the hand at 0–2. The lesson is the one this section
already teaches, applied to a second reading of the same report: a fix that
was measured on the fault it *found* still has to be measured against the
sentence that was reported.

A guard on the `Out` was tried first and is worth writing down, because it
looked so much like the answer: a card still gliding is the thing that moved,
so drop its `Out`. That is wrong twice over. It fires *exactly* when a pointer
leaves a card mid-lift, which is the ordinary case rather than a rare one; and
the dropped `Out` is the only one there will ever be, for the same reason as
above. Deferring rather than dropping fails identically. It also never
explained the measurement, which is what eventually sank it.

The lift geometry was a **second** defect, found while chasing the first and
fixed on its own merits rather than because it caused anything: **a hovered
card's growth must cover its own rise.** A lift of `y` shifts the footprint
along the felt by `y * CameraRig::lean`; growth moves every edge out by half
the card's *smaller* dimension times `scale - 1` — smaller, because the shift
is in world space, always straight away from the viewer, while a pod is rotated
to face its own seat. While the second covers the first, a pointer inside a
card cannot end up outside it. `covered_lift` in `table.rs` is that bound, and
the `const _: () = assert!(…)` lines under it are what make a tuning session
fail to compile rather than fail on the table — the step from hover to
selected included, since that is a rise like any other. The shipped numbers
missed it by nearly double. Both lifts came down and both scales went up,
which is the better affordance anyway: a card that grows says "this one" more
plainly than a card that rises.

A lean that varies (above) then put a second edge on that bound, and took the
margin back. Every one of those assertions divides by the lean, so each is
hardest to satisfy at the *steepest* shot — and checking them at `CAMERA_LEAN`
after `DUEL_LEAN` existed was checking them at a shot the commonest desktop
case does not use. They are written against `STEEPEST_LEAN`, the larger of the
two, and that alone does not compile: at `DUEL_LEAN` the cap is exactly
`scale - 1`, because a card is one unit wide and 0.5 is precisely the lean at
which a rise stops being covered by its growth. The shipped lifts sat *on* that
line — 0.06 against a cap of 0.06, 0.12 against 0.12 — which is not a margin,
and in `f32` the first of them fell on the wrong side of it by one part in a
million. So the lifts came down a second time, to 0.05 and 0.10: what a steeper
shot has to buy is headroom, not another equality. Steepening the shot again
is now a compile error rather than a card that slides out from under the
pointer and flickers.

## Settings, and what belongs to whom

Two stores, split on one question: is this about the *player* or about *this
screen*?

`baylee_client_core::prefs::Preferences` is the player's — the keymap, the
phase rail, and the `AutoRules` switches. It follows the account: the client
`PUT`s it to the gateway (`docs/protocol.md` §"Client preferences"), which
keeps it as an opaque blob because knowing what a keymap is would mean
linking the client's brain. `crate::prefs::Prefs` holds it, and every change
goes through `Prefs::edit`, a borrow that marks the value dirty when it is
dropped — a `pub` field would let one caller forget, and the symptom would be
a setting that survives until the next restart and then silently reverts.
Writes are debounced, so dragging a slider costs one request rather than
twenty.

The rail on that screen has two **presets** above it (`RailPreset`), and they
are buttons rather than a mode: a preset writes twenty-four buttons and then
has nothing more to do with them. Its chip is drawn lit only while the rail
still matches, so the first correction by hand puts both chips out and neither
goes on claiming a rail the player has since edited. Which rows
`Competitive` keeps green is a rules question rather than a taste one — see
`docs/keyboard-map.md` §Automation for why a declaration step can never be one
of the red ones.

`settings::ClientSettings` is the screen's: preview size, interface language,
the text-view latch, and the gateway address. Those are properties of a
device, and putting them in the account would mean a phone and a desktop
fighting over one number.

A client that is *not* signed in still has all of it — an offline duel
against the house AI is played with the same keys — kept in the same local
file or `localStorage` as the client settings. Signing in replaces the local
copy with the account's, which is the only ordering that does not quietly
upload one machine's defaults over a player's real bindings.

The screen itself is `settingsui.rs`, drawn over the lobby rather than beside
it: coming back has to land exactly where the player left, including halfway
through a deck. It is its own module because `lobby.rs` is already the
largest file in the crate, and it borrows that module's `Metrics`, `Press`
and widget helpers so it looks like every other screen without a second copy
of any of them. `SettingsPane` is an enum rather than a flag plus an
`Option<Action>`, because "waiting for a key while closed" is not a state —
and a pair of fields would let it happen, with the symptom that the next key
pressed anywhere rebinds something.

Rebinding takes every key while a row is armed, including the ones that mean
something everywhere else: a player who wants `Esc` on some other action has
to be able to press it. Escape backs out, backspace unbinds, and unbinding is
a real answer because a pointer still reaches everything.

An unbound action is a row with no chords, and that difference is what lets a
new action reach players who saved their keys before it existed. A stored map
replaces the standard one whole, so `⇧E` (take a whole merged card, #210) was
dead on the first machine it was tried on: that machine's keymap had been
saved a week earlier and had no row for it. `Keymap::migrated` now gives each
*missing* row its standard chords, keeping only the ones no row in the map
already holds. An empty row is the player's decision and stays empty, and no
key the player bound is taken or shadowed.

## The interface's own words

Card text has been translated for as long as `/pool?lang=` existed — the
gateway reads it out of the catalog, field by field, and falls back to English
per field rather than per card. The *interface* was English and only English,
every button and status line a literal at the point it was drawn.
`baylee-client-core/src/i18n.rs` is the other half.

A `Phrase` is an enum variant, not a key into a table read at runtime, and the
`messages!` macro writes one arm per language for each: **a phrase with no
German fails the build**. A file of strings — RON, JSON, Fluent — answers a
missing key with a fallback, and a fallback is a screen that is half English.
The cost is that a third language is a sweep through one file rather than a new
file beside it, which is the right way round: the sweep *is* the work, and a
build that ships half of it is what makes it never get finished. `Phrase::fill`
substitutes `{0}`, `{1}` … left to right and leaves an unfilled placeholder
standing, because a visible `{2}` is a bug report and a silently dropped one is
a sentence that means something else.

Three rules are tests rather than conventions: every phrase answers in every
language, a phrase's placeholder *set* is the same in all of them — `{0}`
moving is what translation is, `{0}` vanishing is a bug — and **no phrase
fakes a plural with a bracket**.

A sentence whose subject is counted is written **twice** — once for exactly
one thing, once for everything else — and `Phrase::counted(n, one, many)`
picks between them; both languages split in the same place, so the number
decides and the language is never asked, and zero takes the plural in both.
`card(s)` and `Karte(n)` are not plurals, and the sheet's own typography
greys what a sentence says in brackets (`prose::bracketed`), so the broken
form was drawn as an editorial aside, in grey, beside the number it
disagreed with. A suffix would not have been enough either: German wants a
relative clause here — "Karte, die nach unten geht" against "Karten, die
nach unten gehen" — and the verb inside it agrees too, which only a whole
second literal can say.

The noun is also where a card choice says what it is **for**. `ChoicePrompt`
has six variants; the prompt bar read one, so a library search, a scry, a
put-back and a wish were four copies of "Choose 1 card". `choice_noun` gives
each its own counted pair, which fits inside the counting frame without a
second sentence: "Wähle bis zu 2 Karten, die nach unten gehen". `Delve` is
answered a line earlier (it is part of a cost, not a selection) and
`Generic` stays the plain noun, which is honest — the engine did not say
what it was for either.

A line that talks about **another chair** names it, and `i18n::seat_name` is
the single place that decides how. Four sentences want it — a zone browser's
tab, the player chooser's rows, "waiting for …", and a draw offer — and three
spellings had grown between them: `Phrase::SeatNumbered`, a developer's `#1`,
and a bare `PlayerId` printed straight into the sentence, which is how the
best the prompt bar could say at a table where everyone has a name was "Warte
auf Platz 1". The roster is an `Option` because `GameStatic` arrives once and
frames are drawn before it does, and a seat it does not describe is
**numbered, not dropped** — the sentence is about a chair that exists either
way. The viewing seat's own name is the other function, `own_seat_name`, and
no caller has to choose between the two: a line that says "waiting for" or
"offers a draw" is never about the seat reading it.

That is also what finally put the proposer into a draw offer.
`YesNoPrompt::DrawOffer` carries `proposer` and the line dropped it in a `..`,
so the question read "Ein Remis wurde angeboten. Annehmen?" — obvious at a
duel, and not a question anybody can answer at a table of four.

Who says what:

- The lobby's own status lines go through `Lobby::note`, which reads the
  language off the lobby. The shell has sentences too (`could not reach the
  table: …`), and says them with `Lobby::tell` / `unseat_because` so it never
  has to know which language it is in.
- The gateway's refusals (`{"error":"…"}`) are shown in the words the gateway
  sent. It is the gateway that knows why it said no, and translating those
  means a code beside the prose — a protocol change, and deliberately separate
  work. That is about the **lobby**, and it is the whole of what this bullet
  ever said: `ErrorBody` in `crates/baylee-gateway/src/main.rs` is one `error`
  field, so the code really is a field that does not exist.
- **A duel's refusals are a different channel, and eight of them are this
  client's own sentences.** Nothing the gateway says reaches the prompt bar —
  it forwards `SeatFrame` bytes it never decodes. Two writers reach that slot
  and they are shaped differently, which is the argument for the type having
  two arms. This client's half is a **closed set of eight**: one stale-deed
  line at six call sites, one cast-mode line, and the six a mana run gives up
  with. They were English on a German screen for as long as
  `Duel::last_error` was a `String`, which is a type that can hold prose and
  nothing else. The engine's half **cannot be enumerated** — three fixed
  lines, and four call sites of `error(reason)` forwarding whatever the rules
  kernel refused with — so there is no list there to translate.
  `i18n::Refusal` is the pair instead: `Said(Phrase)` for a sentence this
  client owns, translated like every other word on the screen, and
  `Verbatim(String)` for one another process sent, drawn as it came.
  `LedgeRevision::error` carries the `Refusal` unrendered and `sync_ledge`
  renders it, which is the shape `link_note` beside it already had.
  `Verbatim` is not a deficiency to drive to zero: an engine refusing for a
  reason this client has never heard of renders its English rather than
  nothing, so the two sides need no lockstep deploy. And should a *named*
  engine refusal ever want translating, that is not a protocol change either —
  `v1::Error` has carried a `code` field the whole time, hard-coded to `1` by
  both writers, so the wire is already there and what is missing is a
  taxonomy. The two claims are independent and both are written down, because
  either one alone leaves "it is a protocol change" available to be re-argued.
- Values that are also identifiers stay identifiers. A house AI's difficulty is
  `"sharp"` on the wire and in `SeatSpec`; only its label is translated, by
  `lobby::ui::ai_name`.
  The same line runs through the builder: `KINDS` is `(&str, Phrase)` because
  the key is matched against a printed type line, and `Action::group` is a
  `Phrase` because it is *also* the key the keymap panel groups by — a
  `Phrase` compares as itself in every language, where the English string
  would have been a key that changed meaning when the screen did.
- A whole sentence is one phrase, never a translated verb with a translated
  noun pasted on. `choose_line` takes the noun as an argument to
  `Phrase::ChooseUpTo` for exactly that reason: a count and a noun agree
  differently in different languages. It takes **both forms** of it and picks
  by `max`, because `max` is the number the noun stands next to in all three
  frames — "up to 2 cards", "1 card", "1–3 cards".
- `input.rs` asks for ability labels in English on purpose and says so: that
  path reads only each option's `action` — it picks by position or takes the
  only one there is — and never draws a label.

One setting feeds two readers. `ClientSettings.lang` is both the code the
catalog is asked for and, through `Lang::of`, the language the interface draws
itself in — `Lang::of` reads `de-DE` and `en_GB` by their first part and
answers English for anything it does not know, because a client that refused
to start over a settings file would be worse than one that speaks English. The
picker is a chip per language at the top of the settings screen, each naming
itself in its own words, and it is written to the store on the click: the
settings screen has no way out but a click, and a language that reverted on
the next launch would read as a button that did nothing.

### A card's own name is translated in one place

`Phrase` covers the words the client writes. A **card name** is not one of
them: it comes out of the catalog with the card's text, beside the sentence,
and for a long time only one thing ever read it. `CardFace::build` merged the
printed text over the projection and translated the name on the way past — so
anything drawing a whole card face got a German name, and the seven other
places that wrote a name got `PublicObject::name`, which is the compiled card
registry and is therefore always English. The stack panel is where that read
worst: a German sentence under an English title, on the same row.

The guard those seven needed already existed, inline in `build`. It is now
`card_face::describes` and `card_face::shown_name`, and it is the **clone
guard** rather than a lookup: a Clone carries its own cardboard into every
zone, so the printing beside it is the Clone's while the projected name is what
the object has become, and naming the object after its printing would be a lie
about what is on the table. `shown_name` compares against *both* the served
name and the English one, so an ordinary card matches whichever end asks.

`face::name_of` is the renderer's half — it finds the printing, which is the
part `baylee-client-core` cannot do because it links neither the catalog nor a
socket to fetch it over. For an **ability** that printing is the *source's*,
because an ability has no card of its own, and the face is the one the host
says the sentence is printed on rather than the one the source is showing now:
a Sheoldred who has turned back over while her chapter ability waits on the
stack must not lend that ability the other side's name. An ability whose
source has already left (CR 113.7a) keeps the projected name, because then
there is nothing else to call it by.

The seven callers are the stack panel's title, its `Ability · <name>`
subtitle and its target chips, the combat line's aim, the ends of a combat
tally, the zone browser's rows, and the ability sheet's heading. It stays one
function: a new place that writes a card name calls `face::name_of`, and if it
cannot reach a `PlayerView` it is drawing the wrong thing.

"Anything drawing a whole card face" had one exception, and it took a live
ability on the stack to find it. The **picture** beside a stack entry is built
by `face::of_object`, which merged the printed text over the object's own
`card` — and an ability has no card (`StackKind::Ability`), so there was
nothing to merge: the thumbnail fell back to the compiled registry's English
while the title above it, drawn by `name_of`, read German. `of_object` takes
the view for that one face alone, and finds the printing the same way
`name_of` does — the source permanent's, on the face the host says the
sentence is printed on. Picture and title are now named by one lookup instead
of two that agreed only for a card.

The lookup has to happen at draw time and not in `BoardModel`, and the reason
is one line up: `BoardModel` is built in `baylee-client-core`. A name is
therefore *not* part of what `HudRevision` compares — but `texts` is, counting
how many printings have text, so a name that turns German when the catalog
answers mid-game redraws with the sentence it belongs to.

### And the type line under it, which stayed English

The name was one of two things a translated client got wrong on the same card.
The **type line** was the other: "Nistende Falkentaube / Creature — Bird", in a
client whose catalog had been serving `Kreatur — Vogel` the whole time —
`baylee-catalog` sends `printed_type_line` where a printing has one, so the
right words were sitting in `CardText::type_line` and were thrown away.

They were thrown away by a guard that is right and was asked in the wrong
language. The printed line may only be drawn while nothing has **changed** the
object's types — an animated land *is* a creature and its own card would
contradict the board — and `build` asked that by comparing the printed line
against a line it built out of the projection. That comparison is only
meaningful when both are English. In German the word sets can never be equal,
so every card fell back, every time; in English it worked, which is exactly why
nothing noticed.

The question is not "do the two lines read alike" but "did anything change the
types", and that is a comparison of **bitsets**: `card_face::PrintedTypes` —
supertypes, types and subtypes as the compiled registry has them — against the
projected `Characteristics`. Exact, language-free, and no protocol change. The
renderer supplies it, because `baylee-cards` is not a dependency of
`baylee-client-core`: `face::printed_types` reads the `FaceDef` that
`of_object` and `of_hand` already look up for the mana cost.

`None` is the honest answer for anything the registry cannot be asked about,
and it keeps the projected line: a token has no printed type line to be right
or wrong about, and an ability on the stack borrows its source's *text* but not
its types. Changeling lands on the same side for a better reason — CR 702.73
sets every creature type, so the projected subtypes genuinely differ from the
printed ones and the card's own `— Shapeshifter` would be a lie; the fallback
collapses to `All creature types` as it always has.

The bug had a test named after it. `a_localized_type_line_survives_when_
nothing_changed_the_types` asserted the **name** and never the line, so it was
green throughout — a reminder that a test's title is not one of its
assertions.

## The game log (#262)

The host sends a seat its log beside the view, never inside it. Every
`StateDelta` carries a `LogTail`, the lines this socket has not been sent
(`docs/protocol.md` §"The game log (view version 33, #262)").
`baylee-client-core/src/gamelog.rs` is the client's half and needs no
renderer. `LogBook` keeps the lines and writes them in the reader's language;
the panel and the end screen draw what it writes.

**Every frame's tail goes into the book, with that frame's view**
(`LogBook::append`). A seat with more lines waiting than one frame carries is
sent several frames with the same view and `seq`. A client that reads the log
only from the frames whose view it takes misses lines. Empty `log_json` bytes
mean nothing new and are not a tail to decode. The book appends by `from` and
skips what it already holds, so a chunk sent twice, a snapshot that tells the
log from the start and a reconnect that tells it all again add only what is
missing. An empty tail changes nothing, wherever it says it starts: a question
asked again carries `{from: 0, entries: []}`, and a book that reset on
`from == 0` would empty itself on every refusal. A tail that starts past the
end is refused whole and counted (`gaps`); the next tail from 0 fills the gap.
A tail that disagrees with what the book holds is another game's log, because
the host never changes a line it has sent. It replaces the book from where the
two part, and the book counts it (`rewrites`). The fix for that is a fresh
`LogBook` per game, which is the renderer's to make.

**The renderer keeps one book per game, in `Duel::log`.** `HostMessage::View`
carries a frame's tail beside its view, and `poll_host` appends it before
`Duel::receive_view` takes the view, against that frame's own view.
`DuelCommand::Open` replaces the whole `Duel`, so every game starts with an
empty book; a reconnect keeps it. A frame that only carries the next part of
the log repeats the view the client holds. `receive_view` reads its edges
against the view it replaces, so a repeat strikes no blow, plays no sound and
reopens no reveal the player put away (`log_feed_tests`). Log bytes that do
not decode are dropped with a warning and the view is kept: the next snapshot
or reconnect tells the log again. `LocalHost` hands a refused answer back with
`Session::reask`, which carries no log.

**The book keeps entries, never sentences.** A line is written each time it is
read (`line`, `lines`, `lines_since`, `take_unread`), because two things under
it change mid-game: the language, and the card text, which arrives from the
catalog after the line that names the card. A card is named by
`card_face::shown_name` over the text the renderer's lookup returns
(`CardTextLookup`; a closure over `CardTexts::face` is one). That is the
catalog's name in the reader's language, else the English name the line
carries, so a copy keeps the name it shows. No name is empty: an empty catalog
name falls back to the line's, and an object with no name at all is "a card".
A card the seat may not see is "a card" and carries no handle. A face-down one
is "a face-down card" and points at its object on the table, as the view
already lets it. No line prints a handle.

**The reading seat is "you"**, in the verb's own form. Every sentence about a
player is written twice, the `…You` phrase and the other with the seat's name
as `{0}` (`i18n::seat_name`), because the verb agrees with its subject in both
languages. The "you" itself is an argument too, so a panel can set it in bold
like a name: `{7}` as a subject, `{8}` as a direct object, `{9}` as an
indirect one, which German says as "du", "dich" and "dir"; `Writer::phrase`
passes all three to every phrase. Every line is in the past tense (#300),
as the loss lines always were; a loss the end screen words without a "you"
to mark ("Your life fell to 0") is told in its words, the others in the log's
own `LogLost…You` phrases. No line ends in a full stop, as the loss lines
never did. A line that opens
with our own words ("a card") capitalizes them, and a seat's name keeps the
case its player gave it.

A `LogLine` carries:

- the sentence;
- when the host wrote it (`at`, Unix milliseconds; 0 when the host was never
  told the time), for a panel that shows the time;
- its `NameSpan`s, as byte range, object, card and registry token, for hover
  and preview; the card is the printing the view showed, so a preview wears
  the same finish;
- its `PlayerSpan`s, as byte range and seat, for every player it names: a
  seat's name wherever it stands, and "you" where the sentence says it to
  the reader, never "your". None overlaps a `NameSpan`;
- whether it opens a turn (`header`);
- the ability an ability line names, as the stack names it, so a panel can show
  the printed sentence;
- the seat the line is about (`subject`), for a panel that marks each line
  with its seat: the player the sentence names, "you" included, or whose card
  moved between zones. A line about the table has none: a spell countered or
  not resolving, counters, a block, damage to a permanent, a transform, day
  and night, a loop, and the end of the game, whoever won it;
- `times`, how often a folded line happened. A life total or counters folded
  into one line say 1, because their "was" already spans every change.
  `plain()` adds "(×N)" for a reader that draws plain text.

**The panel** (`hud::ledge::log`) is the tray's scroll button, `L`, and a
column that stands on the tray strip against the right margin, 360 by 420
where the window has room and the room there is where it has not. It stands
on the strip's top edge and not over the strip the way the game menu does:
the menu goes away on the next press, and the log stays up while the game
goes on, so it must not bury the zones button or its own. It is the shelf's
z-rung (`Z_LOG`), under a zone dialog answering a question and under the
menu. `Esc` shuts it after the menu and before the browser; a question never
does, and neither does a press outside it. The game's end shuts it, because
the end screen shows the whole log (below).

It reads every line when it opens, when the language changes, when card text
arrives (the catalog's generation) and when the book is rewritten. Otherwise
it appends only the lines that arrived since it last drew (`LogRevision`):
a line the host sent never changes, and redrawing thousands of lines for
each new one would write the whole log once per action. The list follows
its newest line until the player scrolls up (`LogFollow`), and lines that
arrive under a list scrolled up light a pill at its bottom that takes it
back. The scrollbar is Bevy's own (`ScrollbarPlugin`), so its thumb drags.
A line's sentence clips y on its own node (`TEXT_OWN_HEIGHT`). Bevy reports
a text's content as the text set one word to a line, and taffy carries that
into the list's scroll range: measured, the panel's range came to 776 pixels
against 620 of lines, and the list scrolled past its last line into blank
space. The clip stops the report at the row and clips none of the glyphs.

A turn's heading is a quieter line under a rule. Every other line is the
seat swatch, then the sentence with each name set one weight up and
"(×N)" in the quieter ink after a line that happened more than once. The
swatch is the colour of the seat the line is about (`LogLine::subject`):
`hud::seat_colour`, the one the seat bar's own swatch wears, so the reader's
lines are gold and another seat's are its team's. A line about the table
(a block, counters, day and night, the end of the game) keeps the swatch's
width and draws it empty, so every sentence starts at one edge
(`ledge::log::subject_ink`). The panel's words are `GameLog…` phrases,
because every `Log…` phrase is a sentence of the book's.

**The end screen** (`hud::finish::write_the_log`) carries the whole log
between the loss lines and the way out, under a "Game log" caption, in a box
at most 320 high that scrolls. It uses the panel's own rows
(`ledge::log::spawn_line`) in the sheet's inks (`LineInks`): parchment ink,
headings and "(×N)" in the slip's softer ink, and no seat swatch. Its colours
go on clear and rise with the veil (`finish::Settling`), like the rest of the
sheet. It starts at the top and does not follow its end, because it is read
from the start of the game and nothing arrives any more. It is built once:
the host sends every line before the game's last question (`Session::pump`),
so the book is whole when the sheet is drawn. A game that logged nothing gets
no box. The sheet is at most the window less `SHEET_AIR` high, and a short
window takes its room out of the box and out of nothing else. `hud::scrolls`
is the one input system that runs in `DuelPhase::Finished` as well, so the
wheel reaches the box. `settle_the_sheet` writes a colour only when it
changes, because a long game puts thousands of spans on the sheet.

Known limits, each a later view version:

- `Defender::Planeswalker` names the planeswalker by handle only. The book
  remembers each one's name from the view that arrived with the attack, and
  says "a planeswalker" for one it never saw.
- An ability line cannot tell activated from triggered.
- `CounterKind::badge` is English, so the counter nouns are written here.

## Embedding (the open-world plan)

`DuelPlugin` creates no window and no schedule of its own. An application adds
it, installs a host, and sends `DuelCommand::Open`; the duel takes the screen
and returns it on `Close`, reporting through `DuelReport`. `DuelSet::{Sync,
Input, Present}` let the host application order its own systems around it.

`Close` now really does hand the screen back: the 3D stage was always torn down
there, but the overlay was not, because nothing had ever closed a duel and come
back to something else. `hud::despawn_overlay` runs beside `table::despawn_stage`
and resets `HudRevision` with it — a revision describing a tree that no longer
exists would make the next duel's first frame skip its own rebuild.

## Where a packaged desktop build finds its fonts

`standalone::asset_root` takes the `assets` directory **beside the executable**
when one exists, and the crate's own `assets` directory otherwise. The second
is baked in at build time as an absolute path, which is what `cargo run` needs
from any working directory, and it used to be the only answer. On a player's
machine that answer points into the build machine's checkout: a release built
on CI would look for its fonts under the runner's workspace. On the owner's Mac
a packaged `.app` started identically with and without its bundled assets,
because both runs read the source tree (#216). `target/debug/` has no `assets`
beside the binary, so a development build still takes the baked path.
`BEVY_ASSET_ROOT` and `dev-reload`'s check on it are untouched.

The choice is logged once at info, `assets from <path>`, after `LogPlugin` is
installed. That line is how a smoke test tells the two apart, because both
builds start and look the same. Measured on 24.09.2026 with a copy of the
debug binary: with `assets` beside it the log names that directory, and
without it the log names the crate's directory. The true negative, where the
baked path does not exist either, is a CI-built artifact on another machine.

## In the browser

`trunk serve index.html --release` from `crates/baylee-client/` serves the
client on <http://127.0.0.1:8080> (the lobby, unless the page URL carries a
seat ticket; card art streams from the Scryfall CDN on first use, so the first
minute needs a network connection). Build for deployment with `trunk build
index.html --release` and host the resulting `dist/` statically. Two notes:
always build `--release` (a dev-profile wasm is ~350 MB vs ~36 MB optimized),
and the acceptance deck file is embedded with `include_str!` because a browser
has no filesystem.

Fonts are not embedded, and that is what `<link data-trunk rel="copy-dir"
href="assets">` in `index.html` is for. Bevy's asset server resolves
`fonts/Faustina.ttf` against `./assets/` in a browser exactly as it does
natively, but nothing puts that directory into `dist/` unless trunk is told
to — and the failure is silent: no error, just every glyph the client draws
rendering as nothing. The icons and the mana symbols are that; a native run
never shows it, because a native run reads the directory in place.

**Which gateway.** The page's own origin is only the right answer when the
gateway served the page, and a `trunk serve` build comes off `:8080` while the
gateway is on `:28766`. So `?gateway=http://127.0.0.1:28766` on the page URL
wins — and is remembered in `localStorage` under `baylee:gateway`, because a
browser drops the query string on the first internal navigation and a client
that forgot where its table was would be worse than one that never knew.
`settings::forget_gateway()` clears it.

The same missing filesystem is why settings take a second back end. Natively
they are a JSON file under `~/.config/baylee/`; in a browser the identical
JSON lives in `localStorage` under `baylee:client-settings`, scoped to the
origin the client is served from. Both are best-effort — a corrupt file, a
private window, a browser set to block site data — so `ClientSettings::load`
falls back to defaults rather than failing a launch.

**Only the player's client opens that store.** Both back ends answer nothing
and write nothing until `settings::open_store()` is called, and only
`standalone::run` calls it. A unit test, an integration test and a bench never
pass through `run`, so none of them reads or writes the player's settings,
preferences, hand-built offline decks or card-text cache, on any machine. The
door used to be `cfg!(test)` at two of five callers, which missed both ways:
`cfg(test)` does not reach a test under `tests/`, and `Prefs::local` had no
guard at all. A settings test passed on the owner's machine (their
`preferences.json` had `skip_empty_blocks: false`) and failed on CI, where
there was no file to read, and the same unguarded path saved. A test that needs
a starting state sets it, or asserts relative to what it found.
`settings::store::tests::a_process_that_never_opened_the_store_reads_and_writes_nothing`
is the proof, and goes red when the door is taken out.

## On a phone

One binary, three shapes. The lobby lays itself out from the window width in
three frames — phone below 760 logical pixels, tablet below 1180, desktop above
— and `Metrics` is the single place every size comes from: text, headings, tap
targets, padding, gaps. Breakpoints rather than a continuous scale, because
what changes is the *shape* (one column or two, a card that fills the width or
one that floats) and shape does not interpolate. Resizing inside a frame is
left to flexbox; crossing into another one rebuilds the tree.

Nothing meant to be tapped is under 44 logical pixels on a touch frame, the
smallest target a finger hits reliably. A phone drops what it has no room for
rather than shrinking it — the gateway address goes, the deck and table panels
stack, the top bar and the table rows wrap.

**Text entry is the hard part.** A canvas never raises a soft keyboard, which
would make the sign-in form unusable on a phone and would cost desktop web its
autofill, password managers, IME and paste. `src/softkeys.rs` keeps one real,
invisible `<input>` over the page: tapping a lobby field focuses it, the
browser does the typing, and the client reads the value back and draws it
itself. Invisible, never hidden — neither `display:none` nor
`visibility:hidden` can hold focus, and focus is the whole point. The field's
`FieldKind` picks the input type, the `inputmode` and the `autocomplete` hint,
so the phone raises a text keyboard marked `username` for the sign-in name and
the password manager knows which box is which. The keyboard is not raised on arrival, only when a
field is tapped, and `Lobby::focus_epoch` counts *placements* rather than
changes so tapping the field you are already in still opens it.

Where the platform owns the typing, the client's own key handling is skipped
outright — the browser's input has focus, so the canvas sees nothing, and
anything it did see would be entered twice.

`index.html` carries the rest: `touch-action: none` so a swipe is the game's
and not the page's, `overscroll-behavior: none` against iOS rubber-banding, no
tap highlight, and deliberately *no* `viewport-fit=cover` — the browser then
keeps the canvas inside the safe area on its own, and the client needs no notch
arithmetic it has no way to test.

The duel's own overlay is still written in fixed pixels and has not had this
pass yet.

## Driving the client without its window

`crates/baylee-client/src/devctl.rs` is a loopback HTTP harness that presses
keys, moves and clicks the pointer, dumps what the client believes, and saves a
screenshot — all while the window sits behind everything else on the desktop.
It exists because the alternative is bringing a window to the front, pressing a
key by hand and looking at it, which is neither repeatable nor available to
anything automated.

```bash
BAYLEE_DEV_CONTROL=28770 cargo run -p baylee-client --features dev-control
curl -s localhost:28770/health        # {"ok":true,"frame":1183,"width":1728,"height":1052,"scale":2}
curl -s localhost:28770/state         # the view, the pending choice, the interaction
curl -s -XPOST localhost:28770/pointer   -d '{"x":864,"y":655,"press":true}'
curl -s -XPOST localhost:28770/pointer   -d '{"x":509,"y":966,"press":true,"hold":true}'
curl -s -XPOST localhost:28770/pointer   -d '{"release":true}'
curl -s -XPOST localhost:28770/key       -d '{"name":"Space","shift":false}'
curl -s -XPOST localhost:28770/scroll    -d '{"y":-6}'
curl -s -XPOST localhost:28770/screenshot -d '{"path":"/tmp/table.png"}'
curl -s -XPOST localhost:28770/timescale -d '{"speed":0.1}'
curl -s -XPOST localhost:28770/pause     -d '{}'
curl -s -XPOST localhost:28770/step      -d '{"frames":6}'
```

Seven things about it are load-bearing.

**It is a compile-time feature, not a runtime switch.** A remote-control socket
inside a game binary is a cheat vector, and the only guarantee worth having is
that the code is absent from the shipped build. `BAYLEE_DEV_CONTROL` being
unset is the second lock and loopback the third, never the first.

**Keys are written into `ButtonInput<KeyCode>`, not synthesised as OS events.**
That is both simpler and *more* faithful: `keys.rs` reads exactly that
resource, so an injected press travels through the account's `Keymap` like any
other — and focus stops mattering, which is the whole point. What does not
stop mattering is the **card cursor**: `KeyA`/`KeyD`/`KeyW`/`KeyS` are
`Action::CursorLeft/Right/Up/Down` and are the only thing that sets `hovered`
without a pointer. A harness that never presses one leaves `hovered` at
`None`, where `Enter` ("the card under the cursor, else pass") rightly does
nothing and `Space` has nothing to confirm — which is indistinguishable from
a dead screen, and at a cleanup `DiscardChoice` is indistinguishable from a
hung game.

**A click is five frames, and this is where the first version was wrong.**
Bevy's picking backend does not read `ButtonInput` at all: it reads
`WindowEvent` messages, keeps the last cursor location in a `Local`, and only
turns a press into a `Pointer<Click>` once a press and a release have landed on
the same hovered entity. Setting `Window::cursor_position` and pressing the
resource in one frame therefore answered `{"ok":true}` while nothing whatsoever
was clicked — the screenshot after the click was byte-identical to the one
before it. `/pointer` writes a `CursorMoved` on the frame it arrives, the press
and the release on frames of their own, mirrored into `WindowEvent` exactly as
`bevy_winit` does.

Two of the five came later, and each is a fault this harness had been
reporting as silence. **The move is written twice**, a whole frame apart,
because a single one is sometimes simply lost: measured with a click helper as
a pointer put on Palace Jailer and `hovered` read fifteen times running as
`None`, where sending the *same* move again named the object on the first
read. Repeated reading never repaired it and repeated sending always did,
which is the shape of an event that did not arrive rather than a state that
had not settled — so the harness sends it twice and stops guessing. **And the
answer waits one frame past the release**, which is written on the frame
*before* anything reads it: a caller that clicked Sheoldred's Edict and then
asked `/state` was shown `armed: None`, with the arming standing there a
moment later and no second click sent. A harness that answers before its deed
has been read has every measurement off by one.
`a_click_is_the_move_twice_then_a_press_a_release_and_a_frame_to_read_it` is
the sequence as a test.

A bare move rides the same machine — sent twice, answered on the third frame —
because the lost move was a bare one, and a `/pointer` whose answer does not
mean *the pointer is there* is the one call a caller cannot build on.

**A press can also be held, and that is a different tool.** `{"hold":true}`
presses and stops there; `{"release":true}` is the call that lets go, and the
answer says which of the three it did (`"clicked"`, `"held"`, `"released"`) so
a script cannot mistake one for another. Each of those two implies the press
it is a stage of, which this paragraph claimed before it was true: a body that
said only `hold` fell through to the bare-move return and answered
`{"ok":true,"clicked":false}`, a `200` that reads like a press that happened
— on the one route whose purpose is photographing what lives *between* a
press and a release. `devctl::button_deed` is where that is decided now, and
`a_held_press_does_not_have_to_say_press_as_well` holds it along with the two
halves that make it safe: a bare move stays a bare move, and `release` still
wins over `press`. It is the pointer's half of the pair
`/key` has always had, and it exists because press and release in one call
cannot photograph anything that lives *between* them: a drag, which this
document used to record as unprovable through the harness, and a card giving
way under the finger. That second one is what it was built for — a held press
on a hand card measured its top edge six physical pixels lower and its face
dimmer, and the screenshot after the release was byte-identical to the one
before the press.

**A wheel is written twice, for the same reason a click is.** `/scroll` sends a
`MouseWheel` *and* the `WindowEvent::MouseWheel` beside it, because it is
picking that turns a wheel into the `Pointer<Scroll>` a list listens for, and
picking reads the window event. Written only as the plain message, the wheel
reached everything except the lists. It lands wherever `/pointer` last put the
cursor, the way a real wheel picks the list under it — and without it the
harness cannot reach a control below the fold, which is how a lobby pager stays
untested.

**Coordinates are logical pixels, screenshots are physical.** `/health` reports
`width`, `height` and `scale` so the ratio between the two is read rather than
guessed; on a Retina display a guess is wrong by a factor of two.

**The position is dealt, not played into.** `BAYLEE_DEV_SEAT_BOARD`,
`BAYLEE_DEV_SEAT_HAND` and `BAYLEE_DEV_SEAT_COMMANDER` are semicolon-separated
lists of card names, each optionally prefixed `<seat>:`, that fill a seat's
`starting_battlefield`, its opening hand and its command zone before turn one
— a semicolon because a comma is part of a card's name far too often. A
singleton in a ninety-card deck is not something a game reaches on request,
and ten turns of the offline duel put four lands and no creature on the table,
so anything about how a card is drawn or clicked would otherwise be
unprovable. The board and command variables append; the hand variable
**replaces** the deal, because a `starting_hand` is the whole hand.

The third one names a card no amount of playing can reach: a seat either
started with a commander or it did not. What it does **not** claim is that a
commander was unphotographable before — a deck names its own, and the
acceptance file's Allytifact sits down with General Tazri in the command zone
whether or not the variable is set. What it buys is *which* commander: the
measurement that wanted one wanted it castable on turn one out of a dealt
board and cheap enough to copy, and Tazri costs six.

All three are behind the `dev-control` feature — a shipped binary that seats
cards from the environment is a cheat — and all three are loud on a name they
cannot find, because a typo that quietly dealt nothing turns "this does not
happen" into a conclusion about the code. `host::deal_the_dev_board` is the
reader and `decks::deal_named` does the dealing, which is what keeps a board
dealt here from being a different board than the gateway's `dev-table` deals.

**The clock is a lever, because almost nothing worth photographing here waits.**
A card's exit lives 0.55 s, a sheen sweep less, and one `/screenshot` round trip
is a frame plus a file write — so the harness could prove an animation had
*finished* and never that it had happened. `/timescale` sets
`Time<Virtual>`'s relative speed, `/pause` stops and starts it, and `/step`
lets it go for a fixed number of frames and answers once they have run. Three
things make that the right lever rather than a knob per system. The whole
picture reads through the virtual clock — `table::glide`, `table::retire`, the
sheen clock and every shader's `globals.time` all take `Res<Time>`, which bevy
sets from it each frame — so a tenth speed slows the parts of one movement
together instead of pulling them apart. `pump` counts frames rather than
seconds, so the harness keeps answering while the picture is stopped. And a
step is counted in **frames**, because the request after a step is always a
screenshot and a screenshot is a frame; asking for a tenth of a second would
leave the caller to work out how many frames that was and get a different
answer on a different machine. Zero is refused by `/timescale` rather than
taken as a pause: a caller who could stop the clock two ways would have to
remember which one to undo. `/health` reports `speed` and `paused` for the same
reason it reports the scale factor — a harness that reconnected would otherwise
have no way to ask what it had left running.

**And `/step` cannot photograph anything shorter than a quarter second**, which
is the one trap in that lever and looks exactly like an animation that does not
exist. `Time<Virtual>` clamps its own delta at `max_delta`, 0.25 s by default,
and the first frame after a pause is handed that clamp rather than the
sixteen milliseconds a running frame would carry. So a `/pause` followed by
`/step {"frames":3}` advances the picture by a quarter of a second at the first
of the three — and the zone sheet's flight into the tray, which lives 0.16 s,
was over before the second. Measured on 19.09.2026: the sheet stood in one
screenshot and was off the tree in the next, with three frames between them and
no intermediate position to photograph at all.

`/timescale` is the route for anything under that, and it is not a worse one:
at a twentieth speed the same 0.16 s flight is 3.2 s of wall time, which is
four unhurried `/screenshot` round trips with the picture still moving between
them. Reserve `/step` for what it is good at — a counted number of frames
through something that lasts, and the frame-exact before-and-after of an edit
to a shader. A caller who reaches for `/pause` first and finds nothing moving
should suspect this before concluding the code does nothing.

`/state` is deliberately the *client's* answer and not the engine's — the view
it last received, beside the interaction state it built from it. A disagreement
between the two is exactly the class of bug the endpoint exists to show, and
one a screenshot cannot report.

Four of its fields are there because they answer silently. `outbox`,
`mana_run` and `ability_menu` all look exactly like "the key did nothing" — an
action queued but never sent, a mana run that owns the next few keys, an
ability chooser that swallows the keyboard whole. `departing` is the opposite
problem: it counts the cards playing their way off the table, and they are gone
in half a second, so a caller that wants to photograph one has to be told when
to look. It also answers a question no screenshot can — whether the exit path
ran at all — and that is what it was added for. Three runs failed to catch a
graveyard sink before `departing` said, flatly, that the count never left zero;
the cause was that a card going to a pile is never stale in the first place.

`cast_menu` and `cast_answer` are one more of the same, read at its two ends.
The cast chooser swallows the keyboard exactly as the ability sheet does, so
without the first a standing chooser is indistinguishable from a click that
did nothing. The second is the way the player picked, and it travels through a
whole mana run before it is spent — several round trips with nothing on the
screen to say so — which makes it the only way to tell "the evoke was chosen"
from "the engine picked for us again", the distinction §"Which way to cast it
is asked before anything is tapped" exists to make.

`shelves` is the fifth, and it answers a different kind of question again: not
a state that hides, but an *arithmetic* a picture can only ever suggest. A
seat's bar is placed by projecting its mat's ledge corners rather than by a
layout pass, so "the bar is in the wrong place" is a claim about numbers — the
centre the box is hung on, the projected length and depth of that ledge, and
the `ink` the depth has to be able to hold — and the route reports the numbers
the placement was actually made from, in the same logical pixels `/pointer`
takes. A mat's shelf whose `ink` is close to its `depth` is a bar about to stand on
the creature lane behind it. It was added to settle exactly that question: a
duel's bar reported `mid_y` 605.9 over a depth of 49.0, while the screenshot
put the drawn ledge at 560..611 and the bar's ink at 591..620 — so the drawing
follows the model to the pixel, and it is the projection that disagrees with
the mat under it.

`cards` is the sixth, and it exists because of what driving this client
actually costs: **finding a card to click**. It is every card drawn on the
table, with its object, the name the board model gives it, and
`at_x`/`at_y`/`w`/`h` in the same logical pixels `/pointer` takes. `at_x` and
`at_y` are the box's **centre**, not its corner, here and in `buttons`: send
them to `/pointer` as they come. A driver that added half of `w` and `h` to
them clicked beside `Keep` and sat in the mulligan for six minutes (#284). The
box is measured from the transform `glide` has the card at *this* frame, and
through the card's own four corners put through that transform — so a card
mid-flight reports where it is rather than where it is going, and a tapped
permanent reports the wider, shorter box it really covers. The height a card
is drawn at is not part of it: `CARD_LIFT` moves a card 0.14 px at a duel,
which is why aiming at the felt underneath has worked all along. Before this,
a click meant three lookups — the object out of the view, the lane out of the
board, the pixels off a downscaled screenshot — and all three again after the
lane repacked.

`buttons` is the same answer for the HUD's controls: the prompt bar's answers
by name (`Yes`, `No`, `Keep`, `Confirm`, `DeclareNothing`, `Step(1)`) and the
ability and choice rows by index, which is the handle those two carry
themselves because both are rebuilt from the current `LegalActions` when
pressed. A pointer harness needs a button's position the way a keyboard one
needs its action, and reading it back beats measuring it off a screenshot for
the same reason `cards` does.

An ability row also says what it reads: `words` (the whole sentence the row
draws, cost and all, or the one-line name of a row the card prints nothing
for; `null` for neither), `head` (the cost column as drawn) and `source`,
where the words came from: `localized` (the player's printing), `oracle` (the
compiled English), `token` or `none` (the CR 305.6 tap, a grant whose
grantor is hidden, a sentence the count guard refused; also a pour pip, which
draws a colour and no words). A prepared cast reports the spell's name and
text and where they came from; a grant its grantor's name and sentence, and
where the sentence came from. All three go through the sheet's own doors,
`cardtext::said`, `abilities::printed_words`, `abilities::prepared_words` and
`abilities::grant_words`, so the field cannot say German while the row draws
English. Before them, "is this row
localised" was a screenshot and a reader of German (#212).

It was built, though, on a claim that turned out to be false — that a yes/no
question has no keyboard answer at all, `docs/observed-faults.md` 36, since
withdrawn. `Y` and `N` answer one. What did not answer was `POST /key
{"name":"Y"}`: the harness spells a letter `KeyY`, and a refused key is a
`200` with an error in it. The field is kept because it earns its place
either way, and the story is kept because the next tool built to get past a
wall is worth asking that question about first.

`sources` is the newest, and it is here because the field beside it was a
number that could refuse nothing. `reachable` is a *count* — how many cards in
hand this client is offering to tap lands for — and when it comes back lower
than the board says it should, the count cannot tell you whether the planner
refused the costs or never saw a land to pay with. Those are two different
faults in two different crates, and on #127 the pair cost three round trips
and four probe arms to separate. `sources` is the list the count is derived
from: one row per permanent the client believes it may tap, with the object,
which action taps it (`Intrinsic` for the engine's CR 305.6 shortcut,
`Ability(n)` for a printed one), the colours it may make and how much. It is
built through `manasources::sources`, the same call `fn reachable` makes, so
the endpoint cannot photograph a list nothing acts on.

It is `null` rather than `[]` when no priority question is standing, and the
difference is the whole point: an empty list is the client saying it has
nothing to tap, and `null` is the client not having been asked. Read beside
`interaction.pending.Priority.legal`, the two answer the question this route
exists for in one read — the engine's enumeration on one side, what this
client made of it on the other, and a shortfall attributable to exactly one
of them.

`armed` and `interaction.assignments` are the last two, and both answer a
question that looks like silence. `armed` is the tap that has been made and
not sent: there is no undo in the engine, so the first tap on anything
irreversible only arms it, and a caller that does not know that reads the
*second* tap as the one that did nothing. `assignments` is the pairs a combat
declaration is being built from, beside the `focus` they are aimed at —
necessary because `selected` is **empty** in both combat modes, an attack and
a block being pairs rather than a set. A caller watching `selected` watches a
whole declaration go together and sees nothing move.

`focus` is combat's own, and `aim` is the same question asked of every mode
that has an answer to it — including the row a dialog's keyboard is standing
on, which `focus` is silent about because `Interaction::combat_focus` returns
`CombatFocus::None` outside the two combat modes. Both are reported rather
than the second in place of the first: `CombatFocus` says whether the thing
aimed at is a defender or an attacker, and `Pick` drops that. Without `aim`,
proving that a key moved the focus inside the zone browser takes two
screenshots and a pixel diff, which is what it took the once.

## Editing a shader without stopping the game

`--features dev-reload` puts bevy's embedded-asset watcher behind every shader
registered with `embedded_asset!` — all eight in `src/shaders/`, `arrow.wgsl`
included — so saving `felt.wgsl` repaints the table in the
client already on screen — no rebuild, no restart, no reconnect, and the game
keeps its position. Together with `/pause` it is how a look is worked on: stop
the picture, edit, look, edit again.

```bash
BAYLEE_DEV_CONTROL=28770 BEVY_ASSET_ROOT=$PWD \
    cargo run -p baylee-client --features dev-control,dev-reload
```

`BEVY_ASSET_ROOT` is not optional and the binary refuses to start without it,
which is the interesting part. `embedded_asset!` files each shader under the path
`file!()` gives it, and cargo writes that relative to the **workspace** root;
bevy's watcher strips its *own* base path off every changed file before looking
it up, and that base is `CARGO_MANIFEST_DIR` — this package, two directories
deeper — unless `BEVY_ASSET_ROOT` overrides it. Every lookup would miss, and
the failure is completely silent: a client that watches, notices, and reloads
nothing. A hard stop naming the right value is the only honest answer, since
the reload is the whole feature. Nothing else changes: the asset root proper is
already an absolute path, and an absolute join replaces the base rather than
extending it.

`BAYLEE_DEV_CONTROL` is on that line for a reason of its own and is the piece
most easily dropped: `--features dev-control` builds the harness in but opens
no socket without it, so a launch missing it hot-reloads perfectly and has
nothing to stop the picture with — and stopping the picture is half of what
the line is for.

The proof is the counter-test, because "the picture changed" is worth nothing
on a table that animates on its own. Pause the clock, screenshot twice, and the
two files are byte-identical; then turn `FELT_CLOTH` magenta and the felt is
magenta in the next screenshot; then put the colour back and the frame returns
byte for byte. The middle step is the claim, and the two outer ones are what
make it a measurement.

`--features dev-dylink` is the neighbouring feature and it is documented here
mostly to stop it being rediscovered: it links bevy as one shared library, and
on this crate a one-line edit to `main.rs` rebuilds in 2.0 s statically against
1.6 s through the dylib, for a two-minute first build. The workspace's
`[profile.dev] debug = "line-tables-only"` had already taken the link cost this
would have saved.

Both features were written up as impossible at bevy 0.19.1, with a resolver
error to prove it — `bevy_dylib ^0.19.1` and `notify-debouncer-full ^0.7.0`
each "could not be selected". Both crates had been on crates.io the whole time.
What had stopped was this machine's registry index cache, a year stale for
those two entries and never revalidated, so an error naming crates.io was
describing one laptop. Cargo's manifest carries the longer version of that;
the short one is that a resolver failure against a crate that plainly exists
is a claim about the local index until a `curl https://index.crates.io/…` says
otherwise.

## Playing it in a browser

A `wasm32-unknown-unknown` check says the client *compiles* for a browser. It
says nothing about whether it runs in one, and until 16.09.2026 nobody had
played a game there. Two things were broken, neither of them in the renderer,
and no gate this repo has could have seen either.

**Every font failed to load, so the browser client drew no text at all.**
Bevy's asset server asks for an `<asset>.meta` sibling before the asset
itself, and this repo ships none — natively that is a 404 and the default
meta is used. A static host for a single-page application answers an unknown
path with `index.html` and a **200**, so bevy read a page of HTML as a RON
`AssetMetaMinimal`, failed, and failed the *asset* with it. Measured against
`trunk serve`: `GET /assets/fonts/AlegreyaSans-Regular.ttf` is 200 and
263 804 bytes, the same path plus `.meta` is 200 and `text/html`, and all
eight fonts error. What that looks like from the outside is the part worth
keeping: the felt, the mats, the cards and their Scryfall art all draw
perfectly — those come over HTTP and never through the asset server — and
every glyph the client renders itself is simply not there, life totals,
button labels and prompt bar included. `AssetPlugin::meta_check` is
`AssetMetaCheck::Never` now, set for every platform rather than behind a
`cfg`: it is what a repo with no `.meta` file means everywhere, and a browser
is only where it was noticed.

**The gateway answered no CORS at all, so nothing it said could be read.**
The page is a `trunk serve` on :8080 and the gateway is not — that is the
whole reason `?gateway=…` exists — so every request the client makes is
cross-origin. The two halves fail differently, which is why the test checks
both: a request carrying `Authorization` is preflighted and was never sent,
because `OPTIONS /auth/login` was answered `405 Method Not Allowed` (axum
resolves a path to the methods a handler registered, and none registers
`OPTIONS`); a plain `GET` is a *simple* request that was sent, answered, and
then discarded by the browser for want of an `Access-Control-Allow-Origin`.
The gateway now carries one middleware that answers every preflight `204` and
puts `Access-Control-Allow-Origin: *` on everything. `*` is a decision:
this gateway authenticates with a bearer token in a header and sets no cookie
anywhere, so a cross-origin request brings nothing ambient with it and a
stranger's page can reach these routes as an anonymous client and no further.
`Allow-Credentials` is therefore never sent, and
`crates/baylee-gateway/tests/e2e_cors.rs` asserts its absence beside the
wildcard — the two are only defensible together.

**The page is not in fullscreen, and never asks to be.** It looks like it:
`index.html` gives `html`, `body` and the canvas `height: 100%` with
`overflow: hidden`, so the game occupies the entire viewport with no page
around it. Measured in Chrome, `document.fullscreenElement` is `null` and the
console carries no gesture refusal, and `Window::set_maximized` — which the
native build calls — is an explicit no-op in winit's web backend. So a
browser window that fills the screen is Chrome's own window doing it. Framing
the canvas is a change to that stylesheet and to nothing in Rust.

## Verification

- `cargo test -p baylee-client --test duel_flow` plays real games headlessly
  through the client's own path (host → view → board model → interaction).
- The wasm CI job type-checks all five crates that must keep compiling for
  `wasm32-unknown-unknown` — `baylee-core`, `-protocol`, `-view`,
  `-client-core`, `-client`. The browser-only paths (settings storage,
  entropy) compile nowhere else, so without that job they rot silently. It
  ran as a second workflow of its own until 15.09.2026, which meant
  `baylee-client` was checked twice on every push and the copy outside
  `ci.yml` was outside its `concurrency` group as well — so a superseded
  push cancelled the whole gate *except* the wasm half, which ran to
  completion on a commit nobody was waiting for.
- Browser entropy is the `wasm_js` feature on `getrandom`, and nothing else.
  It used to need a `getrandom_backend` cfg in `.cargo/config.toml` beside it —
  `getrandom` 0.4 dropped that value from the ones it declares, so the flag
  stopped selecting anything and was removed. A stale rustflag is worse than
  no rustflag: it reads as load-bearing and is not, and cargo says nothing
  about a cfg value a dependency no longer knows.

### Dock extension and discard feedback (September 2026)

The occasional drawer uses the ledge rail’s mineral material,
champagne tooling and ivory text, with an open foot joining the rail. Its
material handle is separate because its measured dimensions differ. Forced
hand-size discard keeps a live selected/required count in that extension;
selected hand cards carry checkmarks and the confirmation names the number
of cards to discard instead of saying “OK”.

Ambient sky changes retain the slow six-second crossfade. An active game
day/night designation overrides the ambient preference and settles in about
1.2 seconds, including a warm dawn/dusk between endpoints. Reduced motion
switches immediately. With no designation, the ambient preference applies.


### Printing identity and animated finishes (September 2026)

Print references are local to a game. Closing a duel now clears print-keyed
textures, their arrival/failure state, preloads, and both UI and board material
caches. Previously a second game could reuse the first game's image for the
same numeric print reference, including in the hand. URL-keyed deck previews
and the shared card back remain independent of that reset.

Both card shaders use the same artwork-aware finish function. Foil shifts its
interference hue with sampled pigment and a slowly travelling light; etched
highlights image contours and filtered metallic grain. Both catch light along
the rounded edge, preserve dark ink and bright text boxes, and honor reduced
motion. The same material path covers the hand, board, duel hover preview,
printing picker, deck thumbnails and builder hover preview.
