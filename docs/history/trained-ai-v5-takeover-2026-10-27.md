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

## Follow-up advice on learning progress

At the owner's request, Gemini 4 Argon High and Claude Opus 5.5 xhigh reviewed
the updated results and supplied code excerpts independently. Both calls had
no external tool capabilities; neither inspected additional repository files
or changed the active experiment. Each received a corrective follow-up and
returned a revised recommendation. The four responses and source packet are
preserved under `/home/ace/baylee-data/v5/advice/training-improvement-20261027`.
`review-summary.json` records actual model identities and the accepted scope.

Both now prioritize measurement and learning objectives over another increase
in model size. Argon emphasizes loss normalization, outcome weighting and
diversity; Opus gives greater priority to a shadow house teacher correcting
states visited by the neural actor (DAgger). Neither review establishes the
cause of the weak raw results. The agreed sequence is:

1. Add independent diagnostics by policy head, pending question and format:
   genuine-choice NLL/accuracy, pass versus active labels, free-entry BCE and
   exact-set/legality measures for subset and pair heads. Expose truncation,
   repairs and actual tactical overrides separately. Preserve a common,
   unweighted evaluation protocol across loss variants. Use available source,
   selected and final checkpoints; intermediate logged steps are not necessarily
   retained as separate weights.
2. Test loss changes individually with the same initialization, data, update
   budget and evaluation: outcome weights disabled; unroll-wide instead of
   per-timestep normalization; then isolated changes to critic/auxiliary
   gradients into the shared representation. Keep these opt-in and versioned;
   the running frozen loss and sampler remain unchanged. The current categorical
   weighting cancels for singleton active groups, while BCE denominators have
   a different clamp, so a universal 4.7-fold effect would be an overstatement.
3. Control data balance with whole chronological seat trajectories and bounded
   packages/caches. Do not sample arbitrary mid-game windows with an unrelated
   recurrent state. Measure decision/loss contributions before treating frame
   counts as gradient weights.
4. Build a separate DAgger contract: replay still records the executed neural
   action; an optional sidecar carries the teacher's unexecuted action tied to
   game/frame, view, candidate and version identities. The teacher must receive
   only permitted information and compatible plan/history state. Mapping
   failures are counted. Start with a supervised policy-only comparison against
   ordinary teacher data; these labels are not PPO behavior actions. Actual
   actor outcomes and next observations remain valid for separately scoped
   value/auxiliary work.
5. Specify Encoder8 inventory and payment/cancellation coverage alongside this
   work. Keep Encoder7 archives unchanged. Detailed input completeness is a
   real requirement, but its contribution to the measured losses is unknown.

A new index-only audit of the 251-game package counted 360,859 decision frames.
Duels account for 138 games, 13.87% of all frames and 22.67% of decision frames;
Commander accounts for 46 games, 46.03% of all frames and 44.62% of decisions.
Two eight-player games account for 11.05% of frames but 5.66% of decisions.
The remaining games cover teams, Highlander, three-player and six-player.
These are counts, not measured gradient contributions, and are recorded in
`package-frame-audit.json` and `package-decision-audit.json`.

The reviews' original stronger statements were corrected: `h.detach()` stops
backpropagation across an unroll boundary but preserves forward memory; it
does not erase a payment plan. Critic memorization and policy distribution
shift remain hypotheses. At broad-BC step 5,000, critic MSE was 0.152554, below
the 0.156940 uniform baseline and its 0.163289 source value, while choice NLL
had worsened from the step-2,500 result. No monotonic improvement is established.
Two seeds are a noise probe, not a significance test; gameplay comparisons with
uncertainty remain necessary before any promotion, even when offline metrics
improve. The existing controller retains the GPU and its scheduled comparisons.

## Implemented learning and performance follow-up

The owner authorized implementation and another performance discussion. Gemini
`gemini-4-argon-high` and Claude `claude-opus-5-5` (xhigh) each received supplied
code, measurements and a corrective follow-up. Artifacts are under
`/home/ace/baylee-data/v5/advice/performance-20261027`; `review-summary.json`
records model identities, corrections and retained hypotheses. Neither adviser
changed files or ran training. GPT 6.1 Sol agents implemented and independently
reviewed bounded subtasks in the shared repository.

Implemented, with the frozen broad experiment unchanged:

- Optional complete-history diagnostics by head/pending/format/game, with
  additive per-game summaries and explicit namespace identities. Statistics
  transfer from CUDA once per unroll; pinned loader batches enable prefetch.
  Threshold-based subset/pair agreement is not decoded-action legality.
- Versioned loss5 flags for outcome weighting, full-unroll normalization and
  detached value/auxiliary inputs. Loss4 defaults pass exact loss/stat/gradient
  regression; ordinary and deferred optimized paths support the new loss.
  Checkpoint resume validates the entire objective contract.
