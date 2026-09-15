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

./scripts/gate-rules.sh                                  # the rules half only, while working
```

`gate-rules.sh` is fmt + `cargo lint-rules` + `cargo test-rules`, the last two
being aliases in `.cargo/config.toml` for `--workspace --exclude
baylee-client`. It exists because `baylee-client` is a Bevy crate and
dominates a `--workspace` run: an engine or card change that rebuilt it was
paying twenty minutes for a renderer it had not touched. The renderer is the
**only** exclusion, and that is deliberate — the aliases first named eight
crates by hand, leaving out `baylee-engine-server` (which links
`baylee-gamehost`) and `baylee-gateway` (whose e2e tests link it through
that crate), so a new variant on a public enum would have broken both with
the rules gate green. A hand-written list also goes stale the moment a crate
is added. It is a **filter, not a replacement**: it says nothing about the
client, so the full gate still runs before a push.

Single tests. Most engine tests are inline `#[cfg(test)]` modules under
`crates/baylee-engine/src/engine/*_tests.rs`, so the module path is the filter:

```bash
cargo test -p baylee-engine keyword_tests                # whole module
cargo test -p baylee-engine --lib -- --exact engine::keyword_tests::no_card_claims_a_keyword_the_engine_ignores
cargo test -p baylee-engine --lib -- --list              # discover exact names
```

Card, codegen, and data tooling (`xtask`):

```bash
cargo run -p xtask -- codegen            # regen subtypes, card stubs, registry, scripts index
cargo run -p xtask -- codegen --check    # CI: fail if generated files are stale
cargo run -p xtask -- validate           # card headers vs. the CardDef the code builds
cargo run -p xtask -- adopt --name "Yavimaya Coast"       # take a generated card off the machine, for good
cargo run -p xtask -- refresh-oracle                      # rewrite every `//! Oracle:` header from the cached printing
cargo run -p xtask -- explain --name "Force of Will"      # Scryfall + scripts data side by side
cargo run -p xtask -- card-batch --cards "A,B"            # LLM task packages for unimplemented cards
cargo run -p xtask -- transcode-report                        # how far the card transcoder reaches, and what it needs next
cargo run -p xtask -- cross-read                          # every hand-written card read a second way, and the disagreements
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

`codegen`, `explain`, and `card-batch` read the **card-script reference** — an
external, GPL-licensed corpus of rules scripts, read as an automated lookup
and never copied into this repo (`NOTICE` names it). It is not vendored, so
there is no path that is right for everyone and `xtask::scripts_root` asks
three questions instead of hard-coding one: `--scripts` if what it names
exists, then `BAYLEE_CARD_SCRIPTS`, then a `cardsfolder` directory found
within four levels of the repository's parent. `--cache` defaults to
`data/scryfall-cache`.

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
`/text`, `/pointer`, `/scroll`, `/screenshot`, plus the three clock routes
`/timescale`, `/pause` and `/step`) that drives and photographs the client
while its window is in the background — a compile-time feature, because a
remote-control socket in a shipped game binary is a cheat vector. The clock
routes are what make an *animation* photographable: almost everything worth
looking at here is over before a screenshot can be asked for (a card's exit
lives 0.55 s), so `Time<Virtual>` is slowed, stopped and advanced a counted
number of frames instead. `docs/client.md` §"Driving the client without its
window" has the protocol, why a wheel is written twice, the reason a click
takes three frames, and why zero is refused by `/timescale` rather than taken
as a pause.

The same harness reaches the client **on a phone**, and that is what decided
how a phone runs it at all: a browser build has no sockets, so Android is an
APK (`crates/baylee-client-android`, `scripts/mobile/android-build.sh`) and iOS
a hand-assembled `.app` (`scripts/mobile/ios-sim-run.sh`). The loopback bind is
unchanged — `adb forward tcp:28773 tcp:28770` reaches the *device's* loopback,
and the iOS simulator shares this machine's. `docs/mobile.md` is normative,
including the four different addresses the gateway has depending on where the
client is running, and which two renderers do not work yet.

