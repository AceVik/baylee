# Game Client (M5)

Bevy 2.5D duel client. Three crates, split by what can be tested without a GPU.

| Crate | Contains | Depends on a renderer? |
|---|---|---|
| `baylee-view` | The wire view: projected characteristics, per-seat filtering | no — not even the rules kernel |
| `baylee-client-core` | Table layout, board model, interaction state machine, image policy | no |
| `baylee-client` | Bevy plugin: 3D stage, overlay, input, texture cache | yes |

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

Under the cards, everything is generated rather than shipped:
`baylee-client-core/src/tabletop.rs` computes the felt, the medallion and a
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
viewing seat is gilt, matching the medallion's rings, so "mine" is the one
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

The medallion inlaid at the centre is the colour wheel, in the arrangement
every player already has in their head — so it is orientation as much as
ornament, and it sits on the one patch of felt no seat ever plays on. Around
it, `tabletop::hearth` paints a pool of lamplight with a ring of faint arcs
and tick marks inlaid in it — one texture, because they are one thing to look
at, and because a table with nothing between the seat mats reads as an
infinite green plane however good the grain is. The pool is candlelight and
the inlay is gilt; a test asserts neither ever goes cold, since a blue light
over a green table makes colour identity a guess.

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
hairline rather than two, the pool is half as strong and the medallion's glows
are dimmer.

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
when the seats, the focus or the window change — stopping the moment the
player has aimed the camera themselves.

The inversion is exact rather than tuned, which is why it is arithmetic and
not a magic number per screen size. With the eye at distance `D`, the lean
`L` = `CAMERA_LEAN` and `C = 1/√(1+L²)`, a felt point `s` units from the look
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
**medallion alone**: that is the colour wheel, the one thing on the table
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
`input::camera_controls` no longer has anything to refuse to run under.

