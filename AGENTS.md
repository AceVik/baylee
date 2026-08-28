# AGENTS.md — baylee

## Commands

- Build: `cargo build --workspace`
- Test: `cargo test --workspace --all-targets`
- Lint: `cargo clippy --workspace --all-targets -- -D warnings`
- Format: `cargo fmt --all`
- Codegen (regenerate subtypes, card stubs, registry, forge index):
  `cargo run -p xtask -- codegen`
- Codegen reproducibility check (CI): `cargo run -p xtask -- codegen --check`
- Explain a card (Scryfall data + forge-reference script side by side):
  `cargo run -p xtask -- explain --name "Force of Will"`

## Conventions

- `#![forbid(unsafe_code)]` everywhere; clippy pedantic is enforced in CI.
- Generated files are **committed** and regenerated only via `cargo xtask codegen`;
  never edit them by hand (marked `// GENERATED`).
- No `String`/`HashMap` iteration in engine hot paths; determinism is sacred
  (seeded RNG only, `baylee_core::ids` handles only).
- The engine never interprets `PrintRef` — prints are presentation-only.
- Card implementations carry a mandatory human-readable header (name, oracle
  text, types, set, Scryfall id) — keep it in sync when editing a card.
- Local LLM usage: at most ONE local model active at a time (M1 Max 64 GB).
- **UUIDs: always v7** (`Uuid::now_v7()` — time-ordered, DB-friendly) for
  every id we generate. v4 only when interoperating with external systems
  that require it. Scryfall ids are external and stay as-is.
- **Async policy:** the engine crate is strictly synchronous (determinism +
  speed; no I/O, no async). Async lives only at the edges: `engine-server`
  and `gateway` use `tokio` (+ `tokio-tungstenite` for websockets). Do not
  introduce async into `baylee-engine`/`baylee-core`.
- Git: commits after every working milestone; `rebase` over merge on feature
  branches. Remote: `git@github.com:AceVik/baylee.git`.
- LLM delegation learnings are recorded in `docs/llm-learnings.md` — update
  it after every card batch.

## Legal guardrails

- Unofficial non-commercial fan project. No WotC assets in the repo.
  Forge files are read-only reference, never copied (GPL-3.0). See `NOTICE`.
