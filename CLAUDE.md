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
```

`codegen`, `explain`, and `card-batch` default `--forge` to
`../mtg/forge-reference/forge-gui/res/cardsfolder` (read-only GPL reference,
never copied into this repo) and `--cache` to `data/scryfall-cache`.

Running things:

```bash
cargo run -p baylee-client                               # Bevy duel client, solo vs AI
trunk serve index.html --release                         # from crates/baylee-client/ — browser client on :8080
./target/debug/baylee-gateway                            # accounts/decks/lobby, 0.0.0.0:28766
./target/debug/baylee-engine-server                      # one process per game, 127.0.0.1:28765
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

Env vars: gateway takes `PORT`, `STORE_PATH` (default `gateway-store.json` in
the working directory, and *not* gitignored), `BAYLEE_REGISTRATION=off`,
`BAYLEE_TRUSTED_PROXIES`, `DATABASE_URL`. Engine-server takes `PORT` and
`BAYLEE_BIND` — it defaults to loopback deliberately: it has no
authentication, every action runs as seat 0, so binding it publicly hands out
the game. The client takes `BAYLEE_GATEWAY` (card text, and the table it plays
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
baylee-core ──> baylee-engine ──> baylee-gamehost ──> baylee-{engine-server, gateway}
     │               │                   │
     │               │                   └── builds ──> baylee-view ──┐
     │               └── choice taxonomy ──┬──────────────────────────┤
     │                                     └──> baylee-ai (plays from the view)
     └──────────────────────────────────────> baylee-client-core <────┘
                                                     │
                                                     └──> baylee-client (Bevy)
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
The engine is also strictly synchronous — async lives only in `engine-server`
and `gateway`.

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
a front door. Two buttons exist only because nothing else does yet: "add the
starter deck" (no deck builder; it posts the acceptance file's `Allytifact`
rows) and "play the house AI offline" (a `LocalHost`, no account).

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
join both build the game's session before answering, but an **open** table
holds a seat whose game does not exist — `opening_payload` has no session to
describe, so a socket opened against it is accepted and closed again with
nothing on it. The lobby stays put and re-reads `GET /lobby/games` until that
table's state turns `"playing"`. Nothing pushes that news; there is no socket
to push it on.

Known gap, easy to misread from the code: `client-core`'s `Interaction` exposes
and tests `declare_attacker`, `declare_blocker` and `choose_index`, but
`crates/baylee-client/src/input.rs` calls none of them. Combat is currently
answered only by `automation::AutoAnswer::DeclareNoAttackers/DeclareNoBlockers`
— i.e. with empty lists. The client looks like it has combat and does not; the
missing wiring is in `input.rs`/`hud.rs`, not in `interaction.rs`.
