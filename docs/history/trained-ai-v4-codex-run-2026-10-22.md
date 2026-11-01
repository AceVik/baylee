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

## Broad follow-up completed; next decision point

The broad-data experiment finished 2,683 optimizer updates, final validation,
export parity checks and both GPU evaluations. The supervised stage took
9,018 seconds including cache preparation; training plus final validation
reported 8,347 seconds. On the same 32 paired duel setups, the new BC model
won 7 (initial BC 4, initial PPO 6, archived rl23 v3 11). The new Commander
run won 4/32, with one answer-capped game; do not count that cap as an observed
draw. Both new evaluations reported zero refused actions and zero fallbacks.
The small samples do not establish a strength improvement.

On the expanded validation split, active-choice accuracy is 0.4362. The value
head's Brier score is 0.1578 versus 0.1241 for the uniform-surviving-seat
baseline (lower is better). Its prediction mean is 0.1689 versus outcome mean
0.1565; acceptable aggregate means do not establish good position ranking or
calibration. These frame-weighted metrics are on a changed validation mix and
must not be compared directly with the initial corpus's scores. Extra search
should not be trusted solely on this value head.

`experiments/codex-r4-comparison` now compares broad BC, initial BC, initial
PPO and rl23 v3 on 128 duels and 64 Commander deals each, validation decks,
fresh first seed 980001, the same Sharp opponents and paired Steady baseline.
The binary, helper scripts and deck manifest are frozen in that directory;
each job hashes model inputs, preserves failures and writes uncertainty
reports. One GPU lane runs v4, one CPU lane runs v3; each uses eight game
workers. There is no automatic model promotion. This is a matched-opponent
comparison, not a direct head-to-head or a human-skill test.

The next implementation order is: repair the two offered-choice contracts
with regressions and compatible consumers; replace lossy card summaries with
versioned structural/identity features; repair PPO batching and validate the
critic; then train controlled tactical/recurrent pilots and measure the gain
from additional thinking steps. Start with 128–256 genuine policy decisions
per optimizer update using accumulation, while preserving passive observations
and per-seat memory. Longer RL runs require frozen-policy parity and stable
KL checks first. Current hardware is sufficient for these pilots. Experienced
player reviews of withheld tactical positions are useful later, not a blocker
or a substitute for deterministic correctness tests.

## Completed matched comparison and bounded continuation (23 October, midday)

`codex-r4-comparison` finished both lanes. Every row uses the same frozen engine,
manifest, Sharp opponents and seeds; Steady won 58/128 duels and 11/64 Commander
deals (plus one four-way draw). These are matched opponent tests, not head-to-head:

| Checkpoint | Duel wins / 128 | Commander wins / 64 | House fallbacks, duel / Commander |
| --- | --- | --- | --- |
| Archived v3 rl23 | 41 | 4 | 2313 / 337 |
| Initial v4 BC | 24 | 8 | 43 / 42 |
| Initial v4 PPO | 22 | 3 | 40 / 47 |
| Broad-data v4 BC, +1h | 19 | 5 | 2 / 13 |

All have zero refused actions. One v3 duel and one Commander deal per checkpoint
hit the answer cap; those outcomes are unknown, not draws. Commander still has
only 16 independent tables. The v3 adapter delegates substantially more choices
to the house AI; the table compares complete seat systems, not isolated network
quality. Broader data reduced fallbacks but has not established stronger play.
No model was promoted. Machine-readable results: `codex-r4-comparison/summary.json`
and each evaluation's table-cluster uncertainty report.

While implementing the requested low-priority workstation dashboard, launched
`codex-r4-broad-continuation`: a bounded additional 10800-second BC optimization
budget, resuming the broad run's optimizer, RNG, recurrent state and exact sampler
position (epoch 1, batch/step 2683). Original checkpoints remain unchanged; copies
live in the new experiment. This tests the longer-training explanation, not a
claim that the architecture's known blind spots are solved. Encoder/data/config
stay unchanged. Training scripts are frozen under the experiment's `runtime/`;
each guarded job hashes inputs. Final full validation, ONNX equivalence checks,
128 matched duels and 64 Commander deals follow automatically with the frozen
comparison binary. Final validation is additional time beyond the optimization
budget. No PPO and no automatic promotion. A launcher initially resolved the
venv interpreter symlink and exited before starting any job (missing NumPy); its
log is retained, and launch through the actual venv path resumed successfully.

The user clarified that useful contributions mean information/data, not buying
hardware. Expert-annotated held-out tactical positions remain useful, but no user
input is required for this continuation or dashboard work.


## Training throughput investigation (23 October, afternoon)

