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
cargo run -p xtask -- codegen --check    # fail if generated files are stale (developer's machine only)
cargo run -p xtask -- codegen --tables   # only the two tables built from the compiled pool
cargo run -p xtask -- validate           # card headers vs. the CardDef the code builds
cargo run -p xtask -- ledger             # assign a CardIndex to every corpus card that has none
cargo run -p xtask -- ledger --check     # report what would be assigned instead of writing it
cargo run -p xtask -- adopt --name "Yavimaya Coast"       # take a generated card off the machine, for good
cargo run -p xtask -- refresh-oracle                      # rewrite every `//! Oracle:` header from the cached printing
cargo run -p xtask -- scryfall-cache                      # fill the payload cache and nothing else (cold: ~26 s)
cargo run -p xtask -- explain --name "Force of Will"      # Scryfall + scripts data side by side
cargo run -p xtask -- card-batch --cards "A,B"            # LLM task packages for unimplemented cards
cargo run -p xtask -- transcode-report                        # how far the card transcoder reaches, and what it needs next
cargo run -p xtask -- cross-read                          # every hand-written card read a second way, and the disagreements
cargo run -p xtask -- cr-check                            # every CR citation against a local rules copy (needs one; see below)
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

`cr-check` reads a second external text on the same terms and finds it the
same three ways: a Comprehensive Rules `.txt`, which is Wizards' and is not
vendored either (`docs/legal.md` §2), named by `--rules`, then
`BAYLEE_COMP_RULES`, then any `MagicCompRules*.txt` within four levels of the
parent — and with none of them it says so and checks nothing, because CI has
no copy. It asks two mechanical questions of every citation in the tree:
that the number exists, and that a line using one of the rules' **own**
heading words (every `701.N` is a keyword action, every `702.N` a keyword
ability) cites a number under that word. The second is the shape the rot
takes. Wizards renumber a section whenever they insert a keyword action into
it, so sacrifice moved from 701.19a to 701.21a with nobody here touching a
file, and an audit of every citation in the tree found **246** naming the
wrong rule. One consequence for a writer: a line that *records* an old wrong
citation writes the number without its `CR`, or it is a finding nothing can
ever clear.

**A cold payload cache is one download, not one request per card.**
`fetch_named` answers from disk with no HTTP call at all, so the whole cost of
codegen's Scryfall half is how the cache gets filled the first time — and
filling it one card at a time is one request per card at a 200 ms pause, which
on a hosted runner (where an IP is shared) earned thirteen 60-second
rate-limit backoffs, 13 of 28 minutes spent asleep. Those 28 minutes were
measured over a pool of 1365 and the pool is 2716 now, so a serial fill today
costs about twice that — the bulk figure beside it does not grow the same
way, because it is one download. `scryfall::fill_from_bulk` takes Scryfall's
`oracle_cards` bulk feed instead, which their guidelines ask for: measured
cold, 26 s against 28 min, with no per-card request left to make.

Which feed and which key are both measurements rather than preferences.
`oracle_cards` is one row per oracle card — Scryfall's own default printing,
which is the choice `/cards/named?exact=` makes; `default_cards` carries every
English printing, so 1097 of the pool's names matched several rows there when
the pair was measured over 1365 cards. Cards are matched on **`oracle_id`**
out of the ledger and never on the name, because three cards in this pool
share a name with a token. Held against the 1365 payloads that pool then had,
fetched one at a time, the feed wrote 1359 byte-identical files and disagreed
on six — and the live API has moved on those six too, so the feed introduces
no printing this repo would not have fetched anyway. The
lookup is two-tier, whole name then front face, because the pool names a
two-faced card by its front face (`Sheoldred`) where the ledger follows
Scryfall (`Sheoldred // The True Scriptures`); both tiers are unambiguous
across all 33 694 rows, which is checked rather than assumed, since a
collision would hand a card another card's payload.

Bulk never fails the run. Whatever the feed did not carry is fetched one card
at a time behind it, which is what makes the pair self-healing: a stream that
dies halfway leaves the cards it did write, and they are correct.

Running things:

