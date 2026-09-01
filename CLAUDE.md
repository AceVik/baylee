# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

`AGENTS.md` is the short contract (conventions, legal guardrails) and stays
authoritative — read it too. This file adds the commands it does not list and
the architecture you would otherwise have to reconstruct from a dozen files.

## Commands

```bash
cargo build --workspace                                  # build
cargo test --workspace --all-targets                     # test
cargo clippy --workspace --all-targets -- -D warnings    # lint (pedantic, CI-enforced)
cargo fmt --all                                          # format
```

Single tests. Most engine tests are inline `#[cfg(test)]` modules under
`crates/baylee-engine/src/engine/*_tests.rs`, so the module path is the filter:

```bash
cargo test -p baylee-engine keyword_tests                # whole module
cargo test -p baylee-engine --lib -- --exact engine::keyword_tests::no_card_claims_a_keyword_the_engine_ignores
cargo test -p baylee-engine --lib -- --list              # discover exact names
```

Card, codegen, and data tooling (`xtask`):

```bash
cargo run -p xtask -- codegen            # regen subtypes, card stubs, registry, forge index
cargo run -p xtask -- codegen --check    # CI: fail if generated files are stale
cargo run -p xtask -- validate           # card headers vs. the CardDef the code builds
cargo run -p xtask -- explain --name "Force of Will"      # Scryfall + forge data side by side
cargo run -p xtask -- card-batch --cards "A,B"            # LLM task packages for unimplemented cards
cargo run -p xtask -- dev-table --seats 4 --ai sharp      # a seated dev ticket (add --play to launch the client)
```

`dev-table` skips the lobby's sign-in and deck-picking *screens* and nothing
else: the account (`dev@baylee.local`, reused across runs), the deck, the room
and every AI chair are made through the gateway's own HTTP routes, and what it
prints is an ordinary `SeatTicket`. The game therefore runs over the same
engine ⇄ gateway ⇄ client sockets as any other — it is not a `LocalHost`
shortcut, which is the whole point of having it.

`codegen`, `explain`, and `card-batch` default `--forge` to
`../mtg/forge-reference/forge-gui/res/cardsfolder` (read-only GPL reference,
never copied into this repo) and `--cache` to `data/scryfall-cache`.

Running things:

```bash
cargo run -p baylee-client                               # Bevy duel client, solo vs AI
trunk serve index.html --release                         # from crates/baylee-client/ — browser client on :8080
BAYLEE_AGENT_TOKEN=$(openssl rand -hex 32) ./target/debug/baylee-gateway   # accounts/decks/lobby/proxy, 0.0.0.0:28766
BAYLEE_AGENT_TOKEN=<the same> ./target/debug/baylee-agent                  # starts one engine per game
./target/debug/baylee-engine-server                      # dev harness only, 127.0.0.1:28765
cargo bench -p baylee-engine -- --quick                  # numbers to compare against docs/perf-baseline.md
```

Always build the browser client `--release` (a dev-profile wasm is ~350 MB vs
~36 MB). Servers are quiet without `RUST_LOG=info` — the tracing subscriber
reads `EnvFilter::from_default_env()`.

The card catalog (card text, not images) lives in PostgreSQL and is optional:

```bash
docker compose up -d                                     # postgres 18 on :5432
export DATABASE_URL=postgres://baylee:baylee@127.0.0.1:5432/baylee
cargo run -p baylee-catalog -- ingest                     # ~118k English printings, ~30 s
cargo run -p baylee-catalog -- ingest --all-languages     # every language (392 MB)
cargo run -p baylee-catalog -- search "lightning bolt"
```

Without `DATABASE_URL` the gateway starts as before and simply serves no card
text; the client then draws faces from what the engine projects. Copy
`.env.example` to `.env` for the client's `BAYLEE_GATEWAY` and this URL.

A gateway with no agent connected hosts no games — `POST /lobby/games` answers
`503`. The gateway links neither the engine nor gamehost; see "The gateway runs
no rules" in `docs/protocol.md` for the whole circle.