The user authorized repeated measured improvements while retaining training as
priority. The original recurrent trainer saturated one CPU thread, not the data
loader or PCIe link. A real B=8, U=32 batch occupied roughly 48–51 MiB; transfer
was about 2.2 ms of a 1344 ms update. Different decks use the same network, so
there is no per-deck weight swap. Loading the corpus into VRAM or replacing the
loader with Rust would not address the measured bottleneck.

Implemented an exact CPU-derived execution plan, device-side loss/metric
reductions, and grouping of independent spatial encodings across eight frames.
Only the independent batch axis is merged: memory evolves in original time
order, every observation remains visible, and batch lanes, unroll, loss weights,
optimizer state, RNG and data cursor remain intact on resume. CPU tests check
losses, absent gradients, all action heads, noncontiguous masks, future-frame
isolation, recurrent outputs and gradients. CUDA tests check AdamW scalar step
placement and exact resumed optimizer updates. Ordinary AdamW uses CPU counters;
fused/capturable variants require device counters. Preserve `fused=None` when
not requested so PyTorch can choose its foreach implementation.

All artifacts are under `checks/training-throughput-20261023/`. Full-update
comparisons use identical copied checkpoints and consecutive real batches:

| Comparison | Original updates/s | Optimized updates/s | Decision |
| --- | --- | --- | --- |
| `timing-repeat.json`, step 4000 | 0.744 | 1.334, chunk 8 | deploy chunk 8 |
| `regional-timing/benchmark.json`, step 4500 | 0.782 | 1.550 chunk 8; 1.644 compiled blocks | deploy regional compilation |
| `graphs-timing/benchmark.json`, step 5200 | 0.778 | 2.022 compiled; 1.765 padded | retain unpadded compiled blocks |

These are different batches, not directly comparable absolute rates. The live
regional run reached about 1.47 updates/s before the next handoff. Grouping all
32 frames was slower than grouping 8 and used more VRAM. Fused AdamW did not
produce a useful further gain. Reference profiler counts fell from 64,216 to
25,896 CUDA kernel launches and from 1,015 to four stream synchronizations per
update; instrumentation perturbed subsequent timings, so throughput claims use
separate runs without the profiler. Grouped BF16 arithmetic differs slightly:
full-update gradient relative L2 differences were roughly 0.36–0.44%; FP32
causality/loss/gradient tests and six ONNX dynamic-K contract checks passed.
The comparison helper now deep-copies optimizer input state because CPU scalar
step tensors otherwise alias between variants. Earlier first-step gradient
comparisons and throughput remain informative, but later optimizer counters in
those earlier benchmarks were not strictly identical.

CUDA Graphs are experimental and **not enabled in production**. A small
four-frame probe compiled, but the full recurrent padded variant spent over
180 seconds without completing its first update. It was stopped, preserving
partial results and the reason. Padding alone was already slower. The unpadded
CUDA Graphs variant was not reached; no performance claim is made for it. Dynamic
shapes, checkpointed backward and long recurrent graphs need a separate bounded
experiment before reconsideration. Current production keeps all dynamic entities
without padding or truncation. Unit tests establish that the experimental
padding preserves visible tokens and masks in FP32.

Controlled atomic-checkpoint handoffs went through
`codex-r4-efficient-bc` (step 4000), `codex-r4-regional-bc` (4500), then
`codex-r4-regional-continuation` (5200, 7800 seconds remaining optimization
budget). SIGTERM produces a preserved guard quarantine on old jobs; `handoff.json`
explains the intentional interruption. It is not an engine/numerical failure.
`experiments/codex-r4-active.json` identifies the current continuation. Runtime
Python files **and both packages** are frozen per experiment. Full validation,
ONNX equivalence and the same matched 128-duel/64-Commander evaluation follow
without automatic promotion. The validator now uses device reductions and
float64 totals with the same metrics and denominators as the reference.
More updates per hour do not establish improved playing strength.


### Further measured reductions in repeated work

Two more optimizations passed CPU gradient/memory/loss tests for every thinking
budget and policy head, plus all six Python/ONNX contract checks:

- `--cache-thinking-kv`: the thinking block's board normalization and attention
  K/V projections do not depend on the thinking step. Reuse them within that
  forward call while retaining autograd. No cross-observation or persistent cache.
  `kv-cache-timing/benchmark.json` (step 5800) measured 0.763 original,
  1.635 grouped/compiled, and 1.712 updates/s with reuse (+4.7%). Peak allocated
  memory fell from 4.389 to 3.939 GiB. Gradient relative L2 difference from the
  original was 0.271%, slightly below the uncached grouped variant's 0.278%.
