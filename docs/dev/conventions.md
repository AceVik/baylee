# Conventions

How code is written and committed here, for every agent and person. The
legal guardrails are `docs/legal.md` §"Guardrails for every change"; card
work adds `docs/cards.md`.

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