A shader can be edited **while the game runs**, which is the other half of
working on a look:

```bash
BAYLEE_DEV_CONTROL=28770 BEVY_ASSET_ROOT=$PWD \
    cargo run -p baylee-client --features dev-control,dev-reload
```

`dev-reload` watches this crate's embedded WGSL, so an edit to `felt.wgsl`
repaints the table in a running client with no rebuild and no restart. Neither
variable is optional: without `BEVY_ASSET_ROOT` the watcher looks every changed
file up in the wrong place and reloads nothing — which used to fail *silently*
and now refuses to start — and without `BAYLEE_DEV_CONTROL` the harness opens
no socket, so the picture cannot be stopped while the shader is worked on.
`docs/client.md` §"Editing a shader without stopping the game" has the reason
and the proof shape.

Always build the browser client `--release` (a dev-profile wasm is ~350 MB vs
~36 MB). Servers are quiet without `RUST_LOG=info` — the tracing subscriber
reads `EnvFilter::from_default_env()`.

The browser client renders through **WebGPU**: the workspace's bevy features
list `webgpu`, and `webgl2` is the one-word alternative that would put wgpu
back on the GL backend. Every shader here nonetheless stays inside the older
GL budget — uniforms only, no storage buffers, no texture arrays, no
`bitCount` — because nothing drawn so far wants more and that keeps the
fallback one word away. Reaching past it is allowed; it is a decision a
commit has to state, because it is the commit that closes the fallback.

PostgreSQL holds two things that are only neighbours. **`baylee-db`** is the
gateway's own — accounts, sessions, decks, confirmation links, standing
answers, client preferences — six tables with entities and a migrator, and it
is **required**: a gateway without `DATABASE_URL` refuses to start, because a
gateway with no database has no accounts and a process answering every request
with a 503 looks healthy to anything watching it. A `gateway-store.json` left
over from before is imported on the first start against an empty database and
then moved aside, never deleted.

The **card catalog** (card text, not images) is the optional half, and is the
same database with hand-written SQL instead of entities — its value is in
three index definitions and two queries the planner shapes, where
`baylee-db`'s is that a column's two types cannot drift apart:

```bash
docker compose up -d                                     # postgres 18 on :5432
export DATABASE_URL=postgres://baylee:baylee@127.0.0.1:5432/baylee
cargo run -p baylee-catalog -- ingest                     # every language: 542k printings, ~3 min, 590 MB
cargo run -p baylee-catalog -- ingest --english-only      # ~118k English printings, ~30 s
cargo run -p baylee-catalog -- search "lightning bolt"
cargo run -p baylee-catalog -- project                    # rebuild the search projection alone
```

**Every language is the default and English is the opt-out**, which is the
opposite way round from Scryfall's two feeds. A card's printed text is the one
thing a player reads in their own language — the client asks `/catalog/text`
with the language it is set to and falls back to English printing by printing
— so an English-only catalog is not a smaller install, it is a client that
quietly speaks English to everyone. Measured on this machine: `all_cards` is
392 MB compressed, stores 542 142 printings in 19 languages in about three
minutes, and leaves the database at 590 MB against the 118k rows
`--english-only` stores. Run it with `RUST_LOG=baylee_catalog=info`: a plain
`RUST_LOG=info` puts every `INSERT` through the tracing subscriber and writes
a 92 MB log for one ingest.

**The search does not read those rows.** It reads `card_search`, one row per
oracle face rather than per printing — 41 991 against 554 242 — carrying every
language that face prints in, a fenced and Unicode-folded form of its names,
a bigram array with a GIN over it, and a `tsvector` of the rules text with
another. `Catalog::project()` rebuilds it wholesale at the end of an ingest,
and `migrate()` rebuilds it when its stamped version does not match the code's
or when it is empty beside a full `cards` — which is the upgrade an existing
install would otherwise come up silently broken from, because an empty
projection answers every search with nothing and never errors.