The felt was too dark as well, and that was real: it was authored at about a
quarter of the brightness it needed, and
`the_felt_is_dark_enough_to_read_cards_against` passed every run because it
only ever bounded the bright end. It bounds both now.

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
table (a seat's mat at `ZONE_LIFT`, the glow under it, the medallion) and
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
every blended surface down there — the mats, the table quads, the medallion,
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

Identical permanents draw as one card with a `×N` badge. Two independent
guards keep that honest:

- objects merge only when every visible property matches (name, P/T, damage,
  counters, tap state, controller);
- objects with individual identity never merge, however identical they look —
  attacking, blocking, enchanted, equipped, or targeted by the stack.

Each seat also gets a text chip row (`12× 1/1 Soldier · 3× Treasure`) and a
one-line threat read (power ready, blockers, open mana, cards in hand), which
is what makes an unfocused pod useful at eight seats.

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
(`PlayerAction::SetStandingAnswer`), which is why it deliberately says nothing
about a particular game: a gateway can store *"always say yes to Ondu Cleric's
rally"* against an account and replay it into the next one.

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
both, and `codegen --check` is what keeps the table from going stale. The unit
is a **face**, because abilities are per face (`abilities_for_face`, and a
back face never inherits) while `AbilityRef` carries no face: whoever looks a
line up supplies it, and for a stack entry that is *not* the face the source
object is showing — see below. And the English sentence *count* travels beside the index
(`AbilityLine::of`), because the index is resolved against a text that may be
an older printing or a translation that joins two lines — an index merely out
of range is caught by anyone, but one that is in range and off by one is shown
to the player as precise text, which is worse than `+1`.

It does not reach every ability, and the misses are honest: 318 of the pool's
327 stack-capable abilities know their sentence, the rest being abilities
printed as a keyword (echo, evoke, station), a quoted sub-ability inside a
copy sentence, and a saga threshold row. `cargo run -p xtask -- ability-lines`
is the report that names them; `baylee_cards::lines`' tests are the floor.

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

The arithmetic is what forces it rather than taste. The panel is
`max_height: 62%`; a full row is about 113 logical pixels and a laptop leaves
about 598 after the title, so five uniform rows fit and the sixth is clipped
with nothing to say it was — and a stack of ten is an ordinary storm turn. One
full row and six compact ones fit the same space, and what still does not fit
is *counted* on a last line (`+3 more`). Under the title sits one more line
the prompt slip cannot carry: whose answer the table is waiting for, from
`PlayerView::priority`, and nothing at all while the stack is resolving and
nobody holds it.

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
`toggle` — nothing earlier in that chain is ever true of an object on the
stack — and the picture inside it is `Pickable::IGNORE`, because a pickable
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

The mark is drawn in the card's **top-left** corner, in the crest's alphabet
and at the crest's weight: a filled disc for a token, two offset cards for a
copy. Both still, for the crest's reason — a permanent stops being a copy only
by ceasing to be that permanent (CR 400.7). It costs the crest the strongest
form of its own argument, which was that putting exactly one thing on the top
edge let the silhouette alone answer "is that a commander"; what is left is
that the crest is centred and this is hard against the corner, so their
*positions* separate them once both have collapsed to pips. Round against
rectilinear is what separates the two glyphs at that size, and a card may
honestly wear a crest and a mark at once. It costs about the first two
characters of the *printed* name, which a printing puts hard against the
card's left edge — a real toll, taken because the printed name is the one
thing this client repeats everywhere else and the mark is said nowhere.

Proved on a running table rather than argued: Llanowar Elves, a Spark Double
that entered as a copy of it and a Rite of Replication token of it, drawn side
by side — no mark, two cards, a disc.

The bits are `cardmat::glow::TOKEN` and `::COPY`, the first two in that word
above the rail's twelve-bit field. `glow_of` reaches the registry itself here
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

**Both schemes have to be built in.** Bevy registers `http` and `https` as two
asset sources behind two separate cargo features, and a scheme with no source
does not fail the way a missing file does: the request never leaves, the load
never settles, and `textures::Failure` never hears about it. The table draws
constructed faces on grey slabs and looks like a slow network. The workspace
therefore enables **both**, and `textures`'
`a_card_picture_can_arrive_over_either_scheme` is what stops the pair being
trimmed to one — because a development gateway is `http://127.0.0.1:28766`, so
`https` alone means every picture disappears the moment a client signs in and
adopts the mirror, while an unsigned-in client on the CDN goes on looking
perfectly healthy. That asymmetry is what made it look like a card bug for a
week.


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

Art is the texture; the *finish* and the keywords are the shader. One material
(`cardmat::CardMaterial`, one WGSL file shipped inside the binary with
`embedded_asset!`) draws all three, because a foil that is also indestructible
is one card and not three draws, and a board of three hundred permanents can
afford one pipeline.

Materials are shared on a `CardLook` — art, finish, glow — which is exactly
what the shader draws differently and nothing more. Forty plain Islands stay
one material; a foil Island is a second; an Island the rules have made
indestructible is a third until it stops being one.

**The finish comes from the print table, never from the card.** `GameStatic`'s
print table is per seat, so a printing a seat has not earned resolves to
`None` and is drawn plain — a hole rather than a foil. Reading the finish off
the card instead would be a hidden-information leak with no game object to
hide behind.

**The glows come from `PublicObject.keywords`**, which is already projected —
the layer system has run, so a creature that gained indestructible this turn
glows this turn. `cardmat::glow_of` is the one gatherer for a `PublicObject`,
wherever it is drawn — battlefield, stack, command zone, tray or preview;
inside it `glow_bits` narrows the engine's `u128` to the bits the shader
reads, and a test pins each one against `KeywordSet`, because that numbering
is generated and a card glowing for the wrong keyword would be a rules lie a
player would believe.

The hand bar is the one caller that cannot go through it, because a
`HandObject` is not a `PublicObject`, and it contributes exactly one bit at
its own call site: `glow::COMMANDER`. Nothing else in that word is true of a
card in a hand — the keyword sheaths say what is protected *on the
battlefield*, and a hand that glowed with them would be claiming something
that is not yet so.

What a card *is* and what it can *do* are drawn in different places, and that
separation is the whole grammar:

- **The border says what the card is.** Indestructible is the *base*:
  darksteel, a hard blue-grey with a specular line, the card made of something
  rather than lit by something. Hexproof and shroud are *films* over that base
  — a green fog holding things off, or the same idea taken further, colder and
  denser, since not even its controller may target it. Base × film composes, so
  an indestructible hexproof creature is steel under green and neither claim is
  lost. Two films would not compose, and never have to: `glow_bits` drops
  hexproof whenever shroud is present, because shroud already forbids every
  target hexproof forbids (CR 702.18a against 702.11b) and the green film
  would be advertising a permission the card does not grant. That
  normalisation lives in Rust rather than in WGSL so that it is unit-tested
  once and both shaders inherit it.
- **Depth is a register of its own: a fact about the card may reach in, an
  offer or a deed stays on the rim.** The films used to sit in the same
  `BORDER` band as everything else, and the owner read the result as what it
  was: a border. A permanent on the felt is about 94 physical pixels wide, so
  that band is five of them — a five-pixel green line that blinks on and off,
  which is not what hexproof is. It is something *held around* the card. So
  the films now fall off as `exp(-d · WARD_REACH)` rather than stepping to a
  width: any `smoothstep` to a width still has a hem, and the hem is the part
  that reads as a stroke. The fog is at full density at the edge, still better
  than half of it where the old band ended, a fifth at the art's own edge, and
  nothing worth drawing a third of the way in. The steel keeps the thin band,
  because metal has an edge — and so do the travelling invitation, the armed
  ring and the indigo price, which are all things a player could *do* rather
  than things the card *is*.

  Hexproof and shroud are told apart on four axes and hue was never one of
  them on its own. Hexproof is two coarse noise octaves whose time term
  advances along `d`; `d` is zero at every edge and grows inward, so the plus
  sign carries the wisps *outward*, which is the direction the claim is about,
  and the minus sign would draw a card soaking it up. Shroud keeps its fine
  grain drifting across the whole card as a sheet. Coarse roll out of the edge
  against fine sheet drift, 6 cells against 14, green against cold blue,
  0.55 against 0.65. Nothing about it is a lamp: there is no light in this
  scene and there cannot be one, so the fog is arithmetic on the card's own
  colour. The obvious alternative — a bigger quad behind the card with the
  silhouette punched out — was refused for three reasons, and the decisive one
  is that the felt is green (`FELT_CLOTH` is `(0.071, 0.223, 0.150)`), so a
  halo outside the card would land on the one surface hexproof's hue has no
  contrast against. It also buys no width at this card size and could only
  ever be built for the table, leaving a card in the hand looking like a
  different card. Measured live with a board of eight warded lands: 76/50/30
  peak per channel over 1.6 s inside a warded card, **0/0/0** on an
  opponent's plain land in the same pair of frames, and the green excess over
  the red channel falling from about +30 at the edge to +1 in the middle.
- **The face says what the card can do.** A creature with summoning sickness
  (`glow::SUMMONING_SICK`) is drawn asleep, over the art and never on the
  border. It is not a keyword — it is a fact about *this turn* — and putting
  it on the border would make it read as one. The bit is set only for
  creatures, because summoning sickness is visible on nothing else.

  What "asleep" is drawn as is a **white balance and a blanket** (`SLEEP_*`,
  written out in both card shaders and compared by a test). The face goes
  cold under a moon, and a soft veil lies heavier at the foot of the card than
  at the head, its upper hem rising and falling on a five-second breath. It
  used to be a uniform four-percent luminance dip, and that is nothing on art
  whose own luminance varies by forty points: the two channels a face has
  spare are *colour cast* and *shape*, and the old drawing used neither.
  Asleep is not disabled, so desaturation — which is what reads as "greyed
  out" — stays a minority of the effect at 22%, and the body (power,
  toughness, marked damage, counters) is composited after this block and stays
  crisp, which draws "still blocks perfectly well" for free. The cast is
  bounded: red stays red, green goes teal, white goes coldest, and pushing it
  further would start deciding a card's colour identity for it, which is the
  one thing an unlit stage exists to protect.

  A cast alone was still ambiguous with the art under it — a blue creature
  drawn cold looks like a blue creature — so the night has a second half:
  **slow rings spreading from the middle of the card** (`SLEEP_RING_*`), the
  splash a thing that has only just landed is still settling out of. They are
  the one part of the drawing that moves *across* the face rather than along
  one axis of it, which is what no art can be mistaken for. Water rather than
  roots or frost of the three shapes it could have taken: roots would have to
  be organic shape, and shape on the face is how a creature *type* reads,
  while frost would be crystalline and the border already spends hard
  blue-grey on indestructible. A ring is neither — a luminance swell tinted
  with the same moon, so colour identity survives it. Five crests to the
  corner, chosen against the smallest card the table draws (previewed at
  60, 106 and 220 pixels wide), one leaving the middle every 3.4 s, which is
  clear of every other clock a card can wear. Measured on the running client:
  over two seconds the sick card moves 38 levels per channel while an
  untapped-neighbour control moves 0, and the change varies by 36 levels
  *within a single row* — which is the ring rather than the blanket, since a
  blanket is constant along x.
