# The client, and where it actually gets its game

This file loads only while work is under `crates/baylee-client/`. It was
the last section of the repository's root `CLAUDE.md` and was moved here
because it is 43 000 characters about a renderer that a session writing
cards, rules or gateway routes was paying for on every turn.

`docs/client.md` remains normative for all of it; this is the part a
session has to know before it opens the file.

Work on `crates/baylee-client-core/` does **not** load this file
automatically, and most of what is described here decides there — read it
deliberately when the change is about layout, interaction, the board model
or the lobby.

### Hosts, and where the client actually gets its game

The renderer never touches a socket; it talks to a `DuelHost`, of which there
are two. `LocalHost` is an in-process engine, solo vs the house AI from
`data/acceptance-decks.txt`; `NetworkHost` (`src/net.rs`) is a websocket to
the gateway's `/games/{id}/ws`. `crates/baylee-client/src/main.rs` picks
between them on whether this launch was handed a `SeatTicket` — `BAYLEE_GAME`
+ `BAYLEE_SEAT_TOKEN` natively, `?game=…&token=…` in a browser — and nothing
above the host can tell which it got, because both decode the same protobuf
envelopes with the same function.

**A socket that goes away no longer ends the game.** `NetworkHost` could always
re-dial and ask for the frames the seat missed, and nothing ever called it —
a drop put "the connection to the table was lost" in the prompt bar and left
the player there while the table sat waiting. What was missing was the
*policy*, and the trait was why nothing could supply it: `InstalledHost` is a
`Box<dyn DuelHost>`, so `is_open()` was unreachable from above. `DuelHost` now
answers `link()` with a `LinkState` (`Local` / `Up` / `Connecting` / `Down` —
`Connecting` is its own state, or the system would redial once a frame for as
long as a socket takes to open), and `keep_the_table_connected` in `lib.rs`
drives it. The schedule is `baylee-client-core/src/reconnect.rs`, renderer-
and transport-free like the lobby's decisions, so it is tested without a
gateway to disconnect from: 0.5 s doubling to a 15 s cap, twelve dials, then
it stops and says so. It is allowed to back off at all because the engine's
*decision* clock does not run for a seat with no socket — nobody is losing on
time while it waits. That is not a licence for the banner to say nothing is
happening: the **reconnect** clock does run, and when it expires the house
takes the chair. The bar turns to a second sentence at `Retry::PATIENCE`, in
the future tense, because the window is a per-table 10 s–3600 s that no
client is ever told and could not count down anyway — it is disconnected for
exactly that span. `docs/client.md` §"What the banner may claim, and why it
cannot count".

The **decision** clock is the other one and is now drawn: `PlayerView`
`decision_remaining_ms` is relative milliseconds, so `DecisionClock`
(`baylee-client-core/src/decisionclock.rs`) counts down between views and
takes each view as the correction. It appears at 60 s flat — which makes
`blitz`'s 30 s table right by construction, the number being on from the
first question — and sounds `Cue::ClockLow` at 60 s and 10 s, latched per
question, rung only for this seat although the number is drawn for every one.
The cell's *presence* is in `LedgeRevision`; its *value* never is, or the
shelf would rebuild once a second. Giving up reports `DuelReport::Unreachable`, which is a
variant rather than another `Failed(String)` because the gateway's `Error`
envelope carries the engine's refusal of a *single action* through `Failed`,
and a shell that returned to the lobby on every one of those would eject a
player for a misclick.

Without a ticket the binary adds `LobbyPlugin` (`crates/baylee-client/src/lobby.rs`)
instead of opening a duel, and makes those HTTP calls itself: register/login,
`POST /decks`, `POST /lobby/games` or `.../join`, then the same `SeatTicket`
into the same `NetworkHost`. The decisions live in
`crates/baylee-client-core/src/lobby.rs` and answer input with a
`LobbyRequest` the shell performs — so the flow is tested headless and the
route mapping is tested without a gateway. The lobby runs only in
`DuelPhase::Closed` and brings its own 2D camera; it is a separate plugin
because `DuelPlugin` has to stay embeddable in an application that already has
a front door. Two buttons survive from before there was a builder: "add the
starter deck" (it posts the acceptance file's `Allytifact` rows in one tap)
and "play the house AI offline" (a `LocalHost`, no account).

A table of more than two chairs is a **room**, arranged in the open before
anyone plays: `POST /lobby/games {seats: 2..=8, name, password}` opens one,
`POST /lobby/games/{id}/seats/{seat}` arranges a chair, `POST .../leave` frees
one. Two authorities that do not overlap — the host sets `kind`/`ai` on any
chair nobody is sitting in, every player sets `deck_id` on their own and
nothing else. Eight is `GamePreset::validate`'s bound, so the gateway refuses
exactly what the engine would.

Starting takes **two statements by two people**: `POST .../ready {ready}` is a
player's own (a `409` without a deck, and cleared if the host puts a different
deck in that chair), `POST .../start` is the host's, and it is a `409` until
every chair is ready — an AI chair being ready as soon as it is configured.
The room used to start itself when the last chair got a deck, which meant
picking a deck to look at it put you in a game. `POST .../host {seat}` hands
the room on, and so does leaving: a room passes to whoever **joined earliest**
and is closed only when nobody is left in it. A non-empty `password` locks the
room; the listing carries `"locked"` and never the password.

`GET /lobby/games` describes the whole arrangement in **handles**, never
account ids, with `you`/`yours` answering "is that me" and `startable` saying
whether the host's button would do anything. A handle is `Alice#af03`: a
display name is not unique, and `account.tag` — an identity column drawn as
lowercase hex, padded to four digits and allowed to grow past them — is what
tells two Alices apart. `store::display_names` is the one place the two
halves are joined, so every roster and every `GameSetup` carries a handle and
nothing below the gateway knows a tag exists;
`crates/baylee-gateway/src/handle.rs` renders and parses it, and
`docs/protocol.md` §"A name is not a claim" is normative. It answers **one page** —
`{games, total, offset, limit}`, searched with `q` over a table's name and its
host's — in a **fixed total order** (waiting first, then newest, then id),
because games live in a `HashMap` and paging an unordered collection hands out
some rows twice and never shows others.

