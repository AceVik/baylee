# CLAUDE.md

`AGENTS.md` is the authoritative short contract (conventions, legal guardrails); read it too.
The narrative version this replaced (reasons, measurements, history) is `docs/history/CLAUDE-2026-09-24.md`.

## Commands

### Build, test, gate

```bash
cargo fmt --all
DATABASE_URL=… ./scripts/gate.sh             # full gate as CI: fmt --check, clippy, nextest, validate (needs cargo-nextest)
./scripts/gate-rules.sh                      # while working: fmt, lint-rules, test-rules, validate (if data/scryfall-cache)
./scripts/gate-touched.sh [<rev>]            # fmt + clippy on changed files only; not a gate
DATABASE_URL=… ./scripts/gate-features.sh    # the six non-default features
```

- `lint-rules`/`test-rules` (`.cargo/config.toml`) exclude only `baylee-client`, never a hand-written crate list. Run the full gate before every push.
- Non-default features: `dev-control`, `dev-reload`, `dev-dylink` (client), `dev-control` (Android shim), `dev-table` (gateway), `test-support` (client-core). `--workspace` builds none, so run `gate-features.sh` before every push too (CI's `features` job): shared code (a type `devctl.rs` uses) breaks feature builds, and a feature can make an import live that `-D warnings` flags only then. Never `--all-features`: nobody runs it.
- CI also runs tests in `--release` (never hide required behaviour in `debug_assert!`), a cross-platform `build` matrix (`--bins`, `check --all-targets`), `scryfall-cache` + `validate`, wasm32, benches, MSRV, `cargo-deny`, `cargo-audit`.
- macOS uses `-Csplit-debuginfo=packed`. Builds crawl? Check `stat -f %z target/debug/deps`; sweep by renaming `target/debug` away.
- Servers are silent without `RUST_LOG=info`.

### Single tests

Engine tests: `crates/baylee-engine/src/engine/*_tests.rs` plus directories `card_tests/` (one file per card type) and `combo_tests/`; filter by module path.

```bash
cargo test -p baylee-engine keyword_tests
cargo test -p baylee-engine --lib -- --exact engine::keyword_tests::no_card_claims_a_keyword_the_engine_ignores
cargo test -p baylee-engine --lib -- --list
cargo build --workspace --bins && cargo test -p baylee-gateway --test e2e_processes -- --ignored
cargo test -p baylee-catalog --test cardtext_provenance -- --ignored
cargo bench -p baylee-engine --bench basics -- --quick   # vs docs/perf-baseline.md
```

### xtask

No `cargo xtask` alias exists; every line below follows `cargo run -p xtask --`.

```text
codegen [--check | --tables]      # --tables: no corpus needed
validate
ledger [--check]                  # assign CardIndex; --check writes nothing
adopt --name "<card>"             # hand-own a generated card
refresh-oracle
scryfall-cache                    # fill the payload cache (bulk)
explain --name "<card>"
card-batch --cards "A,B"
transcode-report [--stubs]
cross-read
cr-check
pool-dump --out <path>            # refactor diffs
dev-table --seats 4 --ai sharp [--play] [--teams 1,1,2]
```

- `dev-table` skips only sign-in and deck-pick (username `dev`, gateway HTTP, sends `ready`/`start`, real sockets). `--teams`: sides in seat order (`0` = alone), three chairs minimum.
- `codegen`/`explain`/`card-batch` read the external GPL card-script reference (`NOTICE`): `--scripts`, `BAYLEE_CARD_SCRIPTS`, or a `cardsfolder` within four levels of the repo's parent. Never copy it in.
- `cr-check` finds the Comprehensive Rules the same three ways (`--rules`, `BAYLEE_COMP_RULES`, `MagicCompRules*.txt`); none = no check. Never vendor it (`docs/legal.md`). A line using a heading word must cite under it (keyword action `701.N`, keyword ability `702.N`). Sections renumber: look numbers up, never recall them. Record an old wrong citation without `CR`.
- Scryfall: `fetch_named` answers cached payloads from disk, else fetches one card. Fill a cold cache in bulk (`scryfall::fill_from_bulk`, `oracle_cards`), matched on ledger `oracle_id`, never name. Bulk never fails; gaps fetch singly.
- `data/scryfall-cache` (`--cache` default) is gitignored; copy it into a fresh worktree or codegen fails at the lines stage.
- Never run full `codegen` without the corpus: it rewrites every machine-owned card as a stub. `--tables` regenerates the four compiled-pool tables after a hand-edited card or a `baylee_cards_codegen::lines` change.
- Codegen (even `--check`) never runs in CI; `validate` checks committed cards.

### Running

```bash
cargo run -p baylee-client
BAYLEE_DEV_CONTROL=28770 cargo run -p baylee-client --features dev-control
BAYLEE_DEV_CONTROL=28770 BEVY_ASSET_ROOT=$PWD cargo run -p baylee-client --features dev-control,dev-reload
trunk serve index.html --release          # in crates/baylee-client/
BAYLEE_AGENT_TOKEN=$(openssl rand -hex 32) ./target/debug/baylee-gateway   # 0.0.0.0:28766
BAYLEE_AGENT_TOKEN=<same> ./target/debug/baylee-agent
./target/debug/baylee-engine-server       # dev harness, loopback
adb forward tcp:28773 tcp:28770           # Android dev-control
cp .env.example .env                      # the client's BAYLEE_GATEWAY and DATABASE_URL
```

- `dev-control` is a loopback HTTP harness (input, `/state`, `/screenshot`; `/timescale /pause /step` drive `Time<Virtual>`, zero refused). Compile-time only; never ship it. `docs/client.md` §"Driving the client without its window".
- `dev-reload` hot-reloads embedded WGSL; needs `BEVY_ASSET_ROOT` and `BAYLEE_DEV_CONTROL`. `docs/client.md` §"Editing a shader without stopping the game".
- Mobile is native (browsers lack sockets): `crates/baylee-client-android`, `scripts/mobile/{android-build,ios-sim-run}.sh`; iOS simulator shares host loopback. `docs/mobile.md` is normative.
- Always build wasm `--release`. It renders via WebGPU (bevy feature `webgpu`); `webgl2` instead puts wgpu on GL. Shaders stay in the GL budget (uniforms only; no storage buffers, texture arrays, `bitCount`); a commit exceeding it says so.

### Database and catalog

```bash
docker compose up -d
export DATABASE_URL=postgres://baylee:baylee@127.0.0.1:5432/baylee
RUST_LOG=baylee_catalog=info cargo run -p baylee-catalog -- ingest   # all languages; --english-only opts out
# also: search "<q>", project, mine-types (writes data/type-names.tsv), corpus (ledger input)
```

- `compose.yaml` pins `name: baylee` so worktrees share one container; keep it (`COMPOSE_PROJECT_NAME`/`-p` still override).
- `baylee-db` is the gateway's store (accounts: login by username, email optional, guests; sessions, decks, confirmations, preferences; entities + migrator); no `DATABASE_URL`, no gateway. A leftover `gateway-store.json` is imported into an empty DB (its `automation` map is dropped), then moved aside.
- The card catalog is optional (same Postgres, hand-written SQL); without an ingest games still run without card text: the client draws faces from the engine's projection. Ingest every language: the client asks `/catalog/text` in its own, English fallback per printing. Plain `RUST_LOG=info` logs every INSERT.
- `/catalog/text`, `/catalog/search`, `/pool` and `/printings` answer signed-in sessions only, as `/art` does, and the gateway never asks Scryfall on a caller's behalf (Scryfall: no plain proxy or republish, #270); the client asks the lobby's gateway with its session, and signed in nowhere asks Scryfall itself. `docs/privacy.md` inventories what the gateway keeps about people.
- Search reads `card_search` (per oracle face; bigram and `tsvector` GINs), not `cards`. `Catalog::project()` rebuilds it after ingest, `migrate()` when it is empty or the stamped `SCHEMA_VERSION` (`crates/baylee-catalog/src/lib.rs`) differs; bump that on a shape change.
- Never put name and rules-text matches in one predicate: union the tiers, resolve the printing after `LIMIT`. Names use bigrams, not `pg_trgm`.
- The projection joins `type_names` (committed `data/type-names.tsv`, keyed on English name, never `SubtypeId`) inside its `INSERT`; no second `UPDATE`, no padded `LIKE`. `mine-types` needs a full catalog and writes only fully understood pairs; ties: most frequent form, then newest printing.
- Migrations: `unaccent` `WITH SCHEMA public`; no unqualified `CREATE EXTENSION`/`DROP INDEX`; `information_schema` lookups filter `table_schema = current_schema()`. Two tests keep a schema behind the sandbox to catch escapes.
- `released_at` is `date`; render it with `to_char(released_at, 'YYYY-MM-DD')`, not `::text` (sqlx pins `DateStyle` to ISO today; the SQL should not depend on the driver); `finishes`, `frame_effects`, `rarity`, `layout`, `border_color` stay `text` deliberately.

### Environment

- Gateway: `PORT` (`0` binds any free port; `BAYLEE_PORT_FILE`, a path, receives the bound port just before serving, which is how e2e helpers find it, #279), `DATABASE_URL` (required), `BAYLEE_DB_POOL` (8), `STORE_PATH` (import-only, not gitignored), `BAYLEE_ART_PATH`/`BAYLEE_DECK_IMAGE_PATH` (`off` disables; empty also refuses uploads; `/art` answers signed-in players only and the wasm client takes art straight from Scryfall, `docs/protocol.md` §"Card art"), `BAYLEE_REGISTRATION=off`, `BAYLEE_GUESTS=off` (no guest accounts), `BAYLEE_GUEST_CAP` (live guests, default 1000, `0` = no cap; guests sign in with a display name only, are purged when their last session lapses, 30-day sliding, and may not upload images), `BAYLEE_TRUSTED_PROXIES`, `BAYLEE_AGENT_TOKEN` (none = no agent), `BAYLEE_SMTP_URL`/`BAYLEE_MAIL_FROM`/`BAYLEE_PUBLIC_URL` (no SMTP = no mail or confirmation), `BAYLEE_ENGINE_URL` (set for a remote agent), `BAYLEE_GATEWAY_NAME` (what `GET /info` names the gateway; unset = the client shows the address; over 64 characters or a control/bidi character refuses startup), `BAYLEE_SOURCE_URL` (what `/info` and `/source` name as this build's source, AGPL §13; unset = the repository; not `http(s)://`, over 200 characters, or a whitespace/control/bidi character refuses startup).
- A `dev-table` gateway reads `BAYLEE_DEV_SEAT_BOARD` (`0:Card;1:Card`, `baylee_cards::decks::deal_named`); a bad spec refuses startup. Keep board seeding behind the feature.
- Gateway CORS is `Access-Control-Allow-Origin: *` without `Allow-Credentials`; keep bearer-header auth, never set cookies.
- Agent: `BAYLEE_GATEWAY`, `BAYLEE_AGENT_TOKEN`, `BAYLEE_AGENT_NAME`, `BAYLEE_AGENT_CAPACITY` (0 = unlimited), `BAYLEE_ENGINE_BIN` (default beside the agent).
- Engine: `--attach/--game/--token` or `BAYLEE_ATTACH_URL`/`BAYLEE_GAME`/`BAYLEE_ENGINE_TOKEN`; none = listening dev harness (`PORT`, `0` and `BAYLEE_PORT_FILE` as on the gateway; `BAYLEE_BIND`). Never bind it publicly: unauthenticated, every hand leaks.
- Client: `BAYLEE_GATEWAY` + `BAYLEE_GAME`, `BAYLEE_SEAT_TOKEN`, optional `BAYLEE_SEAT`; browser `?game=…&token=…`.

## Architecture

### Crates

- Graph: core → engine → gamehost → engine-server; gamehost builds view and links `baylee-ai`; engine's choice taxonomy feeds client-core and `baylee-ai` (plays from the view; uses client-core's payment matching); core, view, protocol → client-core → client (Bevy); client-core links protocol only for shared rules such as `names::username`. protocol → gateway (axum), agent (`tokio::process`, no rules). Each arrow drops a capability; test at the lowest layer possible.
- `baylee-view` depends only on `baylee-core` ids, serde.
- `baylee-client-core` is the client brain, knows no renderer, holds most client tests. Only `baylee-client` needs a GPU.
- `baylee-protocol`: protobuf `Envelope`; `Pending`/`PlayerAction` ride as `serde_json`; shared by `LocalHost` and both servers.
- `baylee-cardtext` sits under core, links only serde: `/catalog/text` wire shape and sentence pairing (`pick`, `align`, `verify`, `split_cost`) for catalog and client.
- Must build for `wasm32-unknown-unknown`: `baylee-cardtext`, `-core`, `-protocol`, `-view`, `-client-core`, `-client`.

### Engine

- Exposes essentially `pending()`, `apply(player, action)`, `state()`, `journal()`, `snapshot_hash()`; during the opening-hand window every seat decides at once: `pending()` shows the lowest open seat, `pending_for(seat)`/`awaited()` are queries (`engine/mulligan.rs`). Advances only via `apply`, which validates against what `Pending` enumerated. Never add action methods. Combat options come from `Pending::ChooseAttackers`/`ChooseBlockers`, not the client.
- Layers: cached projection, one `u64` generation compare. Events: propose → replacement → apply → journal → triggers. SBAs: fixpoint before each priority grant. Loops: Brent over `loop_signature`, never `snapshot_hash`. `docs/engine-internals.md` is normative.
- The cleanup step discards first (CR 514.1, "until end of turn" effects still apply), then ends the turn's effects (514.2), then checks once (514.3a). `Engine::cleanup` (`Due`/`Checking`/`Open`) remembers which, because the check and a window look alike. A state-based action or trigger there gives the active player priority, and a round of passes on an empty stack begins another cleanup step. What the check performed is noted where it happens (`cleanup_check_acted`), never read off the journal (704.5n and 704.5q journal nothing). No standing order in client-core passes a cleanup window. `docs/engine-internals.md` §"The cleanup step checks once".
- Every change of control is a layer-2 `GainControl` effect (`resolve::gain_control`, CR 613.1b), even one that lasts the game; `base_controller` is only the default controller, written where an object arrives. Never write a controller by hand. Every `ContinuousEffect` names its `origin` (`Static`, CR 611.3, or `Resolution`, 611.2); a static's controller, like every `ReplacementEntry`'s, is projection output restamped to its source's current controller each refresh (last known once the source has left), so "you" follows a stolen source (CR 109.5). A player leaving follows CR 800.4a–c (`sba::eliminate_player`, `docs/engine-internals.md` §"A player leaving the game").
- Deterministic: seeded ChaCha8, no `HashMap` iteration in hot paths; `std::time`, `std::random`, `algebraic_*` floats banned in engine and core. Synchronous; async only as transport (engine-server, gateway, agent).
- No card text in the engine: abilities are `AbilityRef { card, index }`, reserved indices (`SPELL`, `ENTERS`, …) down from `u32::MAX`; it also keys a seat's standing answers, which the client keeps in its preferences (`ability_orders`).
- An `AbilityList` carries its `PrintedFace`, as do the view's `rules` fields: a copy shows the copied card's text. Never recover a face from a list's address; identical lists are merged constants.

### Hidden information and seats

- The view carries projected characteristics; clients never run layers.
- Hidden information is unrepresentable: libraries and other hands are counts; a face-down `card` is `None` unless entitled. `crates/baylee-gamehost/src/view.rs` tests each guarantee; add one per new one.
- Bump `VIEW_VERSION` (`crates/baylee-view/src/lib.rs`) on any breaking view change; never restate its value.
- The game log (`crates/baylee-gamehost/src/log.rs`) tells each seat its own lines, naming an object only as that seat's view would; an object the log never saw is `Hidden` to all. Every `StateDelta` carries a view and a `LogTail { from, entries }` in `log_json`; several frames may repeat a view and `seq`, so clients read the log from every frame and append by `from`. AI agents are never handed it. `docs/protocol.md` §"The game log".
- A teammate's hand is in a seat's view only while its owner shares it with that seat, both are on one team and both are still in a game that is going. Sharing is a seat setting (`SeatSettingMsg` → `Session::seat_setting`), not a `PlayerAction`: no journal, hash or clock. An AI chair accepts a teammate's request at once and is shown nothing; `Session::agent_view` is the one view an agent answers from. `docs/protocol.md` §"A teammate's hand".
- Granted abilities are offered under `choice::granted_ability(n)` (`GRANTED_ABILITY` is slot 0, up to `GRANTED_SLOTS` = 8); `PublicObject::granted_mana` names the output. Offer and projection share `effects::granted_activated` and `baylee_cards_dsl::simple_mana`; no second path. `docs/protocol.md` §"Granted mana".
- `GameStatic.prints` is `Option<PrintEntry>` per index; the index is the `PrintRef` objects point at, so hide with `None`, never shorten the list. A seat gets its deck's printings plus cards seen; `Session` re-sends before the view needing it.
- AI plays only through `HeuristicAgent::act(&PlayerView, &Pending)`, never an `Engine`/`GameState`. Scouting only for a current `SeatKind::Ai` (`scouting::request`); reports are never serialized, sent over the protocol, merged into views or prints, or retained. The harness uses the same guarded adapter as a hosted AI. `docs/house-ai.md`.
- `SeatKind::Driven`: AI seat taken by a socket (`Session::take_over`; `release` returns it to the retained agent). View senders ask `answers_over_socket()`, never "is human".
- `SeatKind::StandIn`: house answers for an absent player (`Session::stand_in`; `SeatAttached` → `hand_back`). Each awaited seat has one clock (in the mulligan window every deciding seat is awaited): `Deadline::Decide` (answers once) or, socketless, `Deadline::StandIn` (`reconnect_window_secs`, hands the chair over).
- An away chair keeps `is_ai` false (`SeatIdentity.away`). The roster rides in `GameStatic`; any chair change marks every seat's roster stale.
- Engine-server harness: `JoinGame.seat_token` is a seat number; joining an AI chair takes it over, disconnecting returns it. Each socket sends only its own seat's envelopes.

### The gateway runs no rules (`docs/protocol.md`, normative)

- An agent spawns one `baylee-engine-server` per game, which dials the gateway. gateway→agent `StartEngine`; engine→gateway `EngineHello`/`SeatFrame`/`GameEnded`; gateway→engine `GameSetup`/`SeatAttached`/`SeatFrame`; seat sockets forwarded byte for byte.
- The gateway links neither `baylee-engine` nor `baylee-gamehost`; its e2e tests pull `baylee-engine` and `baylee-engine-server` (and gamehost through it) as dev-dependencies only. No agent → `POST /lobby/games` is 503.
- `SeatFrame { seat, envelope }` nests an encoded player `Envelope` the gateway never decodes; keep that shape.
- Secrets: `BAYLEE_AGENT_TOKEN` on `/agent/ws`, per-game token on `/engine/ws`, seat token on `/games/{id}/ws`; not interchangeable. After its secret each peer states `PROTOCOL_VERSION` (a hello field for agent and engine; `?protocol=` on the seat socket, built only by `baylee_protocol::seat_socket_path`), and the gateway refuses a mismatch in one sentence (`version_refusal`), so gateway, agent, engine and clients ship as one build. A seat socket is bounded in size (`MAX_SEAT_FRAME`) and rate (`seatrate::Allowance`, GCRA, measured against a real client in #284); a flood is closed with 1008, never dropped frame by frame. `docs/protocol.md` §"Which side checks the protocol".
- The decision clocks run in the engine (`EngineRunner::clocks`, `crates/baylee-engine-server/src/lib.rs`), at most one per awaited seat, each anchored to `Session::asked_at(seat)`, a `decision_seq` (questions, not frames), never for a socketless seat, whose frames the engine (not the gateway) drops. Losing the engine link ends the game.
- Nothing runs before the curtain: the table opens (`Curtain`, sent last in its batch) when every human seat has sent `SeatReady` or `CURTAIN_SECS` after `GameSetup`; until then attaches get `GameStatic` and their view, no question, no clock, no AI move, and early actions are dropped. Every clock is anchored at curtain-up. Deploy the engine-server before clients. `docs/protocol.md` §"The curtain".
- One process per game is the panic boundary; the hosting path wraps no rules call in `catch_unwind`.
- Gateway e2e tests spawn real gateways (own schema each, pool of two; CI has `postgres:18-alpine`) with the engine in-process (`EngineRunner`); `e2e_processes` is `#[ignore]`d.

### Cards

- Codegen writes stubs, `generated.rs`, `cards/mod.rs`, `generated_tokens.rs` (in `crates/baylee-cards/src/`), `crates/baylee-core/src/generated/` (`subtypes.rs`; `index/`, per-set files globbed into `index::`; prefix rule and door test: `docs/card-identity.md` §"Who may write what") and four compiled-pool tables: `generated_lines.rs` (`docs/client.md` §"Which ability is on the stack"), `generated_names.rs`, `generated_sides.rs`, `generated_oracle.rs`. Two-phase: run codegen twice; `decks::name_table_tests` fail in between.
- Edit only `coverage`, `keywords`, `abilities`; the rest is generated, except a transforming DFC back face's `keywords`, `color_indicator`, `castable_from_hand`.
- Name the index as a constant (`index = index::MOX_OPAL`); a number does not compile.
- `CardIndex` is the append-only ledger `baylee_cards_index::ROWS` (`crates/baylee-cards-index`) over the whole corpus; decks and replays store it, so it never changes. Only `xtask ledger` writes it; codegen fails on a rowless card. Pool cards the corpus filter drops go in additive `data/corpus-keep.tsv`.
- The engine must not link `baylee-cards-index` (own crate, not a core feature). The pool's name table is in `baylee-cards`. `docs/card-identity.md` is normative.
- `validate` holds the `//!` header against the `CardDef`, and oracle text, `pool::type_line(face)`, printed costs (`Implemented` only) against Scryfall; header types use printed spelling (faces joined `" // "`). Never hand-type header oracle text; run `refresh-oracle`.
- DSL can't say it: `Coverage::Partial("reason")` + `// NOT SUPPORTED:`; prefer extending the DSL. `docs/card-dsl.md` is the contract. Update `docs/llm-learnings.md` after every batch.
- Batches go to cheap-model lanes in `scripts/llm/` (`README.md` normative): one script per (model, job) over shared `lane.py`, four prompt contracts in `prompts/`. A card's author model never writes its test. Plan DeepSeek in cards, Gemini in minutes. No lane runs cargo; no codegen during a batch.

#### Layout

- Codegen places files; never move one. `<type>/[<second type or defining subtype>/]mv_<n>/<slug>.rs`, front face decides, lands take a semantic level instead of `mv_` (`layout.rs`; `docs/card-dsl.md` §"Where a card's file lives").
- Land level, first answer wins: `Basic` → second type/defining subtype → additive `data/land-cycles.tsv` (an entry for a card not in the pool bails) → printed nonbasic land subtype → `land_role` (printed text: fast, check, …, `utility`, `tapland`) → basic-type count (`lands/dual`).
- `cards/mod.rs` uses `#[path = …]`, so module paths survive moves. Codegen refuses orphan `.rs` under `cards/` before moving anything.
- List card files with xtask's recursive `card_files`; read fields with `knob(content, field)`, never a literal `"<field> = "` (rustfmt wraps). A textual pool reader asserts a floor and a ceiling on its population (a misread marker inflates the count; a floor alone passes that).

#### Readers and ownership

- Before a stub, codegen tries `landgen.rs` (printed land text) and `scriptgen.rs` (only `K:`, `A:`/`T:`, `S:` (partly), linked `SVar:`; names, costs, types, P/T come from Scryfall).
- CR 305.6 mana is an ability: every land with basic land types carries `landgen::intrinsic_mana_ability` (`stubgen::transcode_card` re-adds it after scriptgen); `casting::intrinsic_mana` covers one basic type, `None` for two.
- One unread clause (effect, parameter, computed `SVar`, data keyword) keeps the card `Coverage::Unimplemented`: generated `Implemented` means fully read (the deckbuilder offers it as playable).
- `tokengen.rs` reads token scripts into `TokenDef` (abilities via scriptgen), refusing fields `TokenDef` lacks and token-making tokens. `generated_tokens::ALL` position is the art key; `tokenledger::assign` refuses same-named tokens with different abilities.
- The ceiling is our DSL. Rank corpus-wide to grow it, `--stubs` to finish cards; say which. Scripts count under their first refusal: judge a fix by its cause reaching zero and, separately, cards finished. Patch the top blocker out and re-measure first. A blocker is a sentence the DSL can't say; grep for existing machinery.
- Refusal causes come from `scriptgen::refusal_reason` (xtask `refusal_cause`). Name a refused value by its `SVar` definition, not letter. Only a rule reading a key claims it; never pad `PROSE_KEYS` or keep a per-rule key table.
- DSL facts: `Effect::AddMana` takes `Amount`/`combination` (`mana_dynamic`, `mana_choice_dynamic`, `mana_combination`). `DB$ Sacrifice` is `SacrificeSelf`; with `UnlessCost$`, `Effect::PlayerMayPayCostOr` (one `CostPart`; naming none declines). `cost_expr` (scriptgen) turns cost spellings (`Sac`, `tapXType`, …) into `CostPart`s; check it before blaming `cost_wizard`. `ChangeTypeDesc$` is claimed via `Params::claim_label` beside `ChangeType$`. Library search: `Optional$` = up to, `Mandatory$ True` = must, neither = refused.
- In a trigger `Amount::X` is `x.unwrap_or(0)`; `Tx::has_x` is per rules line. `S: Mode$ Continuous` becomes one `static_ability!` per layer.
- `stubgen::STUB_MARKER`/`OWNED_MARKER` files are rewritten every run; files with neither are hand-written, untouched. Fix wrong output in the reader, never the card. Only `stubgen::is_machine_owned` asks; never retype the markers. `validate` prints the hand-owned/machine-owned/stub split; watch it.
- `cross-read` is a report (disagreement ≠ defect), blind to costs.

#### Writing a card

- Open with `use baylee_cards_dsl::prelude::*;`; use the macros in `crates/baylee-cards-dsl/src/build.rs` (`card!`, `face!`, `activated!`, `triggered!`, `static_ability!`, `cost!`, …). Never restate a default. Parentheses and `field = value`, never braces or `field: value`. Fields without a rules default are positional.
- Defaults are rules (CR 117.1b, 113.6, 605.1). Use `mana_ability!`, not a flag; `lints::mana_ability_fault` (`mana_ability_fault_of`, also for `Modifier::GrantActivated`) rejects mana abilities that make no mana or target.
- Pool walks visit both grant doors, `AbilityDef::Static` and `Effect::CreateContinuousEffect`; population floors count doors, not instances.
- `static_ability!(filter, modifier)` has no layer (`Modifier::layer`); raw literals must agree (`lints::every_layer_in_the_pool_is_the_one_its_modifier_derives`). `equip!("{2}")` takes only the cost.
- `condition = Some(…)` gives `AbilityDef::ActivatedConditional`; every match handles both twins.
- `cost!("{1}{G}", TapSelf, SacrificeSelf)` in printed order; `Cost::FREE`, `Cost::TAP`. Readers emit costs only via `body::cost_literal`.
- Generic filters on `Filter` (`CREATURE`, `NONLAND`, …); pool-specific ones in `crates/baylee-cards/src/filters.rs` next to `crate::tokens` (Magic knowledge vs. this pool's). No per-card filter statics.
- Some tests exist only to fail on broken convention.
- Every card added, fixed or refactored is played in `crates/baylee-engine/src/engine/card_tests/<card type>.rs` (or `combo_tests/`), on the shared `testkit`, via `card_index("<oracle id>")`; `.claude/hooks/require-card-tests.py` checks recursively (restored stubs exempt). Never in the card file. A fix also needs a test beside the broken rule that fails on the old code. Diff `pool-dump` around no-rules-change refactors.

### Client

- Client architecture: `crates/baylee-client/CLAUDE.md`; `docs/client.md` is normative. Work in `crates/baylee-client-core/` does not load that file; read it by hand.
- No lights: `Tonemapping::None`, everything on the table unlit.
- Never position a table object directly: `table::sync_scene` sets a `Motion` target, `table::glide` moves it.
- Nothing is drawn on a card's print (Scryfall image rules, #274): effects, marks and plates live in our own frame around the print window, and the keyword strip is a separate lifted object that never covers name, cost or the artist/© line. The count badge keeps off every print (`cardplate::BADGE_OFF_THE_PRINTS`) until the owner okays otherwise (AGENTS.md, grey areas).