- **The perimeter says what is on offer.** `glow::ACTIVATABLE` rides in the
  same word but is deliberately *not* in `KEYWORD_BITS`: it comes from
  `LegalActions` rather than from the card, and is drawn as a warm light
  travelling round the border rather than as a material for exactly that
  reason (see "Tapping lands for a spell"). It is added on top of any sheath
  instead of averaged into it, because the two are answering different
  questions and both have to stay legible.
- **And the perimeter also says what has been decided.** Two more bits share
  that register, and the difference between them and `ACTIVATABLE` is motion.
  `glow::ARMED` is the card an armed deed is waiting on (see
  `docs/keyboard-map.md` §Arming): a bright ring pulled in tight against the
  printed edge, breathing in place and **not** travelling, because the offer
  has already been accepted and a light that still moved would say it was
  still a suggestion. `glow::WILL_TAP` is what that deed would spend — the
  sources of an armed mana `Run` — cool where the other two are warm, and a
  beat behind the armed card, because the price follows the verb. `glow_of`
  drops `ACTIVATABLE` on an armed card rather than drawing both: one border
  carrying a chase *and* a ring would be saying the same thing twice with
  nothing left to read the difference from. `Offer::on` answers both from a
  `CardGroup`'s **members** rather than its representative — a plan taps one
  particular Forest and the card drawn for it may stand for four — and both
  are *any* where `CardGroup::activatable` is *all*, because that rule exists
  to stop an offer inviting a click that gets refused and these two invite
  nothing.
