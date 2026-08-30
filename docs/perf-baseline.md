# Performance Baseline

Hardware: MacBook M1 Max, 64 GB. Measured with `cargo bench -p baylee-engine`
(criterion, debug assertions off for benches). CI runs the benches
(`--quick`) so they never rot; comparing numbers against this table is a
**manual** step — shared-runner timing is too noisy for a hard regression
gate. (An earlier version of this paragraph claimed automated 10 %
budgets; that job never existed.)

## M1.S4 (2026-08, Rust 1.98)

| Path | Baseline | Blueprint budget | Status |
|---|---|---|---|
| `setup/from_preset` (2×60-card decks, shuffles, opening hands) | 13.8 µs | — | — |
| `state/clone` (full game state, AI lookahead primitive) | 8.6 µs | < 5 µs | ⚠️ optimize in M2 (arena layout) |
| `state/snapshot_hash` | 5.3 µs | — | — |
| `engine/priority_pass_x4` (4× priority pass incl. legality computation) | 3.7 µs | < 50 µs per `legal_actions` | ✅ far under |

Notes:
- Clone is currently dominated by `Vec` allocations in zones/arena; the M2
  plan is copy-on-write zone storage or arena slabs with shared tails.
- Legality is recomputed per priority grant; M2 adds invalidation-scoped
  caches keyed on the effect/event generation.