Two rules hold it, and both were paid for. **A name and a rules text are
never in one predicate**: an indexable expression `OR`ed with an
unindexable `ILIKE '%…%'` is satisfiable by no index at all, so Postgres
built a `tsvector` per row over the whole table — 2742 ms for `稲妻`, 2132 ms
for `li`. The tiers are unioned instead, and the representative printing is
resolved *after* the `LIMIT`. And **bigrams, not trigrams**: `pg_trgm` pads a
whole word, so `show_trgm('稲妻')` does return three trigrams, but a `%…%`
pattern is not padded and the index could not serve the query that needed it.
Measured against the live catalog, after against before: `稲妻` 1.7 ms against
2742, `li` 44 against 2132, `creature` 90 against 190, `flying` 17 against 81,
`aether` 1.9 against 12, and `Ｌｉｇｈｔｎｉｎｇ` — full-width Latin off a
Japanese keyboard — 2.1 ms against no answer at all. It costs 215 MB and
replaces 124 MB of index.

The only extension it needs is `unaccent`, installed `WITH SCHEMA public`
deliberately: a test runs the whole catalog in a schema of its own, and both
an unqualified `CREATE EXTENSION` and an unqualified `DROP INDEX` reach out
of that sandbox — the first leaves the extension where the next
`DROP SCHEMA … CASCADE` destroys it, the second deletes the developer's own
indexes. Both were observed, not imagined, and so is the third: a migration
asking `information_schema.columns` whether a column still needs converting is
asking about *every* schema on the path, and fires the conversion on a table
that has already had it. Every such lookup carries
`table_schema = current_schema()`, and two tests keep a second schema *behind*
the sandbox so the reach is observable rather than argued about.

**What the catalog stores as what** was re-measured rather than tidied.
`released_at` is a `date`: it sorted correctly as text only by the accident
that Scryfall writes ISO, all 542 177 rows converted losslessly, and `cards`
went 84 MB → 81 MB. The rest of that clean-up was **declined on the
measurement**. `finishes` and `frame_effects` as `text[]` take `cards` to
98 MB, because an array's header costs more than the seven distinct strings
`finishes` ever holds and nothing queries into either column. `rarity`,
`layout` and `border_color` as enums save about 1 MB and buy no honesty,
because nothing branches on the values — `layout` is written and never read
at all, and the other two pass through `Printing` to the wire as strings —
while an enum turns Scryfall's next new `layout` into a failed ingest. A
`date` renders itself through the server's `DateStyle`, so the reader says
`to_char(released_at, 'YYYY-MM-DD')` and never `::text`.

Without an **ingest** the gateway still runs every game and simply serves no
card text; the client then draws faces from what the engine projects. Without
`DATABASE_URL` it does not start at all. Copy `.env.example` to `.env` for the
client's `BAYLEE_GATEWAY` and this URL.

The test suite needs the same server: the e2e tests spawn real gateway
processes, and each one takes a **schema of its own** with its pool capped at
two — which is what lets three dozen of them share a server that allows a
hundred connections, and what keeps them off the `public` schema an ingest
has filled. CI runs a `postgres:18-alpine` service for exactly this.

A gateway with no agent connected hosts no games — `POST /lobby/games` answers
`503`. The gateway links neither the engine nor gamehost; see "The gateway runs
no rules" in `docs/protocol.md` for the whole circle.