- **The rail says what the card does in combat.** Eleven keywords — flying,
  first and double strike, deathtouch, haste, lifelink, menace, reach,
  trample, vigilance, defender — are marks along the bottom edge, one slot
  each, always in the same order (`client-core/src/cardrail.rs`). They are
  marks and not more paint because paint cannot *count*: a creature can carry
  six of these at once, and six colours mixed into one border is one colour
  that says nothing. Hexproof and indestructible are deliberately absent —
  the band already says them, and a mark repeating a sheath would be the same
  claim twice in two languages. The marks shrink rather than spill, so eleven
  keywords are eleven coloured pips where six are six pictograms; that
  degradation is the honest one, since a rail that ran off the card or hid its
  tail would both be lying about the creature.
- **The top edge says whose deck this is.** A commander (CR 903.3) wears a
  crest: one crown on the card's top edge, centred, in the rail's own slot and
  inset so it reads as a twelfth glyph in the same alphabet. It is *not* a
  twelfth rail slot, and the difference is why it has its own corner — the
  rail is eleven equal combat facts a player counts, and being a commander
  would not sort among them. It is an identity: true in every zone, for the
  whole game, before an attack is ever declared.
  The top edge is the one region nothing else claims (the rail and the plate
  share the bottom, the chips climb the right), so the silhouette alone
  answers the question at table distance, long before the crown resolves; a
  test asserts it reaches neither. It is plain `INK` with no accent of its
  own, because every hue here is already spoken for — the chips tint by
  counter kind, the felt by seat — and it does not move, unlike every rail
  mark and every offer light, because those all say something that could stop
  being true and this cannot. It reaches the hand bar too, which is where it
  earns its keep: a commander that declined CR 903.9b's replacement is sitting
  in the hand looking like any other legend.
- **The corner says what the card *is* in numbers.** The fifth of the bottom
  edge the rail has been reserving since it was written now carries a plate:
  a creature's power and toughness, or a planeswalker's loyalty behind a gilt
  rim. `client-core/src/cardplate.rs` decides what it says and packs it into
  one `u32` — three ten-bit numbers and two kind bits — that rides the
  material key beside `glow`, so a creature dealt three damage becomes a
  different material and the corner redraws with no second pass. Marked
  damage is the plate **filling from the bottom** to `damage / toughness`
  rather than a third numeral: what a player needs off a blocked creature is
  how close to lethal it is. The numerals are a 4×6 stencil sampled
  bilinearly, because there is no text on the 3D table and projecting a UI
  numeral onto a card would have to chase its rotation, its lift and its place
  in a stack every frame. The plate is drawn whether or not the card has art —
  a card that could not load its scan is the one a player can least afford to
  guess the body of.
- **And the counters stand above it.** Up to three chips run up the right edge
  from the plate's band, each a flat stamped disc: die pips to six, numerals
  from seven, and a fourth kind collapsing to `+N` rather than vanishing. Which
  counter a chip is has exactly one channel left at that size — its colour —
  so `Chip::tint` lives in the model where a test can reach it, and naming the
  counter is the badge tooltip's job. Two more `u32`s on the material key,
  because a tint and a count four times over do not fit in one. A **saga** is
  the exception that takes the plate instead: a square parchment page with the
  chapter in roman numerals, and its lore counter then draws no chip, because
  the page is already saying that number. `Corner::of` decides plate and chips
  together for exactly that reason, and `Corner::of_object` does the same for
  the hover preview — which drew the *printed* numbers until it did, so a 2/2
  under an anthem was a 3/3 on the table and a 2/2 in its own preview.

