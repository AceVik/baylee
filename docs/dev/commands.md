# Commands

How to build, test, gate, land and run Baylee. Every command runs from the
repository root. `cargo dev --help` lists every development task
(`crates/baylee-dev`), `cargo tools --help` the release and asset tools
(`docs/dev-tools.md`).

## Build, test, gate

```bash
cargo fmt --all
DATABASE_URL=… cargo dev gate                 # full gate: fmt --check, clippy, nextest over what the diff reaches, validate, api (needs cargo-nextest)
DATABASE_URL=… cargo dev gate --workspace     # the same with every crate's tests, as `land` and CI run them
cargo dev gate reach [<path>…]                # which crates the gate would test for this diff (or these paths), and why; runs nothing
cargo dev gate rules                          # while working: fmt, lint-rules, test-rules, validate (if data/scryfall-cache)
cargo dev gate touched [<rev>]                # fmt + clippy on changed files only; not a gate
DATABASE_URL=… cargo dev gate features        # non-default desktop and server features
cargo dev gate shipped-features               # release graphs: shipped hooks, hooks a --workspace build would unify, reviewed advisory scope
cargo dev gate wasm                           # optional experimental browser check (coming soon)
DATABASE_URL=… cargo dev land [--check]       # full + features under the cargo lock, then ff-push to main
cargo dev --help                              # every development task (crates/baylee-dev); `cargo tools --help` the release/asset tools
cargo dev e2e [filter…] [--jobs N]            # the client's end-to-end tests: a real dev-control client per test (docs/dev/e2e.md); local, not CI
cargo dev api [check]                         # write (check) the API documents in docs/api/ from the servers' sources
cargo dev api serve [--port 28768]            # those documents in Swagger UI, on 127.0.0.1 only
```