```bash
cargo run -p baylee-client                               # Bevy duel client, solo vs AI
BAYLEE_DEV_CONTROL=28770 cargo run -p baylee-client --features dev-control   # drivable while unfocused
trunk serve index.html --release                         # from crates/baylee-client/ — browser client on :8080
BAYLEE_AGENT_TOKEN=$(openssl rand -hex 32) ./target/debug/baylee-gateway   # accounts/decks/lobby/proxy, 0.0.0.0:28766
BAYLEE_AGENT_TOKEN=<the same> ./target/debug/baylee-agent                  # starts one engine per game
./target/debug/baylee-engine-server                      # dev harness only, 127.0.0.1:28765
cargo bench -p baylee-engine --bench basics -- --quick                  # numbers to compare against docs/perf-baseline.md
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
cargo run -p baylee-catalog -- ingest                     # every language: 542k printings, ~3 min, 606 MB
cargo run -p baylee-catalog -- ingest --english-only      # ~118k English printings, ~30 s
cargo run -p baylee-catalog -- search "lightning bolt"
cargo run -p baylee-catalog -- project                    # rebuild the search projection alone
cargo run -p baylee-catalog -- mine-types                 # rewrite data/type-names.tsv (needs every language)
cargo run -p baylee-catalog -- corpus                     # the cards an index is assigned over, for `xtask ledger`
```

That works from **any** worktree, because `compose.yaml` pins `name: baylee`
rather than letting Compose derive a project from the directory. This
repository is checked out five times, so the derived name gave each tree a
container and a volume of its own and then collided on 5432 — three volumes
were found that way, one of them 1.3 GB and one an empty cluster. The pin
survives forgetting; `COMPOSE_PROJECT_NAME` and `-p` still override it, which
is the precedence the spec defines and the one worth keeping.

**Every language is the default and English is the opt-out**, which is the
opposite way round from Scryfall's two feeds. A card's printed text is the one
thing a player reads in their own language — the client asks `/catalog/text`
with the language it is set to and falls back to English printing by printing
— so an English-only catalog is not a smaller install, it is a client that
quietly speaks English to everyone. Measured on this machine: `all_cards` is
392 MB compressed, stores 542 142 printings in 19 languages in about three
minutes, and leaves the database at 606 MB against the 118k rows
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
Japanese keyboard — 2.1 ms against no answer at all. It costs 230 MB and
replaces 124 MB of index.

**A card is findable in a language nobody printed it in.** Merging every
printed type line into that one `tsvector` already makes `同盟者`, `Ally` and
`Kleriker` reach the right cards — but only cards somebody printed that way,
and 3634 cards have no German printing at all. So the projection joins
`type_names`, a committed dictionary of what each type and subtype is called,
and puts those words in whether or not the printing exists. It closes 89.5% of
the subtype occurrences on those 3634 cards, against none before, and
`Verbündeter` answers in 4.6 ms.

The dictionary is keyed on the **English name**, which is Scryfall's own key —
deliberately not on `baylee_core::SubtypeId`, because those ids are a running
index into one sorted range partitioned by kind, so one new creature type
renumbers every artifact, enchantment, land, planeswalker and spell subtype
after it, and a catalog keyed that way would need 542 177 printings re-ingested
whenever a set ships. It is mined by `baylee-catalog mine-types` against a full
catalog and committed as `data/type-names.tsv`, the same bargain as the
CardIndex ledger: mining needs every language ingested and CI has an empty
Postgres, so the file is the artifact and the command is the developer's tool.

Three readings fill it, each allowed to write only a pair it understood in
full. The type line's left side as one phrase (a supertype is never
decomposed — `Basic Land` is one German word for two). Cards with exactly one
subtype, which is the clean signal and reaches 316 of 506 in German. Then
**subtraction**: a card whose printed segment tokenises into forms already
known plus exactly one leftover, against exactly one unknown English subtype.
That third reading is where `Druid`, `Warlock`, `Advisor`, `Ninja` and `Ally`
come from — 62 more subtypes and 79.7% → 89.4% coverage — and there is no
fourth, because what is left does not fall to more passes. Where readings
disagree the most frequent form wins and the newest printing breaks a tie,
which is not a style choice: 48 of 317 German cells hold several forms, and
they are old type lines and errata (`Löwe` and `Tiger` on cards Scryfall now
calls `Cat`). Both rules agree everywhere but `Orgg`, a 2–2 tie.

