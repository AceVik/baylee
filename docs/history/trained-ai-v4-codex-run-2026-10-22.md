# Playernet v4 takeover and training log — 22–23 October 2026

The owner authorized autonomous implementation and training after stopping
Argon. This is an engineering log, not a claim of achieved playing strength.

## Preserved starting point

- Original base: `30779bc920147a2a2acc713ebcb784badceced4a`.
- Argon's implementation preserved in local commit `884fe14c1` and the
  external archive under `~/baylee-data/v4/source-checkpoints/`.
- Argon's large v2 training run was killed by the Linux OOM killer at
  23:09 on 22 October. Its Phase A output directory contains no checkpoint.
- Its smaller smoke runs and all old datasets are retained for diagnosis.

## Decisions and work in progress

1. Preserve the seat-only information boundary. Never give the policy an
   engine snapshot, real shuffle seed, opponent deck list or privileged view.
2. Version the trajectory and sampler contracts separately from the feature
   layout. Old v4 trajectories are incompatible with the repaired memory clock.
3. Every seat consumes an observation before each table action when its
   authorized view changes. K=0 updates persistent H; K>=1 reads H and refines
   a temporary Z. A decision never advances H a second time.
4. Live collection advances at most one house input before observing again;
   replay uses the same boundary. Preserve passive-only sequence segments and
   pad exhausted batch lanes instead of dropping another game's tail.
5. Constrained selection visits candidates in canonical order, independent of
   weights. Deterministic scalar/greedy decisions are excluded from PPO until
   an actual stochastic action distribution exists for them.
6. Build a bounded-memory dataset cache, atomic resumable checkpoints, strict
   provenance checks, and real collector/replay/Python parity tests before RL.
7. Export separate observation and fixed-depth decision graphs. Compare model
   sizes using measured GPU training memory/throughput and CPU inference.
8. Train on the existing legal working-card corpus first while extending the
   corpus from permitted web sources. Generated/adapted decks stay labelled as
   such; held-out decks/cards must stay out of training.
9. Judge progress with paired held-out games, confidence intervals, tactical
   fixtures, critic calibration, fallback rates and thinking-depth ablations.
   Passing unit tests or copying the house policy does not establish human
   playing strength.

## Repaired contracts and evidence

- Move-only encoder split: 54 `baylee-train` library tests passed with ONNX
  enabled, before behavior changes.
- Current code: 57 Rust library tests passed with ONNX; release binaries rebuilt.
- Six existing Python contract checks, six recurrent/data/GAE regression tests,
  and two process-watchdog tests passed.
- Actual CLI resume (including two DataLoader workers): 2 updates + resume for
  2 more produced bit-identical weights to 4 continuous updates (max error 0).
- ONNX parity checked all depths and 79 real recurrent frames; maximum absolute
  output error about `5.96e-6`.
- Current r3 live collector → replay → frozen Python PPO audit: 710 stochastic
  decisions across 16 seat trajectories, max log-probability error `7.15e-7`.
  Three real optimizer updates with supervised anchor replay also passed.
- Small r3 contract games had no refused answers, fallbacks or caps. They used
  a random fixture model and establish no playing strength.

| Contract | Current value |
| --- | --- |
| Product | Playernet v4 |
| Encoder / DSL / model interface | 5 / 2 / 2 |
| Trajectory / sampler | 2 / 2 |
| Unknown card | `u32::MAX`; index 0 is the real Air Elemental |

Old r1/r2 datasets/checkpoints are incompatible and retained for diagnosis.
`models/codex-r3-contract` is a random parity fixture, never a trained player.

### Memory, actions and returns

The collector now uses both `pump_at_most(1)` and `act_slice(..., 0)`:
ordinary `Session::act` itself auto-pumps house answers and had silently skipped
intermediate observations. Replay and live play now observe the same boundaries.
The observation hash covers the whole authorized view except transport/clock/
policy bookkeeping. K=0 advances persistent H once; K≥1 refines a separate Z
and leaves H unchanged. The final frame, not last decision or chunk end, is
terminal. Passive-only chunks and all batch-lane tails are retained.

