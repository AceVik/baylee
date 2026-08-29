# LLM Learnings — baylee

Running log of what works when delegating card implementations to local
LLMs (hardware: MacBook M1 Max 64 GB — at most ONE local model active at a
time). Maintained by the orchestrator; entries dated, newest first.

## Process rules (baseline)

1. First batch per model is verified card-by-card; afterwards only special
   cases (layers, copy, replacement, multi-choice cards) are spot-checked.
2. Fixes are applied by the orchestrator, never by the LLM — but every fix
   is analyzed for a prompt improvement and recorded below.
3. Each card task gets: the stub header (oracle text), the forge-reference
   script, one similar already-implemented exemplar, and the DSL cookbook
   excerpt for its mechanic class. Nothing else (token budget).

## Model scoreboard

| Model | Verdict | Notes |
|---|---|---|
| (unset) | | first batches pending (M2.S8) |

## Prompt learnings

(append after each batch: error class → prompt rule that prevents it)

## State after M2.S8 (2026-08-29)

- DSL frozen (`docs/card-dsl.md`); the cards `AGENTS.md` playbook lives in
  `crates/baylee-cards/AGENTS.md`.
- `cargo run -p xtask -- card-batch` prepares per-card task packages in
  `target/card-batch/<slug>/` (STUB + FORGE + SCRYFALL + EXEMPLAR + PROMPT).
  `--cards "A,B"` restricts to a list; default = all unimplemented
  acceptance cards (83 at freeze).
- `cargo run -p xtask -- validate` enforces conventions (194 conform).
- Coverage at freeze: 92 Implemented, 19 Partial, 83 Unimplemented.
- Batch order: local model implements a card → `cargo check -p baylee-cards`
  → `cargo test -p baylee-cards <slug>` → `xtask validate`. Failures retry
  once with compiler output, then escalate. One local model at a time
  (M1 Max 64 GB).