Two shapes were tried and rejected on measurement. Matching the dictionary
with a padded `LIKE` would also find a *multi-word* subtype, and costs 22.8 s
against 1.3 s — a nested loop rejecting 178 million pairs — to reach the one
multi-word subtype Magic prints, `Time Lord`, which no printing translates
anyway. And translating in a second `UPDATE` after the `INSERT` writes every
row twice: `card_search` 215 MB → 413 MB, while the stored text grew only
76 MB → 85 MB. Folded into the `INSERT` it is 230 MB and 23.8 s against 32.8.

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
agent runs elsewhere). Built `--features dev-table` it also reads
`BAYLEE_DEV_SEAT_BOARD` — the same semicolon-separated `0:Reflecting Pool;
1:Reflecting Pool` the client's offline harness takes, through the same
parser (`baylee_cards::decks::deal_named`), putting those cards on those
seats' battlefields before turn one. A **feature** and not merely a variable,
because a gateway is somebody's server and one that seats cards from its own
environment is a table its operator can stack in silence; a spec that does
not resolve refuses to start the gateway rather than failing at the moment
somebody presses Start. Every response carries
`Access-Control-Allow-Origin: *` and no `Allow-Credentials`, which is what
lets the browser client read an answer at all: its page is a different origin
by construction, and the pair is defensible only because this gateway
authenticates with a bearer token in a header and sets no cookie anywhere.
The agent takes `BAYLEE_GATEWAY`, `BAYLEE_AGENT_TOKEN`,
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
from every release build), `validate`, a
`wasm32-unknown-unknown` check of the five crates that must keep compiling
for it, benches, an MSRV check against
the `rust-version` this workspace declares, `cargo-deny`, and `cargo-audit`.

`--tables` is the way past that on a machine with no corpus. Stages 1–4
read the card-script reference, and with none checked out they rewrite every
machine-owned card as an honest `// GENERATED STUB` — a correct answer to the
question they were asked and a destroyed working tree. The last two stages
read the **compiled** pool instead (`generated_lines.rs`, `generated_names.rs`),
so `--tables` brings those back in step after a card is edited by hand or
after `baylee_cards_codegen::lines` learns to read something new, and needs no
corpus at all. A machine that has one runs the whole thing and never needs it.

**Codegen is a developer's tool and does not run in CI.** It reads the
card-script reference, which is GPL and deliberately not vendored, so a runner
has none of it and the generator writes different files there than on the
machine that committed them — `codegen --check` reported 79 cards stale for
that reason alone, in a job that could never have passed. A card is generated
here and committed as code; what CI checks about it is `validate`, which
compares the committed code against the printing rather than regenerating it.

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
creatures may attack, which defenders may be attacked (CR 508.1b) and which
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
two-phase, so a card added by one run gets its row from the next) and the name
table (`crates/baylee-cards/src/generated_names.rs` — which card a printed
English name is, which `decks::by_name` answers in one hash instead of walking
2716 cards; two-phase for the same reason, so the tests in
`decks::name_table_tests` fail between the run that adds a card and the run
after it). You then
edit **only**
`coverage`, `keywords`, `abilities` in that card's file;
`index`, `oracle_id`, `scryfall_id` and `faces` stay as generated — except
for the three fields a face has no printed data for in the Scryfall payload
codegen reads: `keywords`, `color_indicator` and `castable_from_hand`, which
a transforming double-faced card needs on its back face and which are added
by hand. A card *names* its index rather than spelling a number —
`index = index::MOX_OPAL`, the constant the ledger froze — and `card!` takes
that field as a `path`, so a bare number is a compile error at the matcher.
The index
comes from the append-only ledger `baylee_cards_index::ROWS`, which numbers
**every card there is** and not this pool — `cargo run -p xtask -- ledger`
assigns it over the corpus `cargo run -p baylee-catalog -- corpus` scans,
33 694 rows of which this repo compiles 2716 — so implementing a card inserts
nothing and renumbers nothing, and a `CardIndex` is what saved decks and
replays name. A card this repo implements that the corpus filter drops is
named in `data/corpus-keep.tsv`, the hand-kept additive half, and is admitted
whole.

