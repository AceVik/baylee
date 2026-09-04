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
22° off vertical is the compromise, and it is also why there is **no sky
behind the table**: at that angle the top of the frame still looks 48° below
the horizon, so a backdrop with a horizon in it would render nothing.

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

The medallion inlaid at the centre is the colour wheel, in the arrangement
every player already has in their head — so it is orientation as much as
ornament, and it sits on the one patch of felt no seat ever plays on. Around
it, `tabletop::hearth` paints a pool of lamplight with a ring of faint arcs
and tick marks inlaid in it — one texture, because they are one thing to look
at, and because a table with nothing between the seat mats reads as an
infinite green plane however good the grain is. The pool is candlelight and
the inlay is gilt; a test asserts neither ever goes cold, since a blue light
over a green table makes colour identity a guess.
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
them were wrong. **The own-board overlay is an opaque panel the width of the
canvas, and it defaulted to open** — `palette::PANEL` is `srgba(0.05, 0.06,
0.08, 0.88)`, so the entire table, its mats, its cards and every animation
on them were behind a sheet of 88% black from the first frame. What finally
found it was measuring instead of reasoning: a red clear colour renders at
`(234, 51, 35)` in a stock Bevy app and at `(62, 19, 21)` in ours, and a
clear colour never touches a material, a texture or a shader. The overlay is
opt-in now (`Duel::overlay_open`, default false), which is also why the
canvas is navigable by default — `input::camera_controls` refuses to run
while the overlay covers the table.

The felt was too dark as well, and that was real: it was authored at about a
quarter of the brightness it needed, and
`the_felt_is_dark_enough_to_read_cards_against` passed every run because it
only ever bounded the bright end. It bounds both now.

## Eight seats

- Seats sit on a ring, local seat at the near edge, opponents clockwise **in
  turn order** — the player on your left acts after you.
- Pods get unequal space; the local pod is always largest, and focusing an
  opponent borrows from the other opponents, never from you.
- Lanes fan when crowded and report overflow when even fanning stops being
  legible.

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
// StackItem::Ability { source: ObjectId, ability: AbilityRef }
```

`AbilityRef { card: CardIndex, index: u32 }` is the stable handle. `index` is
the position in that card's `CardDef::abilities`, so a client that knows the
card pool can map it to text; the reserved indices (`SPELL`, `ENTERS`,
`ADDITIONAL_COST`, `MIRACLE`, `UPKEEP_COST`, all counting down from
`u32::MAX`) name the abilities that are not listed on the card, and
`AbilityRef::is_listed_ability` separates the two.

The same handle addresses a seat's standing answers
(`PlayerAction::SetStandingAnswer`), which is why it deliberately says nothing
about a particular game: a gateway can store *"always say yes to Ondu Cleric's
rally"* against an account and replay it into the next one.

**Getting the text is the client's job, and no text crosses the engine
boundary.** The intended source is codegen: the card files already carry the
oracle sentences as `//! Oracle:` header lines, ordered to match the ability
list, so `cargo xtask codegen` can emit a per-card table of ordered sentences
alongside the registry for clients and the gateway to read. Until it does, a
client can render the source permanent's name plus the ability index, which
is already enough to point at the right card on the board.

### The stack panel draws it

`hud::spawn_stack_panel` is where that stops being theory. Each entry is a
card, not a line of text: the spell's own picture, or — for an ability, which
has no card at all — the picture of the permanent it came from, borrowed
through `StackKind::Ability { source }`. Under the name sits what kind of
thing it is and whose (`Ability · Llanowar Elves — You`), and under that a row
of everything it points at, each target drawn as its own smaller card with an
arrow between.

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

## Images and memory

Scryfall CDN, keyed by printing id — no API call is needed to render a board.
Board cards are fetched `small` (146×204); only the focused card is fetched
`normal`. That is the difference between ~36 MB and ~400 MB for a large table.
A byte-budgeted LRU (`TextureBudget`) decides evictions; the browser budget is
deliberately below the desktop one and the ordering is checked at compile time.

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
glows this turn. `cardmat::glow_of` is the one gatherer every battlefield
surface goes through; inside it `glow_bits` narrows the engine's `u128` to
the bits the shader reads, and a test pins each one against `KeywordSet`,
because that numbering is generated and a card glowing for the wrong keyword
would be a rules lie a player would believe.

What a card *is* and what it can *do* are drawn in different places, and that
separation is the whole grammar:

