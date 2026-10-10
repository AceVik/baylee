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
DATABASE_URL=… ./scripts/gate-features.sh    # the eight non-default features
./scripts/gate-wasm.sh                       # the seven browser crates for wasm32 (CI's wasm job)
DATABASE_URL=… ./scripts/land.sh [--check]   # gate.sh + gate-features.sh + gate-wasm.sh under the cargo lock, then ff-push to main
```

- `lint-rules`/`test-rules` (`.cargo/config.toml`) exclude only `baylee-client`, never a hand-written crate list. Run the full gate before every push.
- Non-default features: `dev-control`, `dev-reload`, `dev-dylink` (client), `dev-control` (Android shim), `dev-table` (gateway), `test-support` (client-core), `fuzz` (engine: `Engine::fingerprint`, every field, for a fuzzer; `Engine::projection_is_fresh`), `mutate` (cards: `BAYLEE_MUTATE`, on only through the engine's dev-dependencies; `docs/verification-hooks.md`). `--workspace` builds none, so run `gate-features.sh` before every push too (CI's `features` job): shared code (a type `devctl.rs` uses) breaks feature builds, and a feature can make an import live that `-D warnings` flags only then. Never `--all-features`: nobody runs it.
- Landing (owner, 08.10.2026): a change lands on main after the three local gates without waiting for CI; `land.sh` does it (fast-forward only, never rebases or forces). CI runs on the main push, and a release needs that run green with every `REQUIRED_JOBS` entry (`scripts/release/ci_artifacts.py`); a docs-only push runs only fmt and tests and is not taggable. PRs run the quick half (fmt, clippy, two nextest partitions, wasm, validate, deny; label `ci:full` for everything). Table and triggers: `docs/releasing.md` §"CI".
- Main pushes also run the tests in `--release` (`ci-release` profile; never hide required behaviour in `debug_assert!`), `features`, `scryfall-cache` + `validate`, benches, MSRV, `cargo-deny`, `cargo-audit`, the Intel-mac link and the five dist packages; the nightly adds the five-platform `build` matrix (`--bins`, `check --all-targets`). Only main pushes write Actions caches (10 GB per repository). Every job has `timeout-minutes`.
- macOS uses `-Csplit-debuginfo=packed`. Builds crawl? Check `stat -f %z target/debug/deps`; sweep by renaming `target/debug` away.
- Servers are silent without `RUST_LOG=info`.

### Single tests

Engine tests: `crates/baylee-engine/src/engine/*_tests.rs` plus directories `card_tests/` (one module per card, at the card's own path: `card_tests/<type>/<dir>/<slug>.rs` for `cards/<type>/<dir>/<slug>.rs`; helpers in `card_tests/<type>.rs`) and `combo_tests/`; filter by module path.

```bash
cargo test -p baylee-engine keyword_tests
cargo test -p baylee-engine --lib -- --exact engine::keyword_tests::no_card_claims_a_keyword_the_engine_ignores
cargo test -p baylee-engine --lib -- --list
cargo build --workspace --bins && cargo test -p baylee-gateway --test e2e_processes -- --ignored
cargo test -p baylee-catalog --test cardtext_provenance -- --ignored
cargo bench -p baylee-engine --bench basics -- --quick   # vs docs/perf-baseline.md
BAYLEE_ABILITY_LOG=<dir> cargo test -p baylee-engine --lib   # which abilities fired, per test; BAYLEE_MUTATE=<card>:<index> takes one away (docs/verification-hooks.md)
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
decks-import [--archive <path>] [--refresh]   # MTGJSON precons → data/decks/precon (docs/precons.md)
decks-status [--check]            # STATUS.tsv + the gateway's playable list; rerun when a precon unlocks or is withdrawn (the xtask test says so)
dev-table --seats 4 --ai sharp [--play] [--teams 1,1,2] [--bridge house|scripted|anthropic[:<model>]|openai:<model>|profile:<name>]
```

- `dev-table` skips only sign-in and deck-pick (username `dev`, gateway HTTP, sends `ready`/`start`, real sockets). `--teams`: sides in seat order (`0` = alone), three chairs minimum. `--bridge profile:<name>` seats `baylee-seat join --profile <name>`: that profile of the seat's settings file, under its caps (`docs/llm-seat.md`).
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
DATABASE_URL=… ./target/debug/baylee-gateway invite create --note "…" [--uses N --expires 30d --count K] | list | revoke <id>
sudo baylee-invite create --note "…"      # the same on the server, with the gateway's settings (installed by baylee-deploy stage)
DATABASE_URL=… ./target/debug/baylee-gateway records export --out <new dir>   # anonymised complete game records (no game id, time or account) for training/balancing; the only input they may use (docs/privacy.md)
trunk build index.html --release --locked --public-url /play/   # in crates/baylee-client/: the web build baylee-deploy stages and finish installs to /opt/baylee/web/play (trunk 0.21.14; Caddy: scripts/server/play.caddy; docs/client.md §"Serving it at /play/")
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
- `baylee-db` is the gateway's store (accounts: login by username, email optional, guests; sessions, decks, confirmations, preferences, upload owners; entities + migrator); no `DATABASE_URL`, no gateway. A leftover `gateway-store.json` is imported into an empty DB (its `automation` map is dropped), then moved aside.
- `DELETE /account` (#292): a registered account names its password again (403 if wrong; the sign-in limiter counts it under `account:{id}`), a guest sends `{}`; one statement, the rest by cascade. Every account deletion, the guest purge included, goes through `account::depart`, which holds `cosmetics::Store::hold` until the files of pictures nobody claims any more are gone; an upload writes its owner row before its file. The lock is per process: one image directory per gateway. A deck names only pictures its player uploaded (`check_pictures`, 403). As the gateway starts, before it serves, `account::sweep_pictures` removes every stored picture with no owner row that no deck names (#301), under the same lock, logging a count, never ids.
- The card catalog is optional (same Postgres, hand-written SQL); without an ingest games still run without card text: the client draws faces from the engine's projection. Ingest every language: the client asks `/catalog/text` in its own, English fallback per printing. Plain `RUST_LOG=info` logs every INSERT.
- `/catalog/text`, `/catalog/search`, `/pool` and `/printings` answer signed-in sessions only, as `/art` does, and the gateway never asks Scryfall on a caller's behalf (Scryfall: no plain proxy or republish, #270); the client asks the lobby's gateway with its session, and signed in nowhere asks Scryfall itself. `docs/privacy.md` inventories what the gateway keeps about people.
- Search reads `card_search` (per oracle face; bigram and `tsvector` GINs), not `cards`. `Catalog::project()` rebuilds it after ingest, `migrate()` when it is empty or the stamped `SCHEMA_VERSION` (`crates/baylee-catalog/src/lib.rs`) differs; bump that on a shape change.
- Never put name and rules-text matches in one predicate: union the tiers, resolve the printing after `LIMIT`. Names use bigrams, not `pg_trgm`.
- The projection joins `type_names` (committed `data/type-names.tsv`, keyed on English name, never `SubtypeId`) inside its `INSERT`; no second `UPDATE`, no padded `LIKE`. `mine-types` needs a full catalog and writes only fully understood pairs; ties: most frequent form, then newest printing.
- Migrations: `unaccent` `WITH SCHEMA public`; no unqualified `CREATE EXTENSION`/`DROP INDEX`; `information_schema` lookups filter `table_schema = current_schema()`. Two tests keep a schema behind the sandbox to catch escapes.
- `released_at` is `date`; render it with `to_char(released_at, 'YYYY-MM-DD')`, not `::text` (sqlx pins `DateStyle` to ISO today; the SQL should not depend on the driver); `finishes`, `frame_effects`, `rarity`, `layout`, `border_color` stay `text` deliberately.

### Environment

- Gateway: `PORT` (`0` binds any free port; `BAYLEE_PORT_FILE`, a path, receives the bound port just before serving, which is how e2e helpers find it, #279), `DATABASE_URL` (required), `BAYLEE_DB_POOL` (8), `STORE_PATH` (import-only, not gitignored), `BAYLEE_ART_PATH`/`BAYLEE_DECK_IMAGE_PATH` (`off` disables; empty also refuses uploads; `/art` answers signed-in players only and the wasm client takes art straight from Scryfall, `docs/protocol.md` §"Card art"), `BAYLEE_REGISTRATION` (`off`; `invite`: a closed beta, where a new account or a new guest needs a key, a returning guest none; `docs/protocol.md` §"A closed beta: keys (#317)"), `BAYLEE_GUESTS=off` (no guest accounts, wins over a key), `BAYLEE_GUEST_CAP` (live guests, default 1000, `0` = no cap; guests sign in with a display name only, are purged when their last session lapses, 30-day sliding, and may not upload images), `BAYLEE_TRUSTED_PROXIES`, `BAYLEE_AGENT_TOKEN` (none = no agent), `BAYLEE_SMTP_URL`/`BAYLEE_MAIL_FROM`/`BAYLEE_PUBLIC_URL` (no SMTP = no mail or confirmation), `BAYLEE_ENGINE_URL` (set for a remote agent), `BAYLEE_UNIX_SOCKET` (a path: also serve every route on a unix socket, `0660`, a stale file removed; an agent on it is local, and so are its games, `/health` `games.local_running`; `docs/protocol.md` §"On the same machine: the unix socket"), `BAYLEE_GATEWAY_NAME` (what `GET /info` names the gateway; unset = the client shows the address; over 64 characters or a control/bidi character refuses startup), `BAYLEE_SOURCE_URL` (what `/info` and `/source` name as this build's source, AGPL §13; unset = the repository; not `http(s)://`, over 200 characters, or a whitespace/control/bidi character refuses startup), `BAYLEE_FEEDBACK_URL`/`BAYLEE_FEEDBACK_TOKEN`/`BAYLEE_FEEDBACK_KEY` (`POST /reports` forwards to that service; unset URL = 503; the key makes the reporter pseudonym, unset = derived from the token; `docs/feedback.md`), `BAYLEE_RECONNECT_SECS` (the reconnect window a room gets when it names none, default 180, `10..=3600`), `BAYLEE_ROOM_GRACE_SECS` (how long a waiting room keeps the chair of a player whose sockets all closed, default 60, `1..=600`), `BAYLEE_WS_TICKET_SECS` (a socket ticket's life, default 45, `1..=600`, anything else refuses startup), `BAYLEE_CHAIR_TICKETS=off` (no host may hand a chair to a seat bridge: ticket and redemption `403`), `BAYLEE_CHAIR_TICKET_SECS` (a host's chair ticket for its seat bridge, default 120, `1..=600`; the delegate must be named `LLM-…`; the bridge sits on it with no account, so guests off or a closed beta still seat it; `docs/protocol.md` §"A host's chair for a seat bridge"), `BAYLEE_WS_LEGACY_TOKENS=off` (refuse the old `?token=` on sockets before its end, 2026-10-31; `docs/protocol.md` §"Opening a socket: tickets"), `BAYLEE_TERMS_PATH` (a UTF-8 Markdown file ≤ 64 KiB for every language, or a directory of `terms.<lang>.md` such files, `terms.en.md` required as the fallback, all naming one `<!-- version: … -->`; read at start: `GET /terms?lang=` (the asked language, else `en`, else the file), `/info.terms` (the one version), `terms_stale` at sign-in, `POST /account/terms`; unset = no terms and nothing changes; unreadable, oversize, empty, no `en` or two versions refuses startup; `docs/protocol.md` §"Terms of use (WG-1)"; `docs/terms-placeholder.md` and `docs/terms-placeholder/` are not legal text), `BAYLEE_ADMIN_TOKEN` (≥ 32 characters, none of the gateway's other secrets; unset = no admin console: `GET /admin/stats` (counts only), `GET/POST /admin/invites`, `DELETE /admin/invites/{id}`, as `invite create|list|revoke`) and `BAYLEE_ADMIN_BIND` (`127.0.0.1:28767`; loopback only, else startup refuses; never on the public port or the unix socket; `BAYLEE_ADMIN_PORT_FILE` as `BAYLEE_PORT_FILE`). The console refuses `Origin`/`Sec-Fetch-*` and a non-loopback `Host` before the token, compares it in constant time, shuts for 5 min after ten wrong ones, wants `X-Baylee-Admin` on a change and logs each change on `baylee_gateway::audit` without a key; `docs/protocol.md` §"The admin console".
- A `dev-table` gateway reads `BAYLEE_DEV_SEAT_BOARD` (`0:Card;1:Card`, `baylee_cards::decks::deal_named`); a bad spec refuses startup. Keep board seeding behind the feature.
- Gateway CORS is `Access-Control-Allow-Origin: *` without `Allow-Credentials`; keep bearer-header auth, never set cookies.
- Hosted model seats (`docs/protocol.md` §"Hosted language-model seats", `docs/llm-seat.md` §"A hosted seat"): gateway `BAYLEE_SEATHOST_TOKEN` (≥ 32, none of its other secrets; unset = no hosted seat), `BAYLEE_SEATHOST_BRIDGE_URL` (where hosted bridges dial, default the loopback port). Seat agent `baylee-seathost` (`scripts/server/baylee-seathost@.service`, one OS user per instance): `BAYLEE_SEATHOST_TOKEN`, `BAYLEE_SEATHOST_NAME`, `BAYLEE_SEATHOST_STATE` (its `hosted.json`, `spend/<id>.json`), `BAYLEE_SEATHOST_CAPACITY`, `BAYLEE_SEATHOST_BRIDGE_GATEWAY` (required on `unix:`), `BAYLEE_SEAT_BIN`, `BAYLEE_KEY_STORE=file:<dir>`. Registered hosts only; keys write-only over the unix socket; tests use in-process launchers or the fake CLI, never a real model.
- Agent: `BAYLEE_GATEWAY` (`unix:<path>` dials the gateway's unix socket and hands engines the same; `baylee_protocol::unix_socket` reads it), `BAYLEE_AGENT_TOKEN`, `BAYLEE_AGENT_NAME`, `BAYLEE_AGENT_CAPACITY` (0 = unlimited), `BAYLEE_ENGINE_BIN` (default beside the agent).
- Feedback service (`baylee-feedback`, `docs/feedback.md`): `FEEDBACK_DATABASE_URL` (required, its own database), `FEEDBACK_GATEWAY_TOKENS` (`name=token,…`), `FEEDBACK_READ_TOKEN`, `FEEDBACK_ADMIN_TOKEN` (each ≥ 16 characters, none twice), `FEEDBACK_BIND` (`127.0.0.1:28780`), `FEEDBACK_POOL` (4), `FEEDBACK_WEB_DIR` (the built `web/feedback` UI; unset = `/` is 404), `FEEDBACK_TRUSTED_PROXIES` (for the sign-in limiter and the direct allowance), `FEEDBACK_DIRECT_KEY` (≥ 16, none of the tokens; unset = `POST /client/reports` is 503), `FEEDBACK_GATEWAY_ADMIN_URL`/`FEEDBACK_GATEWAY_ADMIN_TOKEN` (both or neither; the gateway's console and its `BAYLEE_ADMIN_TOKEN`, ≥ 32, none of the service's tokens; `baylee-deploy stage` makes one and writes both env files `0600`): `/ui/api/admin/…` passes a signed-in admin (session, CSRF on writes) on to it, builds each request itself, and answers a refused token `502`, never `401` (`docs/feedback.md` §"The admin console"). The UI's GitHub issue carries only the admin's own summary, a category and the build, never report text or a link back (owner, 08.10.2026). The direct route takes a report from a client signed in nowhere: no credential, a random device id it keeps only as an HMAC, per-address and service-wide hourly allowances in memory, stored `channel = 'direct'`, gateway `(direct)`, its record `record_origin = 'client'`; the only route with `Access-Control-Allow-Origin: *` (`scripts/server/feedback-direct.caddy`). A client's record (either road) is checked for shape only: the service links no engine. Deploy the service before the gateway. The UI (#311) signs in admins made only by `baylee-feedback admin add|remove|list` and talks to `/ui/api/…` with a session cookie (the service's own origin; the gateway's no-cookie rule stays); the token routes are unchanged. `cargo` never needs node: CI's `web-feedback` job and `baylee-deploy` build the UI.
- Engine: `--attach/--game/--token` or `BAYLEE_ATTACH_URL`/`BAYLEE_GAME`/`BAYLEE_ENGINE_TOKEN`; none = listening dev harness (`PORT`, `0` and `BAYLEE_PORT_FILE` as on the gateway; `BAYLEE_BIND`). Never bind it publicly: unauthenticated, every hand leaks.
- Client: `BAYLEE_GATEWAY` + `BAYLEE_GAME`, `BAYLEE_SEAT_TOKEN`, optional `BAYLEE_SEAT`; browser `?game=…&token=…`. At build time `BAYLEE_FEEDBACK_PUBLIC_URL` (`option_env!`) names the feedback service a client signed in nowhere reports to; `feedback_url` in the settings file overrides it (`""` = off); neither = such a client sends nothing. A `LocalHost` game is recorded and the last 20 kept in `records/` beside the settings (native); a report carries one only when ticked for that report, never remembered.
- Seat bridge (`baylee-seat`, `docs/llm-seat.md`): `BAYLEE_SEAT_CONFIG` (the settings file: model profiles and daily/monthly caps; `--config` wins; unset = `llm-seat.json` in the client's config directory; a named file must exist, none at all = no caps, as before). Keys only from the variable a profile names (`key_env`) or the OS credential store (`baylee-seat key set|status|delete`, service `baylee-seat`, account `{key_env}@{host}`; `BAYLEE_KEY_STORE=off` disables), never in the file (refused); tests use `MemoryKeys` or `BAYLEE_KEY_STORE=off`, never the real store. Under a file each game reserves in `llm-spend.json` beside it (`--ledger`) before it sits down and settles after; a killed bridge's reservation counts in full. Tests pass temp paths, never the real config directory. A host's client starts it with `--tethered --chair-ticket`, the chair ticket the first stdin line (never argv, env or a log); it says ready only after `Mind::check` (the cheapest provider call, a CLI's login check). Lines on paths that can outlive their reader go through `client_core::say!`/`say_err!`, never `println!`. `--mind cli:<tool>[:<model>]` (`provider: cli`) plays an agent CLI on its own login, no key (`cli.rs`, `docs/llm-seat.md` §"A CLI as the model"): the resolved program run with an argv array, never a shell; an empty `0700` temp working directory; `env_clear()` plus an allowlist, a key-shaped value refusing the start. Tests drive only `examples/fake-agent-cli.rs`, never a real CLI or model.

## Architecture

### Crates

- Graph: core → engine → gamehost → engine-server; gamehost builds view and links `baylee-ai`; engine's choice taxonomy feeds client-core and `baylee-ai` (plays from the view; uses client-core's payment matching); core, view, protocol → client-core → client (Bevy); client-core links protocol only for shared rules such as `names::username`. protocol → gateway (axum), agent (`tokio::process`, no rules). Each arrow drops a capability; test at the lowest layer possible.
- `baylee-view` depends only on `baylee-core` ids, serde.
- `baylee-client-core` is the client brain, knows no renderer, holds most client tests. Only `baylee-client` needs a GPU.
- `baylee-protocol`: protobuf `Envelope`; `Pending`/`PlayerAction` ride as `serde_json`; shared by `LocalHost` and both servers.
- `baylee-deckio` (core → deckio → client-core): deck documents in four formats (Baylee text, JSON, YAML, Moxfield text), auto-detect, and a registry of deck-link sources that answer an instruction or a fetch plan; pure, never fetches (Moxfield is its text export only). `docs/deck-format.md`.
- `baylee-cardtext` sits under core, links only serde: `/catalog/text` wire shape and sentence pairing (`pick`, `align`, `verify`, `split_cost`) for catalog and client.
- Must build for `wasm32-unknown-unknown`: `baylee-cardtext`, `-core`, `-deckio`, `-protocol`, `-view`, `-client-core`, `-client`.

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
- Hidden information is unrepresentable: libraries and other hands are counts; a face-down `card` is `None` unless entitled. `crates/baylee-gamehost/src/view/tests/hidden.rs` (and its siblings) tests each guarantee; add one per new one. One deliberate exception: in debug builds only (`cfg(debug_assertions)`), an LLM seat's reasoning (`AiLog`) is forwarded to every seat at the table as a sparring tool (owner, 06.10.2026; `docs/protocol.md` §"An AI seat's reasoning"); release builds drop it.
- Bump `VIEW_VERSION` (`crates/baylee-view/src/lib.rs`) on any breaking view change; never restate its value.
- The game log (`crates/baylee-gamehost/src/log.rs`) tells each seat its own lines, naming an object only as that seat's view would; an object the log never saw is `Hidden` to all. Every `StateDelta` carries a view and a `LogTail { from, entries }` in `log_json`; several frames may repeat a view and `seq`, so clients read the log from every frame and append by `from`. Each entry carries the host's time, which the caller sets (`Session::tell_time`, `EngineRunner::tell_time`). The Session reads no clock, because gamehost also runs in the client's `LocalHost`, on wasm too. AI agents are never handed it. `docs/protocol.md` §"The game log".
- A teammate's hand is in a seat's view only while its owner shares it with that seat, both are on one team and both are still in a game that is going. Sharing is a seat setting (`SeatSettingMsg` → `Session::seat_setting`), not a `PlayerAction`: no journal, hash or clock. An AI chair accepts a teammate's request at once and is shown nothing; `Session::agent_view` is the one view an agent answers from. `docs/protocol.md` §"A teammate's hand".
- A per-ability policy never acts silently: where it made the difference the engine journals `GameEvent::AutoAnswered`, and `Session` keeps each seat's latest 16 as `PlayerView::policy_acts`, numbered over the game, cleared by the seat's own answer (not by a clock), and sent to that seat alone. The log writes no line for it. `docs/protocol.md` §"What a policy answered".
- Granted abilities are offered under `choice::granted_ability(n)` (`GRANTED_ABILITY` is slot 0, up to `GRANTED_SLOTS` = 8); `PublicObject::granted_mana` names the output. Offer and projection share `effects::granted_activated` and `baylee_cards_dsl::simple_mana`; no second path. `docs/protocol.md` §"Granted mana".
- `GameStatic.prints` is `Option<PrintEntry>` per index; the index is the `PrintRef` objects point at, so hide with `None`, never shorten the list. A seat gets its deck's printings plus cards seen; `Session` re-sends before the view needing it.
- AI plays only through `HeuristicAgent::act(&PlayerView, &Pending)`, never an `Engine`/`GameState`. Scouting only for a current `SeatKind::Ai` (`scouting::request`); reports are never serialized, sent over the protocol, merged into views or prints, or retained. The harness uses the same guarded adapter as a hosted AI. `docs/house-ai.md`.
- `SeatKind::Driven`: AI seat taken by a socket (`Session::take_over`; `release` returns it to the retained agent). View senders ask `answers_over_socket()`, never "is human".
- `SeatKind::StandIn`: house answers for an absent player (`Session::stand_in`; `SeatAttached` → `hand_back`). Each awaited seat has one clock (in the mulligan window every deciding seat is awaited): `Deadline::Decide` (answers once) or, socketless, `Deadline::StandIn` (`reconnect_window_secs`, hands the chair over) only while another player's socket is at the table; with nobody else there the game pauses (no clock, no house move) under the table's `Deadline::Hold` (24 h, then the house concedes). `SeatDetached.left` (`POST /lobby/depart`, a client quitting on purpose) stands in at once, or ends the game with no player left. A waiting room keeps a gone player's chair `BAYLEE_ROOM_GRACE_SECS` (60) after their last socket closed (`gateway/src/departure.rs`). `docs/protocol.md` §"Leaving, and losing the connection".
- An away chair keeps `is_ai` false (`SeatIdentity.away`). The roster rides in `GameStatic`; any chair change marks every seat's roster stale.
- Engine-server harness: `JoinGame.seat_token` is a seat number; joining an AI chair takes it over, disconnecting returns it. Each socket sends only its own seat's envelopes.

### The gateway runs no rules (`docs/protocol.md`, normative)

- An agent spawns one `baylee-engine-server` per game, which dials the gateway. gateway→agent `StartEngine`; engine→gateway `EngineHello`/`SeatFrame`/`GameEnded`/`GameRecordChunk`/`RecordFlushed`; gateway→engine `GameSetup`/`SeatAttached`/`SeatFrame`/`FlushRecord`; seat sockets forwarded byte for byte.
- The gateway links neither `baylee-engine` nor `baylee-gamehost`; its e2e tests pull `baylee-engine`, `baylee-engine-server`, `baylee-gamehost` (to replay a record), `baylee-client-core` (to build a report as the client does), `baylee-view` and `baylee-feedback` as dev-dependencies only. No agent → `POST /lobby/games` is 503.
- `SeatFrame { seat, envelope }` nests an encoded player `Envelope` the gateway never decodes; keep that shape.
- Secrets: `BAYLEE_AGENT_TOKEN` on `/agent/ws`, per-game token on `/engine/ws`, seat token on `/games/{id}/ws`; not interchangeable. A secret never rides in a socket's address (#294): the session or seat token buys a single-use ticket over `POST /ws-ticket` (`Authorization`), and `/lobby/ws` and `/games/{id}/ws` open with `?ticket=` (`wsticket::Tickets`, memory only, never logged). After its secret each peer states `PROTOCOL_VERSION` (a hello field for agent and engine; `?protocol=` on the seat socket, built only by `baylee_protocol::seat_socket_path`, which takes the ticket), and the gateway refuses a mismatch in one sentence (`version_refusal`), so gateway, agent, engine and clients ship as one build. A seat socket is bounded in size (`MAX_SEAT_FRAME`) and rate (`seatrate::Allowance`, GCRA, measured against a real client in #284); a flood is closed with 1008, never dropped frame by frame. `docs/protocol.md` §"Which side checks the protocol".
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
- Cards are written by Opus, their tests by another model, and Fable reviews (owner, 29.09: the cheap lanes made too many errors); `scripts/llm/` is retired for cards. A card's author model never writes its test. Sets go in release order by first printing, each by `docs/mechanics-roadmap.md` §E8; unplayable cards (ante, dexterity, subgame) are listed in `data/unplayable.tsv`, never built.

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
- DSL facts: `Effect::AddMana` takes `Amount`/`combination` (`mana_dynamic`, `mana_choice_dynamic`, `mana_combination`). `.restricted(filter, rider)` is "Spend this mana only…"; a rider without "only" is `.when_spent(filter, rider)`: ordinary mana (CR 106.6) whose rider fires once per unit spent on a matching spell (106.6a). `DB$ Sacrifice` is `SacrificeSelf`; with `UnlessCost$`, `Effect::PlayerMayPayCostOr` (one `CostPart`; naming none declines). `cost_expr` (scriptgen) turns cost spellings (`Sac`, `tapXType`, …) into `CostPart`s; check it before blaming `cost_wizard`. `ChangeTypeDesc$` is claimed via `Params::claim_label` beside `ChangeType$`. Library search: `Optional$` = up to, `Mandatory$ True` = must, neither = refused.
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
- Every card added, fixed or refactored is played in `crates/baylee-engine/src/engine/card_tests/<type>/<dir>/<slug>.rs`, the card's own path (or `combo_tests/`), on the shared `testkit`, via `card_index("<oracle id>")`; `.claude/hooks/require-card-tests.py` checks recursively (restored stubs exempt). Never in the card file. A fix also needs a test beside the broken rule that fails on the old code. Diff `pool-dump` around no-rules-change refactors.

### Client

- Client architecture: `crates/baylee-client/CLAUDE.md`; `docs/client.md` is normative. Work in `crates/baylee-client-core/` does not load that file; read it by hand.
- No lights: `Tonemapping::None`, everything on the table unlit.
- Never position a table object directly: `table::sync_scene` sets a `Motion` target, `table::glide` moves it.
- The front door stands in a geode's cavity, the portal to a world at first light (`vista.rs`, `vista.wgsl`, #295; uniforms only). `FrontMotion::progress` drives `vista::passage` (1.0 s in, 0.8 s back, panels from 0.20 s), the loading veil holds it open (`vista::waiting`), and `reduce_motion` holds it still. Other games' login screens are inspiration only; no asset of theirs is used (`docs/legal.md` §10).
- The log panel reads like a chat (#300): local HH:MM (`LogLine::clock`), players in bold, and a card the line showed as a [link] that opens the table's preview of the line's printing (`Duel::hovered_log`). A face-down or `Hidden` card is never a link.
- The score is our own code (`client-core::music`) playing CC0-dedicated recordings of real instruments (VCSL, VSCO 2 CE, FreePats bagpipe at pinned commits; hashes, licence files and `NOTICE` shipped beside the bank; `docs/legal.md` §5 is authoritative); interface and gameplay cues are synthesised PCM (`sound.rs`). Never a scene module, a non-CC0 sample or a borrowed tune (`art/music/avoid.json`, the `originality` test). The bank is embedded (`include_bytes!`, `bank.rs` generated by `art/music/prepare.py`): the music plays offline. Its drivers are pure (`music::direct`, from the `PlayerView` only, so `LocalHost` sounds as hosted). Four themes (Settings → Audio → Music theme, per device, default Epic, or rotating) share one bank; a theme is data, never more samples. One orchestra plays through the whole app (`music::perform`), streams on the audio thread (never pre-rendered or run in a frame), and is paused once faded out, muted or silent in the background (`music::sounding`). `ClientSettings.music` is per device, and every screen it plays on has `lobby::music_toggle` (WCAG 1.4.2).
- Nothing is drawn on a card's print (Scryfall image rules, #274). Since #298 the print fills the card, and everything we say lies on objects of our own, each with its own shadow. The strip is the card's label (chip, keyword marks, identity crests); the P/T plate is an object of its own at the printed box and lies on the art at `M15_SEAM`, never over the name, cost, type line or artist/© line. The count badge stands at the card's top-right corner, outside the card: over it in a duel (upright while the card is tapped, `table::Upright`), beside its right edge at a ring (`cardplate::BadgePlace`, `SeatSlot::badge_place`), and never on another card's print. A row fans until a third of each card shows (`MIN_VISIBLE_FRACTION` 0.33) and then scrolls sideways (owner, 25.09). The offer's light is on the felt. The print gets only its finish and passing light (`cardmat::nothing_but_the_finish_is_drawn_on_the_print`). The owner's okays are in `docs/legal.md` §3.