The ledger is a **compiled table**, and was `data/card-index.tsv` until it was
not. A data file is a second truth beside the code: nobody reads it and the
compiler does not check it — a generated table it checks. So
`crates/baylee-cards-index` *is* the ledger, one `Row` per card, and
`xtask ledger` reads the table it is about to rewrite: safe because
assignment only ever appends, and the build is what checks what came out. Its
own crate rather than a feature on `baylee-core`, because the table carries
2.7 MB of `oracle_id`s and names the rules engine has no use for, and Cargo
unifies features across a workspace build — one tool asking for the data
would compile it into everything. The engine does not link the crate, so it
cannot. The **name** table went the other way and lives in `baylee-cards`:
the pool's 2716 names are already compiled into the engine there as
`FaceDef::name`, and the corpus's 33 694 are not.
`docs/card-identity.md` is normative on which handle lives where, what each
one survives, and which of them may be stored.

Codegen only *reads* the ledger and fails loudly on a card with no row: one
writer, and it is not the thing that writes card files. What it *does* write
from the ledger is `crates/baylee-core/src/generated/index/` — the same
assignment as Rust constants, one `set_<code>.rs` per first-appearance set
behind a `mod.rs` that globs them into one namespace, so `index::MOX_OPAL`
resolves without anybody knowing the set. `docs/card-identity.md`
§"Who may write what" has the prefix rule and the generated door test. The `//!`
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

It asks the **price** as well, which is the half of a mana ability nothing
compared: the mana check reads what an ability *produces* and holds it against
the card's "add …" clause, and said nothing about what the card charges. Every
printed `{…}…:` prefix is parsed into a `ManaCost` and held against the
`ManaCost` on an ability, so a difference in spelling is not a finding and a
difference in price always is. Mystic Gate and Fetid Heath each sold for `{1}`
what their printing sells for `{W/U}` and `{W/B}` — cheaper, and colourless
where the card demands a colour — and read as correct from every other side:
right header, right effect, wrong price. The check asks only
`Coverage::Implemented` cards, because on a `Partial` one a missing ability
looks exactly like a mispriced one; the four refusing a printed ability by
name (Kenrith, Lotleth Troll, Urza, Yawgmoth) rejoin the population of 172 the
day that ability exists.

A mechanic the DSL cannot express gets `Coverage::Partial("reason")` and a
`// NOT SUPPORTED:` comment; extend the DSL rather than working around it.
`docs/card-dsl.md` is the authoring contract, `docs/llm-learnings.md` gets
updated after every card batch.

A batch is handed to one of the two cheap-model lanes in `scripts/llm/` —
one script per (model, job) pair over a shared `lane.py`, with the four
prompt contracts beside them in `prompts/`. Its `README.md` is normative on
the one rule that is not obvious: **whoever wrote the card does not write
its test.** A card and its test from the same model share that model's
misreading, which is how Mikaeus got a green test asserting that two
keyword bits no engine rule reads worked. The lanes are also scarce in
different units — DeepSeek is billed per token and is cheap, Gemini costs no
money and spends a refreshing time quota — so they are planned in cards and
in minutes respectively. No lane runs cargo, and no `xtask codegen` runs
while a card batch is in flight.

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
tapped. It is why `lands/` opens into 116 named levels rather than listing
its 1124 files flat, and why growing the map is a browsing improvement rather
than a correctness fix.

Two properties are what make it safe to arrange 2716 files this way, and both
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
at all. **Eleven** textual readers of the pool have now been found answering a
question they could not see; three of them were blind from the day they were
written, and four went blind the day the macros moved — `cross-read`, both
halves of `validate`'s header-against-code identity check, and a stubgen
assertion that no longer had a spelling it could fail on. Each of those four
matched `"<field>: "`, the struct-literal spelling, which not one of the 2716
card files has contained since. Only one of them said so: `cross-read` carries
a bound on how many cards it may read and refused at nought against a floor of
190, while the other three ran green over the whole pool and compared nothing.
**That is the argument for the bound, not for the care** — a reader that
reports a population is worth more than one that is merely correct today.

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

