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
glows this turn. `cardmat::glow_bits` narrows the engine's `u128` to the three
bits the shader reads; a test pins each one against `KeywordSet`, because that
numbering is generated and a card glowing for the wrong keyword would be a
rules lie a player would believe. Indestructible is darksteel — a hard dark
blue-grey with a specular line, the card made of something rather than lit by
something; hexproof is a steady green sheath; shroud is the same idea taken
further, colder and hazier, since not even its controller may target it. Two
keywords share the border rather than stacking to white.

The border is drawn *inside* the card, over its printed frame. The mesh is
exactly the card, and a glow that needed room around it would need every
layout in the client to leave room for it.

The whole thing is WebGL2-safe: uniforms only, no storage buffers, no texture
arrays. Animation reads `globals.time` from the view bind group, so nothing is
written per frame — a material is created once and never touched again while
it is on screen.

`cardmat::tests::the_card_shader_compiles` parses and validates the WGSL with
naga, the same front end wgpu uses. Without it a shader error would surface
only when a real pipeline is built, which on the web is the one environment
that cannot be debugged by looking at a filesystem. It caught a reserved
keyword on its first run.

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

Hosting an open table is the one asymmetric case. A game against the house and
a join are both playable the moment the gateway answers; an open table holds a
seat whose game does not exist yet, so the lobby keeps the table screen up with
a banner and re-reads the game list until somebody sits down opposite. Opening
the socket earlier would connect and close again with nothing on it.

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
arithmetic, and the plugin only draws it and forwards the taps.

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
stay the fast way to add. Leaving a deck with unsaved changes takes two
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

## Verification

- `cargo test -p baylee-client --test duel_flow` plays real games headlessly
  through the client's own path (host → view → board model → interaction).
- The wasm CI job type-checks `baylee-client` for `wasm32-unknown-unknown`.
  The browser-only paths (settings storage, entropy) compile nowhere else,
  so without that job they rot silently.
- Browser entropy needs both `.cargo/config.toml`'s `getrandom_backend` cfg and
  the `wasm_js` feature; either alone is not enough.
