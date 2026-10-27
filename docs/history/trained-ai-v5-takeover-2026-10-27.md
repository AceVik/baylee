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

Completed experiment: `/home/ace/baylee-data/v5/experiments/fresh-relational-20261027`.
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

Eight CPU workers replayed the remaining 1,919 complete training games into
`dataset-followup0001-0015`; the sole capped game was excluded. This conversion
did not modify the active experiment's admitted dataset. Its separate
completion/failure watcher requested review before admitting the new data.
Watchers check every 30 seconds and identify controllers by boot ID and process
start time, not PID alone.

The follow-up conversion subsequently completed: 1,919 games and 24,990,570
frames, zero rejected games, unencoded decisions or validation frames. An
independent audit checked every frame index, cursor, terminal marker and file
offset across 5,756 seat trajectories, plus 17,268 actual frame headers.
It did not independently decompress all 113 GiB of payload or repeat the
engine replay. The successful converter provides the replay evidence.
`teacher-20261027/followup-admission.json` records admission for supervised
staging only; these house demonstrations are not certified PPO rollouts.

Before the next stage, reserve all 128 games from `train-0015-*` as an additional
seed holdout (1,762,161 frames). These share training deck families and are not
an independent family holdout. The original 64-game validation remains fixed.
This leaves 1,791 new training games, with the original 128 training games
available for replay. Separate converter runs reuse numeric game IDs, so the
two sources must stay in separate trajectory stores unless a future explicit
namespace/remapping contract is introduced. Changing filenames is insufficient.

Two follow-up-only packages are now prepared under `teacher-20261027/next-stage`:
251 games / 2,006,968 frames and 278 games / 2,005,009 frames. Their complete-game
selection preserves the source distribution and excludes the reserved seeds;
1,262 eligible follow-up games remain unpackaged. Preparation took 54.3 seconds,
peaked at about 135 MiB PSS and added about 13.1 GiB of caches. Source payloads
are unchanged hardlinks; selected payload hashes were checked before and after
publication. Cache identities and sample reads were verified at their final
paths. The original training source stays separate. No next-stage GPU job had
started at that preparation milestone; initialization and objectives were
chosen after the paired results below.

At fresh v5 step 1,500, genuine single-choice validation accuracy improved from
89.94% to 90.66% and choice NLL from 0.25213 to 0.24069. Value MSE worsened from
0.14598 to 0.15989 (uniform-seat baseline 0.15694), and ECE from 0.07451 to
0.11473. The combined checkpoint criterion therefore still selected step 0
when checked. Finish the frozen v5/control protocol unchanged; inspect both
selected and final checkpoints before making architectural claims. The critic
here learns teacher continuation outcomes on teacher trajectories, not a
calibrated winning probability under the deployed raw policy. A later protocol
should distinguish policy quality from critic quality and report per-format,
per-trajectory and decision-only diagnostics alongside the current frame mean.

Job controllers and completion/failure wakeups continue work without user
confirmation. Successful jobs are not silently restarted. The original live
gateway model is unchanged until comparable evidence supports a replacement.

## Completed 6,000-update comparison

Both arms completed 6,000 optimizer updates. The combined validation criterion
selected v5 step 0 and v4 step 3,000. V5's selected relational output remained
entirely zero: these game results cannot establish a benefit from learned
relational reasoning.

| Selected models against Sharp | V5 wins | V4 control wins | Steady house baseline wins |
|---|---:|---:|---:|
| Raw duels, 64 paired deals | 15 | 11 | 34 |
| Tactical duels, same 64 deals | 40 | 36 | 34 |
| Raw Commander, 32 deals / 8 independent tables | 0 | 1 | 6 |

No games hit decision caps, stalled or refused answers. The raw duel difference
was +6.25 percentage points for the selected v5 model, with a table-clustered
bootstrap interval of -4.69 to +17.19 points. Tactical assistance materially
changes these results; it is not evidence of unaided neural strength. The
Commander sample is particularly small. No model was promoted.

Both actual final checkpoints improved genuine-choice imitation relative to
initialization. Final v5 choice NLL/accuracy was 0.24138 / 90.69%, versus
0.24226 / 90.64% for final control. V5 critic MSE was worse (0.16329 versus
control 0.15400 and initial 0.14598). The final v5 relational projection has
102,400 nonzero weights, L2 norm 1.48179; it was trained, but was excluded by
the combined checkpoint criterion.

The separate `final-checkpoints-20261027` diagnostic exports and evaluates
the exact final step-6,000 weights against their selected checkpoints and
each other. Original results remain intact. It reuses the pilot deals and is
explicitly a post-hoc diagnostic, not independent confirmation.