There is one thing the type line carries that is not a *field* but an
**ability**, and so is on neither side of that split: CR 305.6 mana. No corpus
states it — a printed card restates it as reminder text at most, and a
reference script leaves it to the type line exactly as this repo does — so
`stubgen::transcode_card` puts it back on whatever the script reader wrote,
using the same `landgen::intrinsic_mana_ability` the land reader uses, and the
same card comes out the same whichever reader reached it. With one basic type
the omission is invisible, because `casting::intrinsic_mana` covers that case;
with two it is fatal, because that shortcut returns `None` rather than pick a
colour for the player. Five Turbulent lands were written `Land — Swamp
Forest`, `Coverage::Implemented` and untappable the day the transcoder learned
their enter condition, and it was `baylee-cards`'s own pool lint that said so.

The rule both obey is the whole design: **one unread clause and the card is
refused.** An unknown effect, an unclaimed parameter (`NoRegen$ True`), a
computed `SVar`, a keyword that is data rather than a bit — any of them and
the card stays an honest `Coverage::Unimplemented` stub. A generated
`Implemented` therefore means what a hand-written one means. Getting this
backwards would be worse than generating nothing: the deckbuilder offers
`Implemented` cards as playable.

There is a **third** corpus behind the second, and one reader over both.
`crates/baylee-cards-codegen/src/tokengen.rs` reads a reference *token*
script into a `TokenDef` — a name, colours, a type line, a size, keyword bits
— and hands its `A:`/`T:`/`S:`/`SVar:` lines to `scriptgen`, because
`TokenDef::abilities` is the same `AbilityDef` slice a card face carries and
the engine reads it the same way. A token that carries an ability is 190 of
the reference's 852 scripts and, through the cards that name them, far more
than that: `c_a_treasure_sac` alone is 98. What `tokengen` refuses rather
than handles is a field `TokenDef` has no room for — a `static` filter, an
`EnterModifier`, a keyword a rule produced — and a token whose ability makes
a token, which it refuses by handing the transcoder **no** token lookup, so
recursion is impossible without a depth counter.

What the token's name leaves out is the abilities, and that is a **hole with
a guard rather than a rule**. `SKELETON_1_1_BLACK` is both halves of
`b_1_1_skeleton` / `b_1_1_skeleton_regenerate`, and a token's place in
`generated_tokens::ALL` is the id a client keys its art off — so two
definitions at one name is one of them wearing the other's picture.
`transcode-report` counts and names them (four in the reference today) and
`tokenledger::assign` refuses the pair, so the day this pool reaches for both
halves of one it stops the run and hands a person the example. Naming a token
after its abilities is *a* way out and is not taken on a guess.

`cargo run -p xtask -- transcode-report` says how far the transcoder reaches
over both corpora and ranks what the refused scripts need next. The ceiling
is **our** DSL, not the corpus's — extend the DSL and the transcoder converts the gain into
hundreds of cards at once, which is what `Pump` did: it was the top blocker at ~2300
scripts, and `Effect::PumpTarget` moved the transcoder from 1886 to 2545
scripts read in full.

**`--stubs` is the ranking that ships cards, and the plain one is not.** The
report's default population is all 33 826 reference scripts, which measures the
DSL; `transcode-report --stubs` ranks only the scripts belonging to this pool's
own unfinished cards — 640 of the 648 stubs had one when this was measured on
17.09 — and the two orders disagree so sharply that the corpus one is a trap
when the goal is a card.
Measured on 17.09: `Charm` is 618 corpus-wide and **2** here, an unreadable
`Pump` value 477 and **6**, the `DamageDone` trigger 442 and **1**,
`ChangesZone.OptionalDecider` 439 and **0**. Three commits that day moved the
corpus by 282 scripts and this pool by *no card at all*, which is not a
failure of those commits — a stub is by construction a card no reader could
write, so the residue's blockers are its own and nothing else's. Ranked
`--stubs` and worked, the `Moved` replacement family went 74 → 29 and eight
lands became cards in one commit. Rank corpus-wide to grow the DSL; rank
`--stubs` to finish cards, and say which one a commit was aiming at.