Candidate sampling follows canonical order, independent of weights. Recorded
free masks remain valid for rescoring. Greedy and deterministic scalar answers
do not claim stochastic log probabilities; scalar learning is supervised.
Named-card actions enumerate public registry faces and include candidate DSL
even without a board entity. Copied rules, faces and tokens use their own DSL.
Sparse registry indices are retained. Unknown lookup fails instead of silently
using row 0; encoder 5 fixes the Air Elemental/no-card alias. Subtypes no longer
alias modulo 256.

Before any PPO update, a frozen FP32 scan reconstructs complete trajectories,
audits recorded probabilities, and computes full returns/GAE. Nonterminal
chunks require actual bootstrap state. Initial gamma=lambda=1 avoids relying on
an unproven critic. Multiplayer targets are not shrunk toward 0.5. PPO runs
without dropout, with anchor KL, supervised replay and KL stopping. It retains
behavior-policy H at unroll starts: this is truncated recurrent PPO, not exact
recomputation under every updated policy. Small updates and fresh collection
limit the approximation; this needs monitoring.

### Durability and export

Immutable JSON trajectories now have a compressed disk-backed record store and
mmap index. Loader workers do not replicate millions of decoded frames in RAM.
Cache publication uses locks and atomic replacement. Missing shards, cursor
gaps, invalid labels and incompatible contracts fail explicitly.

`last.pt` atomically stores weights, optimizer, RNGs, batch-lane H, cursor,
configuration and dataset fingerprint. `net.pt` keeps the best validation result.
Validation reports active-action accuracy, frame-weighted Brier score, ECE and
a uniform-surviving-seat baseline. The baseline is not a uniform-team baseline.

`run_guard4.py` creates a manifest before launching an isolated process group:
argv, source revision/patch, input hashes, logs, deadline, peak proportional RAM
and termination cause. It terminates orphaned workers. Failed attempts remain
quarantined. `eval4` journals completed games and records current worker deal,
seed/decks/model/policy seed before entering the engine. The converter records
rejected games; the pipeline refuses datasets containing rejections. Engine
fixes need a reproducer, targeted regression, new source identity and replay;
never change an active dataset's rules or label a crash as a game loss.

Separate ONNX K=0/1/2/4/8 graphs prevent K=1 from executing eight masked loops.
Export checks every depth and real recurrent inputs, fails on nonfinite outputs
or parity errors, and keeps export metadata separate from training metadata.
CPU is required; CUDA optional. Phoenix XDNA and actual `acenb` latency remain
unmeasured. Evaluation latency currently covers decisions, not observation work.

## Model and data

Selected model: d=640, 8 layers, 10 heads, FFN=2560, H=`9×640`, about 54.28M
parameters. Supervised training uses BF16, SDPA, activation checkpointing and
AdamW; export uses portable attention operations. This fits the 4070 Ti without
requiring experimental MoE, attention or optimizer kernels.

Paths here are relative to `/home/ace/baylee-data/v4/`.
`bench-codex-r1.json` compares 384/6, 512/8 and 640/8. The real 640/8 benchmark
in `bench-codex-r2-real.json` used B=8/U=32/K=1: 2,048 frames in 10.8 s, about
3.3 GB peak allocated VRAM. The actual K=1/2/4/8 warmup peaked at 5.18 GB.
Neither measurement predicts worst-case huge-board memory or 7840U latency.

`datasets/codex-r3-main`: 704 games; 3,764,981 train and 463,322 validation steps;
9,344 vocabulary rows including sentinel; zero unencoded decisions/rejections.
Train: 400 duels, 120 teams, 120 four-player FFA from the preserved house runs.
Validation: 48 new duels and 16 Commander games from held-out validation decks.
The warmup deck sets were checked disjoint from these validation decks.
Test decks/cards are not used for checkpoint selection. Source/validation run
paths and coverage counts are in dataset metadata.

`models/codex-r3-warmup`: 300 updates on 24 games, 508.8 s, d=640, K curriculum.
Validation: overall top-1 85.65%, **active top-1 only 26.41%** (1,715 choices),
Brier 0.23325 versus uniform-surviving-seat baseline 0.19626, ECE 0.13777.
This is a warm start, not a strong player; the critic is worse than its simple
baseline and must not be trusted to rank plans yet. The separate r3 PPO smoke
only verifies training execution.

## Training and evaluation plan