Env vars: gateway takes `PORT`, `DATABASE_URL` (**required**), `BAYLEE_DB_POOL`
(how many connections one gateway keeps, default 8 — small because a stock
PostgreSQL allows 100 in total), `STORE_PATH` (default `gateway-store.json` in
the working directory, *not* gitignored, and now only a file to **import** on
a first start), `BAYLEE_ART_PATH` (the card-art
mirror, default `art-cache/` and gitignored; `off` disables it),
`BAYLEE_DECK_IMAGE_PATH` (the sleeve and the playmat a player uploads for a
deck, default `deck-images/` and gitignored for the same reason; `off` or an
empty value refuses uploads outright),
`BAYLEE_REGISTRATION=off`,
`BAYLEE_TRUSTED_PROXIES`, `BAYLEE_AGENT_TOKEN` (the shared
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
so a socket names its own seat (`JoinGame.seat_token` is a seat *number*
there) and is handed that seat's hidden information — binding it publicly
hands out every hand at the table. The
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
(`crates/baylee-cards/src/generated.rs`, `cards/mod.rs`), subtype constants
(`crates/baylee-core/src/generated/subtypes.rs`) and the ability-line table
(`crates/baylee-cards/src/generated_lines.rs` — which printed sentence each
ability came from, so a client can draw a stack entry as what the ability
*does*; `docs/client.md` §"Which ability is on the stack" is normative, and the table is
two-phase, so a card added by one run gets its row from the next). You then
edit **only**
`coverage`, `keywords`, `abilities` in that card's file;
`index`, `oracle_id`, `scryfall_id` and `faces` stay as generated — except
for the three fields a face has no printed data for in the Scryfall payload
codegen reads: `keywords`, `color_indicator` and `castable_from_hand`, which
a transforming double-faced card needs on its back face and which are added
by hand. The index
comes from the append-only ledger `data/card-index.tsv`, so a new card never
renumbers an existing one — a `CardIndex` is what saved decks and replays
name. The `//!`
header (name, cost, oracle text, set, Scryfall id) is the human-verification
surface and `xtask validate` fails if it drifts from the `CardDef` built below
it — **and** if its oracle text is not the one Scryfall prints, which is the
check that turns "a person could have read this card" into something a build
can say. The header is derived data, so `xtask refresh-oracle` writes it and
nobody retypes it; the first run of the pair found 145 disagreements, two of
which were cards built from their own wrong header (Volrath's Stronghold's
`{1}{B}` and Mirrorhall Mimic's disturb cost).
It also asks the printing what the card **is**: `pool::type_line(face)` —
supertypes, types and subtypes in printed order, the same string the
deckbuilder shows a player — against the `type_line` Scryfall prints, face by
face. That was the one printed characteristic nothing compared, and it is
where a hand-written card drifts in silence, because the `//!` header is held
against the *code*: Ondu Cleric said Human in both for as long as it existed
and the printing says Kor. Comparing the one renderer rather than a set built
for the check is what makes it cheap and what makes it reach past subtypes —
Karakas and Volrath's Stronghold print `Legendary Land` and were plain `Land`
in the code, so the legend rule (CR 704.5j) did not apply to either of them.
With that in place the header's own type segment is worth comparing, and the
two together close a triangle: the code is held against the printing, the
header against the **code** — the same `pool::type_line`, joined with `" // "`
for a card with two faces — so the header is right about the card rather than
merely agreeing with the file it sits in. On its own it would have said only
that a person typed the same thing twice, which is exactly what Ondu Cleric
did; it found thirteen lands headed `Land — SWAMP MOUNTAIN` in the constants'
spelling instead of the card's.
A mechanic the DSL cannot express gets `Coverage::Partial("reason")` and a
`// NOT SUPPORTED:` comment; extend the DSL rather than working around it.
`docs/card-dsl.md` is the authoring contract, `docs/llm-learnings.md` gets
updated after every card batch.

**Where a card's file sits is codegen's to say, never yours.** `cards/` is a
taxonomy rather than a flat list of slugs —
`<card type>/<second type or defining subtype>/mv_<mana value>/<slug>.rs`,
with a total order over card types picking the door (Land first, Kindred
last), the **front face** deciding whatever the layout, and lands taking a
semantic level instead of an `mv_` one because 888 of the 1124 in the pool
print no subtype at all. The rule and its reasons are
`baylee-cards-codegen/src/layout.rs`; `docs/card-dsl.md` §"Where a card's
file lives" is normative.

A land's level comes from **two** sources that must not be confused.
`data/land-cycles.tsv` is what no card prints — "fetchland", "shockland" —
so it is hand-kept and additive, and an entry naming a card the pool does not
have is a bail rather than a land quietly filed one level shallower.
`layout::land_role` is the opposite half and reads the printed text: about
thirty cycles by their own sentence (fast, slow, check, crowd, unlucky,
filter, pain, bounce, manlands, cycling, …), then `utility` for a land that
does something other than make mana and `tapland` for one that only comes in
tapped. It is why `lands/` holds 73 files instead of 872, and why growing the
map is a browsing improvement rather than a correctness fix.

Two properties are what make it safe to arrange 1365 files this way, and both
would be easy to lose. The **file moves and the module path never does** —
`cards/mod.rs` declares every card with `#[path = …]`, so
`cards::lightning_bolt` resolves as it did when the directory was flat and a
re-filed card costs one `git mv` and one generated line, leaving
`generated.rs`, the ledger and every path in the workspace alone. And
**placement is a reconciliation**: codegen finds each card wherever it is,
fails on any `.rs` under `cards/` that no card claims, and only then moves
what has drifted. The refusal comes first because the slug a card claims is
known without fetching anything, and a bail halfway through a re-filing would
leave every card moved and `mod.rs` naming the old paths. An orphan left
behind by a move compiles, is declared by
nothing and is read by nobody, which is exactly how an empty
`lightning_bolt.rs` once sat in the tree unnoticed. Anything that reads card
files goes through `card_files` in `xtask` for the same reason the rules gate
excludes only the client: a non-recursive `read_dir` over a tree finds
nothing and reports an empty worklist as an answer.

Which files to read is one question and **what a file says** is the other, and
it has the same answer: `knob(content, field)` in `xtask`, never a literal
match on `"<field> = "`. Card files are ordinary rustfmt output since the
macros moved to parentheses, so a value too long for its line is wrapped onto
the next one — `coverage =\n        Coverage::Partial(…)` — and every reader
that matched the unwrapped spelling read that card as having no coverage flag
at all. Seven textual readers of the pool have now been found answering a
question they could not see; three of them were blind from the day they were
written.

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
- `crates/baylee-cards-codegen/src/scriptgen.rs` reads a **card-script reference
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

`cargo run -p xtask -- transcode-report` says how far the transcoder reaches
and ranks what the refused scripts need next. The ceiling is **our** DSL, not
the corpus's — extend the DSL and the transcoder converts the gain into
hundreds of cards at once, which is what `Pump` did: it was the top blocker at ~2300
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
first refusal (`scriptgen::unclaimed_parameter`), and the entry splits into an
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

A generated card stays **machine-owned**, and that is the point. Two markers
say who owns a file — `stubgen::STUB_MARKER` (nobody has finished it) and
`stubgen::OWNED_MARKER` (a reader wrote it in full) — and `codegen` rewrites
every file carrying either, on every run. A card with
neither is hand-written and is never touched. So a wrong transcoding is fixed
in the reader and never in the card: one rule wrote hundreds of files, so
patching the one in front of you leaves the rest broken and is reverted on the
next run anyway. `cargo run -p xtask -- adopt --name "<card>"` is the way out
— it strips the marker and hands the file over for good. `validate` reports
the split (207 hand-owned, 383 machine-owned, 775 stubs), which is the number
to watch: a machine-owned card is a rule's output, and a rule is testable.
The markers are named here and not quoted, and `stubgen::is_machine_owned` is
the only thing that should ever ask: `cross-read`'s first draft retyped the
owned marker without the backticks it is written with, read all 383
machine-owned cards as hand-written, and filled its report with the
transcoder disagreeing with itself.

The hand-written half is the one surface no program checks against another,
which is what `cargo run -p xtask -- cross-read` is for: the transcoder reads
the same card from the reference script and the two shapes are compared. It is a
**report** and a disagreement is not a defect — a hand-written card is allowed
to say more than one rule can. Two depths, because transcoding needs every
clause claimed and a hand-written card exists precisely because a reader could
not write it: counting the parser's own line kinds reaches 141 of the 207, and
22 of those also transcode in full. Both obey the transcoder's honesty rule —
one unread clause and the script is not counted, so a keyword hiding an
ability inside an `SVar` chain is a *named* skip rather than a finding
invented out of a blind spot. The population is bounded on both sides for the
reason above: the retyped marker made it larger, not smaller, and a floor
alone passed it.

What it compares is **shape**, and reading the report is half the tool. It
pointed at one card — Raffine's Tower, which claimed `Coverage::Implemented`
with no basic land types and no cycling ability at all — and reading the three
neighbours in that cycle by hand found all three cycling for `{2}` against the
`{3}` their own `//! Oracle:` header prints. `cross-read` cannot see a wrong
cost and never could; what it can do is put a person in front of the right
four files.

A card file is written with the macros in `baylee-cards-dsl/src/build.rs` and
opens with one import, `use baylee_cards_dsl::prelude::*;`. `card!` and
`face!` are the `CardDef`/`FaceDef` literals with their `..DEFAULT` tail
supplied (and `card!` writes the doc comment on the `pub static CARD` it
defines); `mana_ability!`, `activated!`, `triggered!`, `spell!`, `loyalty!`,
`modal_triggered!` and `mode!` do the same for the six ability shapes that
make up most of the pool. **Never restate a default** — that rule is why all
of it exists. Every one of them is invoked with parentheses and takes its
optional fields as `field = value`: a macro called with braces is left
unformatted by rustfmt in its entirety, and `field: value` is not an
expression, so the brace form put the whole pool outside `cargo fmt --check`
while that gate stayed green.

What is load-bearing about the ability macros is that their defaults are
*rules* defaults, not merely common ones: instant speed is CR 602.2, the
battlefield is CR 113.6, and `mana_ability = false` is CR 605.1 making a mana
ability the exception. That last one is why a mana ability has its own macro
instead of a flag — an ability wrongly marked `true` would silently skip the
stack, and nothing in the test suite reads that as a rules bug. Fields with no
rules answer (a trigger, an effect list) are positional arguments, so they
cannot be forgotten.

A cost is `cost!`, which reads left to right the way the card prints it —
`cost!("{1}{G}", TapSelf, SacrificeSelf)`, `cost!(TapSelf, SacrificeSelf,
PayLife(1))` for a fetchland — with `Cost::FREE` and `Cost::TAP` for the two
the macro would spell with no argument and one. `Cost { mana, parts }` is the
struct underneath and is what a reader in `baylee-cards-codegen` builds
through `body::cost_literal`, which both readers share so that the emitted
spelling cannot drift between them.

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
currently 17) is asserted in gamehost and client tests — bump it on any breaking
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

That boundary is also what makes an AI chair **drivable**. `SeatKind::Driven`
is an AI seat whose controls someone has taken: `Session::take_over` puts a
socket where the agent was, `release` hands it back — keeping the agent, so a
developer who disconnects mid-game leaves a playable opponent rather than a
table that stops at the next question — and everything that sends views asks
`answers_over_socket()` instead of "is this a human", because to a session
the two are the same thing. Since `act` already took `(&PlayerView,
&Pending)`, a program driving a chair sees exactly what a networked player
sees and no more: the anti-cheat line holds for a development tool by
construction rather than by remembering to.

`baylee-engine-server`'s listening harness is where that is reachable —
`JoinGame.seat_token` is a seat number there, an AI chair is taken over on
join and given back when the socket drops. That is the rules-side counterpart
of the client's `dev-control`: a scripted opponent over the real wire, seeing
what a seat is entitled to see. It also forced the harness to *route*. It had
been handing every seat's envelopes to whichever socket was holding the lock,
which is invisible while exactly one seat answers over a socket and wrong the
moment two do, so a table now fans `(seat, envelope)` out and each connection
sends on what is addressed to its own seat.

`SeatKind::StandIn` is the **mirror**, and the reason both exist: a chair and
whoever is answering for it are two different questions. A seat with no socket
is on no decision clock — nobody should lose on time to a question they never
saw — which is right, and left the whole table waiting on a player who had
closed their laptop. So the awaited seat is on exactly one of *two* clocks
(`Deadline::Decide` / `Deadline::StandIn` on `EngineRunner::clock`), the second
being `HouseRules::reconnect_window_secs`, a field carried through three crates
and read by nobody until now. They expire into different things: a decision
timeout answers once, a reconnect timeout hands the chair to the house
(`Session::stand_in`), because a player absent for this question is absent for
the next one too. `SeatAttached` takes it straight back (`hand_back`).

Two details that are easy to get backwards. A held chair is **not** relabelled
an AI — `SeatIdentity` has `away` beside `is_ai` (`VIEW_VERSION` 12), because a
seat that renamed itself to the house after a thirty-second hiccup would keep
saying so after the player returned. And the roster travels in `GameStatic`,
sent once per socket: a chair changing hands now marks every seat's roster
stale so the next view carries a fresh one, which had been missing since
`take_over`/`release` existed and was invisible for exactly that reason.

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
decision clock does not run for a seat with no socket — nobody is losing on
time while it waits. Giving up reports `DuelReport::Unreachable`, which is a
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
`baylee-client-core/src/tabletop.rs` computes the centre medallion and the
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

What it draws is **casino baize inside a padded rail, on a slab with a real
thickness**: `rounded_slab_mesh` builds the table and the card both, a rounded
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
`RAIL_WIDTH` is 0.55 and `table_corner` 0.11 of the short side, both narrowed
with it. The table also carries **its own lamp** (`under_lamp`): an
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
applies it — so a dusk crossfades the sky and cools the baize on the same
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
`on_turn` at zero — and the bare felt beside it both move 0.0. Everything down there is `unlit`
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
is written along it. *Which* edge `SeatSlot::ledge_is_outer` answers, and it
is two rules with `is_local` as the seam. **The local seat's bar is on the
near edge of its own mat — the bottom of the screen, always.** That is the
owner's decision and it costs the table one symmetry: what a player reads
about themselves now sits between their board and their hand, where their
eyes already are. Every *other* seat's bar is drawn above the board it
describes on the one screen there is, which is `facing.cos()` leaning past
`SIDE_SEAT_TILT` — the outer edge for a seat across the table, the
centre-facing edge for a side seat, where the mat runs up and down the screen
and neither edge is above anything. That last case is why the test has a
tolerance and is not a comparison against zero: `cos(FRAC_PI_2)` is -4.4e-8 in
f32 and `cos(3·FRAC_PI_2)` is +1.2e-8, so `< 0.0` sends the left flank of a
four-seat table to one edge and the right flank to the other, and the two
flanks of a table have to answer alike. Only the
shelf changes ends: the three lanes run from the centre-facing edge outwards
at every seat, so `MatParams::ledge_outer` is a flag and not a flipped `uv.y`,
which would carry the lane veils along with it. What *does* follow the shelf
is where those three lanes start — a seat whose shelf is on its near edge
gives that strip up and its board sits a `MAT_LEDGE` nearer the hearth, so no
card is ever drawn where the bar is. The bar carries
the priority caret, the seat's colour, its name, life,
its four zone counts, the turn number with the day/night designation on its
hinge, then the twelve steps of that turn. It is a **second retained tree**
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