Read its deltas with the arithmetic in mind, or a good commit looks like a
poor one. A script is listed under its **first** refusal, so closing a cause
moves every card that had it to whatever it is refused for next: 43 stubs sat
under the two `Moved` causes that fell, 8 of them had nothing else left and
became cards, and the other 35 went into the long tail — where the visible
top ten did not move at all. The cause going to zero is the measure of the
rule; the cards finished is the measure of the residue, and the two are not
the same number.

The same "first refusal" rule makes the ranking **overstate** an entry that
sits in front of a card with two of them, which is why the top one is the
worst buy on the list. `AlternateMode:` was 91 pool stubs; reading only the
front half of every two-faced script — a throwaway patch, not a commit — left
**16** of 729 read in full, so at most 16 of those 91 could ever become cards
and the true figure is lower still, because the back face has to read too and
a Split or an Adventure is not its front half at all. An entry standing for
*n* cards is worth *n* only where the card has one way left to fail. Before
committing to the top of the list, cut the blocker out and see what is behind
it.

That entry is also the cautionary tale about reading the report as a list of
missing *subsystems*. `Pump` did not need a new one: `PumpFilter`,
`EffectFilter::ObjectIs` and the `Layer::PtModify` machinery were all already
there, and `CreateContinuousEffect { filter: &Filter::This }` had bound to the
first target since M2. What was missing was one variant that says "the
target" without overloading a `Filter` to mean it, takes `Amount`s so `+X/+X`
is expressible, and carries `KW$` in the same effect. Read a blocker by
finding what the DSL cannot *say*, not by assuming the mechanism is absent.

The pool's `Mana` entry was the same lesson a third time and needed no new
rule at all. `Effect::AddMana` has carried an `Amount` and a `combination`
flag since it was unified, and `mana_dynamic`, `mana_choice_dynamic` and
`mana_combination` are the three constructors for them — Gaea's Cradle,
Harabaz Druid and Cascading Cataracts are written with them by hand. The
reader forced every `Amount$` through `plain_number` and threw all three
away, so twenty-nine stubs were refused for a sentence the DSL had always
been able to spell. It was also four different sentences wearing one entry:
a counted amount (`Count$Valid …`), a pick per mana (`Combo`), a pick for the
whole amount (`Any` with an amount) and a commander's identity. Fourteen
cards came out of it, and the entry left the list instead of shrinking.

`Sacrifice` (24) was the lesson a *fourth* time and the one place it needed a
new variant. `DB$ Sacrifice` was not in `SUPPORTED_APIS` at all, and it is two
sentences: bare it is `SacrificeSelf` (111 corpus scripts), and with an
`UnlessCost$` it is the Karoo sentence — "sacrifice it unless you return an
untapped Plains you control to its owner's hand", 131 more and the API's
largest single shape. That half had no shape in the DSL, because
`Effect::PlayerMayPayOr` charges *generic mana* and a Karoo charges a
permanent. `Effect::PlayerMayPayCostOr` is the sibling that charges one
`CostPart`, and it asks no yes-or-no question: the player is shown the list of
what may pay and naming nothing is how they decline, which is also why a price
nobody can pay asks nothing at all. `cost_wizard` supplied all three halves
(`options`, `prompt`, `pay`) unchanged from the activation path.

`cost 'Sac'` (11) and `cost 'tapXType'` (6) fell the same day for a smaller
reason and are the warning against reading an entry as a missing subsystem:
`cost_expr` had learnt `Return<1/…>` and none of its three siblings, while
`cost_wizard` had answered all four `CostPart`s since activation costs were
written. One reader, four spellings, thirteen more lands. Twenty-seven cards
came out of the three entries together, and each of them was a sentence the
DSL was already most of the way to saying.