1. Initialize the main 54M run from the warmup. B=8/U=32, lr=1.5e-4,
   K=1/2/4/8, atomic checkpoint every 100 updates, validation every 1,000.
   Budget the first main experiment for five hours, not as a promised time to
   reach expert strength. Retain resumption state and best validation weights.
2. Export with parity checks. Play held-out paired duels and Commander against
   Sharp, using Steady on the same table as baseline. Report coverage, caps,
   refusals and confidence intervals clustered by table: seat rotations are
   correlated, not independent trials.
3. Collect fresh stochastic duels/FFA/teams; audit behavior probabilities;
   run a small anchored PPO update and repeat identical validation. Keep both
   checkpoints. Never select from training loss alone or overwrite the best
   player silently. `pipeline4.py` implements stages 1–3 with preserved jobs;
   `run_v4_pipeline.sh` delegates to it and no longer deletes directories.
4. Expand 3/6/8-seat, mechanics and deck-family coverage and tactical fixtures.
   Fix/replay engine failures before admitting their data. Keep real public
   decks distinct from generated/adapted variants and preserve held-out splits.
   Before a longer scaling campaign, replace the lossy card-summary bottleneck
   described below with a versioned structural representation. Keep this first
   run as an instrumented baseline, not the final universal-player architecture.
5. Compare K=1/2/4/8 at equal wall time and ablate/shuffle/reset memory. Longer
   iteration is useful only if decisions improve. Add action-conditioned plan
   quality, counterfactual tests and calibrated stopping before spending
   15–45 s on several candidate plans. An unvalidated state critic cannot do this.
6. Implement the proposal's PlayerView-only hybrid System-2 design: public legal
   root actions, explicit beliefs, unseen worlds sampled consistently with the
   observer's knowledge, action-conditioned learned rollouts, and opponents
   restricted to their own information. Never clone the true hidden snapshot.
   Legality/calibration/adversarial anti-cheat tests gate search use.
7. Require disjoint tactical suites and experienced-human evaluations after
   beating diverse house opponents. Parameter count and teacher agreement do
   not prove that threshold. Measure `acenb` CPU before distillation/quantization.

## Web decks and research

