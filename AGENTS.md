# AGENTS.md — baylee

## Commands

- Build: `cargo build --workspace`
- Test: `cargo test --workspace --all-targets`
- Lint: `cargo clippy --workspace --all-targets -- -D warnings`
- Format: `cargo fmt --all`
- Codegen (regenerate subtypes, card stubs, registry, scripts index):
  `cargo run -p xtask -- codegen`, **only with the card-script corpus**
  (`BAYLEE_CARD_SCRIPTS` at a clean upstream Card-Forge `cardsfolder`, never
  a fork). Without it, it rewrites every machine-owned card as a stub. It is
  two-phase: run it twice. After a hand-edited card,
  `cargo run -p xtask -- codegen --tables` is enough and needs no corpus.
- Codegen reproducibility check (developer's machine, **not** CI — a runner has
  no card-script reference): `cargo run -p xtask -- codegen --check`
- Explain a card (Scryfall data + card-script reference script side by side):
  `cargo run -p xtask -- explain --name "Force of Will"`
- Run the Bevy duel client (vs AI): `cargo run -p baylee-client`
  (design: `docs/client.md`; renderer-agnostic logic lives in
  `baylee-client-core`, the wire view in `baylee-view`)
- Run the client in a browser: `trunk serve index.html --release` from
  `crates/baylee-client/` (always `--release` — a dev wasm is ~350 MB)

## Conventions

- **Agent test cleanup (owner, 19.10.2026):** whenever an agent starts a client
  for testing (including dev-control/passive clients), it must close that
  client after the test, including on failure or interruption. Track the exact
  process/session at launch, clean it up in a finally/trap block, and verify it
  exited before handing back to the owner. Passive windows cannot be closed
  by the owner normally. Stop only processes started for that test, including
  its helper services; never terminate unrelated clients. Leave a test client
  running only if the owner explicitly requests it.
- `unsafe_code = "allow"` workspace-wide: `unsafe` code, intrinsics and
  assembler (`core::arch`, `asm!` for `x86_64`, `aarch64`/ARM, and modern Wasm
  when needed) are allowed and explicitly encouraged everywhere when they
  deliver good performance or RAM gains, or replace markedly more complex code.
  Every `unsafe` site must carry a `// SAFETY:` comment naming the invariant
  that makes it sound, and must receive particularly thorough test coverage
  that fails if that invariant is broken. Never reach for `unsafe` merely to
  silence a borrow-check error. Clippy pedantic is enforced in CI.
- Leave code better than you found it (owner, 06.10.2026): while doing a task,
  make the existing code you touch more efficient and cleaner — that is part of
  the task, not scope creep. Performance and RAM optimisations with `unsafe`,
  intrinsics or assembler (`core::arch`, `asm!`; x86_64, aarch64, and wasm
  `simd128` where it pays) are explicitly wanted. Each one keeps a portable safe
  reference path, states its invariants in `// SAFETY:`, is measured (a bench
  or a number in the commit), and is compensated by thorough tests: the fast
  path checked against the reference on property/fuzz inputs on every target
  CI builds. In the engine and core a fast path must stay bit-identical across
  architectures (determinism). For wasm, target the newest standard (Wasm 3.0:
  `simd128`, `relaxed-simd`, bulk memory, tail calls, …): the web client
  supports only the latest Google Chrome, so whatever it ships may be used;
  relaxed SIMD is nondeterministic by design, so never on a rules path.
- Third-party crates may be added for performance where they are maintained
  (active upstream, recent releases) and `cargo-deny`/`cargo-audit` stay green.
- Cleaner includes splitting large files into a directory module of small,
  single-purpose files (`example.rs` → `example/mod.rs` + `example/*.rs`, or
  `example.rs` + `example/` where the crate already does that). Move code
  without changing it in one commit, change it in the next, so the diff of the
  move stays reviewable; card files are placed by codegen and never moved.
- Generated files are **committed** and regenerated only via `cargo xtask codegen`;
  never edit them by hand (marked `// GENERATED`).