- On a branch the full gate tests the crates the diff against the merge base with `origin/main` reaches (owner, 13.11.2026; `crates/baylee-dev/src/reach.rs`): the crate a changed file lies in, every crate whose source names the file or a directory above it (`include_str!`, `#[path]`, a joined path, a walk from `join("crates")`), and from there every crate that depends on one of them, dev- and build-dependencies included. A manifest, the lock file, the toolchain, `.cargo/` or `.config/`, and a file outside every crate that no source names and CI would build, make it every crate; so does a diff that reaches none. fmt, clippy over every crate and target, the benches, `validate` and the API documents run at every scope, so a change that breaks a dependent's build is caught. `--workspace` tests every crate, `cargo dev land` does the same before it pushes, and CI runs everything; `cargo dev gate reach` prints the crates and the reason for each.
- The gate's `test` step is built unstamped (`BAYLEE_BUILD_STAMP=none`, `crates/baylee-build/build.rs`) and without debug info (`[profile.gate]`). A stamp made every commit recompile and relink the client, the gateway, the game host and their test binaries, and `dsymutil` took about ten minutes for each of the client's: a gate build with no source change took 934 s on 13.11.2026. A by-hand run in that profile sets the same variable, or it re-stamps `target/gate`: `BAYLEE_BUILD_STAMP=none cargo nextest run --cargo-profile gate …`.
- A cargo started by `cargo dev` gets the caller's environment and not `baylee-dev`'s package variables (`CARGO_PKG_*`, `CARGO_MANIFEST_DIR`; `run::cargo`). With them `ring`'s build script ran again whenever a gate followed a cargo command typed by hand, or the other way round, and everything above `ring` was compiled again each time.
- The full gate and the feature gate compile through [sccache](https://github.com/mozilla/sccache) where it is installed (`brew install sccache`; owner, 13.11.2026, not measured yet): a dependency compiled before, in any worktree, is taken from its cache instead of being compiled again in a cold `target`. Workspace crates are built incrementally and pass through it uncached. `BAYLEE_SCCACHE=0` turns it off, a `RUSTC_WRAPPER` of your own is left alone, and `sccache --show-stats` shows what it did.
- The full gate leaves out the tests in `LOCAL_TO_THEIR_PATH` (`crates/baylee-dev/src/gate.rs`) while the diff against the merge base with `origin/main` (commits, working tree, untracked files) touches nothing under their path, and prints one line naming them (owner, 06.11.2026). Today that is `streamed_frames_match_collecting_conversion_with_bounded_memory` under `crates/baylee-train/`. The test is not ignored: `cargo nextest run -p baylee-train streamed_frames` runs it, `cargo dev gate --all` or `BAYLEE_GATE_ALL=1` (also honoured by `cargo dev land`) runs every test, and a gate that cannot diff (no `origin/main`) runs every test. CI does not run `baylee-train` at all (`--exclude baylee-train`), so nothing changed there. A test belongs on the list only when it is slow because of what it exercises, never because it is badly built.
- The gate's `test` step builds in the profile `gate` (`Cargo.toml`): the dev profile with the engine, core, card DSL, AI, gamehost, client-core, client and trainer at opt-level 1, in `target/gate`. The slowest tests run 3 to 5 times faster there; the dev profile stays at opt-level 0 because the same setting there made every rebuild 30 to 35 s slower (measured 10.11.2026). A test that fails only in the gate is reproduced with `cargo nextest run --cargo-profile gate …`.
- Benches are not run by nextest: it starts a bench target with `--bench`, so criterion measured every bench for about ten seconds as a "test" (26,757 tests, 2,000 s of the 6 Nov. gate). The gate's `test` step runs `--lib --bins --tests --examples`, and its `bench-smoke` step runs every bench once (`cargo test --workspace --bench <name>` for each `[[bench]]` without `required-features`: `decisions`, `client`, `frames`, `basics`; a new bench joins that list in `crates/baylee-dev/src/gate.rs`: criterion's test mode, no `--bench`). A bench may size itself down in that mode (`baylee-ai`'s army) by testing for `--bench` in `std::env::args()`; `cargo bench` still measures the full size under the same names. `test-rules` runs the same four target kinds without the smoke step.
- `lint-rules`/`test-rules` (`.cargo/config.toml`) exclude only `baylee-client`, never a hand-written crate list. Run the full gate before every push.
- Non-default features: `dev-control`, `dev-reload`, `dev-dylink` (desktop client), `dev-table` (gateway), `test-support` (client-core), `fuzz` (engine: `Engine::fingerprint`, every field, for a fuzzer; `Engine::projection_is_fresh`), `fork` (engine: `Engine::fork_for`, through gamehost's `foresight`, which the engine server and the client turn on for the house's bounded search, so `--workspace` builds have both and `shipped-features` checks that the shipped graph names the fork; its bench is `cargo bench -p baylee-engine --features fork --bench fork`), `simulation` (gamehost: the harness around the fork, `Session::fork_for` and the measurement hooks, for training and evaluation only; `shipped-features` fails if a hosting crate enables it, so the training crates turn it on through `baylee-train/onnx`, never a default, since `--workspace` builds unify every member's defaults into shipped binaries (`shipped-features` checks that too)), `mutate` (cards: `BAYLEE_MUTATE`, on only through the engine's dev-dependencies; `docs/verification-hooks.md`). `--workspace` builds none, so run `cargo dev gate features` locally before every push: shared code (a type `devctl.rs` uses) breaks feature builds, and a feature can make an import live that `-D warnings` flags only then. This development matrix does not run in CI. CI's `shipped-features` job runs only `cargo dev gate shipped-features`, checking that verification hooks stay out of shipped feature graphs without building the development configurations. Android's dev-control shim is not checked while mobile support is coming soon. Never `--all-features`: nobody runs it.
- The API documents (`docs/api/`: the gateway's public and admin routes, its sockets as AsyncAPI, the feedback service) are generated by oas-forge (crates.io, `tools/api-docs`) from the handlers' doc comments (`@route`, `@return`, … and `/// @openapi` on the types; the DSL is oas-forge's `docs/route-dsl.md`). The gate's `api` step (`cargo dev api check`) and CI's `validate` job run the same check of `tools/api-docs`: it fails on an error, on a stale document, and on a route a router registers that its documents lack (or the reverse). Write the API text with `@summary` and `@description` and keep developer prose below `@internal`; `$Type` and fragment names go in backticks (clippy's `doc_markdown`). Swagger UI comes from jsDelivr at a pinned version with subresource integrity (owner, 21.10.2026).
- Landing (owner, 08.10.2026; desktop scope updated 24.10.2026): a change lands on main after the full and feature gates without waiting for CI; `cargo dev land` does it (fast-forward only, never rebases or forces). CI runs on the main push, and a release needs that run green with every `REQUIRED_JOBS` entry (`baylee-tools release`); a docs-only push runs only fmt and tests and is not taggable. PRs run the quick half (fmt, clippy, two nextest partitions, shipped-features, validate, deny, web-feedback, each as its inputs changed; label `ci:full` for everything). Android, iOS and wasm are coming soon, outside CI and release requirements. Table and triggers: `docs/releasing.md` §"CI".
- Main pushes also run the tests in `--release` (`ci-release` profile; never hide required behaviour in `debug_assert!`), `shipped-features`, `scryfall-cache` + `validate`, benches, MSRV, `cargo-deny`, `cargo-audit` and the six client packages (`cargo build-client` in `package-check`, never shipped: a tag builds `dist` and PGO, `docs/releasing.md` §"Build reuse and caches"; `cargo check-client`; Intel macOS cross-compiled on the arm runner). Only the client is built for every platform; servers and tools run on Linux x86_64 (owner, 29.10.2026). The nightly adds the five-platform `build` matrix: every binary and target on Linux x86_64, the client elsewhere, and on Linux arm64 also the gateway, agent and engine server (`cargo build-servers`). The development-feature matrix is local-only. Only main pushes write Actions caches (25 GB per repository since 29.10.2026, `docs/releasing.md` §"CI"). Every job has `timeout-minutes`.
- macOS uses `-Csplit-debuginfo=packed`. Builds crawl? Check `stat -f %z target/debug/deps`; sweep by renaming `target/debug` away.
- Servers are silent without `RUST_LOG=info`.

## Single tests

Engine tests: `crates/baylee-engine/src/engine/*_tests.rs` plus `combo_tests/`; filter by module path. The card tests lie in `card_tests/` (one module per card, at the card's own path: `card_tests/<type>/<dir>/<slug>.rs` for `cards/<type>/<dir>/<slug>.rs`; helpers in `card_tests/<type>.rs`) but compile in test crates of their own, `baylee-card-tests-<part>`, one per `card_tests/shard_<part>.rs`; `baylee-card-testkit` is what they share (`docs/cards.md` §"Where a card test compiles").

```bash
cargo test -p baylee-engine keyword_tests
cargo test -p baylee-engine --lib -- --exact engine::keyword_tests::no_card_claims_a_keyword_the_engine_ignores
cargo test -p baylee-engine --lib -- --list
cargo test -p baylee-card-tests-instants-1 lightning_bolt          # one card's tests, in its crate
cargo test -p 'baylee-card-tests-*'                               # every card test
cargo build --workspace --bins && cargo test -p baylee-gateway --test e2e e2e_processes:: -- --ignored
cargo test -p baylee-catalog --test cardtext_provenance -- --ignored
cargo bench -p baylee-engine --bench basics -- --quick   # vs docs/perf-baseline.md
BAYLEE_ABILITY_LOG=<dir> cargo test -p baylee-engine -p 'baylee-card-tests-*' --lib   # which abilities fired, per test; BAYLEE_MUTATE=<card>:<index> takes one away (docs/verification-hooks.md)
```

## xtask

No `cargo xtask` alias exists; every line below follows `cargo run -p xtask --`.

```text
codegen [--check | --tables | --registry]  # narrow modes need no corpus
codegen --reclaim "<card>" --scripts <upstream-cardsfolder> [--check]
codegen --promote-generated "<card>" --scripts <upstream-cardsfolder> [--check]
codegen --complete-stub "<card>" --scripts <upstream-cardsfolder> [--check]
codegen --hold-back "<card>" --hold-back-reason "<review finding>" [--check]
codegen --hold-back-file <names.txt> --hold-back-reason "<review finding>" [--check]
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
- `codegen`/`explain`/`card-batch` read the external GPL card-script reference (`NOTICE`): `--scripts`, `BAYLEE_CARD_SCRIPTS`, or a `cardsfolder` within five levels of the repo's parent (hidden dirs, `target`, `node_modules` skipped), preferring one in a checkout whose git `origin` is github.com/Card-Forge/forge over a fork, else the first hit; stderr names the choice. A fork beside upstream: name the upstream one with `BAYLEE_CARD_SCRIPTS`. Never copy it in.
- `cr-check` finds the Comprehensive Rules the same three ways (`--rules`, `BAYLEE_COMP_RULES`, `MagicCompRules*.txt`); none = no check. Never vendor it (`docs/legal.md`). A line using a heading word must cite under it (keyword action `701.N`, keyword ability `702.N`). Sections renumber: look numbers up, never recall them. Record an old wrong citation without `CR`.
- Scryfall: `fetch_named` answers cached payloads from disk, else fetches one card. Fill a cold cache in bulk (`scryfall::fill_from_bulk`, `oracle_cards`), matched on ledger `oracle_id`, never name. Bulk never fails; gaps fetch singly.
- `data/scryfall-cache` (`--cache` default) is gitignored; copy it into a fresh worktree or codegen fails at the lines stage.
- Never run full `codegen` without the corpus: it rewrites every machine-owned card as a stub. `--tables` regenerates the four compiled-pool tables after a hand-edited card or a `baylee_cards_codegen::lines` change. A newly added hand-owned card first needs `--registry`: it reconciles placement, the card module list and registry, and preserves every existing implementation. Rebuild and run `--tables` afterwards so the compiled-pool tables include the new card.
- `codegen --reclaim` returns only explicitly named `Implemented` cards to
  generator ownership after their readers catch up. Repeat the flag to select
  several. It uses cached pinned printings and existing subtype assignments,
  requires a complete local script read and accepted canonical card tests,
  and rejects identity or path changes and coverage demotion before writing
  any card. It rechecks all planned sources before its first write. `--check`
  prints complete source diffs without writing cards and exits nonzero when
  changes remain. It never downloads, moves files, or writes other cards,
  registries, indices or tables; follow an applied transfer with
  `codegen --tables` and the card tests, validation and gates.
- `codegen --promote-generated` changes only explicitly named, finished
  machine-owned cards from `Generated` to `Implemented` after their canonical
  tests are accepted. It shares reclamation's pinned printing, Oracle,
  identity, path and whole-selection preflight checks; it does not turn a
  partial read into a complete one or bypass the honest-stub rule. `--check`
  prints the selected diffs without writing and exits nonzero while changes
  remain. Applying writes only those selected card files, never unrelated
  cards, tables, registries or indices; run `codegen --tables` separately and
  then the card tests, validation and gates. The full contract is in
  `docs/cards.md`.
- `codegen --complete-stub` finishes only explicitly named existing
  machine-owned stubs whose local scripts are now fully read. Repeat the flag
  for multiple names. It shares reclamation's pinned printing/Oracle/index/path
  checks and whole-selection preflight, but does not accept hand-owned,
  Partial or completed generated inputs. The normal renderer emits `Generated`
  when canonical tests are absent or held, and `Implemented` when accepted.
  `--check` prints full diffs, writes no cards and exits nonzero for pending
  changes. Applying writes only those selected stub paths; it never downloads,
  adopts a card, or changes tables, registries or the index. Use `--tables`
  separately after the reviewed completion. Once completed, the card is no
  longer eligible for this stub-only mode.
- `codegen --hold-back` restores only selected reviewed cards to canonical
  `Coverage::Unimplemented` stubs, recording the required reason as a generated
  comment and removing every ability and keyword. Repeat the flag or use
  `--hold-back-file` (one name per line, blanks and `#` comments skipped).
  It reads the ledger, existing subtype assignments and cached **pinned**
  printings only; missing or mismatched printings fail before any card is
  written. Finished machine-owned cards are refused: fix their reader first.
  Existing file locations, unrelated cards and tables stay untouched. `--check`
  compares the selected output without writing; follow a restoration with
  `codegen --tables` to update the compiled-pool tables.
- Codegen (even `--check`) never runs in CI; `validate` checks committed cards.

## Running

```bash
cargo run -p baylee-client
BAYLEE_REPLAY=<record or tape file> cargo run -p baylee-client   # a replay, every perspective (report records: never commit them)
BAYLEE_DEV_CONTROL=28770 cargo run -p baylee-client --features dev-control
BAYLEE_DEV_CONTROL=28770 BEVY_ASSET_ROOT=$PWD cargo run -p baylee-client --features dev-control,dev-reload
trunk serve index.html --release          # in crates/baylee-client/
BAYLEE_AGENT_TOKEN=$(openssl rand -hex 32) BAYLEE_AUTH_MODE=guests ./target/debug/baylee-gateway   # 127.0.0.1:28766 (BAYLEE_BIND=0.0.0.0 for every interface); guests, no Central
BAYLEE_AGENT_TOKEN=<same> ./target/debug/baylee-agent
./target/debug/baylee-engine-server       # dev harness, loopback
DATABASE_URL=… ./target/debug/baylee-gateway records export --out <new dir>   # anonymised complete game records (no game id, time or account) for training/balancing; the only input they may use (docs/privacy.md)
trunk build index.html --release --locked --public-url /play/   # in crates/baylee-client/: the web build baylee-deploy stages and finish installs to /opt/baylee/web/play (trunk 0.21.14; Caddy: scripts/server/play.caddy; docs/client.md §"Serving it at /play/")
adb forward tcp:28773 tcp:28770           # Android dev-control
cp .env.example .env                      # the client's BAYLEE_GATEWAY and DATABASE_URL
```

- `dev-control` is a loopback HTTP harness (input, `/state`, `/screenshot`; `/timescale /pause /step` drive `Time<Virtual>`, zero refused). Compile-time only; never ship it. `docs/client.md` §"Driving the client without its window".
- `dev-reload` hot-reloads embedded WGSL; needs `BEVY_ASSET_ROOT` and `BAYLEE_DEV_CONTROL`. `docs/client.md` §"Editing a shader without stopping the game".
- Android, iOS and browser/wasm clients are coming soon. The commands above are experimental scaffolding, not supported release targets; `docs/mobile.md` describes the mobile design. They run only when explicitly requested, outside the landing gates and CI.
- When working on the experimental wasm client, build `--release`. It renders via WebGPU (bevy feature `webgpu`); `webgl2` instead puts wgpu on GL. Shaders stay in the GL budget (uniforms only; no storage buffers, texture arrays, `bitCount`); a commit exceeding it says so.