The new importer uses only the published
[17Lands bulk datasets](https://www.17lands.com/public_datasets), records CC BY 4.0
attribution, strips player information, and rejects a deck if any card is
unsupported. Exact/near duplicates and draft-family splits are checked;
held-out cards stay in test. The first scan yielded **zero acceptable decks**:
most sets barely overlap the working pool; sampled Cube lists had unsupported
cards. No basics were substituted and rejected lists are not counted as imports.
See `decks/17lands-r3/import-report.json`.

[TRM](https://arxiv.org/abs/2510.04871) and
[Probabilistic TRM](https://arxiv.org/abs/2605.19943) motivate evaluating shared
recursive computation and multiple refinement trajectories. The
[TRM analysis](https://arxiv.org/abs/2512.11847) motivates depth ablations and
generalization controls. These puzzle results do not establish MTG planning;
noise plus an unvalidated critic is not a proven self-correction mechanism.

## Remaining work

The implementation does not yet deliver a production always-observing seat,
a proven 15–45 s planner, learned halting, Expert 2, all multiplayer evaluations,
or measured experienced-human strength. Caps (E=1024, edges=8192, C/P=65535)
are finite, not a losslessness proof. Ability summaries still cap at eight;
chosen-name state features, arrangements, huge boards and explicit overflow
diagnostics need further work. Next decisions follow gameplay evidence.

## Follow-up findings during the main run

- Main experiment started at about 00:25 CEST on 23 October:
  `experiments/codex-r3-main-54m`, source `612c5aa50`. The detached pipeline
  writes `pipeline.log`, per-stage `jobs/*/{job.json,stdout.log,stderr.log}`,
  atomic BC checkpoints and subsequent evaluation/rollout/PPO outputs.
- The 300-step warmup lost all eight pilot duels against Sharp; its same-table
  Steady baseline scored 0.5 utility. No caps/fallbacks/refusals. CPU decision
  latency on this 3900X was p50=34.4 ms, p95=64.6 ms (observation work excluded).
  Four paired tables are a diagnostic pilot, not a precise strength estimate.
- Extra house collection: 24 three-seat games finished, 18 six-seat games had
  one answer cap, all 16 eight-seat games finished (three draws). Every run used
  fair scouting and train decks. The six-seat cap was not accepted as data.
- The capped game (`codex-r3-six-seat`, game 14, seed 891015) repeated
  CastSpell(Stir the Grave)/ChooseNumber(1) on turn 118. Doubling the cap to
  36,000 answers did not advance the turn. A view-only house-policy correction
  rejects a positive-X graveyard target if no eligible public target fits the
  payable X. Its regression fails on the old code and passes with the fix;
  all 274 house-AI unit tests pass. The identical original game now finishes
  on turn 124 after 8,913 answers. No engine rule or card script was changed.
- Authoritative deck parsing found three invalid Cryptic Spires lists in
  Argon's manifest (missing circled colors), previously skipped silently by
  collectors. `deck_catalog4 --manifest` reports them explicitly: 849 of 852
  parse and pass the working-card check. Keep the original experiment manifest
  immutable; filter/revalidate future manifests instead of pretending all 852
  loaded successfully.
- New public source: [fbettega's MTGO deck cache](https://github.com/fbettega/MTG_decklistcache),
  GPL-3.0, pinned at `6c8c09979f6b388a7aa5a6e13c685d1c96ffc3da`; archive SHA-256
  `9ff22e8b81cb27182dbf8740f312a029de0660368f46af86e9db209e8eca0dea`.
  The importer checked 314,031 deck entries in 12,905 tournaments, strips player
  names/results, records exact source row/revision and licence, and compares
  exact multiset Jaccard against all 849 valid existing decks and accepted lists.
  One empty source JSON is reported, not silently treated as a deck.
- Whole-deck/sideboard filtering initially admitted only all-basic placeholder
  submissions. They were never trained on and are marked quarantined; the new
  quality filter rejects them. Explicit **first-game mainboard projection**
  yields 91 fully supported, unchanged mainboards, with the entire sideboard
  omitted and this transformation labelled: `decks/mtgo-r3-mainboards`.
  All 91 pass the real Rust parser and working-card check. These are primarily
  88 Yawgmoth lists plus three Psychic Frog lists, **not 91 archetypes**.
  Existing card holdouts leave seven train, one validation and 83 test lists.
  The current card pool and arbitrary 5% card holdout constrain real-deck
  diversity strongly; do not relabel test cards just to inflate training counts.

## Overnight continuation and first full validation

At step 1,000 the main model completed all 463,322 validation frames:
active single-choice accuracy 44.17% on 17,793 choices, overall top-1 86.28%,
Brier 0.25081 versus uniform-surviving-seat 0.19971, ECE 0.21449.
Mean predicted utility was 0.10887 versus observed 0.30534. This is a poorly
calibrated critic, not evidence for trusting a plan-ranking head. This validation
set differs from the warmup's, so their percentages are not a controlled before/
after comparison. Training resumed successfully after validation; peak training
allocation subsequently reached 5.45 GB. Full validation used about 10.1 GB
device memory including allocator reservations. No CUDA OOM occurred.

Three detached processes implement the continuation without needing approval:

- `experiments/codex-r3-main-54m`: five-hour BC budget, export/parity, paired
  evaluation, fresh stochastic collection, audited bounded PPO, re-evaluation.
  Full validation consumes part of the BC budget; five hours is not five hours
  of optimizer work. Best weights and resumable latest weights remain separate.
- `experiments/codex-r3-broad-corpus`: the original sources plus 3/6/8-seat
  games, 48 real-web-mainboard training games and new held-out 3/6/8-seat games.
  All 48 web games completed. Three new held-out multiplayer games reached their
  answer caps at turns 265/269/275; the examined final states advanced rather
  than repeating the Stir the Grave loop. Caps remain findings, not losses or
  accepted training examples. Excluding capped games can bias coverage and must
  remain visible in `admission.json`.
- `experiments/codex-r3-broad-followup`: waits for the first pipeline to release
  the GPU and for a successfully validated broad corpus, then runs one bounded
  additional BC hour from the first BC checkpoint, exports and evaluates it.
  It keeps the previous corpus in the mixture. It does not silently replace
  BC with the exploratory PPO checkpoint or deploy either model.

The broad conversion's first attempt was terminated by its watchdog after
906.5 s at 51.71 GB proportional RAM, exceeding its 48 GiB limit. The process
group exited, the incomplete dataset remains quarantined, and the GPU training
job continued. Two avoidable copies were removed: replay finalization now moves
frames backwards while computing next-seat-life labels, and serialization buffers
one frame instead of a whole game's JSON. The old/new binaries produce exactly
the same 21,847 serialized frames from six live duel/team records and bit-identical
DSL tables (`checks/conversion-memory-parity.json`). Small-case peak PSS fell
from 312 MB to 208 MB. The retry uses two workers, a new job directory
`jobs/convert-retry-1`, a new `dataset-convert-retry-1` and a two-hour deadline.
Source game records and the failed attempt are preserved. The followup consumes
only the dataset published in the successful `result.json`.

The largest completed eight-seat validation record by compressed input size
(game `eight-val-0000001`, 21,935 answers, turn 219) then converted successfully
in 554.9 s with 19.31 GB peak PSS and no rejected/unencoded decisions. This is
why conversion concurrency is two, not four, on the 64 GB training host.
Subsequent converter builds also journal each worker's in-flight shard/member
before replay and report completed-game counts. The already-started broad retry
uses the preceding, memory-corrected executable; its executable hash is saved.

`tools/trainer/report_experiment4.py` observes both training experiments for up
to 16 hours. Current local status is
`experiments/codex-r3-status/progress.md` (also `progress.json`), refreshed every
30 seconds. It also adds paired uncertainty reports to completed evaluations,
including the already-running pipeline that predates automatic report creation.
Seat rotations remain one statistical cluster. Capped outcomes are unknown in
`[0,1]`: reported intervals include their best/worst possible completions; an
imputed 0.5 is not presented as an observed draw. The watcher never retries or
promotes a model. Per-stage `job.json`, stdout/stderr and source hashes are the
authoritative failure evidence. On failure, inspect those and resume into a new
attempt; do not delete a failed directory or restart a second GPU job blindly.

## Representation audit: next architecture priority

`audit_representation4.py` compared all 9,265 working-pool front faces in the
converted vocabulary: only 8,217 unique 160-float summaries, 553 collision
groups containing 1,601 cards. Adding all eight stored ability rows still leaves
443 groups containing 1,198 cards. Some groups are legitimate functional
reprints, but others include materially different rules: Mana Leak / Negate /
Remove Soul, different fetch lands, and different utility lands. See
`checks/representation-r3.json` for the full groups and table checksum.

The actor does not consume all stored ability rows. Activation candidates
currently receive only the effect-channel slice of their selected ability;
named-card choices receive the card summary without an identity embedding or
entity pointer. Exactly colliding named-card summaries are indistinguishable
to that head. Chosen-name object state also records presence without the actual
chosen identity. Legal offers can distinguish some situations indirectly, but
cannot recover all missing card semantics or support reliable hypothetical plans.

Therefore the next contract must combine a stable public identity/reference
channel with an actual hierarchical typed AST encoder: ability kind, costs,
timing/zone, nested filter logic (including negation), target scope/controller,
numeric and X-dependent predicates, conditions, effect sequence/branches,
replacement rules, dependencies and selected-ability links. Identity alone
cannot generalize to unseen cards; pooled effect counts alone cannot distinguish
these rules. Preserve every ability, or report explicit overflow and route it
through a defined fallback. Add counterexamples that distinguish the above
cards and hidden-information invariance tests. Version/reconvert/export this
separately; never mutate encoder 5 under the active experiment. This is a higher
priority than increasing depth, adding MoE or trusting a larger value head.

Another label limitation to migrate explicitly: `opp_min_final_life_norm`
currently takes the minimum over all other seats, including teammates. Its
name overstates opponent-only semantics in team games. Correct the team mask
with a trajectory-version migration and regression before relying on this
auxiliary target; do not silently change labels under a resumable checkpoint.

## Verification and deployment limits at handoff

The latest replay change passes all 57 trainer library tests. The relevant
trainer library plus `selfplay`, `convert4`, `eval4` and `deck_catalog4` pass
Clippy with `-D warnings`. Six existing Python contract checks, six recurrent
regressions, two watchdog tests, two deck-import tests and two statistical
evaluation tests pass. The large DSL and evaluation source files are now split
into directory modules. Whole-repository gates have not been run; this work is
neither pushed nor landed on `main`.

The overnight binaries use ONNX CPU execution. A previous smoke command set
`BAYLEE_EP=cuda` on a binary lacking `onnx-cuda`; that was CPU execution and is
not evidence for CUDA EP support. Phoenix NPU execution remains untested.
Release binaries use panic-abort: the external watchdog and saved deal identity
survive, but in-process `catch_unwind` cannot salvage an aborting game. An unwind
collector build or per-game isolation is needed for stronger crash containment.
The implemented loop fix changed the house policy only. The pipeline does not
automatically apply engine rules patches or train through an unverified fix.

## Morning follow-up: matched v3 baseline and GPU inference (23 October)

The initial experiment completed. PPO Commander won 3/32 deals against Sharp
(no answer caps, one refused action, 56 house fallbacks); this is not evidence
of strong play. Future evaluator summaries record refusals per deal and log
their seed, seat, pending kind and engine error to make diagnosis possible.

`eval4 --legacy-v3 --thinking-steps 1` now evaluates archived v3 exports using
the same current engine, decks, rotations, scouting rules and house opponents.
It refuses stochastic rollout recording; v3 actions cannot masquerade as v4
behavior labels. Only the deciding seat's own deck enters its network input.
On the same 32 validation duels (16 independent tables), BC v4 won 4, PPO v4
won 6, archived `a-dyn` v3 won 7 and archived `rl23` v3 won 11; Steady won 15.
The paired v4 PPO minus rl23 utility difference is -0.15625, table-bootstrap
95% interval [-0.3125, 0.03125]. This small matched-opponent test does not
establish statistical superiority, and is not a head-to-head match. Current
v4 has not demonstrated an improvement over v3.

The PPO KL stop was a trust-region guard, not a crash. Replaying the exact
first eleven minibatches gives active PPO decision counts
`5,2,2,4,0,1,5,0,1,5,5`: the ten applied updates contained only 25 such
decisions. The logged count of 30 also includes the five in the rejected
eleventh batch. Two updates had no policy examples but still changed shared
weights through value/anchor losses. KL 0.0583 exceeded the configured 0.02;
the stop retains earlier updates, without rollback. The next PPO revision
needs enough genuine decisions per update, separate value/anchor weighting,
and a stable KL check. Do not solve this by disabling the guard. Keep passive
observations for recurrent memory even when a chunk contributes no policy loss.

`bench_inference4.py` compares CPU ONNX with CUDA PyTorch on actual frames,
including transfers. Repeated-frame batches measure batching potential, not
whole-game speed. The decisive measurement uses the **same CUDA-capable Rust
binary** with CPU/CUDA provider selection. CUDA really runs on the 4070 Ti:
32 complete duels plus their baseline games take 32.6 seconds with four
workers (decision p50 3.6 ms), or 28.7 seconds with eight workers (p50 6.4 ms).
Per-deal outcomes agree with the original CPU evaluation. The eight-worker
CUDA run also has identical coverage counts; the four-worker CUDA run has
6,459 neural answers versus 6,517 in the other two. Thus these measurements
do not establish identical action traces across providers or worker counts.
Four stochastic CUDA rollout games were replayed and audited: 565 decisions,
maximum log-probability discrepancy 5.78e-6. See `checks/full-game-*-job`,
`checks/cuda-eight-games-job` and `checks/cuda-rollout-audit-job`.

CUDA inference explicitly disables TF32 to agree with the FP32 policy audit.
`BAYLEE_EP=cuda` now fails on a CPU-only v4 build instead of silently using CPU.
Use `--profile selfplay --features onnx-cuda --bin eval4` and the CUDA 13/cuDNN 9
library directories. `pipeline4.py` accepts an evaluation binary and worker
count; its job manifest records non-secret provider/library settings. Evaluation
and collection stages can therefore use the GPU after training releases it.

The broad follow-up keeps its one-hour supervised training budget and validates
at the end. Repeated full validation every 1,000 updates would be expensive on
the new 3.07M-frame validation split. The stage deadline now includes a scaled
allowance for that final validation. Training rechecks its wall budget after
periodic validation, preventing a redundant update and full validation pass.
Twelve Python regression tests, six legacy contract checks and strict Clippy
for the CUDA trainer library/evaluator pass. Full repository gates remain
outstanding; nothing was pushed or promoted.

The completed CPU control took 350.5 seconds for the same 32 duels, versus
28.7 seconds for eight CUDA workers: 12.2x the throughput of **four** CPU
workers, not a measurement against all twelve physical cores. Both providers
used the same executable. Commander PPO replay also completed on CUDA:
155.0 seconds with eight workers versus the original CPU run's 3,204.6 seconds
with four. Its outcomes matched, including the refusal at deal 6, seed 870002,
seat 2, `ChooseNumber`. The diagnostic evaluator can replay an individual deal
using `--deal-offset 6 --deals 1` while preserving deck/seat/seed assignment,
and logs the offered pending question and attempted action on refusal.

The deferred broad-data queue was resumed under commit `936682165` with eight
GPU evaluation workers. Its training process first builds the immutable data
cache; a separate four-worker validation-cache preparation overlaps that work.
The first preparation command accidentally resolved the virtualenv Python
symlink to the system interpreter and failed to import numpy; it is preserved
in `checks/prewarm-broad-val-job`. The corrected attempt keeps the virtualenv
path and has a separate `checks/prewarm-broad-val-retry-job` manifest. No data
was removed or relabelled. The final CPU-feature build also passes strict
Clippy and all 57 trainer library tests.

### The Commander refusal was a player bug

The single-deal replay preserved the original deck/seed/seat assignment and
exposed the attempted action: `PassPriority` for `ChooseNumber { min: 0,
max: 4294967295, reason: X }`. The engine correctly rejected it. v4's fallback
used `timeout_answer(...).unwrap_or(PassPriority)`, but clock defaults deliberately
omit number, naming and mandatory-choice questions. A missing clock answer does
not authorize passing. The fallback now keeps valid offered/default actions and
otherwise uses `HeuristicAgent::act` with the player's filtered view and public
teams, without scouting or an engine reference. A failed forced decoder routed
through that fallback is labelled House, not Forced. Old trajectory data and
the running supervised job are unchanged.

The new regression reproduces the old invalid pass for unbounded X, and also
checks a strictly positive number range. It fails before the fix and all 58
trainer library tests pass afterwards. CUDA library/evaluator Clippy passes.
No engine rule was changed for this finding. The broader training job has
finished both data caches and is performing GPU optimizer updates; its later
evaluations use the corrected runtime.

The full failing deal was replayed after the fix using the same PPO checkpoint,
decks, seed and seat: zero refusals, zero stalls/caps, still a loss. Its original
11 fallback decisions are still explicitly counted. Both before/after records
and logs are preserved in `checks/commander-number-{reproducer,fixed}-*`.
The change repairs a legal-action failure; it does not claim improved strength.

### CPU robustness audit while the GPU trains

`checks/parallel-engine-audit` completed 256 games across 72 generated working
decks (constructed/highlander/Commander) on eight CPU workers in 8.7 seconds.
It used the optimized unwind profile, seed 970001, deck seed 971001, chaos 0.2,
and run name `codex-r3-resource-audit-20261023`. No panic, replay divergence,
refusal-induced state change or cap was reported. Two random attacks violated
an explicit `MustAttack` constraint and are correctly separate from defects.

Two **unfixed engine offer-contract defects** remain, with records and seeds:

- Game 127 / seed 970128 / decision 546: a `ChooseCards` Band offer permits
  two non-banding members by its count bounds and has `total: None`.
  `answer_band` then rejects the selection. The bound must be machine-readable;
  simply weakening the engine rule would be wrong. Mixed banding and
  "bands with other" groups need regression coverage as well.
- Game 208 / seed 970209 / decision 301: three offered attackers pass
  `Pending::answer_fault`, but the engine rejects all three with
  `IllegalAction("too many attackers declared")`. `AttackRules::max_attackers`
  is checked by apply but missing from `Pending::ChooseAttackers`.
  `probe_refusal` verified all 301 recorded inputs and reproduced that exact
  error, with the rejected action leaving the state unchanged. Its report is
  `checks/attack-208-probe.json`.

The new offline `probe_refusal` example accepts an uncompressed record, a JSON
action file and a seat, verifies the record hashes, and compares the offered
constraint with the actual engine result. It is omniscient diagnostic tooling,
never an actor input. The example passes strict Clippy. The next engine repair
must update the offered constraints, all consumers and appropriate protocol/
training contracts together; preserve this experiment and its old records as
versioned evidence. The robustness findings are separate from training data.