The pictograms live in a third shader file, `card_common.wgsl`, together with
the printed corner both shaders cut at: it is everything the table and the
overlay have to agree about. It contains no bindings and no bevy syntax at all
— every shader-global it needs, the time and the colour underneath, arrives as
a parameter — which is what lets bevy compile it as an imported module, what
keeps it clear of the two different bind groups the two shaders read `globals`
from, and what lets the naga test parse it on its own. Which mark a fragment
is inside is found by walking the eleven bits to the k-th set one: a loop
bound at compile time, no dynamic indexing, and inside the GL budget like
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

The bottom-right fifth of the card is left empty on purpose —
power/toughness and the counter dice are going there, and a rail that had to
move once they arrived would move on every card in every screenshot ever taken
of this client.

The border is drawn *inside* the card, over its printed frame. The mesh is
exactly the card, and a glow that needed room around it would need every
layout in the client to leave room for it.

**The corners are cut twice, at the printed radius, in two different ways.** A
Scryfall scan is a rectangle: the card's rounded corner is in the file as
white paper, and drawn untouched it is the single most obvious way for a card
to look like a photograph of a card. On the table the mesh is already rounded
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
rather than answering the camera. A card in hand carries the finish but no
keyword glow: the border tells a player what is protected *on the
battlefield*, and a hand that glowed would be saying something that is not yet
true.

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
of that for free — phase zero *is* the mean — which is the breath of summoning
sickness, the hexproof sheath and the armed ring. Where the term also carries
a *spatial* phase the freeze is a real frame rather than the average, and that
is still what is wanted: an indestructible border rests with its catch-light
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
- **`Retry`** (`baylee-client-core/src/reconnect.rs`) is the schedule and
  nothing else: 0.5 s, doubling to a 15 s cap, twelve dials, then `exhausted`.
  Renderer-free and transport-free for the same reason the lobby's decisions
  are — a schedule that can only be exercised by disconnecting a real gateway
  is a schedule that is never tested. It is allowed to back off at all because
  the engine's decision clock does not run for a seat with no socket
  (`docs/protocol.md`), so nobody is losing a game on time while it waits.
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

Running out reports `DuelReport::Unreachable`, once rather than once a frame.
Its own variant, because the gateway's `Error` envelope carries the engine's
refusal of a *single action* through `DuelReport::Failed` — a shell that
returned to the lobby on every `Failed` would eject a player for a misclick.
Nothing reads `DuelReport` yet; `Unreachable` exists so that whatever does can
match on it rather than on prose.

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
*stub*, which is a different thing from pretending they are fine. The whole pool arrives once per session and every
filter — text, colour identity, type, mana value, sort — runs locally, so
search answers at keystroke latency and never at the gateway's.

**One row per card, in every language.** The pool sends the card, not its
printings, and each row carries `alt_names` — every name that card is printed
under, anywhere. So a German player types "Blitzschlag" and finds the row a
deck stores as "Lightning Bolt", and finds it *once*: a list that repeated the
card for each of the forty sets it appeared in would be answering a question
nobody asked.

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
`manaui.rs` draws them with the OFL-licensed Mana font (`docs/legal.md` §2; no
WotC artwork anywhere). The font gives a monochrome mark only, so the coloured
disc behind it is the client's, which is also what makes hybrids drawable: a
hybrid has no single glyph, so the pip is one disc with two glyphs clipped to
opposite halves. Generic costs run out of glyphs at 20 and fall back to digits
rather than drawing the wrong number.

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
for floating mana before any tap, then for the *least* flexible source that
fits, so the Forest pays the green pip and the Command Tower is still untapped
afterwards.

Three rules keep it honest, and they are the reason to read the module before
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

Knowing that ability 2 of a Command Tower makes mana takes the compiled card
registry, which `baylee-client-core` deliberately does not link, so that half
lives in `baylee-client/src/manasources.rs`. It refuses an ability that costs
mana to activate (the plan would have to recurse) and one that does anything
besides make mana (the player should decide about that themselves).

