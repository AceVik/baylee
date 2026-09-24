# AGENTS.md — baylee

## Commands

- Build: `cargo build --workspace`
- Test: `cargo test --workspace --all-targets`
- Lint: `cargo clippy --workspace --all-targets -- -D warnings`
- Format: `cargo fmt --all`
- Codegen (regenerate subtypes, card stubs, registry, scripts index):
  `cargo run -p xtask -- codegen`
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

- `unsafe_code = "deny"` workspace-wide: safe Rust is the default everywhere.
  `unsafe` is allowed where it buys measured performance or replaces markedly
  more complex code, and then only with a local `#[allow(unsafe_code)]`, a
  `// SAFETY:` comment naming the invariant that makes it sound, and tests
  that fail if that invariant is broken. Never reach for it to silence a
  borrow-check error. Clippy pedantic is enforced in CI.
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
  ones so far: the Mana font's mana and tap symbols (§2a), and foil over a
  card image.
- Anything **clearly forbidden** is never built, merged or shipped quietly.
  Stop and tell the owner in plain words, with the rule quoted verbatim and
  the file and line that break it. A session tells the PM, and the PM tells
  the owner. This applies even when the change was requested and even when
  it looks small.
