---
name: rust-2024
description: Rust edition 2024 idioms, the toolchain versions this workspace pins, and the determinism and lint rules baylee enforces. Use when writing or reviewing Rust in this repo, when a clippy pedantic error is unclear, or when deciding whether a language feature is available under the MSRV.
---

# Rust in this workspace

Verified 2026-09-06. Latest stable is **1.98.1** (2026-09-03; a point release
fixing a vtable miscompilation). This workspace pins `edition = "2024"` and
`rust-version = "1.88"`, and CI checks the MSRV against exactly 1.88 — so a
feature stabilised after 1.88 does not exist here, however green it is on your
local toolchain. Check `Cargo.toml` before reaching for anything recent.

## The rules this repo adds on top of the language

- `#![forbid(unsafe_code)]` in every crate, and a line in `AGENTS.md` saying
  so. A blanket "unsafe is allowed" from the user does not override a crate
  attribute — if unsafe is genuinely wanted, that is a change to `AGENTS.md`
  and to the crate, made deliberately and separately.
- **Clippy pedantic with `-D warnings`**, CI-enforced. The two that bite most
  often are `missing_docs` on anything `pub` and `too_many_lines` (100). An
  `#[allow]` is acceptable when the lint is wrong for the case, but it needs a
  one-line comment saying why, matching the surrounding style.
- **Determinism** in `baylee-engine` and `baylee-core`: seeded ChaCha8 only,
  no `HashMap` iteration in hot paths, and `std::time`, `std::random` and the
  `algebraic_*` float methods are banned outright. A rules crate that reads the
  clock is a replay that cannot be reproduced.
- `baylee-core`, `baylee-protocol`, `baylee-view`, `baylee-client-core` and
  `baylee-client` must keep compiling for `wasm32-unknown-unknown`.

## Edition 2024 points worth remembering

- `gen` and `async` are reserved; `unsafe_op_in_unsafe_fn` is the default;
  `impl Trait` capture rules changed (`use<>` bounds).
- `if let` chains (`if let Some(x) = a && let Some(y) = b`) are stable and used
  in this codebase — see `deckbuilder/builder.rs`.
- `let ... else` is the idiom for early return, and is what to use instead of
  `.expect()` on a `Result` whose error type has no `Debug` (a real gate
  failure in this repo: `ErrorBody` does not implement `Debug`).

## Working habits that save gate runs

The gate is expensive, so mistakes are expensive. Two that recur:

- **Never insert a function between an existing `///` block and its
  signature.** Anchor on the closing `}` of the function *above*. The clippy
  error points at the old function ("missing documentation") and reads like a
  problem with the new one.
- **One cargo gate at a time.** Two runs block each other on the target lock.
  Use a separate `CARGO_TARGET_DIR` when another agent or rust-analyzer may be
  building.

Run `./scripts/gate-rules.sh` while working (fmt + clippy + test, excluding
the Bevy client); the full gate before a push.