`GET /lobby/ws?token=…&q=…&offset=…&limit=…` is the same page, pushed: sent on
connect and again on every lobby change, rendered per socket because
`yours`/`you` are per account. The client's search box and pager are part of
the subscription, so changing either re-dials; a client with no socket falls
back to polling the HTTP route. `docs/protocol.md` §Rooms and §"The lobby
feed" are normative.

The deck builder is `Screen::Build`, and the same split once more: all of it
decides in `crates/baylee-client-core/src/deckbuilder.rs`. Two things fix its
shape. Its pool is `GET /pool` — the compiled **registry**, not the catalog's
118k printings — because a builder offering catalog cards would offer cards
the engine cannot play; every row carries its `Coverage`, and the default
"playable only" hides the stubs (partial cards stay — they do play, and are
marked). And
`DeckBuilder::problems` is a mirror of what `POST /decks` enforces, split into
blocking (which greys the save button) and advisory (60 cards, a 15-card
sideboard, a land count, unimplemented cards — never blocking): if the button
is live, the deck saves. The pool arrives whole once per session and every
filter runs locally, so search costs no request. A sideboard is a real second
list now — through the store, `DeckBody`, `LoadedDeck` and into `SeatSpec`.

**The printing picker** is the third thing that fixes the builder's shape. The
pool is one row per *card* and searches every name that card is printed under
(`alt_names`, from `/pool`), because "do I own this" has one answer and a list
that repeated a card once per set would be answering a different question.
Which piece of cardboard is a separate question, asked in a dialog:
`DeckBuilder::open_picker` fires `GET /printings`, and the carousel over the
answer picks the set, the language and the finish. What comes out is a
`baylee_core::deckrow::PrintChoice`, which is why a `DeckBuilder::Entry` is
`(slot, count, print)` and two printings of one card are two rows — while the
copy limit stays on the card, as the gateway enforces it.

The rule that keeps old decks clean: a choice that changes nothing writes
nothing. Picking the default printing leaves `4 Lightning Bolt` exactly as it
was, so re-saving a deck built before any of this existed adds no noise.

Three things about the builder's surface that are easy to break by accident.
Mana costs are drawn, not printed: `baylee-client-core/src/manapip.rs` is the
renderer-free pip table and `baylee-client/src/manaui.rs` draws it with the
OFL Mana font (`docs/legal.md` §2) — the font supplies a monochrome mark only,
so the coloured disc is the client's, and a hybrid is one disc with two glyphs
clipped to opposite halves because hybrids have no single glyph. The hover
preview is **not** part of the retained tree: it is its own entity behind an
epoch counter (`Hovered` → `CardPreview`), because rebuilding two hundred rows
per pointer move would make the list unusable. And a card's `?` panel is a
menu, not a label — add, move between deck and sideboard (keeping the
printing), remove, set as commander.

The lobby is the client's one responsive screen: `Metrics::of(width)` picks a
phone / tablet / desktop frame and every size comes from it, so a phone stacks
the panels, drops the gateway line and gives every target 44 logical pixels.
Text entry on a canvas is the part with no obvious answer —
`crates/baylee-client/src/softkeys.rs` keeps one invisible but *focusable*
`<input>` over the page on wasm, which is what raises a phone's keyboard and
what buys autofill, IME and paste; the client's own key handling is skipped
there so nothing is typed twice. In a browser the gateway comes from
`?gateway=…` (remembered in `localStorage`), because the page origin is a
`trunk serve` on :8080 and the gateway is not.

The trap in that flow: a seat token is not always usable yet. `mode:"ai"` and a
join both order an engine before answering, so the socket can be opened at once
and simply waits (up to 30 s) for that engine to attach. An **open** table
orders nothing — it holds a seat whose game does not exist, so a socket opened
against it is accepted and closed again with nothing on it. The lobby stays put
until that table's state turns `"playing"` — which the lobby feed pushes, the
table's state being a lobby change like any other. `lobby/feed.rs` holds that
socket; the two-second re-read that used to be the only way to learn it is
still there behind `Feed::live()`, for a gateway that has no `/lobby/ws` or a
socket that could not be opened.

A card is a **slab**, not a decal: a rounded face with a thin wall around its
edge (whose UVs borrow the face's, so the edge is the card's own border
colour) and a contact-shadow child under it. Neither reads at a camera exactly
overhead, which is what `table::CAMERA_LEAN` is for — about 20° off vertical,
enough for both. It was 22°, and a lean is paid for by the seat furthest from
the camera and collected by the seat nearest it, which is always the player's
own: three boards laid out at the same 12.0 units were drawn 450, 381 and 378
pixels wide, 18.9% apart. Halving the lean and `FOV` together brought that to
6.3%, and `every_seat_is_drawn_a_board_of_the_same_width` is what holds it —
so the angle and the equal widths are traded directly against each other.
`CAMERA_LEAN` has been 0.40, 0.24, 0.27 and is **0.36**: the owner asked a
third time for more angle after being told the trade, which is a decision, so
the bound moved from 1.08 to 1.12 and the worst spread with it, 6.3% → 10.6%
on an eight-seat ring. The bound is that measurement plus a hair, and still
fails the 18.9% shot it was written for. The lens buys nothing — 0.34 at a
`FOV` of 0.36 spreads the same 8.2% as 0.33 at 0.42, because the larger of
the two error terms is foreshortening, which no lens shortens.

The 3D table under the cards is **generated, not shipped**: no sprite, no
photograph, no downloaded texture anywhere on it.
`baylee-client-core/src/tabletop.rs` computes the
glow under a mat into RGBA8 buffers with a seeded value-noise fbm (no `rand`,
no clock — every player sees the same grain), and the two surfaces a player
actually reads are drawn in WGSL: `shaders/felt.wgsl` for the slab and
`shaders/mat.wgsl` for a seat's mat. Both moved out of a texture for the same
arithmetic — a slab thirty-five units across wants four thousand texels to
stay sharp at this camera, and a mat thirteen units across wants two thousand
and had five hundred. `docs/legal.md` §2 decided the whole approach: ornament
is the easiest thing to borrow by accident, and arithmetic borrows nothing.
`tabletop::felt` and `tabletop::seat_mat` remain as the same arithmetic in
Rust, where a test can measure it, and `table::camera_tests`'
`the_shader_and_the_generator_agree_about_the_{cloth,mat}` read the constants
back out of the WGSL so the pair cannot drift.