- `--prune-unused-policy-heads`: skip only policy branches which do not contribute
  to that frame's loss. Value, auxiliary targets and passive memory updates
  remain present. Default inference/export still computes every output.
  `head-selection-timing/benchmark.json` (step 6100) measured 0.776 original,
  1.457 with grouped/compiled/KV reuse and 1.527 updates/s with unused heads
  omitted (+4.8%). Chunk 16 measured 1.522 and reserved 7.588 GiB instead of
  5.877 GiB, so retain chunk 8. FP32 whole-unroll losses, memory, parameter
  gradients and None-gradient patterns agree; BF16 full-update gradient relative
  L2 difference from the original was 0.329%.

A separate 120-second live sample of the earlier regional run averaged 52.8%
GPU utilization (3–100%), 138.5 W GPU power and 6.5% total CPU utilization.
This is not a paired energy benchmark. CPU dispatch and changing observation/
decision work still produce peaks; a perfectly flat utilization chart is not
an efficiency requirement. Throughput, retained supervision, memory bounds and
correct outputs are the adoption criteria. Reserved allocator memory is not
identical to live tensor memory; avoid filling the displayed VRAM headroom with
an unneeded copy of the dataset.

`codex-r4-projection-cache-bc` resumed step 5800, then
`codex-r4-throughput-bc` resumed step 6100 with 6900 seconds optimization budget
remaining, chunk 8, regional compilation, K/V reuse and unused-head omission.
Each handoff preserved its atomic checkpoint and frozen runtime; standalone
resume supervisors selected only passing variants and restarted training as soon
as the bounded benchmark finished. Old job guard failures are annotated as
intentional handoffs. Full validation and matched games remain pending.


### Rejected readout compilation and longer validation audit

`readouts-timing/benchmark.json` at step 6500 compared the old execution
(0.777 updates/s), selected production path (1.481), and additionally compiled
recurrent/readout blocks (1.249). The extra mode hit TorchDynamo's recompilation
limit for differently shaped Sequential modules and was rejected. Its first
full-update gradient relative L2 difference was 6.67% (max absolute 0.00832).
The production path on this low-loss batch differed by 4.35% (max 0.000489):
relative gradient differences are not a fixed constant across positions, and
cannot alone establish optimizer/playing equivalence. This does not overwrite
the smaller differences measured on previous batches. Extra readout compilation
remains opt-in experimental code and is **off** in the actual continuation.

The 16-batch validation probe with experimental readout compilation showed the
same choice accuracy but an amplified value-prediction mean difference of 0.0041.
A longer, separate audit therefore checked the actual production configuration
without those extra compiled modules: `production-validation.json`, same step
6500 weights, 64 held-out trajectory batches, 16,216 valid frames and 1,179 active
choices. Reference and optimized execution had identical total/active choice
accuracy and frame counts. Policy loss delta was -0.0000316, value MSE delta
+0.0000123, auxiliary MSE delta -0.00000241, prediction-mean delta +0.000104 and
ECE delta -0.000167. Timing (76.8 vs 18.7 seconds) was gathered alongside live
training, so it is **not** an uncontended validation speed benchmark. This prefix
checks numerical execution parity; it is not the full corpus or a strength test.

`codex-r4-compiled-bc` resumed step 6500 with 6600 seconds of optimization budget.
Despite the directory name, `selection.json` explicitly records
`compile_readouts: false`. Enabled: chunk 8, ordinary regional transformer/
thinking compilation, local K/V reuse and unused-policy-head omission. Disabled:
CUDA Graphs, entity padding, fused AdamW and extra recurrent/readout compilation.
The active pointer identifies this run. Full validation, export parity and
matched 128-duel/64-Commander tests follow automatically; no automatic promotion.


Final 120-second live observation of the selected production path, with no other
GPU job running: GPU utilization averaged 66.3% (median 81.5%, range 0–100%),
GPU power 157.2 W, total CPU 6.4%, RAM 13.2% and NVML VRAM 72.6%. Observed log
transitions from step 6875 to 7025 took 104.0 seconds, about 1.44 updates/s on
this portion of the corpus, including ordinary checkpoints. One-second sampling
and 25-step logs bound precision; this is not a paired energy benchmark.
Artifacts: `production-resource-samples.json` and `production-resource-summary.json`.
The process had passed step 7050 without a reported error when checked. All
benchmark/probe/sampling processes had exited; training and the requested
workstation monitor remained running. The final unittest discovery ran 21 tests:
20 passed, the CUDA placement test was skipped in that CPU-only run (it passed
separately earlier). The six explicit model/ONNX contract checks also passed.


## Causal memory pass and grouped readouts (23 October, later afternoon)

The user asked whether removing the remaining GPU utilization valleys could
again double throughput. NVML reports time with at least one running kernel,
not the fraction of peak FLOPs achieved. At the previously sampled 66% busy
time, eliminating all idle time alone would suggest about 1/0.66 = 1.5x under
unchanged active-kernel speed; this is a rough illustration, not a performance
prediction or a bound on optimizations that also improve the kernels.