`unclaimed parameter ChangeZone.ChangeTypeDesc` (12) is the smallest of them
and the one worth reading, because claiming a word uncovered a rule that was
quietly wrong. The key is a label — `ChangeTypeDesc$ basic land` spells
`ChangeType$ Land.Basic` for a human — but it is **not** a `PROSE_KEYS`
entry, because it is prose only while the filter it describes stands beside
it: on a line that wrote the label and no filter it would be the only thing
said about what is being found. `Params::claim_label` is that guard, and the
measurement it rests on is that all 369 occurrences in the reference are a
`ChangeZone` and **not one** of them lacks a `ChangeType$`. Twelve lands read
in full — and the twelfth was Blighted Woodland, "search your library for up
to two basic land cards", written as a card that must find both. `optional`
is that word, `ChangeNum$` alone does not carry it, and the corpus proves it
cannot: of the 135 lines searching a library for more than one card without
an `Optional$`, 70 print "up to" and 65 do not, with identical fields.
`Mandatory$ True` separates one side cleanly (36 lines, none of them "up
to"), so a script saying either word is read and a script saying neither is
refused. Eleven lands, not twelve, and that is the honest-stub rule paying
for itself: reading the absence of a field as "up to" would have been an
inference wearing a reading's clothes.

The `Mana` entry used to be followed by one reading "supported effects, unsupported
parameters (~4200)", described as the honest-stub rule showing its cost. Most
of it was not that. `refusal_cause` was *guessing* — re-reading the script and
naming the first thing it did not recognise — and the bucket was where every
refusal it could not explain ended up. The transcoder now reports its own
first refusal (`scriptgen::unclaimed_parameter`), and the entry splits into an
API with no rule at all (a missing effect) and a rule that met a value it
cannot say (a missing case in one that exists), which are different work.

A refused *value* is named by what it resolves through and never by its own
spelling, for the same reason. `token amount \`X\`` was the largest token
blocker at 122 scripts and was one entry standing for thirty different
questions: of the 356 scripts writing `TokenAmount$ X`, 58 define
`SVar:X:Count$xPaid` — the number the player announced, which `Amount::X`
reads straight back — and the rest count opponents, damage dealt, creatures in
a graveyard. Naming the definition split the entry into the counts it is
really made of, and dropped it out of the top twenty where it had been
outranking work that was genuinely one rule. The letter is what the corpus
writes; the count is what the DSL is missing.

Reading an announced number also has a half that is invisible at the use site:
a **triggered** ability announces no `X`, so `Amount::X` there is
`x.unwrap_or(0)` — a card that compiles, claims `Implemented` and makes
nothing. `Tx::has_x` is set per rules line rather than per script, because one
card writes both kinds.

The eight scripts that refusal still holds back are the *next* entry rather
than a settled answer. They are the Verdeloth shape — "kicker {X}; when this
enters, if it was kicked, create X tokens" — and CR 107.3m is already
implemented for the neighbouring case: `EnterModifier::WithCounters` reads the
announced number off the object's `x_value` and only while the arrival came
off the stack. A triggered ability reading the same field is one rule, not a
subsystem.

The rule that still holds: a key claimed there has to be claimed by a *rule*,
never by adding it to `PROSE_KEYS` to make the number move. And the reason
the transcoder reports rather than a table listing each rule's keys is the
one `SUPPORTED_APIS` already answers — the copy would rot the first time a
rule learned a new key, and a stale worklist is worse than none.

The current top entry, `S:` static abilities, is *partly* read: `Mode$
Continuous` becomes one `static_ability!` per layer it touches, because a
printed sentence usually is several ("get +1/+1 and have flying" is layers 7c
and 6, and CR 613.1 applies 6 first). Which layer that is nobody writes down:
`Modifier::layer` derives it, so the emitter names the modifier and the layer
follows, and the order the two abilities happen to sit in says nothing at all.
What is left there needs genuinely
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
the split (328 hand-owned, 1697 machine-owned, 691 stubs), which is the number
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
`static_ability!`, `chapter!`, `modal_triggered!` and `mode!` do the same for
the ability shapes that make up most of the pool, and `equip!` for the one
keyword whose whole ability is defined by the rules. **Never restate a
default** — that rule is why all of it exists. Every one of them is invoked with parentheses and takes its
optional fields as `field = value`: a macro called with braces is left
unformatted by rustfmt in its entirety, and `field: value` is not an
expression, so the brace form put the whole pool outside `cargo fmt --check`
while that gate stayed green.

What is load-bearing about the ability macros is that their defaults are
*rules* defaults, not merely common ones: instant speed is CR 117.1b, the
battlefield is CR 113.6, and `mana_ability = false` is CR 605.1 making a mana
ability the exception. That last one is why a mana ability has its own macro
instead of a flag — an ability wrongly marked `true` would silently skip the
stack, and nothing in the test suite reads that as a rules bug. Fields with no
rules answer (a trigger, an effect list) are positional arguments, so they
cannot be forgotten.

Two of them take the rule one step further and remove a field the card never
decided. `static_ability!(filter, modifier)` has **no layer argument**:
CR 613.1 makes the layer a function of the modifier, and `Modifier::layer` is
that function — derived from the compiled pool, where 108 `layer`/`modifier`
pairings used 25 modifiers and put no modifier on two different layers.
`equip!("{2}")` takes only the cost, because CR 702.6 supplies everything
else — sorcery speed, "target creature you control", and attaching this
permanent to it. A raw literal is still allowed in both places, and
`lints::every_layer_in_the_pool_is_the_one_its_modifier_derives` is what
stops one disagreeing with the macro beside it. `activated!` and
`mana_ability!` reach `AbilityDef::ActivatedConditional` through one optional
`condition = Some(…)`, which is the only difference between the twins — six
readers across the engine, the client and the lints once matched the
unconditional one alone.

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
Expect that shape when adding data.

**A card that is added, fixed or refactored is played once in an engine
test**, and `.claude/hooks/require-card-tests.py` is what asks for it: a
`Stop` hook that lists every card written this session whose `oracle_id`
appears in none of `crates/baylee-engine/src/engine/*_tests.rs`. The id and
not the name, because `card_tests.rs` addresses a card as
`card_index("<oracle id>")` and a name would be satisfied by a doc comment
mentioning it. A restored stub owes nothing — a refusal is a correct outcome.
The test does **not** go in the card file: that module stays empty for the
reason above, and what is worth proving is that the engine does what the
printed sentence says, which only playing it can show.

A **fix** owes a second test, and a different one. A card that was wrong was
wrong because a rule was wrong, and the rule is where hundreds of other cards
live — so the regression test belongs beside the rule it broke, and has to be
a test that fails against the old code. Writing only the card's own scenario
repairs one card and leaves the next fifty to be found by a player. That is
the same argument as "fix the reader, never the card" one section up, and it
is why the hook says both halves out loud. For a change that is meant to move no
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
read there rather than restated here — this file has been stale on it twice)
is asserted in gamehost and client tests — bump it on any breaking view change
so a client refuses a host it cannot render.

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