The middle of the table went the same way and further. It was a quad with a
512-texel medallion on it — two gilt rings and five soft discs of the pie —
and it is **five flames painted into `felt.wgsl`** since September 2026,
because the owner could not see the black one: a dark disc on a dark table is
a gap, not a colour. `baylee_client_core::firewheel` is the model half and
`docs/client.md` §"The wheel is five flames" is normative. Three things about
it are easy to get backwards. They are painted **flat in the table plane**,
not standing up — this camera projects a standing length at `sin(lean)` and a
lying one at `cos`, which is 2.7× the picture, and a fixed eye loses no
parallax by it. The light they throw is added **inside the felt's own
shader**, after the sky, so the weave shows through it; a quad over the cloth
can only cover it. And the black flame's body is a **multiply** rather than a
colour, with the light on its rim — nothing can emit black. The rings moved
into the cloth as hairlines at the same time, and they are drawn *before* the
flames, or three of the five would be etched by the ring they cross.

What it draws is **midnight mineral cloth inside an aged champagne rail, on
a slab with a real thickness** (casino baize until the September 2026
redesign): `rounded_slab_mesh` builds the table and the card both, a rounded
top face with a wall around its edge, and the table hangs below the plane
everything else is placed against while a card stands on it. Nothing lights
this stage, so the wall reads as a wall only because `tabletop::APRON` is a
darker colour than the rail above it. The slab is cut as a **racetrack**, and
the corners it gives up are load-bearing: the camera frames the layout plus
`AIR` and the slab is cut to the layout plus `SLAB_MARGIN`, so a rectangle
fills the window edge to edge and nothing behind the table could ever be seen.

The edge is an **altar's, not a tray's**, and that inversion is the whole of
`felt.wgsl`'s edge shading. The cloth used to fall into `FELT_DEEP` over 1.8
units as it reached the rail and the rail crowned in its own middle, which
together read as a surface *sunk inside a frame*. Now the cloth keeps a crest
of light at its own boundary and the rail falls from `RAIL_LIP` at the inner
edge to `RAIL_HIDE` at the outer one on a quarter circle's cosine — so the
top is the highest thing there is and its edge is rounded over and away.
`RAIL_WIDTH` is 0.55, narrowed with it from 0.9, and `table_corner` went 0.11
the same day — and is **0.065** of the short side since `d368cb56` on 17.09,
which this file did not follow. The table also carries **its own lamp** (`under_lamp`): an
elliptical pool that darkens the ends rather than lifting the middle, because
the cloth is already as bright as `the_felt_is_dark_enough_to_read_cards_
against` allows. It is a multiply on the table's colour for the same reason
`under_sky` is — there is no light in this scene and there cannot be one.

**Behind it is a sky**, and it is weather rather than rules.
`baylee-client-core/src/sky.rs` decides which one — `SkyMode { Auto, Day,
Night }` and a pure `phase(mode, hour)` with dawn and dusk ramps — and
`baylee-client/src/sky.rs` draws it as one quad parented to the camera,
painted in **screen space** by `shaders/sky.wgsl` so it holds still when the
player orbits and so the sun and the moon can be put where the table is not.
Magic's day/night designation (CR 731) is a different thing entirely, and it
is now in the pool — five werewolves print daybound and nightbound, the
engine keeps `GameState.day_night`, and `PlayerView.day_night` carries it.
The two must not be joined up. Nothing about this sky changes a legal
action, which is exactly why a client may decide it alone and why the clock
is the player's own; `SkyMode::Day`/`Night` exist so a player can override
that clock, so a sky driven by the rules would either overrule the player's
setting or be overruled by it, and either way could never be *read* as a
rules fact. The designation is drawn on every seat's own bar instead, on the
hinge beside the turn number, where a fact about the game already lives — and
the sky is allowed to disagree with it. The hour comes from `web-time` in the shell, because
`std::time::SystemTime::now` panics on `wasm32-unknown-unknown` and
`baylee-client-core` compiles for it — and it is **UTC**, with `Day` and
`Night` there for a player the offset bothers.

The sky also **lights the table**, and the two are one movement rather than
two: `sky::sync_sky` eases the phase, `sky::table_light` turns the eased phase
into a multiplier on the table's own colour, and the felt shader's `under_sky`
applies it — so a dusk crossfades the sky and cools the cloth on the same
frame with nothing told that a transition is happening. It is a tint and not a
lamp, for the reason the next paragraph gives, and it stops at the phase lamp,
which is light the table *emits* and carries a meaning of its own.

Every seat plays on its own mat, sized from its `SeatSlot`, banded for the
three lanes, with the rim carrying the seat's colour — gilt for the viewing
seat, the pie in ring order for the rest — and its opacity carrying `Mood`, so
"who is everyone waiting for" is answered on the felt while a light on one rim
answers "whose turn is it". Those are two questions and
`Mood` carries two fields for them: `Standing` is a rank that collapses them,
and a rim light driven off the rank would leave the active seat the moment an
opponent responded to something. That light was a short comet at 3.6 seconds a
lap and is now a **breath**: the whole rim of the active seat glows
(`TURN_BASE`), a wide swell travels it every 11 seconds and the rim rises and
falls together every 7, two periods with no common multiple worth noticing, so
it never repeats a pose. Measured live: the active rim swings 4.3/3.6/2.9 per
channel over eight seconds while the opponent's — same shader, same material,
`on_turn` at zero — and the bare felt beside it both move 0.0. A **third**
thing the rim says is that nobody is sitting in this chair
(`SeatRole::Away`), and it says it by being broken into dashes rather than by
being drawn quieter — the opacity is spent on `Standing` down to 0.22, so a
held chair that borrowed brightness would be read as a seat losing interest.
`tabletop::rim_dash` is a gain of mean exactly one over the whole rim signal,
which is the only application that survives the shader's clamp on the chairs
the house most often holds; `docs/client.md` §"The rim says it where the bar
cannot" has the measurement. Everything down there is `unlit`
deliberately: scene lighting on card art would make colour identity
unreadable. The stage therefore has no light in it at all, and the camera
carries `Tonemapping::None` so a future Bevy default cannot quietly treat
display values as radiance.