Env vars: gateway takes `PORT`, `STORE_PATH` (default `gateway-store.json` in
the working directory, and *not* gitignored), `BAYLEE_REGISTRATION=off`,
`BAYLEE_TRUSTED_PROXIES`, `DATABASE_URL`, `BAYLEE_AGENT_TOKEN` (the shared
secret an agent presents; without it no agent may connect) and
`BAYLEE_ENGINE_URL` (what an engine is told to dial, default
`ws://127.0.0.1:{PORT}/engine/ws` — right for one box, wrong the moment an
agent runs elsewhere). The agent takes `BAYLEE_GATEWAY`, `BAYLEE_AGENT_TOKEN`,
`BAYLEE_AGENT_NAME`, `BAYLEE_AGENT_CAPACITY` (0 = no limit) and
`BAYLEE_ENGINE_BIN` (default: `baylee-engine-server` beside the agent). An
attached engine takes `--attach <ws>` `--game <id>` `--token <tok>`, or the
same three as `BAYLEE_ATTACH_URL`/`BAYLEE_GAME`/`BAYLEE_ENGINE_TOKEN`; with
none of them it falls back to the listening dev harness, which takes `PORT` and
`BAYLEE_BIND` and defaults to loopback deliberately: it has no authentication,
every action runs as seat 0, so binding it publicly hands out the game. The
client takes `BAYLEE_GATEWAY` (card text, and the table it plays
at) plus `BAYLEE_GAME`, `BAYLEE_SEAT_TOKEN` and optionally `BAYLEE_SEAT`: with
those three it plays against the gateway instead of against the house AI in
process. In a browser the same handover is `?game=…&token=…` on the page URL.

CI (`.github/workflows/ci.yml`) runs more than the four commands above: the
test suite **also in `--release`** (a `debug_assert!` once hid mana payment
from every release build), `codegen --check`, `validate`, a
`wasm32-unknown-unknown` check of the client, benches, an MSRV check against
exactly 1.88, `cargo-deny`, and `cargo-audit`.

## Architecture

### One-way data flow, and why each seam exists

```
baylee-core ──> baylee-engine ──> baylee-gamehost ──> baylee-engine-server
     │               │                   │
     │               │                   └── builds ──> baylee-view ──┐
     │               └── choice taxonomy ──┬──────────────────────────┤
     │                                     └──> baylee-ai (plays from the view)
     └──────────────────────────────────────> baylee-client-core <────┘
                                                     │
                                                     └──> baylee-client (Bevy)

baylee-protocol ──> baylee-gateway   (axum, lobby, store — no engine, no gamehost)
       └─────────> baylee-agent      (protocol and std::process, nothing else)
```

Each arrow drops a capability on purpose, so test what you can without the
layer above:

- **`baylee-view`** does not depend on the rules kernel at all — only on
  `baylee-core` ids plus serde. A spectator overlay needs nothing else.
- **`baylee-client-core`** holds the whole client brain (layout, board model,
  interaction state machine, image policy) and knows no renderer, which is why
  it carries the bulk of the client's tests.
- **`baylee-protocol`** is the protobuf wire framing (`Envelope`); complex engine
  structures (`Pending`, `PlayerAction`) ride inside it as `serde_json` payloads.
  Both servers and `LocalHost` use it, so an in-process duel and a networked one
  exercise the same envelopes.
- **`baylee-client`** is the only crate that needs a GPU.

`baylee-core`, `baylee-protocol`, `baylee-view`, `baylee-client-core` and
`baylee-client` must all keep compiling for `wasm32-unknown-unknown`.

### The engine advances only through choices

`Engine` exposes essentially `pending()`, `apply(player, action)`, `state()`,
`journal()`, `snapshot_hash()`. There is no "cast this spell" method: the
engine publishes a `Pending` with *enumerated legal actions*, and
`apply` validates the answer against that same enumeration. A client cannot
name an option the engine did not offer — combat is the clearest case: which
creatures may attack, which defenders may be attacked (CR 508.1a) and which
blocker may be paired with which attacker all come from
`Pending::ChooseAttackers` / `ChooseBlockers`, not from the client's own
candidate list.

Consequences worth knowing before you touch the engine: continuous effects are
a cached layer projection (validity = one `u64` generation compare), events go
propose → replacement → apply → journal → triggers, SBAs run as a fixpoint
before every priority grant, and endless loops are found with Brent's
algorithm over a rules-visible `loop_signature` (not `snapshot_hash`, which
never repeats). `docs/engine-internals.md` is normative on all of this.