Implemented an additional opt-in `--decision-chunk`: compute the recurrent
memory pass in original chronological order, then group independent decision/
value readouts using each observation's own state. The recurrent autograd chain
remains attached; grouping is across batch lanes, never a time-attention axis.
The optimization requires dropout=0, respects thinking budgets and candidate/
pair size buckets, and bounds the large scorer input shapes to avoid inflating
rare NameCard decisions. It does not truncate any frame or choice. The ordinary
forward/export path remains available with the same checkpoint keys and ONNX
inputs. Tests verify every policy head, K=1/2/4/8 with passive lanes, invalid-row
memory preservation, losses, None-gradient patterns, all parameter gradients,
validation resets/metrics, and immunity of earlier decisions to future input
changes. Discovery: 24 tests, 23 passed and one CUDA-only optimizer test skipped;
all six explicit model/ONNX contract checks passed.

`checks/training-throughput-20261023/deferred-timing/benchmark.json` captures the
same step 13500 checkpoint, B=8/U=32, optimizer/RNG/memory/cursor and 24 measured
updates per variant. Here gradients are compared with the existing optimized
production path, rather than the original slow implementation:

| Readout execution | Updates/s | Peak allocated GiB | Gradient relative L2 difference |
| --- | --- | --- | --- |
| Existing per-frame readouts | 1.5604 | 4.370 | reference |
| Groups of at most 4 | 1.7796 | 4.032 | 0.377% |
| Groups of at most 8 | 1.3942 | 4.036 | 0.377% |

Grouping four improved throughput by 14.0%. Group eight's median update was
548.8 ms versus 539.8 ms for group four, but it also had a 4436 ms outlier; it was
not adopted. No allocation retries occurred in this benchmark. Training now logs
allocator retry counts, allowing memory-management stalls to be distinguished
from steady compute. A previous allocator warning in the old live run recovered;
that run subsequently passed step 13000, so the warning was not a training abort.

`deferred-timing/validation.json` compares the selected group-four path against
existing production on 64 held-out batches / 16,216 frames. Policy loss delta
was -0.000110, value MSE delta -0.0000412, prediction-mean delta +0.0000110.
Active accuracy changed by +0.000848: one additional correct label among 1179
active decisions, consistent with BF16 near-tie rounding. Do not claim bitwise
identity or a playing-strength gain. FP32 causal memory is exact in the tests.

A controlled checkpoint handoff resumed step 13500 as `codex-r4-deferred-bc`,
with decision chunk 4 and approximately 1682 seconds of the prior optimization
budget left (derived from the old job/handoff times). The runtime and packages
are frozen; all previous checkpoints and benchmark failures remain preserved.
`selection.json` records the checks and `codex-r4-active.json` points here.
After over 400 updates, the live run reported about 2.03 updates/s on that data
portion with no allocation retries; unlike the matched 14% result, this rate
must not be directly compared with earlier runs on different trajectory batches.
Full validation, export equivalence and the frozen matched games still follow
automatically. No model promotion was added.


## Bounded asynchronous GPU input buffering

The user proposed overlapping SSD/RAM/GPU input movement using double, triple
or quadruple buffering. CPU-side preparation was already parallel: four
DataLoader workers, pinned batches and memory-mapped trajectory storage backed
by Linux's filesystem cache. `/proc/meminfo` at the check reported about 52 GiB
in Cached (including shared memory); a low "used RAM" chart does not mean this
reclaimable cache is unused. Those figures cover the machine, not only this job.

Implemented `--gpu-prefetch 0..3`: up to three future input batches on a dedicated
CUDA copy stream (0 is the default; 1 corresponds to current + one future batch).
Each staged input is limited to 128 MiB. Oversized/unpinned batches use the
existing transfer path. CUDA events enforce copy-before-use; `record_stream`
protects device allocation lifetime, and iterator cleanup drains transfers
before releasing pinned sources. Retiring/current tensors and model activations
are additional to the lookahead queue's memory bound. Neither examples nor
trajectory lanes are reordered. DataLoader prefetch and OS caching continue
independently; VRAM addresses are owned by the allocator.

`prefetch-timing/benchmark.json` uses step 15300, identical batches and 36 timed
updates per variant. CPU staging and iterator work are inside the measured wall
time, rather than incorrectly counting prefetched data preparation as free:

| Future GPU batches | Total input buffers | Updates/s | Peak allocated GiB |
| --- | --- | --- | --- |
| 0 | current only | 2.0186 | 4.057 |
| 1 | double | 2.0367 | 4.115 |
| 2 | triple | 2.0396 | 4.158 |
| 3 | quadruple | 2.0435 | 4.197 |

