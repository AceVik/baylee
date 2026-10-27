# BayleeAI v5 takeover, review and next training

The owner authorized Codex to take over training, consult Gemini Argon through
`agy`, include Claude Opus 5.5 at xhigh, and implement the relational extension
as the new major version BayleeAI v5. Codex coordinates mutations and jobs;
Argon and Opus reviewed read-only. GPT 6.1 Sol helpers implemented and tested
bounded changes. The original Argon controller was stopped after its work
was preserved; no competing training controller was left running.

## Decisions from the review

Argon explained that loss multipliers originally countered excessive passing,
that head-only PPO protected memory from small, narrow rollout batches, and
that its logged raw-network probabilities did not describe the tactical
behavior policy. The latter is a methodological error, not an acceptable
audit tolerance. Outcome-weighted imitation remains explicitly weighted
imitation; no calibration claim follows from it.

Opus independently checked warmstart mathematics, graph aggregation, decoding,
provenance, evaluation and the existing corpus. We retained its concrete
findings and tested them rather than treating every recommendation as proven:

- Zero only the final relational output projection. Shared weights and
  initialization RNG remain unchanged; inner relational weights begin learning
  after the output's first update.
- Revision 1's message sums can saturate highly connected seat nodes. In a
  controlled five-seed probe, 100 repeated incoming messages put about 71% of
  update preactivations beyond absolute value 3. This is synthetic evidence,
  not a measurement of saturation during actual matches.
- Card multiplicity was already observable through entity features. The
  claim that all counts were lost was too strong. Revision 2 makes typed
  source-card counts explicit and uses a multiplicity-weighted mean, preserving
  count channels across equivalent grouped/ungrouped inputs. It does not claim
  identical nonlinear outputs for all grouped/ungrouped encodings.
- Keep 28 directional group-count and 28 source-card-count channels. Unlike a
  learned mixture of relation embeddings, these give explicit cardinalities;
  their small cost is preferable to relying on implicit count recovery.
- Raise learning rates for the new module separately, with warmup and a more
  conservative output rate. This is a separately recorded training protocol,
  not a pure architecture-only causal comparison.

Version identities are `model_version=5`, Encoder7, and
`baylee_ai_v5_relational_v1` / `baylee_ai_v5_relational_v2`. V4 and v5 revision 1
remain loadable. New training explicitly defaults to revision 2. The module
uses two shared relational rounds of width 160 before the transformer;
persistent memory still updates exactly once per authorized observation.
No hidden engine state, invented legal-target oracle or new physical geometry
is supplied. This is relational processing, not a proof of general planning.

## Correctness repairs

- Loss4 removes candidate-index favoritism and asymmetric positive-label
  multipliers. Existing outcome weights remain documented.
- Tactical answers have House provenance and no PPO behavior probability.
  Raw sampling bypasses strategic logit and action overrides. Sampler2's
  existing greedy conventions remain; raw does not mean unconstrained argmax
  of every individual output neuron.
- A real Kessig Wolf Run payment test demonstrated that choosing the canonical
  member of a permanent group could tap the source reserved for an announced
  ability, causing its reversal. Deterministic equivalent-member decoding now
  works in both modes and preserves the encoded action target and probability.
  Argon's initial Mind Stone example was not reachable through the offered
  neural menu; the engine-backed reproduction used a variable activation cost.
- Raw decoded actions must re-encode to the sampled target. A repair such as
  replacing an invalid attack cannot retain the sampled mask's Net provenance.
- Collector metadata now includes temperature and sampling/thinking versions.
  Provenance version2 includes the exact-decoding requirement. The converter
  and PPO loader reject old version1 claims. Probability audit tolerance is
  tight again; archived tactical rollouts cannot become on-policy data merely
  by changing a manifest.
- Evaluations report accepted deck pools, model/graph hashes and engine
  identity. Frozen comparison scripts reject changed or silently skipped
  decks. CUDA execution requires the actual `onnx-cuda` build feature.

Focused Rust tests, strict Clippy and release builds passed. CPU regressions
cover both v5 revisions, exact warmstart, masks, padding, permutation, directed
multi-hop behavior, cardinality channels, gradients, optimizer continuation,
catalog refresh, and nonzero-weight ONNX parity across K0/1/2/4/8. These checks
do not replace a completed game benchmark or deployment testing on the AMD NPU.

## Completed offline pilot and catalog boundary

Artifacts: `/home/ace/baylee-data/v4/experiments/v5-relational-20261027`.
Both arms used the same clean 75.37M-parameter, 12-layer, width 640 v4 source,
185 training games and 19 validation games, loss4, B8/U32/K2, seed 31437 and
1,000 updates. Initial metrics matched within 3.1e-6 under BF16. V5 revision1
has 75.84M parameters; measured training allocation peaked at 8.94 GB versus
8.87 GB for the control, without allocation retries.