Determinism is the constraint behind most engine rules: seeded ChaCha8, no
`HashMap` iteration in hot paths, and `std::time`, `std::random` and the
`algebraic_*` float methods are banned outright in `baylee-engine`/`-core`.
The engine is also strictly synchronous — async lives only in `engine-server`,
`gateway` and `agent`, and in all three it is transport, never rules.

### Cards: generated stubs, hand-finished, machine-checked

`cargo xtask codegen` writes the stub, the registry tables
(`crates/baylee-cards/src/generated.rs`, `cards/mod.rs`) and subtype constants
(`crates/baylee-core/src/generated/subtypes.rs`). You then edit **only**
`coverage`, `keywords`, `abilities` in `crates/baylee-cards/src/cards/<slug>.rs`;
`index`, `oracle_id`, `scryfall_id` and `faces` stay as generated. The index
comes from the append-only ledger `data/card-index.tsv`, so a new card never
renumbers an existing one — a `CardIndex` is what saved decks and replays
name. The `//!`
header (name, cost, oracle text, set, Scryfall id) is the human-verification
surface and `xtask validate` fails if it drifts from the `CardDef` built below
it. Field defaults come from `CardDef::DEFAULT` / `FaceDef::DEFAULT` via a
struct-update tail — never restate a default. A mechanic the DSL cannot express
gets `Coverage::Partial("reason")` and a `// NOT SUPPORTED:` comment; extend
the DSL rather than working around it. `docs/card-dsl.md` is the authoring
contract, `docs/llm-learnings.md` gets updated after every card batch.

Several tests exist purely to turn convention into a build failure — a card
sitting at the wrong `CardIndex`, or a card claiming a keyword no rule reads.
Expect that shape when adding data.

### Hidden information is unrepresentable, not omitted

`baylee-view` carries **projected** characteristics (P/T after anthems, a
clone's name, an animated land's types), because a client cannot run the layer
system. Hidden information has no field to leak through: libraries are counts,
another seat's hand is a count, a face-down permanent's `card` is `None` for
anyone not entitled to look, and `crates/baylee-gamehost/src/view.rs` has a
test per sentence of that. `VIEW_VERSION` (`crates/baylee-view/src/lib.rs`,
currently 7) is asserted in gamehost and client tests — bump it on any breaking
view change so a client refuses a host it cannot render.

The print table is the one place that rule was broken, and not through a view:
`GameStatic.prints` is shared by the whole game and deduplicated per card, so a
seat sent all of it was being sent the union of every decklist at the table.
Its entries are now `Option<PrintEntry>` — a seat starts entitled to its own
deck's printings and earns the rest by seeing the cards, and `Session` re-sends
the payload (before the view that needs it) when one is earned. A hole rather
than a shorter list, because the index *is* the `PrintRef` every object points
at.

The house AI is held to the same line, and by the type system rather than by
convention: `HeuristicAgent::act` takes `(&PlayerView, &Pending)` — what a
networked seat gets — so `baylee-ai` cannot reach an opponent's hand even by
mistake. That is also why the AI-vs-AI harness (`gamehost::harness::play_game`,
with the acceptance-deck soak) lives in gamehost: building a view takes the
engine, which is the boundary the agent may not cross.

The engine never carries card text, so a client names an ability through
`AbilityRef { card, index }`; reserved indices (`SPELL`, `ENTERS`, …) count
down from `u32::MAX`. The same handle addresses per-account standing answers,
which is how the gateway replays "always say yes to this trigger" into a new
game.

### The gateway runs no rules

A game lives in an engine process an **agent** started and that dialled the
gateway back. The gateway routes between that process and the seats, and links
neither `baylee-engine` nor `baylee-gamehost`:

```
gateway ── StartEngine ──> agent ── spawn ──> baylee-engine-server
gateway <── EngineHello / SeatFrame / GameEnded ──┘
gateway ── GameSetup / SeatAttached / SeatFrame ──> engine
gateway ── the seat sockets, byte for byte ──────> clients
```