In the hand, this is a third state and it is drawn as one:
`BoardModel::from_view` takes an `Openings { playable, reachable, activatable }`
rather than one set, because they are different claims — gold is the engine
saying yes, indigo is this client offering to tap lands first. Clicking either
casts; the difference is what happens in between.

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
option activates on the click that found it; several open a chooser in the
prompt bar, on its own row, because these are not answers to the pending
choice and a mana ability does not belong next to the button that ends the
turn. The chooser sends by *position*: the list is rebuilt from the current
`LegalActions` when the button is pressed, so a bar drawn a frame ago cannot
send an ability the engine has since withdrawn.

The chooser is also answerable without a pointer, which it was not: it opened,
took the keyboard hostage and let go of it only for `Esc`. Confirm reached
`Interaction::confirm` — which during priority means *pass* — and the cursor
keys walked the table behind the open menu. It now owns the keyboard while it
stands: the cursor keys ring through the entries, the primary key or confirm
activates the highlighted one, cancel puts it away, and the entry the keyboard
is on is drawn as the chosen one so the two ways of answering are visibly the
same menu. The list is rebuilt from `LegalActions` on the key as well as on
the click, for the same reason.

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
had no way to find out. There is a line under the headline now, and the cards
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

**Which of the two long edges is two questions, and `is_local` is the seam.**

The local seat's bar is on the **near** edge of its own mat — the bottom of
the screen, always. That is the owner's decision and it is not derived from
anything: what a player reads about themselves belongs between their board
and their hand, where their eyes already are, and the table gives up the
symmetry of one rule for every seat to put it there.

Every other seat's is the viewer's question, not the seat's:
`SeatSlot::ledge_is_outer` falls through to `facing.cos() < -SIDE_SEAT_TILT`,
and the whole of that is that a bar is drawn *above the board it describes on
the one screen there is*. Table `+y` runs away from the camera, so a seat
across the table takes the outer edge, behind its land row. A seat exactly at
the side of the ring is a tie — its mat runs up and down the screen and
neither edge is above anything — and keeps the centre-facing edge it had.

Both halves used to be the other way round, at the centre-facing edge for
every seat, so a bar always stood between its owner's board and the hearth.
That is the reading from each seat's own chair and it is perfectly coherent;
it is not what anybody sees. Two players put their bars back-to-back across
the middle of the table and the opponent's sat *under* their creatures, which
is not "above the battlefield line" for the one person at the table with a
screen. Then the local seat left the rule again, the other way: above its own
creatures is still the deepest point on the screen that belongs to it, and
the bottom of the screen is not.

`the_local_bar_is_the_nearest_ink_at_its_own_seat` measures the duel rather
than restating the rule: every one of the local seat's three lane centres is
further from the camera than its own shelf, every one of the opponent's is
nearer than theirs, and the two shelves are at opposite ends of the table.

