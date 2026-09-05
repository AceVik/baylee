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
cargo run -p xtask -- forge-report                        # how far the card transcoder reaches, and what it needs next
cargo run -p xtask -- pool-dump --out /tmp/pool.txt       # every CardDef, for refactor equivalence diffs
cargo run -p xtask -- dev-table --seats 4 --ai sharp      # a seated dev ticket (add --play to launch the client)
cargo run -p xtask -- dev-table --seats 3 --teams 1,1,2   # the same, as a 2v1
```

`dev-table` skips the lobby's sign-in and deck-picking *screens* and nothing
else: the account (`dev@baylee.local`, reused across runs), the deck, the room
and every AI chair are made through the gateway's own HTTP routes, and what it
prints is an ordinary `SeatTicket`. The game therefore runs over the same
engine ⇄ gateway ⇄ client sockets as any other — it is not a `LocalHost`
shortcut, which is the whole point of having it. It also says `ready` and
`start` for the dev account, because a room does not start itself: a table of
more than two chairs used to be arranged and then left sitting there.
`--teams` puts the chairs on sides in seat order (`1,1,2` is a 2v1, `0` leaves
a chair on its own side), which needs three chairs or more — a duel already
has exactly two sides.

`codegen`, `explain`, and `card-batch` default `--forge` to
`../mtg/forge-reference/forge-gui/res/cardsfolder` (read-only GPL reference,
never copied into this repo) and `--cache` to `data/scryfall-cache`.

Running things:

```bash
cargo run -p baylee-client                               # Bevy duel client, solo vs AI
BAYLEE_DEV_CONTROL=28770 cargo run -p baylee-client --features dev-control   # drivable while unfocused
trunk serve index.html --release                         # from crates/baylee-client/ — browser client on :8080
BAYLEE_AGENT_TOKEN=$(openssl rand -hex 32) ./target/debug/baylee-gateway   # accounts/decks/lobby/proxy, 0.0.0.0:28766
BAYLEE_AGENT_TOKEN=<the same> ./target/debug/baylee-agent                  # starts one engine per game
./target/debug/baylee-engine-server                      # dev harness only, 127.0.0.1:28765
cargo bench -p baylee-engine -- --quick                  # numbers to compare against docs/perf-baseline.md
```

`dev-control` opens a loopback HTTP harness (`/health`, `/state`, `/key`,
`/text`, `/pointer`, `/scroll`, `/screenshot`) that drives and photographs the
client while its window is in the background — a compile-time feature, because
a remote-control socket in a shipped game binary is a cheat vector.
`docs/client.md` §"Driving the client without its window" has the protocol,
why a wheel is written twice, and the reason a click takes
three frames.

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
secret an agent presents; without it no agent may connect), `BAYLEE_SMTP_URL`
/ `BAYLEE_MAIL_FROM` / `BAYLEE_PUBLIC_URL` (confirmation mail — without the
first of them the gateway sends none and requires no confirmation, which is
the development default) and
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
it. A mechanic the DSL cannot express gets `Coverage::Partial("reason")` and a
`// NOT SUPPORTED:` comment; extend the DSL rather than working around it.
`docs/card-dsl.md` is the authoring contract, `docs/llm-learnings.md` gets
updated after every card batch.

#### Two readers write finished cards

A stub is not always a stub. `codegen` runs two readers before it falls back
to `// GENERATED STUB`, and each may only produce a card it understood *in
full*:

- `crates/baylee-cards-codegen/src/landgen.rs` reads a land's **printed
  text**. Land text is formulaic — thirteen sentence shapes cover most of the
  1206 unique lands Scryfall prints — so `{T}: Add {W} or {U}. This land deals
  1 damage to you.` becomes a two-effect mana ability, and the intrinsic mana
  of a Mountain Forest comes off the type line (CR 305.6), because Taiga
  prints nothing but reminder text.
- `crates/baylee-cards-codegen/src/forgegen.rs` reads a **forge-reference
  script** (read as an automated lookup, never copied).
  Names, costs, types and P/T are ignored there; Scryfall already carries
  them. What it reads is `K:` keywords, `A:`/`T:` abilities and the `SVar:`
  chains they link into.

The rule both obey is the whole design: **one unread clause and the card is
refused.** An unknown effect, an unclaimed parameter (`NoRegen$ True`), a
computed `SVar`, a keyword that is data rather than a bit — any of them and
the card stays an honest `Coverage::Unimplemented` stub. A generated
`Implemented` therefore means what a hand-written one means. Getting this
backwards would be worse than generating nothing: the deckbuilder offers
`Implemented` cards as playable.