`SeatFrame { seat, envelope }` nests an *encoded* player-facing `Envelope`, so
the gateway forwards bytes it never decodes and the player-facing protocol
keeps exactly the shape it had. Three sockets, three secrets, none of them
interchangeable: `BAYLEE_AGENT_TOKEN` on `/agent/ws`, a per-game engine token
on `/engine/ws`, a seat token on `/games/{id}/ws`.

Two things moved out of the gateway with the rules. The **decision clock** now
lives in the engine process, because that is where `awaiting_seat()` and
`seq()` can be read; it is anchored to the sequence number it was armed at, and
does not run for a seat with no socket. And one process per game *is* the
**panic boundary**, so the `catch_unwind` around every rules call is gone.

`docs/protocol.md` ("The gateway runs no rules") is normative, including why a
seat's frames are dropped in the engine rather than at the gateway while it has
no socket, and why losing the engine link ends the game.

The gateway's e2e tests run the engine in-process (`EngineRunner` over the real
`/engine/ws`), which is what keeps them fast and independent of what happens to
be built. The version with all three real processes is `#[ignore]`d:

```bash
cargo build --workspace --bins
cargo test -p baylee-gateway --test e2e_processes -- --ignored
```

### Hosts, and where the client actually gets its game

The renderer never touches a socket; it talks to a `DuelHost`, of which there
are two. `LocalHost` is an in-process engine, solo vs the house AI from
`data/acceptance-decks.txt`; `NetworkHost` (`src/net.rs`) is a websocket to
the gateway's `/games/{id}/ws`. `crates/baylee-client/src/main.rs` picks
between them on whether this launch was handed a `SeatTicket` — `BAYLEE_GAME`
+ `BAYLEE_SEAT_TOKEN` natively, `?game=…&token=…` in a browser — and nothing
above the host can tell which it got, because both decode the same protobuf
envelopes with the same function.

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
anyone plays: `POST /lobby/games {seats: 2..=4, name}` opens one, `POST
/lobby/games/{id}/seats/{seat}` arranges a chair, `POST .../leave` frees one.
Two authorities that do not overlap — the host sets `kind`/`ai` on any chair
nobody is sitting in, every player sets `deck_id` on their own and nothing
else. There is deliberately **no start route**: a room starts the moment every
chair is ready (a human chair with an account and a deck, or an AI chair, which
is ready as soon as it is configured), which is the same rule the two-seat open
table always followed when its second player sat down. `GET /lobby/games`
describes the whole arrangement in display names, never account ids, with
`you`/`yours` answering "is that me". `docs/protocol.md` §Rooms is normative.

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
and re-reads `GET /lobby/games` until that table's state turns `"playing"`.
Nothing pushes that news; there is no socket to push it on.

A card is a **slab**, not a decal: a rounded face with a thin wall around its
edge (whose UVs borrow the face's, so the edge is the card's own border
colour) and a contact-shadow child under it. Neither reads at a camera exactly
overhead, which is what `table::CAMERA_LEAN` is for — about 22° off vertical,
enough for both and far too little to bring a horizon into frame, which is
also why there is no sky behind the table and no point drawing one.

The 3D table under the cards is **generated, not shipped**:
`baylee-client-core/src/tabletop.rs` computes the felt, the centre medallion
and a seat's mat into RGBA8 buffers with a seeded value-noise fbm (no `rand`,
no clock — every player sees the same grain). `docs/legal.md` §2 decided it:
ornament is the easiest thing to borrow by accident, and arithmetic borrows
nothing. Every seat plays on its own mat, sized from its `SeatSlot`, banded
for the three lanes, with the rim carrying the seat's colour — gilt for the
viewing seat, the pie in ring order for the rest — and its brightness
carrying `Mood { local, Standing }`, so "whose turn" and "who is everyone
waiting for" are answered on the felt. Everything down there is `unlit`
deliberately: scene lighting on card art would make colour identity
unreadable.

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

Every key comes from the account's `Keymap` (`baylee-client-core/src/prefs.rs`),
resolved through `crates/baylee-client/src/keys.rs` — the one place that knows
a stored key name is a Bevy `KeyCode`. Input handlers ask *actions*, never
keys; `W` and `⇧W` are two chords and telling them apart is the keymap's job.
The keymap, the phase rail and the automation switches travel with the account
over `GET`/`PUT /settings`, and `crates/baylee-client/src/settingsui.rs` is
where a player changes them.