- No `String`/`HashMap` iteration in engine hot paths; determinism is sacred
  (seeded RNG only, `baylee_core::ids` handles only). **Banned in
  baylee-engine/baylee-core: the `algebraic_*` float methods (Rust 1.98+),
  `std::time`, and `std::random` — all non-deterministic.**
- The engine never interprets `PrintRef` — prints are presentation-only.
- Card implementations carry a mandatory human-readable header (name, oracle
  text, types, set, Scryfall id) — keep it in sync when editing a card.
- Local LLM usage: at most ONE local model active at a time (M1 Max 64 GB).
- **UUIDs: always v7** (`Uuid::now_v7()` — time-ordered, DB-friendly) for
  every id we generate. v4 only when interoperating with external systems
  that require it. Scryfall ids are external and stay as-is.
- **Async policy:** the engine crate is strictly synchronous (determinism +
  speed; no I/O, no async). Async lives only at the edges: `engine-server`,
  `gateway` and `agent` use `tokio` (+ `tokio-tungstenite` for websockets),
  and in all three it is transport, never rules. Do not
  introduce async into `baylee-engine`/`baylee-core`.
- Git: commits after every working milestone; `rebase` over merge on feature
  branches. Remote: `git@github.com:AceVik/baylee.git`.
- LLM delegation learnings are recorded in `docs/llm-learnings.md` — update
  it after every card batch.

## Card work

Anyone implementing cards or writing card tests reads
`docs/card-agent-brief.md` first, then `CLAUDE.md` whole: its "Cards"
section binds every agent, whatever its file name says. The short form:

- Sets go in release order. Fix the reader (`scriptgen`) before
  hand-writing, and never fix generated output in the card.
- A single-faced card writes `abilities` at card level, never inside
  `face!`. Edit only `coverage`, `keywords`, `abilities`.
- A sentence the DSL cannot say makes the card `Coverage::Partial("…")`
  with a `// NOT SUPPORTED:` comment, never `Implemented`.
- A new DSL or engine piece is named for the rule, never a card (no
  `Effect::WandOfIth`). No card text in `baylee-core` or `baylee-engine`.
  It gets a test beside the rule that fails on the old code, and its own
  commit.
- Every card is played in its own test file at its own path,
  `crates/baylee-engine/src/engine/card_tests/<type>/<dir>/<slug>.rs`,
  naming it by a literal `card_index("<oracle id>")`. The model that wrote
  a card never writes its test.
- Before done: `./scripts/gate-rules.sh`, `xtask validate`,
  `./scripts/baylee-tools hooks require-card-tests`, then the full gate.

## Legal guardrails

- Unofficial non-commercial fan project. No WotC assets in the repo.
- The card-script reference is an external, GPL-licensed corpus that codegen
  may read as an automated lookup from a local checkout. No file of it is
  ever copied into this repository or distributed with it. `NOTICE` names it
  exactly and is the one place that does; everywhere else it is "the corpus".
- Every change to cards, icons, symbols, card images, fonts, audio or
  anything else shown to a player is checked against the rules we depend on:
  the Wizards Fan Content Policy and its FAQ, Scryfall's API and image
  terms, and the licences of what we ship (`docs/legal.md`, #270). Quote the
  rule from its page, never from memory.
- A grey area (the rule's wording is met loosely or not at all, but the
  practice is tolerated) is allowed only with the owner's explicit okay,
  recorded in `docs/legal.md` with the rule quoted beside it. The accepted
  ones so far: the Mana font's mana and tap symbols (§2a), foil over a
  card image, and a passing state over a card image while it lasts (dimmed
  while a question leaves the card out, phased out, asleep; the owner,
  13.10.2026, §3: the image rule is about lasting alteration).
- Anything **clearly forbidden** is never built, merged or shipped quietly.
  Stop and tell the owner in plain words, with the rule quoted verbatim and
  the file and line that break it. A session tells the PM, and the PM tells
  the owner. This applies even when the change was requested and even when
  it looks small.