The table nonetheless shipped as a black screen with two gold rings in it,
and the cause was none of that: `OwnBoardOverlay` was an opaque
`palette::PANEL` (88% black) the width of the canvas, and it **defaulted to
open**, so the felt, the mats, the cards and every animation were behind it
from the first frame. It was made opt-in, and is now **gone entirely** — a
second, flat drawing of the one board the camera already frames best was
answering a question the 3D table had stopped asking, and the panel that
could hide the whole game had no reason left to exist. The felt was also
authored about four times too dark, which a one-sided "dark enough"
assertion let through; that bound goes both ways now. `docs/client.md`
§"The table itself" has the measurement that found it — a red clear colour
renders `(234, 51, 35)` in stock Bevy and `(62, 19, 21)` here, and a clear
colour touches no material, texture or shader.

Removing an action is what made `Keymap`'s reader tolerant. It is
`#[serde(transparent)]` over a map keyed by `Action`, so a stored blob naming
`toggle-overlay` was refused *whole*, and `Preferences::from_json` answers a
refusal with the defaults — every key a player had ever bound, lost to one
retired row. An unknown name is now dropped and the rest of the map kept, in
both directions of an upgrade.

**Every seat reads itself off its own mat.** One long edge of each mat is a
`tabletop::MAT_LEDGE` shelf — a fourth band, dimmer than the quietest lane, so
the ink on it is the brightest thing on a seat's ground — and the seat's bar
is written along it. *Which* edge is `layout::LEDGE_IS_OUTER`, and it is one
rule with one answer: **every seat's ink sits on the edge of its own
battlefield nearest the middle of the table — the edge that seat reads as
above its board.** So the local bar is at the top of the local mat and an
opponent's is at the *bottom* of theirs, below their creatures, and the two
face each other across the hearth. It used to be two rules with `is_local` as
the seam, and both of them chose from the viewer's chair: an opponent's name
and life ended up beyond their far rim, at the top of the window, as far from
their own board as the mat allows. The owner asked for the mirror. Only the
shelf changes ends: the three lanes run from the centre-facing edge outwards
at every seat, so `MatParams::ledge_outer` is a flag and not a flipped `uv.y`,
which would carry the lane veils along with it. What *does* follow the shelf
is where those three lanes start — the board now sits a `MAT_LEDGE` back from
the centre-facing rim and gives that strip up, so no card is ever drawn where
the bar is, which `no_card_reaches_the_band_its_seat_writes_on` is what
checks. The band carries
the priority caret, the seat's colour, its name, life,
its four zone counts, the turn number with the day/night designation on its
hinge, and the twelve steps of that turn **horizontally in its middle** —
`hud/seatbar/attached.rs` places all three panels through one `pose_on` and
one shared scale. The steps were a vertical column beside the command-zone
rim; the middle of the table, which is where "above the board, in the middle"
would otherwise put them, belongs to the firewheel. It is a **second retained tree**
with its own `hud::BarRevision`, because `HudRevision` counts the hover and a
bar rebuilt on every pointer move would be rebuilt hundreds of times a turn.
Placement cannot use the camera's propagated `GlobalTransform` — `bevy_ui`
orders `UiSystems::Layout` *before* `TransformSystems::Propagate`, so a
`Node` written from it is a frame stale — so `table::Lens` projects the
mat's ledge corners from the same `CameraRig::eye` the camera is set from, and
`camera_tests` checks that against the hand-derived projection to half a pixel.
A rotated seat's bar is rotated with it (`UiTransform::from_rotation`), which
carries picking too: Bevy 0.19's UI backend inverts `UiGlobalTransform`.
The bar comes in **four densities** and the fourth is not decoration — the
shelf projects 1141 px at a duel and 151 px on an eight-player ring, so
`seatbar::Density::for_length` drops from the full bar to a compact one to
pips to `Mark` (a caret, the seat's colour and twelve 8×10 pips). What a
density drops, the seat sheet carries on hover.
Those four are a ladder in **length**, and `Density::Split` is the fifth form
and the one chosen a rung above them, on *two* measurements. It is the bar
the owner asked for: the twelve steps alone along the mat's top edge, spread
across its whole width, with the seat's identity on its own row beneath. It
asks for a **shorter** shelf than the full bar (two stacked rows are as long
as the longer of them) and a **deeper** one than any single-row form, which
is why `Density::for_shelf` exists and why `Shelf::depth` is finally read
rather than merely asserted about. Its tiles are the only ink on a bar that
is not a fixed number of pixels: they have the row to themselves, so they
grow together to a cap and the slack past it goes into the gaps, which is
what lets `Shelf::box_size` measure a split bar's box from the ledge instead
of from the form. It reaches a duel and stops there — three seats and up have
shelves deep enough and far too short (372×46 against the 585 px two rows are
wide), and the length ladder takes over untouched.
**The twelve tiles stand in the five phases of a turn** (CR 500.1), and that
grouping is the row's whole structure: three in the beginning phase, five in
combat, two at the end, and the two main phases as one tile each. Two gaps
say it — `tile_gap` inside a phase, the wider `phase_gap` between — and on a
split bar the slack goes into the four phase gaps and never into the seven
tight ones. Even gaps had spread the tiles as twelve equal pills, which says
a turn has twelve equal parts. Two more things came out of the same pass. A
main phase is drawn `MAIN_SPAN` step-widths wide, because it *is* a whole
phase standing where a step stands and it is where most of a turn happens —
which also stops the two of them reading as stranded singletons between the
runs. And the tile width is one division in `Density::tile_width_on` rather
than a `flex_grow` with a cap: flex hands each *group* its share, so a phase
of one tile and a phase of five end up with tiles of different widths and a
pocket of dead space in the short groups. On a split bar the hinge moved to
the **far end of the identity row**, which anchors a row that used to run out
after the counts and gives the twelve tiles the whole ledge. The tiles also
**follow their shelf**: the tree is built when the density changes and the
box is placed every frame, so a bar built while the camera was still easing
in kept tiles a tenth too narrow and let `SpaceBetween` spend the difference
on its phase gaps — 66 px tiles and 45 px gaps against the model's 72 and
24.5, on a shelf the bar had been told was 1127 long. `stretch_step_tiles` is
the other half of `place_seat_bars`. Measuring that found the second half of
it: the **far** seat of a duel gets a shorter shelf, so its tiles cap with
four pixels to spare and its phases stood ten pixels apart against three
inside them, which is why the split form's `PHASE_GAP` is five times its tile
gap and not three.
**A mat is drawn `tabletop::MAT_MARGIN` wider than its playing extent on all
four sides**, and every band on it is a fraction of a depth — so while that
constant was `table::ZONE_MARGIN` and lived in the renderer, the shelf and
the three lanes were laid out over `POD_DEPTH` and painted over `POD_DEPTH +
2·MAT_MARGIN`. Every band was stretched 18.5%: the shelf sat 0.46 units from
where the geometry reserved it, with the bar following `ledge_corners`
faithfully off its own ledge and onto the creature lane, and the lane seams
missed the rows of cards they fence by 0.06 and 0.25 units. Nothing failed,
because nothing had ever measured the drawn mat against the layout in the
same unit; `/state.shelves` against a photograph is what found it. There is
one rectangle now — `MAT_MARGIN` in `client-core::tabletop`, `LEDGE_FRAC` /
`LANE_FRAC` / `MARGIN_FRAC` fractions of `MAT_DRAWN_DEPTH` summing to 1 under
a `const _`, `ledge_corners` returning that same band — and
`the_mat_fences_its_bands_where_the_layout_put_them` reads the fences out of
the texture rather than out of the constants it was built from.
Paying for it moved `tabletop::MAT_LEDGE` from 0.95 to 1.00, and the
interesting half of that is what stopped it: **not** the `MAT_LEDGE <
CARD_HEIGHT * 0.75` assertion (1.00 is 0.72 of a card, so the bound never
bit) but `layout::MAX_RING_Y` at about 1.01, where a 2v2's ring clamps and
its partners overlap. Deepening the ledge costs *a duel* no board size — the
pod grows with it and the camera frames the pod, so a duel's shelf projects
*longer* at 1.00 than at 0.95; a duel is framed by its width, and three seats
and up, framed on their depth, pay for the deeper pod as normal. The untap and cleanup steps are
dead in the model, not merely drawn grey: `RailRow::grants_priority` is false
there and `PhaseOrders::toggle` refuses both, so those two tiles carry no
frame and are `Pickable::IGNORE`. They keep a 4% ground all the same — a fill
is not a frame, and with no ground at all they were two bare words at the
extreme ends of the row, reading as stranded text rather than as the first
and last things a turn does. The predicate is about a *standing order*,
not about the rules, which is what puts cleanup on the list: untap grants no
priority at all (CR 502.4), and cleanup grants a round only *because* a
state-based action was performed or an ability triggered (CR 514.3, 514.3a) —
a window the engine opens on its own and asks about when it opens. A green
button in either is a stop that can never fire. The one thing greying cleanup
costs is a speculative hold there, arranged in advance against a trigger that
may not come. Both rows of standing orders are seen at once in
`settingsui.rs`, which is where a player arranges them; a bar shows only the
row its own turn belongs to.

The **camera frames the table against the part of the window it is seen
through**, which is not the window: the hand bar is an overlay on the same
full-window camera. It used to be far more than that — a strip of seat tabs
and a phase rail under it took 110 logical pixels off the top of every window
in every game, and both of them said what a seat's own bar says now, so
`Canvas::hud`'s `top` is **zero**. The camera came in by that much: measured
at 1728×1052, pixels per table unit went 37.9 → 43.8 at a duel and 28.2 →
29.6 on an eight-seat ring. Every seat count gained except **three**, which
lost 34.9 → 32.0 — and that one is not a cost of the camera at all. A taller
canvas is a squarer one, which is what finally let a three-player
free-for-all pass `layout::ROUND_COST` and sit on the **circle** the design
has always wanted for it: the ring went 12.95 × 4.99 to 8.28 × 8.28, and a
circle at three seats costs 9.3% of reach against the 30% that filter allows.
The ellipse it replaced put the two opponents at 150° and 210°, side by side
across the top, which is the silhouette a 2v1 draws.
`three_seats_playing_for_themselves_sit_on_a_circle` is what holds it, because
a slightly different window can flip that filter and nothing else at the table
notices. A rounder ring also turns its side seats further from the camera,
which is why `every_seat_is_drawn_a_board_of_the_same_width` moved from 1.12
to 1.13.
A hard-coded 20-unit rig aimed at the middle of the felt put the local
seat's own mat *underneath the hand bar* on every screen.
`table::CameraRig::home(layout, canvas)` computes it instead, from
`TableLayout::extent` (each pod's box rotated by its `facing`, because a seat
on your left plays across the table) and a `Canvas` naming what the HUD
covers; `table::frame_table` reapplies it as seats, focus and window change
and stops only while the player is looking at one seat. **No hand moves this
camera**: the orbit went because the button that plays cards was also the
button that turned the table, the zoom went because the wheel argued with
every scrolling panel, and the pan went with the rest on 14.09.2026 — the
owner's report being that arrow keys drove the table *through* a focused text
field, `input::camera_controls` having read `KeyCode` directly rather than
through the keymap, which is the one route around every other guard. The
system is gone; `navigate_to_player`/`navigate_home` (`F`/`H`) are viewpoints
and are in the keymap. The
inversion is exact: the lean's cross terms cancel, so a felt point's screen
position is *linear* in the eye distance and the fit is one division rather
than a search. `camera_tests` projects every pod's corners forwards — written
out a second time, because a test reusing the inverse would agree with it
however wrong both were.

The card quad has four mesh tests because it shipped once as a bowtie — every
corner arc swept its neighbour's quarter turn, the outline folded through the
middle, and each permanent drew as a small bright X.
`an_untapped_card_lies_flat_on_the_table` passed the whole time: it checks the
*transform*, which was never wrong. Geometry needs tests about geometry.

Combat goes through a **focus**: the defender (or attacker) the next
declaration is pointed at. `Interaction::toggle` pairs a creature against it,
`cycle_focus` moves it, `assignment` and `focus_position` are what the prompt
bar draws, and a pointer can skip the aiming by tapping the planeswalker (or
the attacker) it means. Both halves are wired — `input.rs` for the keyboard,
`hud.rs` for the buttons — which they were not before: `toggle` used to write
to `selected` while `confirm` read `pairs`, so a player could light up their
whole board and still declare nothing.
`crates/baylee-client/tests/duel_flow.rs` is where that stops being a claim:
`a_whole_game_can_be_won_through_the_clients_combat_path` starts seat 0 with a
squad on `starting_battlefield` and plays the duel out to `GameOver` with every
decision — combat included — built by `Interaction` from what the engine
offered. A client that cannot express an attack fails it instead of quietly
passing the turn.

What the table *draws* is one model over **two** sources.
`baylee-client-core/src/combat.rs` reads `view.combat` — the engine's accepted
declaration, and the only place an attack made *against* this seat exists —
together with `Interaction::assignments`, the one this seat is still building
and no view knows about. Both produce the same `Line`; `standing` says which,
and is a flag rather than a third kind because a proposed attack and a
confirmed one are the same claim drawn at different weights.
`combatlines.rs` draws them as stretched unlit quads (there are no gizmos —
`bevy_gizmos` is not in the workspace's bevy features) recomputed each frame
from the cards' live `Transform`s, because a line built from `Motion::target`
snaps to the destination while the card it belongs to is still gliding there.
`Combat::tally` is the one arithmetic §6 of `docs/design.md` allows, and it
counts a block this seat has only *proposed* — the number answers "what still
reaches me if I block here", so a block that has not been sent has to count.
Both systems are *run* in `combatlines::running` — an `App` with the resources
they ask for and two creatures on the table — because "declared but never
wired" is a bug this client has shipped before. Every assertion there is on an
outcome (entities that exist, a transform that moved, a ring at a named
point) and never on `update()` having returned, and the harness is checked by
the reverse: deleting one resource from it fails six tests.

A spell whose mana is not floating yet is **not** a card with nothing to do.
`baylee-client-core/src/manaplan.rs` decides which lands to tap for it —
Kuhn's algorithm over demands against available mana, not a greedy sweep,
because greedy pays the generic pip with the only black source and then cannot
pay `{B}`; `baylee-client/src/manasources.rs` is the half that needs the card
registry to know what a printed mana ability makes. Three rules hold it
honest: every step is an action `LegalActions` offered and is re-checked
against the *current* one before it is sent (`ManaRun` in `lib.rs`), Phyrexian
mana is never paid with life and `{X}`/`{S}`/restricted mana are refused
outright, and a source that makes two mana of *one chosen* colour counts as
one — under-counting costs an extra land, over-counting leaves a player tapped
out halfway through. In hand this is a third state: `Openings { playable,
reachable }`, gold for what the engine offered and indigo for what this client
is offering to do about it.

Activating an ability by hand did not exist at all until now —
`Interaction::activate` was written and nothing called it, so a Forest, a mana
dork and a planeswalker were equally inert under the pointer.
`crates/baylee-client/src/abilities.rs` is the list, built only from
`LegalActions`, each entry labelled out of the registry ("Tap for {G}", "+1",
"{T}, Sacrifice this, Pay 1 life") because "Ability 2" is a label a player has
to guess at. One option activates on the click that found it; several open a
chooser on its own row of the prompt bar, which sends by *position* and
rebuilds the list from the current `LegalActions` when pressed — a bar drawn a
frame ago must not be able to send an ability the engine has since withdrawn.

Finding the permanent that has something to do is the other half, and it is a
third `Openings` set: `activatable`, straight off `LegalActions`, reaching
both card shaders as `glow::ACTIVATABLE`. It is drawn as a warm light running
*round* the border rather than as one of the steady keyword sheaths, because a
keyword is what a card is and this is what a player could do — two different
claims that must not read as the same light. `CardGroup::activatable` is true
only when every permanent the card stands for can act, so a stack of three
never invites a click that gets refused.

There is no undo in the engine, so anything irreversible is **two-stage**: the
first tap arms (`Duel::armed`, an `ObjectId` and a `Deed` — `Play`, `Ability`
or a mana `Run`), a second tap on the same card sends, `Esc` takes it back, and
every reader re-resolves against the *current* `LegalActions` so a deed the
engine has withdrawn disarms rather than firing. Mana abilities are the one
exemption and stay one tap, read off the card's own `mana_ability` flag
(CR 605.1) — floating mana is the cheap mistake. The drawing is two more glow
bits in the same border register: `ARMED` holds still where `ACTIVATABLE`
travels (the invitation was accepted), `WILL_TAP` marks the lands the plan
would spend, and an armed card drops `ACTIVATABLE` so one border never carries
both. `docs/keyboard-map.md` §Arming is normative.

The card's bottom edge carries two rows that were designed together. The
**rail** (`client-core/src/cardrail.rs`) is eleven combat keywords as marks,
`RAIL_SPAN` wide; the fifth it stops short of is the **plate**
(`client-core/src/cardplate.rs`) — a creature's power and toughness, or a
planeswalker's loyalty behind a gilt rim. Both follow the same split: the
shader draws them, a renderer-free module says where and what, and a test
reads the WGSL and fails when the two drift. A rail mark is the **Mana
font's own ability glyph**, baked to a distance field at startup
(`markatlas.rs`) and sampled in the shader rather than drawn there; the
twelve procedural pictograms it replaced are gone, and with them every
motion that changed a mark's *shape* — a sampled glyph can only scale.
`cardrail::MARK_GLYPHS` is one of the three doors those codepoints come
through, which `docs/legal.md` §2a makes a rule. The plate is one `u32` (three
ten-bit numbers, two kind bits) riding the material key beside `glow`, so a
creature dealt damage becomes a different material and redraws with no second
pass, and a number too big to pack clamps rather than wrapping. Damage is the
plate filling from the bottom to `damage / toughness`, not a third numeral.
Adding loyalty to the plate meant adding it to `ObjectSummaryKey` too — it is
drawn now, so two walkers of a name must stop grouping.

The numerals **are type** and were a 4×6 stencil. They looked blurry and
off-centre, and that was one fault twice: a mask that coarse has no edge to
sharpen, and a fixed four-cell box puts a `1`'s ink somewhere else than an
`8`'s. They now come out of the *same atlas* as the rail's marks —
`AlegreyaSans-Bold`, cells 12 onward, shaped with `lnum` **and** `tnum`
because this face's default figures are oldstyle and its proportional ones
would shunt the slash sideways when a `9/9` becomes a `10/10`. The shader
walks pen positions over `TEXT_ADV`, which is a hand-written table of font
metrics pinned against the shipped file by
`markatlas::the_advances_are_the_shipped_font_s_own`. Deathtouch greens the
**power alone** (`cardplate::Tone`, read off the rail's badges so the mark
and the colour cannot disagree); toxic is in the enum and reaches nothing,
because `board::keyword_bits` has no toxic bit.

Above the plate stands the **swing**: the net power and toughness the
permanent's ±1/±1 counters add, written `+2/-1` one size down, green when it
grew and violet when it shrank. It replaced a column of stamped chips (pips
to six, a colour per kind) that the owner read as saying nothing — a green
disc with three pips is a rebus for `+3/+3` and the plate below it was
already writing the answer. What that costs is named out loud in
`cardplate::counter_swing`: charge, time, level, keyword and
loyalty-on-a-non-planeswalker counters have no mark on the table any more,
and the tooltip that was to name them instead does not exist — they are drawn
and named **nowhere**, which `docs/observed-faults.md` 58 measures and which
is why a Class at level 2 is indistinguishable from one at level 1.
`+1/+1` and `-1/-1` annihilate
as a state-based action (CR 704.5q), so a net of nothing draws nothing. A
**saga** takes the plate itself — a square parchment page with a roman
chapter — which is why `Corner::of` decides the plate and the line together
rather than each on its own. `Corner::of_object` is the same answer for the
hover preview, which showed the printed body until it existed. Lore counters
are only ever on sagas (CR 714), so no subtypes are needed to recognise one
— and a `CardGroup` has none to offer.

Under the printed name, on the card's **left** margin, hang the **identity
slips** (`client-core/src/cardcrest.rs`): small paper tabs saying what the
permanent *is* rather than what it can do — a squirrel for a token, two
cards for a copy, a commander's shield, all three the Mana font's own and
out of the rail's atlas. They were a crown on the card's **top edge** and
then a column in its **right** margin, and the owner sent both back: the
first sat on the printed name, the second put something that is not a number
in the corner where the numbers are. `SLIP_TOP` is a constant and places the
tab's *top* edge; the tab hangs into the art, because a modern frame opens
its art where it closes its title bar and there is no band between them.
Three things changed with the move and each withdrew an argument the column
had made — the slips **pack** (a lone commander takes the first slip), they
are **coloured** (verdigris, violet, gilt: the stock, not ink on the mark,
which is what makes packing safe), and they **move** (one band of light
about every thirty seconds, rarer than anything else on the card). The
papers are **linear** constants over an sRGB framebuffer: the first draft's
displayed at 221–246 and a 45% sheen moved them 16 levels, under the 20 a
still mark already swings. Three procedural drawings left with the first
move, and `sd_tri`, `sd_circle` and the triangle-winding test with them —
that lesson is in the shader's prose now, because a floor of nought over a
population of nought is exactly the vacuous assertion it warned about.

The stack is drawn as cards, not as a list of names. Each entry in
`hud::spawn_stack_panel` is the spell's own picture — or, for an ability,
the picture of the permanent it came from (`StackKind::Ability { source }`,
because an ability has no card of its own) — followed by a row of everything
it targets, each drawn as its own smaller card. The lookup that makes that
possible is in the model, not the renderer: `BoardModel::from_view` resolves
each `TargetRef` into a `StackTarget { what, name, art }` through
`PlayerView::object`, and a target's art joins `required_images` because a
spell can point at a card in a graveyard nothing else is drawing. A player
target keeps `name: None` — seat names live in `GameStatic`, which the board
model has never carried.

An ability's full row also says what it **does**, rather than that it is an
ability: the host names the printed sentence it came from as
`StackText { face, line, of }` and `card_face::sentence_blocks` cuts the
catalog's oracle text at that index, so a loyalty ability reads as its own
paragraph instead of as "+1". The count is the guard and it is *English* —
the host generated it from `baylee_cards::lines`, so a text of a different
length is refused whole rather than indexed into, which is what a translation
one sentence shorter would be. The borrowed picture comes from the face the
sentence came from and not the face the source is showing, because an ability
on the stack is independent of its source (CR 113.7a).

**The shelf has three attachments, and all three outlive its rebuild.**
`hud::ledge` despawns every child of the Actions Bar on each sentence, so
anything that has to survive a change of question hangs beside it rather than
in it: the **drawer** (`ledge::drawer`, centred, grows upward, carries the
pick hint and the stepper), the **tray** (`ledge::tray`, right-aligned,
shorter, one button today and room for more) and the **mana pool**
(`ledge::pool`, the tray's mirror on the left — retained so a pip that
arrives can be seen arriving). Each has a revision counter of its own for the
same reason: two things change on different clocks and one counter would have
to lie about one of them.

The two strips are one node: `ledge::strip_node` decides the height, the
pixel of overlap with the shelf's lip, the corner and the `EDGE` inset, and
each side supplies only which end it hangs off. The owner asked for the pool
*as* the tray — *"Es soll symetrisch zum Tray aussehen nur auf der linken
Seite"* — and a second copy of those four numbers would be symmetric only on
the day it was typed. What is left on the shelf's left is the hand's sorting
buttons, moved to `EDGE` and shrunk to `TOOL_H`; `LEFT_RESERVED` is gone and
`tools_reserved(window_w)` is what `arrange` is fed.

The tray is the zone dialog's door, and `hud::tray` is **not** the tray —
that module is the dialog itself and its name predates the owner's word.
Renaming it to `hud::zones` is 409 occurrences and has not been asked for; the
map is in `ledge::tray`'s own module doc. What the tray changed is the
vocabulary and not the state: `Browser::close` always kept the ticks, the
filter and the placement, and a button standing on the shelf is what makes
"minimised" the honest word for it. `Browser::may_be_put_away` is the single
predicate all four doors ask — `G`, `Escape`, the sheet's own head button and
the tray's — after two of them had spent their whole existence closing a
question's sheet that `Browser::follow` re-opened a frame later.

**Putting it down is a movement, so `sync_tray` does not despawn it.** It
writes `TrayReveal::closing` and returns; `reveal_tray` flies the sheet to
`ledge::tray::zones_button_centre` and takes it off the tree at the end,
`.after(sync_tray)` so a sheet reopened on the last frame of a flight is not
despawned by one system and rebuilt by the other. The gate is on `showing =
drawn && !closing`, or the body tears down the thing that is still moving on
the very next frame. The dialog's veil is marked `TrayVeil` for the same
handover — `TableVeil` is worn by the end screen too, and tearing down every
one of them is a finished game losing its darkening.

The head carries **both** size buttons now (`TrayMinimise`, `TrayMaximise`);
the resize corner kept the drag and lost its hidden maximise-on-tap, which had
been a four-pixel `TAP_SLOP` justified only by the `⤢` it wore. A maximise
travels the **rectangle** (`Placement::lerp` through `input::glide_the_sheet`)
and never a `UiTransform`: a stretched transform carries the type column and
the row heights with it and snaps straight at the end. Any control added to
the head has to join `tray_drag`'s press-exclusion list, or pressing it nudges
the sheet first and saves that.

The question itself is a **sheet**, and two things about sheets are easy to
get wrong twice. `hud::sheet()` on a panel paints that panel's *content box*,
so any padding shows as a ring of flat `PARCHMENT` around the grain with the
sheet's corners cut inside the panel's — `hud::sheet_surface()` is the
parchment as an absolutely-positioned first child instead, because an
absolute child is measured against its parent's *padding* box. And a `Text`
is a `Node`: a label inside a button is a pickable child in front of it, so
every label inside a control carries `Pickable::IGNORE` or `Feel` animates
the button in its padding and goes dead across the middle. The slip's prose
is **Faustina Italic**, a second file rather than a switch (nothing
synthesises an oblique from an upright), and its
bracketed asides are greyed by `client_core::prose::bracketed`, which refuses
to grey an unclosed bracket.

**The interface is two families, not one.** Alegreya Sans carries the
interface and Faustina carries what a card *says* — a rules paragraph is a
quotation and should not share a voice with the button beside it. That
overrides `docs/design.md` §1.2, which shipped three cuts of Inter and said
there was no fourth; the override is recorded in both files. `hud::UI_SCALE`
(1.2) and `hud::SERIF_SCALE` (1.1) multiply a caller's nominal size on the
way into `TextFont`, because every size here was chosen against Inter's
0.546 em x-height and the two new faces are authored at 0.458 and 0.494 —
so three hundred call sites keep their numbers, and `stack::CHAR_WIDTH`'s
0.52 still brackets both faces (0.534 and 0.520 against the nominal). Every
number a *font* produced was re-measured instead: `docs/client.md` §"Two
families" lists them, and the zone badge was 1.2 px short until it was. The answers share the sheet's width with
`flex_grow: 1` and a `flex_basis` of **zero** — grow alone divides only the
slack left after the labels.

Nothing on the table is positioned directly. `table::sync_scene` writes a
`Motion` target and `table::glide` moves the card there, so a repacked lane, a
tap, a hover and a card entering play all animate through one door and cannot
desynchronise from the board model. The curve is exponential
(`1 - e^(-rate·dt)`) so it is frame-rate independent and so a half-millimetre
correction does not take as long as a card arriving. `ShownRig` does the same
for the camera, faster and with yaw interpolated the short way round;
`Preferences::reduce_motion` turns both off.

Every key comes from the account's `Keymap` (`baylee-client-core/src/prefs.rs`),
resolved through `crates/baylee-client/src/keys.rs` — the one place that knows
a stored key name is a Bevy `KeyCode`. Input handlers ask *actions*, never
keys; `W` and `⇧W` are two chords and telling them apart is the keymap's job.
The keymap, the standing orders and the automation switches travel with the account
over `GET`/`PUT /settings`, and `crates/baylee-client/src/settingsui.rs` is
where a player changes them.

The interface speaks the player's language, and `Phrase` is an enum rather
than a key into a file: `baylee-client-core/src/i18n.rs` writes one arm per
language with a macro, so **a phrase with no German is a compilation error**
and there is no fallback that renders half a screen in English. Two tests hold
it — every phrase answers in every language, and a phrase's `{0}`/`{1}` set is
the same in all of them. The lobby's own lines go through `Lobby::note`, the
shell's through `Lobby::tell`/`unseat_because`; the gateway's `{"error":…}`
stays in the gateway's words, because translating those means a code beside
the prose and that is a protocol change. Values that are also identifiers
(`"sharp"` for a house AI) keep their wire spelling and translate only the
label. `ClientSettings.lang` feeds both readers — the catalog's `lang=` and,
through `Lang::of`, the interface itself — and the picker in `settingsui.rs`
writes it on the click. `docs/client.md` §"The interface's own words" is
normative.