This short test's apparent improvement is only 0.9–1.2%, below the 3% adoption
threshold; the reverse-order confirmation was therefore **not run**. No candidate
was promoted. The first gradients also were not bit-identical (relative L2
0.051–0.078% versus the no-prefetch baseline). This is far below prior BF16
execution changes but does not pass the stricter transport-only numerical gate;
it must not be presented as bitwise-equivalent network execution. A separate
real-data audit checked four fully staged batches and all 140 input tensors
bit-for-bit equal after transfer. The source of the remaining arithmetic
variation was not isolated. CUDA unit tests passed for every queue depth,
ordering, allocation lifetime, oversized fallback and early close. CPU-only
unit discovery: 26 tests, 24 passed, two CUDA tests skipped; the new CUDA test
was run separately and passed.

The baseline copies occupied 1.857 ms of an approximately 495 ms update. With
lookahead the copy work happens on its separate stream, so the benchmark's
compute-stream H2D interval is about 0.010 ms; that does not mean the physical
transfer became free or 185 times faster. The full-update numbers above are the
relevant throughput measurement. More buffers cannot compensate for a sustained
input bandwidth deficit; they can absorb temporary jitter.

Resumed step 15300 as `codex-r4-buffer-reviewed-bc`, preserving the approximately
629 seconds of optimization budget remaining at handoff. `--gpu-prefetch 0`
remains selected, while grouped boards/readouts and the prior verified
optimizations remain enabled. The active pointer and `selection.json` record
this explicitly. Full validation/export/matched games follow unchanged.

A read-only inotify observation also timed a 651,862,065-byte checkpoint from
partial-file creation to atomic publication at 0.514 seconds. At roughly two
updates/s and one checkpoint per 100 updates, this accounts for about 1% of
wall time: it can make a visible dip but cannot explain a factor-of-two reserve.
Artifact: `deferred-timing/checkpoint-timing.json`.

## Larger compute batches and spending spare VRAM (23 October, follow-up)

The owner suggested two roughly 6 GB batches and requested further web research.
Distinguish input staging from the training working set: measured ordinary input
batches are about 50 MiB, whereas weights, gradients, AdamW moments, retained
activations and kernel workspaces account for most GPU memory. Two 6 GB input
buffers cannot fit alongside that working set on a 12 GB card. Input double
buffering also does not merge the model's individual kernel launches. Larger
compute batches can amortize those launches, but saturating a utilization chart
is neither a throughput measurement nor evidence of better learning.

Primary sources checked in this follow-up:

- [NVIDIA's fully-connected training guide](https://docs.nvidia.com/deeplearning/performance/dl-performance-fully-connected/index.html)
  explains how batch size changes forward/backward GEMM tiling and arithmetic
  intensity. Its hardware-specific examples are not predictions for this GPU.
- [PyTorch's activation-checkpointing discussion](https://pytorch.org/blog/activation-checkpointing-techniques/)
  describes spending memory to avoid recomputation, including selective operator
  policies. Our initial experiment retains whole trunk layers; it does not yet
  implement operator-level selective checkpointing or the experimental budget API.
- [PyTorch's async checkpoint recipe](https://docs.pytorch.org/tutorials/recipes/distributed_async_checkpoint_recipe.html)
  describes immutable CPU staging and limiting outstanding writes. We retain our
  single-file atomic checkpoint/resume format instead of introducing distributed
  storage solely for a single GPU.
- [CUDA graph API constraints](https://docs.pytorch.org/docs/main/generated/torch.cuda.make_graphed_callables.html)
  support testing a small fixed-shape recurrent region, with strict call order,
  gradient signatures and disabled autocast caching. This remains a candidate;
  the previously rejected whole-unroll capture is not silently enabled again.
- [Critical Batch Size Revisited](https://arxiv.org/abs/2505.23971), revised
  November 2025, measures the throughput/data-efficiency tradeoff and batch-size
  warmup on language models. It motivates a separate learning pilot here, not
  importing its model-specific optimal batch sizes into a recurrent game policy.
- [PyTorch CUDA allocator documentation](https://docs.pytorch.org/docs/2.14/notes/cuda.html)
  describes expandable segments for varying allocation sizes. Consider this if
  measured allocation retries/fragmentation recur; it does not cure kernel-launch
  overhead. Current selected training reported no allocation retries.

Added two opt-in experiments, both off by default and absent from the frozen
running trainer: `--saved-activation-layers N` retains the last N trunk layers'
activations; `--async-checkpoint` stages a consistent GPU/CPU snapshot and writes
it in the background. The writer overlaps staging with read-only forward/
backward, waits before the next parameter/optimizer mutation, preserves CPU RNG
and optimizer counters, propagates errors and drains on exit. One outstanding
snapshot bounds its additional pinned memory to approximately one checkpoint
(about 650 MB here). Staging allocation/API costs still occur on the main thread;
zero pause or faster full training has not yet been demonstrated.

CPU tests compare all retained-layer counts on a small model, including dropout
RNG preservation, all parameter gradients and deferred recurrent memory. All are
exact in those tests. Checkpoint tests hold the disk writer behind a barrier,
mutate the original CPU payload, check atomic publication and inject a write
failure. A separate CUDA test stages from a non-default compute stream, performs
another forward/backward/update and mutates memory; the saved weights, moments,
step counter and memory are exact, and optimizer resume advances correctly.
Unit discovery: 30 tests, 27 passed and three CUDA tests skipped; the new CUDA
snapshot test was run separately and passed. These are correctness checks, not
end-to-end performance measurements.
All six explicit Python/ONNX contract checks also passed after these changes.
Logs are `activation-checkpoint-unit-tests.log` and
`activation-contract-tests.log` under the throughput-check directory.

The benchmark can now compare batch lanes 8/16/24 using explicit
`--fresh-trajectories`. It refuses changing lane count while resuming the old
memory/cursor, reports valid frames and decisions/s and accepts a CUDA allocator
ceiling. Different lane counts sample a different trajectory mix and change the
per-head loss normalization: cross-size timing is exploratory, not a matched
equivalence or quality result. The production B=8/U=32 configuration remains
selected. Full held-out validation, export and frozen games have priority over
the queued isolated performance comparisons.

## Paired evaluation and adverse starts (owner follow-up, 23 October)

The owner emphasized that deck strength and drawing luck vary, and that a loss
does not necessarily mean the AI made a mistake. The existing evaluator already
plays each table seed/deck assignment from every focal seat, with a paired
Steady house baseline against Sharp opponents. v3 and the archived v4 models
use the same frozen evaluator and manifest. A 128-deal duel suite has 64 table
seeds, and 64 Commander deals only 16 table seeds; rotations are correlated,
not independent samples. Those counts are still too small for fine strength
claims. They compare substituting one policy against fixed opponents, not
direct v3-versus-v4 games.

Added `paired_eval4.py` to compare models directly case by case. Job manifests
must confirm the same engine executable hash, deck-manifest content hash,
limits and scouting settings. Report metadata and every table/seed/deck/focal
seat must match; differing, missing or duplicate cases are rejected instead of
quietly intersected. Repeated house-baseline outcomes must reproduce. The tool
reports candidate-better/equal/worse, both-lost/both-won subsets, unresolved caps
and table-clustered uncertainty. Fallback/refusal counts remain visible,
particularly because archived v3 uses substantially more house fallback.

Important interpretation and training rules:

- Keep bad draws and weak-deck games in the suite. Both policies losing yields
  zero outcome difference for that pair, not a claim that no winning line
  existed. Never remove hard games after seeing their outcomes or classify them
  as objectively unwinnable using an unvalidated value network.
- Identical engine seeds and ordered decks reproduce initial conditions.
  Different mulligans, tutors, shuffles and decisions can change later random
  consumption and cards drawn. Do not force equal later draws across different
  legal actions or expose the seed/hidden library to the policy.
- Evaluate advantages across many table seeds and deck matchups, report deck/
  archetype slices when sample sizes permit, and retain an untouched final
  seed/deck test set. Repeatedly optimizing a fixed validation suite can overfit
  it even if it never enters gradient training. Expand baseline profiles beyond
  the current Steady focal baseline/Sharp opponents in a separate frozen suite.
- For tactical precision, use reproducible view-only cases with independently
  checked legal solutions (for example a forced lethal, mandatory defense or
  avoiding a known rules violation). Survival time, remaining life/cards or a
  private critic score are not reliable substitutes for move quality.
- A losing trajectory can contain good moves; retain such policy examples.
  Value targets remain stochastic outcomes whose expectation/calibration is
  learned over many games, and PPO advantages need enough samples to reduce
  outcome noise. Do not relabel every losing move as bad or replace terminal
  goals with rewards for prolonging a lost game.

Five paired/uncertainty tests passed, including adverse cases, answer caps,
seat clustering, worker-order invariance and strict mismatch rejection. Existing
archived results were reanalyzed without running GPU games. For example, broad
v4 BC versus initial v4 BC on 128 duel cases has 7 improved outcomes, 12 worse,
109 equal (97 both lost, 12 both won). That is a -3.91 percentage-point observed
utility difference on this suite, not an established universal strength change.
The running optimized checkpoint is a different model and has not completed its
new full evaluation yet. Artifacts: `checks/paired-evaluation-20261023/`.

Performance comparisons are queued separately in
`checks/training-throughput-20261023/compute-batching-timing/`, using the frozen
trainer at `97bccc5e5`. They wait for the current production pipeline and an idle
GPU, then measure activation retention, checkpoint staging/publication and fresh
B=8/16/24 trajectory lanes. They do not interrupt validation or promote a model.

## Completed performance comparisons and model-quality gate (23 October, 16:30)

The queued experiments completed automatically by approximately 16:28. No job
failed; the idle GPU afterward was a completed queue, not a stuck launcher.
`compute-batching-timing/` contains raw samples and all job manifests.

| Fresh trajectory lanes | Valid frames/s | Decisions/s | Peak allocated GiB | Allocator retries |
| --- | --- | --- | --- | --- |
| 8 | 648.4 | 102.8 | 3.283 | 0 |
| 16 | 911.4 | 163.3 | 5.365 | 0 |
| 24 | 1180.8 | 201.3 | 7.354 | 1 |

The observed frame-rate increases are 40.6% and 82.1%, respectively. These are
exploratory throughput results on different trajectory mixtures, not matched
training-quality improvements. All three sampled at most 123 entity rows;
larger boards and rare huge candidate sets still need memory testing. Batch 24
already used 10.406 GiB of reserved allocator memory and retried one allocation.
Do not switch the production recurrent cursor into a different lane count.

Retaining two trunk layers' activations improved the first same-batch comparison
from 2.0146 to 2.1147 updates/s (+5.0%), but the reverse-order confirmation was
2.0448 versus 2.1001 (+2.7%). That misses the standing 3% confirmation threshold
while spending about 1.77 GiB more peak allocated memory. It remains off.
First-gradient relative L2 difference was 0.077%, max absolute 0.0004883;
no allocation retries occurred on these batches (up to 174 entity rows).

The full-size checkpoint benchmark round-tripped every tensor/state exactly.
Synchronous saves returned after 495–515 ms. Warm asynchronous saves returned
after 9.9–10.1 ms and allowed parameter mutation after approximately 26.7 ms;
publication still took 449–533 ms. Its first pinned-memory staging cost 588 ms
and publication 1014 ms. This measures checkpoint API/staging latency, not an
equivalent speedup of complete training. Async writing is suitable for the next
bounded training pilot; the new pinned snapshot and cold allocation cost remain.

The completed K=1 model at step 16442 scored 17/128 duel wins and 1/64 Commander
wins, compared with paired Steady house baselines of 58/128 and 11/64 against
Sharp opponents. It had zero refused actions and zero answer caps, but 9/33
stall fallbacks in duel/Commander. On the exact same duel cases versus broad v4
BC it improved 10 outcomes and worsened 12; versus v3 it improved 5 and worsened
30, with one unknown capped reference. Those are regressions in observed play,
not evidence that throughput optimization has yielded a stronger model.

Full validation included 3,072,672 frames and 45,951 active single choices:
active accuracy 37.35%, value MSE 0.150865 versus uniform-seat MSE 0.124138,
predicted utility mean 0.10107 versus observed 0.15646. Overall top-1 accuracy
91.32% includes many trivial/passive cases and must not be advertised as tactical
precision. The export passed 143 frame checks, max absolute error 2.26e-5.

Started `experiments/codex-r4-thinking-eval`: same frozen checkpoint, evaluator,
manifest, seeds and rotated focal seats at K=4, with the same 128/64 games. This
tests whether the trained extra thinking helps before committing a long new
training run. `paired_eval4.py --allow-thinking-difference` permits only this
explicit budget difference and records both depths; other pairing checks remain.
The active experiment pointer was advanced; no checkpoint was promoted.

## Thinking result and evening learning pilot (23 October)

The K=4 run completed: 18/128 duel wins versus K=1's 17/128 (two improved,
one worse, 125 equal cases). Commander remained at one win, with one capped
game whose outcome is unknown; one case improved and 62 were equal, while the
capped case cannot be ordered. The paired intervals include zero. Extra depth
has not yet demonstrated a reliable strength improvement. The owner explicitly
wants thinking/difficulty levels retained; training continues to cover K=1/2/4/8.
Difficulty labels should eventually reflect measured playing strength, rather
than assuming that increasing K necessarily makes the policy stronger.

At the owner's request, plan `experiments/codex-r4-evening-batch-pilot` to finish
training plus validation around 19:05 Europe/Berlin, then automatically compare
play. Two bounded B=8/B=16 pilots start from the same step-16442 model weights,
each with fresh AdamW/memory, LR 5e-5 and sampler seed 43 at complete trajectory
beginnings. This is an explicitly new fine-tuning phase, not migration of an
existing recurrent cursor or optimizer to different batch lanes. Fresh optimizer
state and lower LR apply to both; it is not a claim that batch size alone fixes
the prior model's shortcomings. All prior artifacts remain preserved.

Validation stays at B=8/U=32 for both models, over all 3,072,672 held-out frames.
`--validation-batch-size` is recorded in last/best checkpoints and metadata;
resume rejects a change to that scoring protocol. Old checkpoints default to
their training batch size. A real synthetic-data CLI integration test trained
at B=4/validation-B=2, checked all 32 held-out frames, rejected incompatible
resume and successfully continued with the original scoring configuration.
Sixteen focused existing tests ran (14 passed, two CUDA skips); the new CLI
integration test passed. Full validation now logs progress every 500 batches.

The orchestrator allocates equal initial optimization budgets after reserving
35 minutes per full validation and startup overhead. It records and, if needed,
reduces the second budget using the first measured validation duration to target
19:05. Any unequal budgets are disclosed in `plan.json`. No intentional waiting
for the clock is added. Completion is an estimate because validation time varies.
An OOM in B=16 preserves the failed experiment; if time permits, the runner
continues the known B=8 checkpoint with intact lanes in a separate output folder.
Other failures are kept visible rather than silently reinterpreted as results.

Both pilots use the measured async checkpoint writer and the previously selected
execution optimizations; extra retained activations and GPU input prefetch remain
off. The frozen dataset/evaluator/deck manifest are unchanged by the ongoing card
review. After the final training/validation stage: ONNX checks, then K=1/4/8
evaluations with 128 duels and 64 Commander deals per model/depth, paired against
the same house baseline, v3, the prior model, each other and their own K=1 results
as applicable. There is no automatic model promotion or declaration of human
strength. This remains one-seed exploratory evidence, not a final held-out test.

## Evening result at 19:23 and checkpoint selection audit

Training plus full validation finished at **19:03:19 CEST**, within the planned
window. B=16 failed after 290 seconds with a CUDA OOM (requested 740 MiB, only
32 MiB free; 10.29 GiB allocated and approximately 970 MiB reserved but unused).
Its last logged step was 325. This invalidates the intended completed B=8/B=16
learning comparison: the short batch benchmark did not cover these larger
states. The failure and checkpoint are preserved. The fallback resumed the
valid B=8 lanes/optimizer in `b8-continued` for the remaining 1680-second budget.
Recoverable allocator retries also occurred in B=8, but both B=8 jobs completed;
an allocator warning alone is not a terminated training run.

The first B=8 pilot finished at its phase step 3116. Full-validation active
teacher-choice accuracy rose from 37.35% to 42.81%, policy loss fell from 0.22983
to 0.22139, while value MSE worsened slightly from 0.15086 to 0.15329. In matched
K=1 games it won **31/128 duels and 10/64 Commander games**, versus the prior
model's 17/128 and 1/64. Duels: 17 better outcomes, three worse, 108 equal.
Commander: nine better, none known worse, 54 equal, one capped pair. The house
baseline still won 58/128 and 11/64; v3 won 41/128 and 4/64. Commander has only
16 independent tables. These are improvements on this tuning suite, not proof
of general superiority to the house AI or experienced humans. K=4 had the same
win totals as K=1; the unique K=8 comparisons follow.

The continuation reached phase step 5748, active teacher-choice accuracy 43.84%,
policy loss 0.21920 and value MSE 0.18929. The combined selection score
`val_pol_loss + 2 * val_mse` therefore retained step 3116 as `net.pt` while the
actual latest weights remained in `last.pt`. Byte hashes confirmed that the two
selected `net.pt` files and their K=0/1/4/8 exports were identical. The original
queue unnecessarily replayed the identical selected models at K=1/4. Those
repeats are not additional independent evidence.

A controlled follow-up waits for the unique selected-model K=8 Commander job to
complete, then suppresses the remaining identical-model comparisons and records
the interruption/deduplication explicitly. It extracts the real step-5748 state
into `experiments/codex-r4-latest-checkpoint-eval/model`, without replacing any
selected model. Its validation metrics were recovered from the exact matching
step in the preserved trainer stdout; provenance includes the source `last.pt`
hash. ONNX checks and matched K=1/4/8 games then determine whether its stronger
policy imitation but weaker value estimates help or hurt actual play.

Future `last.pt` files now carry the most recent validation step and metrics,
avoiding log parsing for this audit. Metrics are associated with their explicit
step, never silently attached to subsequent unvalidated updates. The CLI resume
integration test checks that both saves carry the correct validation step and
complete held-out frame count. Checkpoint selection itself is unchanged pending
the playing-strength evidence. No model is automatically promoted.

The unique selected-model K=8 evaluation subsequently completed with 32/128
duel wins and 9/64 Commander wins (one Commander cap), versus K=1/K=4's 31 and
10. More depth is not a monotonic strength improvement here. The old launcher
was stopped after its unique K=8 work completed, identical remaining comparisons
were suppressed, and `deduplication.json` records the matching hashes. The latest
step-5748 export passed its checks and its first matched K=1 games are running.
