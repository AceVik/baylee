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