`cargo run -p xtask -- forge-report` says how far the transcoder reaches and
ranks what the refused scripts need next. The ceiling is **our** DSL, not
Forge's — extend the DSL and the transcoder converts the gain into hundreds of
cards at once, which is what `Pump` did: it was the top blocker at ~2300
scripts, and `Effect::PumpTarget` moved the transcoder from 1886 to 2545
scripts read in full.

That entry is also the cautionary tale about reading the report as a list of
missing *subsystems*. `Pump` did not need a new one: `PumpFilter`,
`EffectFilter::ObjectIs` and the `Layer::PtModify` machinery were all already
there, and `CreateContinuousEffect { filter: &Filter::This }` had bound to the
first target since M2. What was missing was one variant that says "the
target" without overloading a `Filter` to mean it, takes `Amount`s so `+X/+X`
is expressible, and carries `KW$` in the same effect. Read a blocker by
finding what the DSL cannot *say*, not by assuming the mechanism is absent.

That entry used to be followed by one reading "supported effects, unsupported
parameters (~4200)", described as the honest-stub rule showing its cost. Most
of it was not that. `refusal_cause` was *guessing* — re-reading the script and
naming the first thing it did not recognise — and the bucket was where every
refusal it could not explain ended up. The transcoder now reports its own
first refusal (`forgegen::unclaimed_parameter`), and the entry splits into an
API with no rule at all (a missing effect) and a rule that met a value it
cannot say (a missing case in one that exists), which are different work.

The rule that still holds: a key claimed there has to be claimed by a *rule*,
never by adding it to `PROSE_KEYS` to make the number move. And the reason
the transcoder reports rather than a table listing each rule's keys is the
one `SUPPORTED_APIS` already answers — the copy would rot the first time a
rule learned a new key, and a stale worklist is worse than none.

The current top entry, `S:` static abilities, is *partly* read: `Mode$
Continuous` becomes one `AbilityDef::Static` per layer it touches, because a
printed sentence usually is several ("get +1/+1 and have flying" is layers 7c
and 6, applied in that order by CR 613.1). What is left there needs genuinely
new rules — `IsPresent`/`Condition` (a static that is only sometimes on),
`AddAbility`/`AddTrigger` (granting an ability rather than a bit).

A generated card is hand-owned from then on: `codegen` only writes files that
are missing or still carry the `// GENERATED STUB` marker.

A card file is written with the macros in `baylee-cards-dsl/src/build.rs` and
opens with one import, `use baylee_cards_dsl::prelude::*;`. `card!` and
`face!` are the `CardDef`/`FaceDef` literals with their `..DEFAULT` tail
supplied (and `card!` writes the doc comment on the `pub static CARD` it
defines); `mana_ability!`, `activated!`, `triggered!`, `spell!`, `loyalty!`
and `mode!` do the same for the five ability shapes that make up most of the
pool. **Never restate a default** — that rule is why all of it exists.

What is load-bearing about the ability macros is that their defaults are
*rules* defaults, not merely common ones: instant speed is CR 602.2, the
battlefield is CR 113.6, and `mana_ability: false` is CR 605.1 making a mana
ability the exception. That last one is why a mana ability has its own macro
instead of a flag — an ability wrongly marked `true` would silently skip the
stack, and nothing in the test suite reads that as a rules bug. Fields with no
rules answer (a trigger, an effect list) are positional arguments, so they
cannot be forgotten.

Filters compose inline; a slice promotes to `'static` in a `static`. Named
predicates live on `Filter` itself (`CREATURE`, `NONLAND`, `BASIC_LAND`,
`INSTANT_OR_SORCERY`, …) and pool-specific ones in
`crates/baylee-cards/src/filters.rs` beside `crate::tokens` — the line is
whether the knowledge is about Magic or about *this pool*. Before that split,
"a creature" was spelled out in a differently-named `static` in twenty-six
card files.

Several tests exist purely to turn convention into a build failure — a card
sitting at the wrong `CardIndex`, or a card claiming a keyword no rule reads.
Expect that shape when adding data. For a change that is meant to move no
rules at all, `cargo run -p xtask -- pool-dump --out <path>` renders every
compiled `CardDef`; take one before and one after and diff. That is what held
the macro refactor of all 197 card files to a byte-identical pool.