- A view-only Sharp/Expert shadow collector and an opt-in verified converter.
  Neural actions remain the executed record; counterfactual labels go into a
  separate BC dataset. All observations remain chronological. Python requires
  completed metadata and intact companion hashes, and both PPO paths reject
  shadow-label datasets before model execution.
- A current-architecture performance probe with correct v5 optimizer groups,
  fixed K, raw pre-clipping gradient parity outside timed updates, bounded
  pinned input storage and retained production finite-value guards. No extra
  per-update timing synchronization is introduced. The old benchmark used v4
  construction and curriculum K and cannot establish current v5 throughput.
- Optional compact thinking on actual decision rows. The complete K0 memory
  pass still executes for every row; CPU output, H, loss and gradient tests
  cover v4/v5, mixed K, empty/all/sparse decisions and padded frames.

The shadow path has a real producer-to-consumer integration artifact:
`performance-20261027/shadow-contract-fixture`. An engine fixture with a mock
actor created the original record and shadow sidecar; the actual `convert4`
binary produced 3,275 frames in one complete game with zero rejections. Python
admitted its exact metadata/companions and opened the binary dataset as 206
segments in two trajectories. PPO admission rejected it. This checks the data
contract, not ONNX actor probabilities or improved play. Existing streaming
package builders must not discard companion files or publish in-progress
shadow metadata; actual-network collector auditing remains a next step.

### What the performance measurements establish

The first 20-second process/GPU sample was taken during validation. Its idle
training workers cannot diagnose validation-worker starvation. A second short
training sample saw GPU utilization reach 100% in eight of eleven samples,
while the two loader processes averaged about 5.8% CPU each and disk reads
were negligible. These observations do not establish an epoch-wide idle
fraction, an exclusive host bottleneck, or a power-limit diagnosis.

`audit_batches5.py` then inspected 48 deterministic, evenly spaced batches from
the actual frozen epoch-0 sampler (B8/U32/seed31440, 7,888 total batches). It ran
with one CPU thread at nice19 in 11.81 seconds without a GPU forward. All 48
fit the 128 MiB input-prefetch threshold: backing storage median 46.61 MiB,
p95 89.14 MiB, sampled maximum 126.61 MiB. This is not a global bound.

Across sampled allocated entries, entity padding was 58.52%, the E-squared
attention estimate 74.91%, and candidate padding 92.08%. Only 2,142 of 12,064
seat/time rows in active thinking timesteps were decisions (17.76%). This
motivates compact thinking and a later investigation of board padding.
It does not predict a whole-training speedup: board encoding, backward,
readouts, validation and synchronization remain. Larger buffers alone are
not supported as the leading improvement by this sample.