| Validation metric | Initial | V4 control | V5 revision1 |
|---|---:|---:|---:|
| Active-choice accuracy |57.96%|59.36%|59.25%|
| Choice NLL |0.23626|0.23404|0.23428|
| Value MSE |0.17877|0.17384|0.17426|

These values establish neither a v5 strength improvement nor its absence.
The sample, update budget and single seed are limited. Original checkpoints
and successful exports are preserved. Startup failures occurred before any
match: one invalid Commander list, a CPU-only evaluator build, derived-cache
reuse bookkeeping, and finally the important catalog mismatch. They are
retained with their recovery artifacts, not counted as benchmark results.

The old model catalog `231953f7…` has 2,431 rule tags; the current compiled
catalog `ababe756…` has 2,478. Its 47 new tags require learning. An explicit
training initialization maps shared tags by name and initializes new symbols
from recorded donors; old validation scores remain source-catalog evidence.
It is not valid to present this as a weight-neutral evaluation repair.

## Fresh training and evaluation plan

Raw collection: `/home/ace/baylee-data/v5/teacher-20261027`.
The frozen current training/validation manifest excludes test decks and has
no exact ID, declared hash or canonical-content overlap. Independent audits
also check the binary, working-card hash, deck card multisets, seeds and raw
record origins. Sharp/Expert profiles are sampled; not every pairing is
necessarily Expert versus Sharp. Fair scouting applies to every seat.

All 2,112 attempted games are collected: 1,831 wins, 262 team wins, 18 draws and
one decision-cap termination. The capped game is quarantined. No engine panic
or time-cap outcome appeared in this collection. Successful play is followed
by replay admission; it is not sufficient by itself.

The first 128 training games and 64 validation games replayed successfully:
1,456,925 training frames and 769,611 validation frames, zero rejected games
and zero unencoded decisions. They cover duels, Highlander, three-player,
four-player Commander, teams, six-player and eight-player tables. Only four
Commander validation deck lists are available, so larger validation tables
repeat decks; this is a diversity limitation.

Active experiment: `/home/ace/baylee-data/v5/experiments/fresh-relational-20261027`.
The v5 revision 2 arm has 75.86M parameters and started on the GPU with full
initial validation. Its frozen Python runtime is revision `bbcc46a75`; the
CUDA evaluator and replay converter are pinned separately with binary hashes.

1. Use one explicitly migrated current-catalog v4 initialization for both arms.
2. Train v5 revision2 and a v4 control for 6,000 updates each, full trunk/memory/
   critic/policy, B8/U32/K2, seed 31438. Shared rate 1e-5 retains the existing
   critic-rate multiplier; v5 inner relational parameters use 1e-4 and the
   zero-decay output uses 3e-5, warming up over 100 updates. Validate initially,
   every 1,500 updates and at completion; retain selected and final checkpoints.
3. Export all thinking depths with parity on actual current-catalog data.
4. Compare matched seeds/decks against Sharp: 64 raw duels, 64 tactical duels,
   32 raw four-player Commander deals, with identical house baselines.
   The frozen comparison pool has 29 Constructed and five valid Commander lists.
   Truncated games remain unknown, not losses or draws invented by the evaluator.
5. Review playing outcomes, uncertainty, fallback/repair rates, per-format
   behavior and selected-checkpoint identity. Do not publish automatically.
6. Admit further completed packages while preserving split and catalog
   contracts; seek independent seeds and family holdouts before promotion.

In parallel, eight CPU workers replay the remaining 1,919 complete training
games into `dataset-followup0001-0015`; the sole capped game is excluded.
This conversion does not modify the active experiment's admitted dataset.
Its own completion/failure watcher requests review before the new data can
enter training. Both watchers check every 30 seconds and identify controllers
by boot ID and process start time, not PID alone.

Job controllers and completion/failure wakeups continue work without user
confirmation. Successful jobs are not silently restarted. The original live
gateway model is unchanged until comparable evidence supports a replacement.

## Remaining priorities

Before treating the system as complete, repair the PlayerView-to-encoder gaps:
unseen own-deck inventory entities, payable-cast paths, payment-source context,
and explicit truncation reporting. Add per-head and per-format diagnostics;
frame-weighted aggregate accuracy can hide attack/block/target failures and
overweight long multiplayer games. Separate tactical demonstrations from true
fallback counters. V5 PPO/calibration loaders still need explicit support and
real stochastic rollout audits. Longer thinking, search, pondering and trained
self-critique require their own evidence; adding relational layers alone does
not implement them. Experienced-human strength is still an objective, not a
measured result.