### Hidden information is unrepresentable, not omitted

`baylee-view` carries **projected** characteristics (P/T after anthems, a
clone's name, an animated land's types, the mana a Chromatic Lantern's grant
lets a land make), because a client cannot run the layer system. Hidden
information has no field to leak through: libraries are counts,
another seat's hand is a count, a face-down permanent's `card` is `None` for
anyone not entitled to look, and `crates/baylee-gamehost/src/view.rs` has a
test per sentence of that. `VIEW_VERSION` (`crates/baylee-view/src/lib.rs`,
currently 11) is asserted in gamehost and client tests — bump it on any breaking
view change so a client refuses a host it cannot render.

The last of those is the one that says why the whole field exists.
`PublicObject::granted_mana` is not decoration: an ability a continuous effect
grants is offered under the synthetic index `choice::GRANTED_ABILITY` and is
printed on no card, so a client knew the handle and not what came out of it —
and the mana planner counted such a land for nothing. One lookup
(`effects::granted_activated`) and one reading (`baylee_cards_dsl::simple_mana`)
serve every caller, because an offer and a projection that disagreed would be
a land the planner counts on and the engine refuses. `docs/protocol.md`
§"Granted mana" is normative.

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
lives in the engine process, because that is where `awaiting_seat()` and the
session's counters can be read; it is anchored to `decision_seq` — questions
asked, not frames sent, so an opponent's priority hold or reconnect cannot wind
it — and does not run for a seat with no socket. And one process per game *is*
the
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

`GET /lobby/games` describes the whole arrangement in display names, never
account ids, with `you`/`yours` answering "is that me" and `startable` saying
whether the host's button would do anything. It answers **one page** —
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
unreadable. The stage therefore has no light in it at all, and the camera
carries `Tonemapping::None` so a future Bevy default cannot quietly treat
display values as radiance.

The table nonetheless shipped as a black screen with two gold rings in it,
and the cause was none of that: `OwnBoardOverlay` is an opaque
`palette::PANEL` (88% black) the width of the canvas, and it **defaulted to
open**, so the felt, the mats, the cards and every animation were behind it
from the first frame. `Duel::overlay_open` is opt-in now. The felt was also
authored about four times too dark, which a one-sided "dark enough"
assertion let through; that bound goes both ways now. `docs/client.md`
§"The table itself" has the measurement that found it — a red clear colour
renders `(234, 51, 35)` in stock Bevy and `(62, 19, 21)` here, and a clear
colour touches no material, texture or shader.

The **camera frames the table against the part of the window it is seen
through**, which is not the window: the tab strip, the hand bar and the phase
rail are overlays on the same full-window camera and cover about a quarter of
it. A hard-coded 20-unit rig aimed at the middle of the felt put the local
seat's own mat *underneath the hand bar* on every screen.
`table::CameraRig::home(layout, canvas)` computes it instead, from
`TableLayout::extent` (each pod's box rotated by its `facing`, because a seat
on your left plays across the table) and a `Canvas` naming what the HUD
covers; `table::frame_table` reapplies it as seats, focus and window change
and stops as soon as the player has aimed the camera themselves. The
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
reads the WGSL and fails when the two drift. The plate is one `u32` (three
ten-bit numbers, two kind bits) riding the material key beside `glow`, so a
creature dealt damage becomes a different material and redraws with no second
pass, and a number too big to pack clamps rather than wrapping. Damage is the
plate filling from the bottom to `damage / toughness`, not a third numeral;
numerals are a 4×6 stencil, because there is no text on the 3D table. Adding
loyalty to the plate meant adding it to `ObjectSummaryKey` too — it is drawn
now, so two walkers of a name must stop grouping.

Above the plate stand the **counter chips**: three flat discs, pips to six and
numerals from seven, with a fourth kind collapsing to `+N`. Colour is the only
channel left to say *which* counter, so `Chip::tint` is in the model where a
test reaches it and the badge tooltip is what will name them; two more `u32`s
carry them, because a tint and a count four times over do not fit in one. A
**saga** takes the plate instead — a square parchment page with a roman
chapter — and then draws no lore chip, which is why `Corner::of` decides the
plate and the chips together rather than each on its own. `Corner::of_object`
is the same answer for the hover preview, which showed the printed body until
it existed. Lore counters are only ever on sagas (CR 714), so no subtypes are
needed to recognise one — and a `CardGroup` has none to offer.

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
The keymap, the phase rail and the automation switches travel with the account
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