The advisers agreed after correction to test saved activation layers 1/2,
unchanged-batch board chunking and compact thinking, and to use a trace before
changing recurrent compilation or pair grouping. Increasing saved activations
trades recomputation for memory; the current peak cannot guarantee worst-case
headroom. This follows the mechanisms documented in PyTorch's
[activation checkpointing guide](https://pytorch.org/blog/activation-checkpointing-techniques/)
and [asynchronous transfer guide](https://docs.pytorch.org/tutorials/intermediate/pinmem_nonblock.html).
The production loss/gradient finite checks remain before the optimizer update;
moving their failure detection later would change behavior. Whole-trajectory
sorting and different batch lane counts remain separate learning experiments.

### Current run and automatic next window

The broad v5 arm finished all 7,888 updates. Its existing policy score selected
step 2,500 (choice NLL 0.238308, active-choice accuracy 58.8318%, value MSE
0.160583). The final checkpoint instead has choice NLL 0.243727, active-choice
accuracy 60.8216%, value MSE 0.152104 and ECE 0.041909. Different metrics prefer
different checkpoints; these values do not establish a playing-strength gain.
The matched control arm and original scheduled comparisons subsequently
completed. Control selected its final step 7,888: choice NLL 0.239097,
active-choice accuracy 61.7027%, value MSE 0.173852 and ECE 0.131599.

Raw-policy matches against Sharp, with the same deals and seat rotations:

| Model | Duels | Commander |
| --- | --- | --- |
| v5 source | 12/64 | 2/32 |
| v5 selected after broad BC | 16/64 | 1/32 |
| control source | 14/64 | 1/32 |
| control selected after broad BC | 14/64 | 1/32 |

No game was capped or refused. The v5-versus-control duel difference was
3.125 percentage points, with a table-cluster bootstrap 95% interval of
[-7.8125, 14.0625] points. The v5-versus-source difference was 6.25 points,
interval [-3.125, 15.625]. Both include zero. Commander has only eight
independent tables and no positive signal. These results establish neither
architecture superiority nor experienced-human strength. No model was promoted.
Both ONNX exports passed 79-frame parity checks. The complete results are in
`broad-bc-20261027/result.json` and its referenced paired reports.

A separate queued controller, PID 454387, is registered at
`/home/ace/baylee-data/v5/experiments/post-broad-review-20261027` and in the broad
run's `queued-successor.json`. It waits for the exact predecessor identity to
exit successfully, completed comparisons, verified model hashes and a free GPU.
It then copies immutable inputs and diagnoses v5 before/after and the selected
control on all 769,611 validation frames. A bounded performance screen compares
the baseline with saved1, saved2, compact thinking, prefetch2 and board8 in
separate candidate jobs. Fresh-prefix scope, numerical rejection and memory
failures remain explicit; no configuration or model is promoted automatically.
It has now advanced from waiting to the GPU diagnostics. The runtime is frozen
at `dac7498501f051dae314174a946d1b5df7498c2b`, with 150
hashed Python/controller files. The existing broad-run watcher remains active;
`baylee-v5-postreview-watch.timer` independently wakes this session after the
new review completes or fails.

The next learning comparison uses the same source checkpoint, data, seed and
update budget for unchanged loss, outcome-off and full-unroll normalization.
Critic/auxiliary detachment follows independently. Review the new head/pending
diagnostics and current playing-strength results before selecting that pilot;
do not silently combine the ablations or start a duplicate controller on a
completion wake. No user confirmation is needed to continue the authorized
work. Encoder8 inventory/payment completeness remains separately versioned
work, not a modification to the running Encoder7 archive.

### Avoiding repeated input reads in future controllers

The completed comparison controller spent substantial CPU time rehashing
unchanged sealed inputs between small jobs. Its cumulative logical read count
exceeded 960 GB; that is not a measurement of physical disk traffic. The active
frozen controller was left unchanged. New optional process-local digest
receipts reuse a hash only when resolved path and strict file identity still
match; fixed expected seals remain separate. CPU tests cover same-size edits
with restored mtime, replacement, symlink retargeting, mid-read changes and
actual guarded-job integration. All 12 focused tests pass.

A bounded warm-cache probe used three files totalling 128 MiB and three passes:
384 MiB of logical reads fell to 128 MiB, with identical SHA-256 results and
six cache hits. Observed elapsed time was 0.241 versus 0.083 seconds. This
demonstrates the redundant-read reduction, not a whole-training speedup. The
measurement is retained in `performance-20261027/input-digests-benchmark.json`.

The next objective study is prepared in `objective-ablation-20261027`.
Package 0001 contains 278 complete games and 2,005,009 frames. A metadata audit
found no source-record or byte-interval overlap with package 0000, the original
training/validation splits or the reserved seed holdout. It is unexposed
relative to those audited inputs; ancestral-model exposure and identical
payloads under different paths remain unknown. This is not a held-out-family
test. A matched loss5 legacy arm is included alongside the unchanged loss4
reference so implementation-version effects can be separated from the
outcome-weight and unroll-normalizer changes.

### Queued objective comparison

The next controller is now running as PID 471736 in
`/home/ace/baylee-data/v5/experiments/objective-ablation-20261027`. It waits for
the exact post-review predecessor to finish; no competing GPU training was
started. Its frozen trainer revision is
`b587590a475a00e6a9fc6bc37f32f0264b028b8a`, with 151 Python files and the same
evaluation binary as the completed broad comparison. The input seal
`4278156e2fcf047f7c2c82d9e773e885365f65f3d01366c57bf8fc53d91d1fd1`
covers 1,484 files (22.59 GiB of logical input sizes). CPU preparation verified
the copied source, loaded the model and parsed all four actual training commands.

All four arms start from broad v5's selected step 2,500 with fresh optimizers
and package 0001: unchanged loss4, loss5 with legacy settings, loss5 without
outcome weighting, and loss5 with full-unroll per-head normalization. Each
receives exactly 1,500 updates, B8/U32/K2 and seed 31441. Initial/final validation
is identical; asynchronous checkpoints remain enabled. These are partial
training windows, not complete epochs. New performance variants remain off.

The primary comparison uses each fixed final checkpoint, independently of
which snapshot the existing policy score selects. Common diagnostics cover all
769,611 validation frames. Raw Sharp matches repeat the same 64 duel and 32
Commander cases for source and all four final models, with comparisons against
the source, loss4 reference and loss5 legacy bridge. This remains reused
development evidence; no automatic promotion follows.

Seven controller tests pass, covering duplicate locks, predecessor failure,
corrupt seals, changed produced artifacts, incomplete update/validation counts,
invalid diagnostics and loaded-model/evaluation identities. An independent
review also checked the actual CLI and checkpoint/export schemas. Produced
model and export hashes are pinned once and checked across consumers. The
controller uses process-local digest receipts throughout, avoiding repeated
full reads while preserving fixed expected hashes. The independent
`baylee-v5-objective-watch.timer` is active for completion/failure delivery to
this session. Estimated duration after GPU acquisition is about 3–4 hours;
compilation and data shapes can change it.

The first two full post-review diagnostics already explain why aggregate
accuracy is insufficient. Source-to-selected v5 pass-label agreement rose
98.29% to 98.51%, while active-label agreement fell 59.64% to 58.81%. Subset
free-entry BCE worsened from 0.3271 to 0.4109 while mean per-decision BCE improved
from 0.6118 to 0.5408. These distinct reductions emphasize different decisions;
both must be retained. They do not establish a causal explanation or certify
decoded-action legality. The selected control diagnostic and bounded GPU
performance screen remain in progress at this entry.

### Mulligan capability and remaining input gap

A follow-up inspection confirms that Encoder7 offers Keep and, when legal,
Take, and uses the subset head with exact cardinality for MulliganBottom. The
live seat defaults to Tactical mode, whose view-only mulligan rules may override
the model; raw evaluation bypasses those rules. These are supported actions,
not evidence that the learned strategy is strong.

The selected v5 diagnostic agrees with 212 of 258 mulligan labels. Reading the
validation cache headers found exactly 212 Keep and 46 Take labels: always
keeping achieves the same aggregate score. This does not prove the model always
keeps; a prediction confusion matrix is needed. The audit is preserved in
`/home/ace/baylee-data/v5/advice/mulligan-20261027/inspection.json`.

Encoder7 does not explicitly encode `Pending::Mulligan.taken` or
`next_is_free`; only the candidate legality reflects `can_take`. The tactical
helper can read the missing fields directly, while the neural policy must infer
history/context without an explicit free-mulligan signal. Add these authorized
inputs to the next versioned encoder with matching data, and evaluate Keep/Take
separately from bottom-card quality. Existing Encoder7 archives and active
frozen comparisons remain unchanged.

### Post-review completed; objective study started

Completion event `ba33abe08ebc50fae9ec149c` was checked against the actual
post-review artifacts. All three full diagnostic reports cover the same 64
games and 769,611 frames, with matching source/data identities and no coverage
or finite-metric blocker. The detailed review is retained as
`objective-ablation-20261027/diagnosis-review.json`. The already queued
controller PID 471736 advanced to `train-loss4-reference`, beginning with its
initial full validation. Its child is PID 473664. No duplicate was launched.

The completed performance screen measured five warmup and 20 timed updates
per variant on fresh chronological prefixes, with at most 112 entity slots in
these batches. Inputs were already pinned in RAM; loading waits were outside
the measured window. Ratios below compare each variant with the baseline in
the same process, and each candidate was timed only once:

| Execution variant | Throughput ratio | Peak allocated VRAM | Result |
| --- | --- | --- | --- |
| Baseline | 1.000 | 3.47 GiB | Available in every probe |
| Save one activation layer | 1.011 | 4.10 GiB | Within sampled numerical tolerances |
| Save two activation layers | 1.040 | 4.73 GiB | Within sampled numerical tolerances |
| Compact thinking rows | 0.993 | 3.24 GiB | Within sampled numerical tolerances |
| Prefetch two future batches | 0.995 | 3.48 GiB | Within sampled numerical tolerances |
| Board chunk eight | Not timed | Not established | Numerical check rejected |

Board chunk eight had relative gradient L2 difference 0.2204 and maximum
hidden-state difference 0.0840, exceeding declared tolerances of 0.02. It was
correctly rejected; this is not a training crash or a memory-limit result.
Even unchanged-pipeline variants have small BF16 differences, so these are
tolerance checks, not bit-exact equivalence claims.

No execution setting was adopted. Saving two activation layers is only a
candidate for repeated measurements on larger boards with valid recurrent
histories and sufficient worst-case memory headroom. The short screen does
not establish a whole-training 4% gain, and neither larger prefetch nor compact
thinking improved this sampled throughput. Its review is preserved in
`performance-20261027/screen-review.json`; the ongoing four-arm comparison
retains its sealed execution settings.

The first real guarded training job used 1,484 digest-cache hits, hashing only
the previously uncached Python executable (31,508,264 bytes). This confirms
that its job manifest avoided rereading the 22.59 GiB sealed inputs already
verified by the controller. It does not change GPU arithmetic or learning.

An independent code review found no weight, optimizer, RNG or initial-H
restoration error; four focused CPU tests pass. It did identify measurement
limits beyond the short window: prefetch2's correctness pass uses the same
compute path as the baseline, yet has a relative gradient difference of 0.0096.
That is evidence of a nonzero repeat-computation noise floor, not an effect of
prefetching. An explicit baseline repeat is needed to measure it. Baseline
throughput varied by about 1.05% between these separate probes, so saved1's
1.06% result is not distinguishable from that variation here. The original
report also omitted effective TF32/float32 precision flags; later shape
compilation may remain inside timed windows. Follow-up probes must record
precision, include repeated controls and broaden multi-step/shape coverage.

Commit `31dcef534` adds the identical `baseline-repeat` execution variant to
future probes, records effective precision and determinism flags, and checks
that these flags remain unchanged across variants. The explicit
`--float32-precision highest` option disables TF32 probe-wide; the default
`inherit` preserves existing behavior. Reports now label numerical coverage
`FIRST_UNROLL_ONLY`. Eight focused CPU tests pass. This instrumentation does
not retroactively change the completed screen or the frozen training runtime.
The first objective arm completed its initial full validation and reached
175 optimizer updates with no reported allocator retries or stderr errors.

### Objective fits complete; native evaluator runtime recovery

Wake event `bf9ad9e69dd9d623f4ff1656` reported a failure at
`eval-source-raw-duels`. The original controller had exited. All four fits had
already completed 1,500 updates and full 769,611-frame final validation;
selected steps also happened to be 1,500. Five common diagnostics and five
79-frame ONNX export checks had completed. No training was lost or repeated.
An independent audit verified the completion, checkpoint, diagnostic and job
identities; the separate runtime preflight also rehashed the complete original
input and produced-artifact seals.

| Fixed-final model | Active teacher-label agreement | Active categorical NLL | Critic MSE |
| --- | --- | --- | --- |
| Source | 58.82% | 1.03684 | 0.16058 |
| Loss4 reference | 59.79% | 1.00807 | 0.15212 |
| Loss5 legacy bridge | 59.50% | 1.01158 | 0.15234 |
| Outcome weighting off | 59.72% | 1.00596 | 0.14299 |
| Unroll per head | 59.96% | 1.00256 | 0.15406 |

All arms improve active-label NLL, while pass-label agreement falls slightly.
Unroll normalization improves subset free-entry BCE to 0.33402 versus the
bridge's 0.38260, but its mean per-decision BCE is worse (0.51395 versus
0.50031). Outcome-off has the lowest observed critic MSE. Target and card
choice errors remain substantial. These are reused-development-label metrics
from one training seed, not proof of better play or grounds for promotion.
The detailed audit is
`/home/ace/baylee-data/v5/advice/objective-review-20261027/diagnosis-review.json`.

The failure was native runtime packaging: the copied `runtime/bin/eval4`
looked beside itself for `libonnxruntime_providers_shared.so`, which had not
been copied. Both required provider libraries were already sealed at the
original evaluator directory, but `LD_LIBRARY_PATH` alone did not satisfy this
load. The original executable has the exact frozen SHA-256
`0938b6179863b77109e5a1262d38be5b0af3954487ad510775cbd7fa1c843388`.
Running that executable with its existing colocated libraries completed a
two-deal CUDA preflight in 14.27 seconds. Its loaded model identity and raw
protocol passed verification, with no caps, stalls or refusals. This confirms
runtime viability, not playing strength. The original failed directory and
its status are preserved; recovery uses a separate evaluation-only controller.

The reviewed recovery controller was launched as PID 491128 in
`/home/ace/baylee-data/v5/experiments/objective-eval-recovery-20261027`.
Ten focused CPU tests pass. Its 1,576-file input seal has SHA-256
`90ddf9e9097f0b275de60ece203ac09a6456b66460405544fc4f84f78b43812e`.
It rechecks the original training completions and all reused artifacts, records
loaded CUDA provider mappings in its own short preflight, then runs the same
64 duel and 32 Commander cases for the source and four fixed-final models.
Eighteen paired reports compare source, loss4 reference and loss5 bridge.
Completed fits, diagnostics and exports are reused; no new training or automatic
promotion occurs. `baylee-v5-objective-recovery-watch.timer` reports completion
or failure to this session. Previously delivered experiment timers were retired.

### Objective playing comparisons complete

Completion event `8578befdaf4edcc78880f24e` was reviewed against the actual
recovery result. Ten evaluations and eighteen paired reports passed an
independent identity/hash audit and exact recomputation of their paired
statistics. Every evaluation had zero caps, refusals and stalls. Small explicit
house fallbacks remain part of the recorded raw-policy execution contract.

| Fixed-final model | Duel wins / 64 | Commander wins / 32 |
| --- | --- | --- |
| Source | 16 | 1 |
| Loss4 reference | 15 | 2 |
| Loss5 legacy bridge | 18 | 2 |
| Outcome weighting off | 13 | 2 |
| Unroll per head | 17 | 3 |
| House Steady baseline | 34 | 6 |

All models face Sharp on the same starts. These are policy-substitution
comparisons, not model-versus-model matches. The bridge improves two duel
outcomes and worsens none relative to the source, but its table-bootstrap
interval for the win-rate difference includes zero: 0 to +7.81 percentage
points. Unroll versus bridge is -1.56 points, interval -7.81 to +4.69. Every
one of the eighteen paired empirical and bounded intervals includes zero;
Commander has only eight independent tables. Repeated use of this development
suite further limits any selection claim. No objective or model is promoted.
The lower critic error of outcome-off did not translate into the best observed
duel result, reinforcing the need to keep value diagnostics and playing strength
separate. Full audit:
`/home/ace/baylee-data/v5/advice/objective-final-review-20261027/review.json`.

The next data experiment targets states reached by the neural actor itself,
with independent Sharp teacher advice for those exact authorized views. Before
scaling collection, compare real-neural games with shadow advice disabled and
enabled, verify unchanged executed records/probabilities, replay all records and
audit Python admission of complete chronological BC-only labels. Use separate
TRAIN decks and fresh seeds, with complete table rotations kept within one
split. This is a corrective-data pilot, not a claimed strength improvement.

The bounded real-neural pilot was launched as PID 496881 at
`/home/ace/baylee-data/v5/experiments/shadow-pilot-20261027`. Its 1,232-input
seal is `527391116b99117d414142b5818e2886ff6a95d1da9a71d7c502d9c576917681`.
The actor remains the unchanged broad-selected source, avoiding selection from
the noisy objective leaderboard. A newly archived CUDA evaluator/converter at
revision `9282a32f6` includes shadow collection and colocated provider libraries.
The archive's CPU mock replay/conversion/admission recheck passes; this alone
does not prove real-neural correctness.

The pilot runs matched shadow-off/on pairs for four greedy duel deals and
eight greedy Commander deals, plus two stochastic duel deals at temperature
0.7. These represent two, two and one independent tables respectively, with
all seat rotations retained. Fresh seeds start at 500270000, 500271000 and
500272000; frozen TRAIN-only pools contain 613 constructed and 285 Commander
decks. Eight CPU contract tests pass, including correct f32 temperature
comparison. Executed record hashes, original behavior sidecars and outcomes
must match between off/on. Nonzero stochastic likelihoods test noninterference,
not independent probability calibration. Explicit House fallbacks are counted
and excluded from shadow labels.

Only the twelve greedy shadow-on games are converted, preserving passive
observations and complete recurrent histories. Conversion must replay correctly,
retain the model catalog and completed teacher bindings, and pass Python BC-only
admission and PPO rejection. The stochastic pair remains audit-only. No model
training or automatic dataset expansion is launched by this pilot. Its
`complete-review-required` result is monitored by
`baylee-v5-shadow-pilot-watch.timer`; the completed recovery watcher was retired.

### Shadow pilot failure reviewed; corrective collection started

Wake `f6ca2d2a9c6f3ccffa612a74` was deduplicated and checked against the
exited controller. All six collection jobs completed. Every paired engine
record, executed action and outcome was exact. Greedy sidecars were also exact
for all four duel and eight Commander deals. The stochastic pair differed only
in 117 behavior log-probabilities, by at most 1.6689301e-6; its strict check
correctly left the original pilot failed.

Three same-runtime/model/seed repeats in `shadow-numerics-20261027` showed
similar variation without the teacher: two OFF repeats changed 116 and 110
likelihoods, with maxima 1.2516975e-6 and 1.5497208e-6. Another ON repeat changed
119, maximum 3.0845404e-6. All engine records and non-likelihood fields remained
exact. Independent audit:
`/home/ace/baylee-data/v5/advice/shadow-numerics-20261027/review.json`.
This identifies a repeat-computation noise floor, not its exact numerical cause
or a worst-case bound. No probability tolerance was waived and no stochastic
records were admitted to training or PPO.

Only the twelve exact greedy games were recovered. A first recovery launch
failed before starting the converter because its guard expected a repository
working directory. The next launch completed both shadow and ordinary
reference conversions. Its new differential audit then exposed two incorrect
audit assumptions, not changed game data: passive observations store legacy
Forced provenance K1 although every training path computes effective K0, and
four real teacher decisions also carry the terminal flag. These decisions must
retain their targets. The initial review script additionally pinned a mutable
workspace driver while independent mixture development changed it; the final
review instead verifies the identical archived runtime modules already recorded
by the independent reviewer. All failed attempts remain preserved; completed
conversions were reused without changing their bytes or repeating games.

The final review at
`/home/ace/baylee-data/v5/experiments/shadow-replay-reviewed-20261027/result.json`
passed full payload comparison: 150,393 frames, 40 complete seat histories,
125,106 passive frames and 5,964 verified Sharp labels. Observation/menu bytes,
value and auxiliary targets, outcomes, stored K, decision flags and terminal
clocks match the ordinary replay reference. Only the declared teacher policy
fields differ. Nonlabels have zero policy weight; likelihood fields are removed
from this BC-only dataset. Companion admission and explicit PPO rejection pass.
The ordinary reference is audit-only, never a training source. The semantics
review is `advice/shadow-replay-audit-20261027/review.json` under the v5 data root.

A bounded successor was launched as PID 500747 at
`/home/ace/baylee-data/v5/experiments/shadow-corrective-20261027` with seal
`27633e64bc4ac3a7e057c2aa15e18cbfbee928521287b4a402940929432d10e4`.
It collects 128 duel deals across 64 tables and 128 Commander deals across 32
tables. Fresh seeds are 500280000–500280063 and 500281000–500281031; all table
rotations stay in TRAIN. It retains the original broad-selected source, K2 raw
greedy policy, Sharp opponents, independent view-only Sharp advice and the
frozen 613/285 deck pools. Six controller/payload CPU tests pass. Both complete
conversions and the same full-frame comparison must pass before a dataset is
reviewed for training. No model or objective is promoted.
`baylee-v5-shadow-corrective-watch.timer` wakes this session on completion or
failure; the delivered pilot timer was retired.

The next learning comparison will mix old examples with these corrective
examples while keeping validation separate. An opt-in two-store loader preserves
full chronological seat histories, isolates colliding local game IDs, groups
rotated starts by their source table, and keeps the old-data half of the batch
identical between control and corrective arms. Positive fixed-update budgets
must remain within both arms' fully occupied lane prefixes. This is data
augmentation research; no improvement in playing strength is established yet.

The implemented mixture passes 15 focused CPU tests covering source isolation,
chronology, reset flags, whole-table partitions, all-seat coverage, resume
identity, explicit budget admission, shadow/PPO admission and existing sampler
behavior. Real metadata/index checks pass for old training, the separate 64-game
validation set and the recovered twelve-game dataset. At B8/U32 and seed31445,
the old-only halves have fully occupied prefixes of 8,077/7,664 updates; the
small recovered corrective source has only 1,178, so it cannot support a matched
1,500-update comparison. It remains an integrity fixture. Receipt:
`/home/ace/baylee-data/v5/advice/bc-mixture-20261027/metadata-check.json`.

The next bounded learning protocol is 1,500 updates per arm from the same
broad-selected checkpoint, fresh identical optimizers, unchanged default loss4,
B8/U32/K2 and seed31445. Control uses old half A + old half B; the corrective arm
uses the identical old half A + admitted new games. After collection, verify
that all four source prefixes cover the budget, freeze the tested new runtime
and the complete data identities, and retain full initial/final validation.
Compare fixed-final checkpoints against the source on fresh paired duel and
Commander seeds against Sharp; report per-head active-choice diagnostics
separately from win differences. Do not select a winner from teacher agreement
alone or automatically deploy either candidate.

### Corrective cap exclusion and matched BC control

Wake `907332fd75a5319b1da1a931` was reviewed against the stopped corrective
controller. All 256 collection jobs were recorded; two Commander games reached
the configured 10,000-decision limit. Both belong to table21, seed500281021,
rotations86/87. Their source outcome is `answer_cap`, not a draw, and their
shadow files correctly remain incomplete. The other two rotations of that
same table ended normally after 8,664/8,623 decisions, at turns179/214.
Changing actions and public state hashes do not establish a simple idle loop;
this inspection also does not rule out every possible engine problem.

The original failed result and counters are preserved. Recovery excludes all
four table21 rotations, including the two completed ones, and retains128duels
and124Commander games:252 complete games from95tables. This is explicitly a
training-data selection and can bias coverage away from very long games; it is
not a revised win-rate estimate. The cap investigation is retained at
`/home/ace/baylee-data/v5/advice/shadow-caps-20261027/review.json`.

Recovery PID503628 runs at
`/home/ace/baylee-data/v5/experiments/shadow-corrective-recovery-20261027`, input
seal `b5adc4fab5ef6f8e3a2d6e5b3d50b4551f1187ec5c80f378f47d5dd2a898a337`.
Its filtered source indices preserve each selected original line exactly.
Canonical run and record paths still resolve to the original immutable sources;
completed shadow labels are identical hardlinks within the new alias directory.
The separate selection manifest records excluded IDs, seeds and outcomes.
Six CPU tests pass, and independent preflight verified all252 source bindings.
Both full replay conversions and their payload comparison are required before
corrective training admission. No source records or counters were rewritten.

The old-only GPU control started independently as PID506784 while CPU conversion
continued. Its root is
`/home/ace/baylee-data/v5/experiments/shadow-mixed-bc-20261027`, with per-arm status
under `arms/control`; final prelaunch seal is
`a8f437cd73da0d7b2f33fc99ad89e538b43ba832bbb9df649db95cfd62b4e739`.
The frozen Python runtime is revision71390ccf5. Source weights, metadata, native
providers, old training and fixed validation inputs were verified. The model has
75,862,632 parameters. Five controller tests pass, and the control's full-lane
prefixes8077/7664 exceed the1500-update budget. It uses unchanged loss4,
B8/U32/K2, seed31445, and fresh optimizer groups. Initial full CUDA validation has
started. The augmented arm requires the independently reviewed corrective
replay, a matching old-data partition and sufficient occupied lanes; neither arm
is automatically promoted. Separate watchers monitor replay and GPU control.

At the owner's request, integration uses a separate worktree,
`/home/ace/Projects/baylee-ai-land`, so the frozen experiments keep their original
workspace inputs. Rebase with merge preservation onto origin/main49c9a4d8f
retains the training branch's previous resolutions. Compared with the previous
training tip, only the three incoming runner commits change the resulting tree.
The full and development-feature gates use a separate local PostgreSQL test
database and must pass before the fast-forward push to main.

### Corrective admission and fixed-budget comparison

Recovery completed with all 252 games converted, no rejected or unencoded
games, 2,197,633 frames and 99,883 Sharp labels. Independent review rehashed
the produced artifacts and compared every frame against the ordinary replay:
state, menus, chronology, values, auxiliary targets and unlabeled actions stay
unchanged. Only the admitted teacher action labels differ. The review is
`/home/ace/baylee-data/v5/advice/shadow-corrective-recovery-20261027/review.json`,
SHA256 `bdf86322ff9405b824e6187a75f8c2bd2b095bb67ca8ee8d62bb306e0dfbed94`.
It admits supervised BC only; these teacher labels are not PPO samples.

The mixed-data cache has 752 complete seat trajectories. Its independently
checked lane prefixes exceed the 1,500-update budget, and the shared old-data
half produces exactly the control arm's batch sequence. Four lanes per source
do not imply equal labeled-choice counts or equal gradient weight.
The reviewed admission file is `approved-corrective-review.json` under the
mixed-BC root, SHA256
`8d41be0bcc95ec537a1ac85de43bbcafde6abdcce23b5dc0871b20b578510640`.

The control completed all 1,500 updates. Full validation active-choice accuracy
is 60.30%, policy loss 0.13845 and value MSE 0.15833; the latter still exceeds
the uniform-seat baseline 0.15694. These are validation diagnostics, not a
playing-strength result. The augmented arm started as PID694072 after the
independent admission and has also reached 1,500 updates; final full validation
is running. Each arm retains its fixed-final checkpoint separately from any
checkpoint selected during training. Neither model is promoted.

Evaluation PID513026 waits for both training controllers to finish and verifies
their sealed inputs and final checkpoints before using the GPU. Its root is
`/home/ace/baylee-data/v5/experiments/shadow-mixed-bc-eval-20261027`, seal
`163fb6d084fa4a187d3641988b53859f00e28b103d83078531cd0caac38659d2`.
It checks K0/1/2/4/8 export parity and common chronological validation, then
compares source, control and augmented models on 64 duels and 32 Commander
games per model against Sharp. Identical fresh deals and seat rotations support
paired comparisons; answer caps remain unknown outcomes in their intervals.
The 40 table seeds were checked against resolved reserved records and immutable
historical seed snapshots. Deck families are reused, so this is not a family
holdout. Separate watchers cover augmented training and evaluation.

The full corrective label audit found 31,750 exact disagreements with the
executed network action (31.79%). This is disagreement, not a correctness rate:
different mana sources or ordering may be equivalent. All 252 labeled opening
hands were kept by the network; Sharp recommended 45 mulligans. Counterfactual
continuations were not played. The report is
`/home/ace/baylee-data/v5/advice/shadow-label-disagreement-20261027/REPORT.md`.

Landing checks found and fixed a missing `play`-feature dependency on
`baylee-ai` for shadow-teacher validation; no-default-feature tooling remains
independent. All development-feature checks pass. The default suite passed
23,694 tests and exposed one golden test that generated its supposedly fixed
table by running the changing House policy. A historical input fixture captured
from origin/main preserves both original golden values without changing the
public-state encoder. All eight targeted `record::seen` tests pass on the
current engine, including every historical engine hash and both original
goldens; this confirms the changed policy had changed the fixture inputs.
The 174.7-second bounded-conversion test now starts with the existing long-test
priority group instead of occupying the very end of the suite; no check is
removed or relaxed.
