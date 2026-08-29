# AGENTS.md — baylee-cards (LLM card implementation playbook)

You are implementing MTG cards as Rust code in this crate. Follow the
contract in `docs/card-dsl.md` exactly. Your task per card:

## Input package (provided per card)

1. The generated stub file `src/cards/<slug>.rs` (header with name, mana
   cost, type line, oracle text, set, Scryfall + Oracle IDs).
2. The forge-reference script for the card (ground truth for mechanics).
3. One similar already-implemented exemplar card file.
4. This playbook.

## Rules of engagement

1. **Preserve the stub's data**: `index`, `oracle_id`, `scryfall_id`, and
   the `faces` literals are generated facts — do not edit them (except
   `keywords`, `abilities`, `coverage`).
2. Keep the `//!` header comment in sync with the oracle text. Add an
   `// IMPLEMENTED — …` or `// PARTIAL — …` summary line.
3. Implement **every oracle sentence** with the DSL. If something is
   inexpressible, use `Coverage::Partial("exact reason")` plus a
   `// NOT SUPPORTED: <reason>` comment on the exact line — never hack.
4. Use the existing vocabulary (see `docs/card-dsl.md`). Do NOT invent
   new `Effect`/`Modifier`/`Filter` variants — if you need one, stop and
   flag the card instead.
5. Write `#[cfg(test)] mod tests` per card per the DSL contract. Tests go
   in the card file as unit tests for data correctness (cost, types,
   subtypes, color identity) and reference engine-level group tests for
   behavior (name them in a comment if they belong elsewhere).
6. Compile clean: `cargo check -p baylee-cards` must pass. Clippy
   pedantic must pass on your file (the `#![allow(unused_imports,
   missing_docs)]` header covers most lints — keep it).
7. One card = one file. Never touch `src/generated.rs`, `src/cards/mod.rs`,
   or other cards.

## Exemplars (match these patterns)

- Activated ability with composite cost: `polluted_delta.rs`
- Trigger with filter algebra: `ondu_cleric.rs`
- Alternative cost (pitch): `force_of_will.rs`
- Modal spell (overload): `cyclonic_rift.rs`
- Evoke: `mulldrifter.rs`
- Static via layers: `maskwood_nexus.rs`
- Trigger multiplication: `elesh_norn_mother_of_machines.rs`
- Clone: `spark_double.rs`
- Planeswalker: `jace_the_mind_sculptor.rs`

## Verification loop

1. `cargo check -p baylee-cards` — must pass.
2. `cargo test -p baylee-cards <slug>` — your tests must pass.
3. `cargo run -p xtask -- validate` — header/coverage conventions must pass.

If a check fails twice, stop and report the card as blocked with the
compiler/test output. Do not guess.