**The lanes keep their order; only where they start follows the shelf.** The
three run from the centre-facing edge outwards at every seat — creatures
nearest the middle of the table, lands at the back — because that is where
the cards stand and a card does not turn round because the ink did. So
`MatParams::ledge_outer` is a flag rather than a flipped `uv.y`: flipping the
uv would move the lane veils with the shelf and put the brightest of the
three behind the lands. What the flag *does* change is `lane_center`'s
`front`: a seat whose shelf sits on the centre-facing edge starts its lanes a
`MAT_LEDGE` past it, and a seat whose shelf sits on the near edge — which is
now every local seat — starts them at the edge itself and gives the near
strip up instead. Either way a lane is exactly as tall as it was before the
bar existed, and no card is drawn where the ink is.
`seat_mat` takes the same flag and
`a_flipped_shelf_takes_the_other_end_and_leaves_the_lanes_alone` reads both
mats — it has to read the shelf's *fence* rather than its veil, because a
`Texture` is eight bits a channel and the shelf's 0.0060 and the quietest
lane's 0.0080 are the same 2/255.

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
rather than the first, because the number that decides the ledge is the
*shallower* of the two. Those two are about a tenth apart (55.0 against 61.1
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
duel's 1165-pixel ledge, where the tiles reach their cap long before the ends,
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
fixed number of pixels: the twelve tiles kept the width the ledge projected at
the moment the tree was built, and a camera still easing towards its home —
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
shelf. It also made the bar opaque, and the ledge is crossed by things the
table draws — a combat line to the far seat, a card lifting under the
pointer, a permanent falling in from `ENTRANCE_RISE`. Twelve solid chips
floating over all of them is most of what "it floats over stuff" was.

So the rare state is the marked one:

- **The ground is the standing order.** A stop is `PARCHMENT` at 0.14 with
  its glyph at full ink (6.99:1 on that ground); a skip is ink at half alpha
  on bare cloth (3.75:1 over the measured ledge, which is above the 3:1 a
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
or night has exactly one of the two from that point forward (CR 731.1), so
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
tile and the digit in an ability row's roundel are bold; the ability's own
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
poison — is **not here**, and not for want of trying: `SeatView` carries
`has_lost` and no reason for it, and deriving one from the last view's life
totals would be the client deciding a rules fact, which is the line
`CLAUDE.md` draws. It needs a field on the view and a `VIEW_VERSION` bump.

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
answering nothing. `overlay::spawn_menu_row` exists so that "not once the
game is over" is one `if` at the call site rather than an indent around
seventy lines. Measured at the same corner across the same concession: the
pill patch reads (54, 61, 68) while the game is on and (18, 20, 29) once it
is over, which is the veiled night sky with nothing in front of it.

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
and drawing one is one, and a bounce with no draw is none. And counters count
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

## The zone browser is a dialog, which is a different material

`docs/redesign-proposal.md` §1.3 draws the line and §6 applies it:
**parchment is a sheet you read from, a panel is a place you work in.** The
browser was parchment, and a grid of ten card columns, on the argument that a
graveyard is something a player *reads*. It is not — §6 puts a checkbox, a
tally and a Confirm on it, and that is work — so it is a dark panel in its
own warm near-black (`palette::DIALOG` and the four inks beside it; the HUD's
older `PANEL` is a cool near-black and is the one surface in this client that
was never on a candlelit table).

What is in it is a **list**, not a grid: a row is a checkbox, a thumbnail, the
name, the cost in pips, the type line and a badge saying which pile it is in.
The grid grew sideways to show more at once, which is the axis that buys
nothing for a card's three facts, and the list grows down, which is where a
hundred-card library is. The chosen row goes **candle** — a wash rather than a
fill, because a chosen row is still a row being read — and never the teal §1
retires; `the_dialog_says_nothing_in_teal` reads the file back.

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

The same opening pins the tab. `Browser::locked` is beside `tab` rather than
inside it because the two answer different questions — `tab` is what is
showing, `locked` is whether the player may change it — and it is decided by
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
is srgb8 (28, 25, 19) and the veiled baize measures (15, 29, 26), so the
panel is the darker of the two in green and brightness cannot separate them.
Temperature can, and the veil is the same blue-black the hand bar's own
ground already is. The alpha was measured on screen either side of one
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
`Node` instead. That test also found the reason `ClientSettings::save` is a
no-op under `cfg(test)`: releasing the pointer wrote the developer's real
`~/.config/baylee/client-settings.json`, moving the sheet in their own client
by the delta the test had invented.

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
along the felt by `y * CAMERA_LEAN`; growth moves every edge out by half the
card's *smaller* dimension times `scale - 1` — smaller, because the shift is
in world space, always straight away from the viewer, while a pod is rotated
to face its own seat. While the second covers the first, a pointer inside a
card cannot end up outside it. `covered_lift` in `table.rs` is that bound, and
the `const _: () = assert!(…)` lines under it are what make a tuning session
fail to compile rather than fail on the table — the step from hover to
selected included, since that is a rise like any other. The shipped numbers
missed it by nearly double. Both lifts came down and both scales went up,
which is the better affordance anyway: a card that grows says "this one" more
plainly than a card that rises.

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

Two rules are tests rather than conventions: every phrase answers in every
language, and a phrase's placeholder *set* is the same in all of them — `{0}`
moving is what translation is, `{0}` vanishing is a bug.

Who says what:

- The lobby's own status lines go through `Lobby::note`, which reads the
  language off the lobby. The shell has sentences too (`could not reach the
  table: …`), and says them with `Lobby::tell` / `unseat_because` so it never
  has to know which language it is in.
- The gateway's refusals (`{"error":"…"}`) are shown in the words the gateway
  sent. It is the gateway that knows why it said no, and translating those
  means a code beside the prose — a protocol change, and deliberately separate
  work.
- Values that are also identifiers stay identifiers. A house AI's difficulty is
  `"sharp"` on the wire and in `SeatSpec`; only its label is translated, by
  `lobby::ui::ai_name`.
  The same line runs through the builder: `KINDS` is `(&str, Phrase)` because
  the key is matched against a printed type line, and `Action::group` is a
  `Phrase` because it is *also* the key the keymap panel groups by — a
  `Phrase` compares as itself in every language, where the English string
  would have been a key that changed meaning when the screen did.
- A whole sentence is one phrase, never a translated verb with a translated
  noun pasted on. `choose_line` takes `Phrase::NounCards` as an argument to
  `Phrase::ChooseUpTo` for exactly that reason: a count and a noun agree
  differently in different languages.
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
so the phone raises the address keyboard for an e-mail and the password manager
knows which box is which. The keyboard is not raised on arrival, only when a
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
other — and focus stops mattering, which is the whole point.

**A click is three frames, and this is where the first version was wrong.**
Bevy's picking backend does not read `ButtonInput` at all: it reads
`WindowEvent` messages, keeps the last cursor location in a `Local`, and only
turns a press into a `Pointer<Click>` once a press and a release have landed on
the same hovered entity. Setting `Window::cursor_position` and pressing the
resource in one frame therefore answered `{"ok":true}` while nothing whatsoever
was clicked — the screenshot after the click was byte-identical to the one
before it. `/pointer` now writes a `CursorMoved` on the frame it arrives, the
press on the next and the release on the one after, mirrored into `WindowEvent`
exactly as `bevy_winit` does, and answers the caller only once the release is
out. `devctl::tests::a_click_is_a_move_then_a_press_then_a_release` is that
sequence as a test.

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

`shelves` is the fifth, and it answers a different kind of question again: not
a state that hides, but an *arithmetic* a picture can only ever suggest. A
seat's bar is placed by projecting its mat's ledge corners rather than by a
layout pass, so "the bar is in the wrong place" is a claim about numbers — the
centre the box is hung on, the projected length and depth of that ledge, and
the `ink` the depth has to be able to hold — and the route reports the numbers
the placement was actually made from, in the same logical pixels `/pointer`
takes. A shelf whose `ink` is close to its `depth` is a bar about to stand on
the creature lane behind it. It was added to settle exactly that question: a
duel's bar reported `mid_y` 605.9 over a depth of 49.0, while the screenshot
put the drawn ledge at 560..611 and the bar's ink at 591..620 — so the drawing
follows the model to the pixel, and it is the projection that disagrees with
the mat under it.

`cards` is the sixth, and it exists because of what driving this client
actually costs: **finding a card to click**. It is every card drawn on the
table, with its object, the name the board model gives it, and
`at_x`/`at_y`/`w`/`h` in the same logical pixels `/pointer` takes. The box is
measured from the transform `glide` has the card at *this* frame, and through
the card's own four corners put through that transform — so a card mid-flight
reports where it is rather than where it is going, and a tapped permanent
reports the wider, shorter box it really covers. The height a card is drawn at
is not part of it: `CARD_LIFT` moves a card 0.14 px at a duel, which is why
aiming at the felt underneath has worked all along. Before this, a click meant
three lookups — the object out of the view, the lane out of the board, the
pixels off a downscaled screenshot — and all three again after the lane
repacked.

`buttons` is the same answer for the HUD's controls: the prompt bar's answers
by name (`Yes`, `No`, `Keep`, `Confirm`, `DeclareNothing`, `Step(1)`) and the
ability and choice rows by index, which is the handle those two carry
themselves because both are rebuilt from the current `LegalActions` when
pressed. A pointer harness needs a button's position the way a keyboard one
needs its action, and reading it back beats measuring it off a screenshot for
the same reason `cards` does.

It was built, though, on a claim that turned out to be false — that a yes/no
question has no keyboard answer at all, `docs/observed-faults.md` 36, since
withdrawn. `Y` and `N` answer one. What did not answer was `POST /key
{"name":"Y"}`: the harness spells a letter `KeyY`, and a refused key is a
`200` with an error in it. The field is kept because it earns its place
either way, and the story is kept because the next tool built to get past a
wall is worth asking that question about first.

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

## Verification

- `cargo test -p baylee-client --test duel_flow` plays real games headlessly
  through the client's own path (host → view → board model → interaction).
- The wasm CI job type-checks `baylee-client` for `wasm32-unknown-unknown`.
  The browser-only paths (settings storage, entropy) compile nowhere else,
  so without that job they rot silently.
- Browser entropy is the `wasm_js` feature on `getrandom`, and nothing else.
  It used to need a `getrandom_backend` cfg in `.cargo/config.toml` beside it —
  `getrandom` 0.4 dropped that value from the ones it declares, so the flag
  stopped selecting anything and was removed. A stale rustflag is worse than
  no rustflag: it reads as load-bearing and is not, and cargo says nothing
  about a cfg value a dependency no longer knows.