| Actual step-6,000 models against Sharp | V5 wins | V4 control wins | Steady house baseline wins |
|---|---:|---:|---:|
| Raw duels, 64 paired deals | 12 | 14 | 34 |
| Raw Commander, 32 deals / 8 independent tables | 2 | 1 | 6 |

Neither arm hit decision caps or stalled. The final v5-minus-control difference
was -3.125 percentage points in duels (table bootstrap interval -10.94 to +4.69)
and +3.125 points in Commander (-9.375 to +18.75). These small development
samples establish no architecture gain. Final v5 improved teacher imitation
while its duel wins fell from 15 to 12; imitation accuracy alone is not a
playing-strength criterion. Both actual final checkpoints remain the sources
for the next learning experiment, without promoting either to the live gateway.

Isolated v5 value-head calibration is now implemented with strict v1/v2
identity preservation and 13 passing CPU tests. It does not alter policy,
backbone or recurrent weights. Current house-teacher data does not meet its
stochastic-neural eligibility contract, so no calibration was started.

## Broader supervised continuation

`/home/ace/baylee-data/v5/experiments/broad-bc-20261027` starts from both exact
step-6,000 checkpoints with fresh optimizers and recurrent lanes. It uses only
the first prepared follow-up package: 251 games, 2,006,968 frames, 690 seat
trajectories and 63,056 segments. Enumerating the frozen sampler proves one
complete epoch is exactly 7,888 updates, with every segment visited once.
The original 64-game validation stays fixed; the reserved 128 seed-holdout games
remain excluded. No cross-conversion numeric game IDs are merged.

Train v5 first, then the matched v4 control, B8/U32/K2, seed 31440 and shared
learning rate 5e-6. V5 relational inner/output parameters use 1e-5 with 100-update
warmup. Both arms retain the 12-layer, width-640 backbone and full-network
training. Initial validation, every 2,500 updates and final validation select
snapshots using the existing weighted policy loss across all supervised heads.
This is not categorical-choice NLL alone; that unweighted diagnostic and critic
MSE remain separately reported. A selected step 0 is allowed and would mean
the inherited checkpoint beat this continuation on the selection criterion.

The six-hour training budget and seven-hour job deadline are safeguards, not
permission to label a partial epoch complete. Completion requires the exact
7,888 updates, matching sampler identity, a complete epoch cursor and validation
at the final step. Selected and final weights are retained. The controller then
exports all thinking depths with parity checks and runs raw 64-duel/32-Commander
comparisons against each source checkpoint and the other arm. These repeated
development deals do not authorize promotion. PPO and critic calibration are
not part of this run.

The reviewed seal is
`0b603be5a7cc018ea5df78a0cd691a955ef824d8abc9dbbf069d314f2dd32e8a`;
`launch.json` records controller PID 431041 and the exact invocation. Exclusive
start markers reject duplicate starts; the completion/failure watcher continues
the review workflow. Repository guard input hashing now streams instead of
allocating whole files, with three focused tests passing. This improvement does
not mutate the older guard code in the frozen experiment runtime.

## Remaining priorities

Before treating the system as complete, repair the PlayerView-to-encoder gaps:
unseen own-deck inventory entities, payable-cast paths, payment-source context,
and explicit truncation reporting. Add per-head and per-format diagnostics;
frame-weighted aggregate accuracy can hide attack/block/target failures and
overweight long multiplayer games. Separate tactical demonstrations from true
fallback counters. V5 PPO and optimizer/package continuation still need explicit
support; value calibration requires genuine own-policy records and provenance
audits. Longer thinking, search, pondering and trained
self-critique require their own evidence; adding relational layers alone does
not implement them. Experienced-human strength is still an objective, not a
measured result.

The next encoder priority is concrete: `live4/brain.rs` registers the submitted
own deck and `features7/inventory.rs::remaining_groups` exposes its remaining
library/sideboard groups. `features7/state.rs` emits no corresponding entities;
only aggregate inventory globals reach the model. A new versioned encoder should
represent the known card identities, structural references and remaining counts
without library order or opponent-private inventory. Regression tests must
cover an unseen tutor target, sideboard, observed zone changes, uncertainty and
entity-budget exhaustion. Current public-object emission silently stops at
1,024 entities. More imitation data cannot restore exact inventory omitted from
the inputs, but this audit does not establish the cause of the measured losses.

The action-menu gap is separate: both production inference and conversion use
`encode_candidates7_grouped`, which intentionally omits `legal.payable`; the
opt-in announce-before-payment encoder is not wired into those paths. Enabling
it needs a complete payment/cancellation contract and matching demonstrations,
not just concatenating new candidates into the archived training ABI. Payment
globals already contain owed-cost context; they should not be described as
entirely absent.