`HeuristicAgent::act(&PlayerView, &Pending)` remains the ordinary interface.
House AI scouting is an explicitly authorized exception: gamehost may answer
an in-process `ScoutingRequest` with deck lists, hands, sideboards and bounded
or full library order. `scouting::request` checks the current `SeatKind::Ai`
on every request. Human, Driven and StandIn seats are refused. The report has
no serialization and no protocol endpoint, is never merged into a player view
or print table, and is not stored in the agent retained during takeover.
The AI still receives no `Engine` or `GameState` reference. The harness uses
the same guarded adapter as a hosted AI. See `docs/house-ai.md`.

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

### The client, and where it actually gets its game

Moved to `crates/baylee-client/CLAUDE.md`, which loads when work is under
that directory: hosts and tickets, the lobby and the deck builder, the 3D
table and its two shaders, seat bars, the camera fit, combat lines, the
mana planner, arming, the card rail and plate, the sky, and the interface's
own words. `docs/client.md` is normative for all of it. A change under
`crates/baylee-client-core/` does not load that file on its own — open it
by hand, because most of what it describes is decided there.

Two invariants stay here, because they are the ones a session breaks by
accident from outside the client: **the stage has no lights at all** and
the camera carries `Tonemapping::None` — scene lighting on card art would
make colour identity unreadable, so everything on the table is `unlit`.
And **nothing on the table is positioned directly**: `table::sync_scene`
writes a `Motion` target and `table::glide` moves the card, so a repacked
lane, a tap, a hover and a card entering play all animate through one door
and cannot desynchronise from the board model.