- **The border says what the card is.** Indestructible is the *base*:
  darksteel, a hard blue-grey with a specular line, the card made of something
  rather than lit by something. Hexproof and shroud are *films* over that base
  — a steady green sheath, or the same idea taken further, colder and hazier,
  since not even its controller may target it. Base × film composes, so an
  indestructible hexproof creature is steel under green and neither claim is
  lost. Two films would not compose, and never have to: `glow_bits` drops
  hexproof whenever shroud is present, because shroud already forbids every
  target hexproof forbids (CR 702.18a against 702.11b) and the green film
  would be advertising a permission the card does not grant. That
  normalisation lives in Rust rather than in WGSL so that it is unit-tested
  once and both shaders inherit it.
- **The face says what the card can do.** A creature with summoning sickness
  (`glow::SUMMONING_SICK`) is drawn asleep: desaturated, dimmed, breathing
  slowly, over the art and never on the border. It is not a keyword — it is a
  fact about *this turn* — and putting it on the border would make it read as
  one. The bit is set only for creatures, because summoning sickness is
  visible on nothing else.
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

The pictograms live in a third shader file, `card_common.wgsl`, together with
the printed corner both shaders cut at: it is everything the table and the
overlay have to agree about. It contains no bindings and no bevy syntax at all
— every shader-global it needs, the time and the colour underneath, arrives as
a parameter — which is what lets bevy compile it as an imported module, what
keeps it clear of the two different bind groups the two shaders read `globals`
from, and what lets the naga test parse it on its own. Which mark a fragment
is inside is found by walking the eleven bits to the k-th set one: a loop
bound at compile time, no dynamic indexing, and WebGL2-safe like everything
else here. *How many* marks there are is counted in that same loop, and that
is not stylistic: `countOneBits` is what WGSL offers, naga lowers it to GLSL's
`bitCount` with no version check, and `bitCount` arrived in ES 3.10 while
WebGL2 compiles ES 3.00. The browser would have rejected the shader and the
card pipeline with it, so the table would have drawn no cards at all — a
whole-client failure from one builtin, in the one target
`cargo check --target wasm32` cannot see, because WGSL is not lowered to GLSL
until the pipeline is built in the browser.

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
at, since hand, preview, own-board overlay and printing picker all drew the
scan square. Both cut with the same `corner_sdf` at the same `PRINTED_CORNER`
(4.76%, which is 3 mm on a 63 mm card, and lives in `card_common.wgsl` with
the rail), and `hud::card_radius` is the same number again, because that
wrapper node clips the card and carries its shadow.
All of them used to be 10%, which took the white away by taking a tenth of the
card with it, and made every permanent read as a token.

The whole thing is WebGL2-safe: uniforms only, no storage buffers, no texture
arrays. Animation reads `globals.time` from the view bind group, so nothing is
written per frame — a material is created once and never touched again while
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

The camera follows its rig the same way, but faster: a drag that lags behind
the pointer feels broken where a card that snaps feels cheap. Yaw interpolates
the short way around, or focusing the seat on your left would spin the table
three-quarters of the way to reach it. `ShownRig` is a second copy rather than
smoothing `CameraRig` in place, because the rig is *input* and everything that
writes it wants to be able to say "there".

`Preferences::reduce_motion` turns all of it off, and it travels with the
account for the same reason the keys do: a player who cannot read a moving
board cannot read one on any machine.

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

The lobby is `DuelPhase::Closed` only, and brings its own 2D camera — the duel
brings its own and the two never coexist.

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
`fonts/Inter.ttf` against `./assets/` in a browser exactly as it does
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
curl -s -XPOST localhost:28770/key       -d '{"name":"Space","shift":false}'
curl -s -XPOST localhost:28770/scroll    -d '{"y":-6}'
curl -s -XPOST localhost:28770/screenshot -d '{"path":"/tmp/table.png"}'
```

Five things about it are load-bearing.

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

`/state` is deliberately the *client's* answer and not the engine's — the view
it last received, beside the interaction state it built from it. A disagreement
between the two is exactly the class of bug the endpoint exists to show, and
one a screenshot cannot report.

## Verification

- `cargo test -p baylee-client --test duel_flow` plays real games headlessly
  through the client's own path (host → view → board model → interaction).
- The wasm CI job type-checks `baylee-client` for `wasm32-unknown-unknown`.
  The browser-only paths (settings storage, entropy) compile nowhere else,
  so without that job they rot silently.
- Browser entropy needs both `.cargo/config.toml`'s `getrandom_backend` cfg and
  the `wasm_js` feature; either alone is not enough.
