# BayleeAI v5: Rust migration and bounded learning checks

Date: 29 October 2026. Worktree: `baylee-ai-payment`, branch
`codex/payment-context-v5-20261028`. The owner's stopped continuous BC recipe
remains stopped. This report does not authorize automatic model promotion.

## Implementation

The implementation and experiment contracts are described in
[the refactoring plan](../proposals/trained-ai-v5-refactor-training.md) and
[trained-ai](../trained-ai.md). R0–R4 now provide:

- Strict architecture, encoder, candidate-context and thinking-schedule
  identities across loading, resume, publication and export.
- A separately versioned multiplicative candidate-context gate. The old shared
  additive term cancels from categorical softmax; the zero-initialized new
  gate changes relative action scores while preserving initial behavior.
- Explicit Encoder8 payment context, kept separate from Encoder7 and from the
  live socket capability. No silent reinterpretation of existing artifacts.
- Native admission for both PPO paths, independent learner/opponent provenance,
  complete chronological history and actual executed behavior probabilities.
- Full-model PPO history reconstruction and joint parameter/Adam/RNG rollback
  when an update exceeds its declared limits.

Rust now owns active orchestration, process supervision, packing/cache
administration, imports, admission, host-side metrics, reports and monitoring.
The remaining Python files perform PyTorch computation, tensor adaptation or
direct tests; native calls have no Python operational fallback. Even retained
v3 reports and benchmark quantiles use native implementations. Their distinct
rounding and summation conventions have explicit compatibility tests.

The language rule is global for all coding agents, including Claude,
Gemini/Antigravity and Codex. It is recorded in `/home/ace/AGENTS.md` and each
installed agent's global instruction file, with repository instructions for
other checkouts. Python is allowed only for PyTorch and its direct tests;
TypeScript only for web applications and web-specific tooling.

All three local gateway supervisors are native. Their previously deployed
engine/runtime/model selections were retained. Five obsolete controllers are
archived and masked. One abandoned collection engine was removed only after
checking ownership, stopped collector state and absent players. Its incomplete
record still has no fabricated outcome and is ineligible for training. See
[the operational receipts](native-host-migration-2026-10-29/README.md).

## E1: first complete paired head-only pilot

Run: `refactor-investigations-20261029/`
`e1-head-pilot-recovery-01a187b3-cdb9-7199-8a5e-4563371c4a70` below
`/home/ace/baylee-data/v5/ops-codex-20261028/`.

Both arms started from the identical twelve-layer d640 R6 checkpoint. Only
policy heads were trainable. The repaired arm added 819,840 zero-initialized
gate parameters. Both used seed 33002, K2, the same learning rate and loss,
32 complete training games, 66,739 frames and 64 seat histories. Each completed
one full pass: 266 batches and 266 optimizer updates, with 19,375 observed
decisions and the same policy-weight dose, 9,637.25. There was no prefix cutoff.

The real-data resource probe passed, with identical initial outputs on its
four sampled batches and nonzero gate gradients. Training took roughly 130
seconds per arm. The postflight and independent tensor check found 538 frozen
tensors exactly unchanged from R6; only the permitted policy heads and the
new gate changed. Final and resume checkpoint weights matched. These checks
establish that the small learning experiment actually ran as declared.

Two earlier technical failures remain preserved: missing Git identity in an
isolated trainer copy and a Linux procfs `ESRCH` race when a short-lived child
exited during memory accounting. The latter has native regression coverage;
permission errors remain fatal. Neither attempt supplied a recovered model.

## Separate FP32 diagnosis

Run: `refactor-investigations-20261029/`
`e1-policy-diagnosis-01a187bf-a683-7279-ae6e-547b2df1302d`.
Three K2/FP32 inference jobs completed in 73 seconds, with independently reset
complete histories, unchanged model hashes and no optimizer updates.

The five reused development games contain 7,608 frames and 1,030 genuine,
positively weighted categorical choices, including 314 pass targets and 716
non-pass targets. All figures below use this same eligibility mask. They are
teacher agreement, not objectively correct play or win rates.

| Metric | Initial R6 | Original context | Repaired context |
| --- | ---: | ---: | ---: |
| Choice NLL | 1.26730 | 1.23206 | 1.19644 |
| Exact choice matches | 538 / 1,030 | 532 / 1,030 | 573 / 1,030 |
| Exact pass matches | 201 / 314 | 197 / 314 | 251 / 314 |
| Exact non-pass matches | 337 / 716 | 335 / 716 | 322 / 716 |
| Predicted pass on a non-pass target | 23 / 716 | 27 / 716 | 35 / 716 |
| Non-pass target NLL | 1.53359 | 1.49247 | 1.47377 |

The repaired context improves likelihood in all five games, but 35 of its 41
additional exact matches come from one game. Its gain is dominated by pass
imitation, while non-pass rank accuracy drops in four of five games. Non-pass
NLL nevertheless improves: lower average loss and better argmax behavior are
different properties. Subset BCE improves slightly; four pair labels and two
scalar labels are too few for a useful conclusion about those heads.

Training-time BF16 metrics differ slightly from this FP32 diagnosis. The legacy
`val_active_top1` metric also has a different eligibility mask: it does not
require positive weight or genuine alternatives. Do not combine those series.

## Fixed-seed replication

The four predeclared additional arms (seeds 33003 and 33004, both revisions)
completed at unchanged dose. Each made 266 updates; both tensor audits passed.
The frozen sampler uses the seed for trajectory lane allocation and ordering:
all 266 batch rows differ between each seed pair, with identical coverage of
2,120 segments. These are different training orders, not independent test data.

| Seed | Original/repaired NLL | Original/repaired correct pass | Original/repaired correct non-pass | Original/repaired false pass |
| --- | ---: | ---: | ---: | ---: |
| 33002 | 1.232064 / 1.196441 | 197 / 251 | 335 / 322 | 27 / 35 |
| 33003 | 1.237819 / 1.227728 | 196 / 206 | 336 / 326 | 26 / 28 |
| 33004 | 1.230897 / 1.195560 | 201 / 256 | 339 / 325 | 27 / 35 |

All figures use the same five development games and eligibility mask above.
Likelihood and pass agreement improve in all three pairs, while exact non-pass
agreement worsens in all three. This justifies retaining the architectural bug
fix but does not justify promoting a trained candidate or increasing BC dose.
The subsequent check compared the original seed-33002 pair in predeclared,
seat-swapped games against Sharp, as reported below. Choosing that pair before gameplay avoided selecting the
best of these small validation results. New deal seeds still do not make known
deck families an unseen-card/generalization holdout.

Replication evidence: `e1-seed-replication-01a187c6-d64b-7f9e-a641-c6e779229c78`
and `e1-seed-continuation-01a187cf-c385-75fc-b169-7e6aa1a0b4e8` under the same
investigation root. The first research audit incorrectly compared the checkpoint's
tuple `(2,)` with JSON's list `[2]`. That failed namespace and traceback remain
intact. A fresh continuation normalized only that representation, audited the
already complete seed-33003 checkpoints and reused them unchanged; it did not
retrain them. Its four FP32 diagnoses and full tensor audits completed in the
declared continuation. `assessment.md` gives every per-game delta.

No architecture winner, generalization improvement or playing-strength gain
has been established, and no candidate has been deployed.

## Completed E1 gameplay and payment diagnosis

The preselected seed-33002 pair and unchanged R6 completed all 48 model cases:
eight tables, two focal-seat rotations, three models, against Sharp. All used
raw greedy FP32, K2, the same frozen deck order and deal seeds. There were no
caps, refusals, outer stalls or draws. The following are descriptive outcomes
of the executed protocol, which includes the reported house interventions:

| Model | Wins / losses | House fallback events | Affected games |
| --- | ---: | ---: | ---: |
| R6 | 6 / 10 | 13 | 2 |
| Original-context candidate | 4 / 12 | 8 | 1 |
| Repaired-context candidate | 4 / 12 | 3 | 1 |

The repeated Sharp reference produced eight wins and eight losses over the
16 unique cases; its deck/seat/seed/outcome rows agree across all three runs.
The predeclared ledger conservatively invalidates an entire four-case chunk
when it contains a fallback. It therefore retains 36 valid and 12 unknown
model rows, including clean rows in the affected chunk. All 48 rows remain in
the denominator. Neither paired inference nor selection of a clean subset is
permitted. Fewer interventions do not establish stronger play; neither trained
candidate is promoted. This small known-family development comparison is not
a v4-versus-v5 architecture test.

All 12 report hashes were independently checked against the completed pipeline.
The pipeline took 1,250.78 seconds; final supervisor/ledger completion took
1,266.98 seconds, within the 1,800-second design budget. The earlier debug-build
admission timeout was traced to graph hashing; the unchanged models passed
native admission with the optimized tools-profile evaluator. No failed attempt
was overwritten or counted as a successful gameplay stage.

All 13 R6 interventions were `repeated_view`. Hash-verified replay of both
affected games (2,072 input snapshots) showed three cast/cancel cycles before
each intervention: 39 `CancelCast` choices, involving Traumatic Critique and
Energy Bolt. These were real repeated actions, not benign recurrence falsely
identified by the guard. Encoder7 already represented the outstanding cost;
Encoder8 additionally retains the announced source through nested questions.
This diagnosis does not establish that Encoder8 would correct the choices.

Exact-menu replay showed that every cancellation also offered `PassPriority`;
18 menus additionally offered two abilities. Independent reconstruction of
each recorded prefix then replaced just that cancellation with one pass:

- In 21 of 39 branches, payment settled and the same spell entered the stack.
- In 18 branches, the engine accepted the pass but unwound the unpaid cast,
  leaving or returning the spell to hand.
- No branch was refused, left unclassified or left awaiting further payment.

The 39 branches verified 27,984 prefix input hashes in addition to the complete
canonical replays. They took 5.44 seconds with existing pinned Rust artifacts;
no neural inference or training was needed. Successful casting is a rules fact,
not a proof that casting was strategically optimal. The 21/18 counts describe
these repeated decision states, not 39 independent games. Evaluation situations
remain diagnostic fixtures, not new training examples.

This exposed two distinct weighting problems, now corrected: live/LLM record
conversion classified payment-settlement passes as ordinary low-value passes
and could even remove their live policy weight on an opponent's empty stack.
Separately, opt-in loss5 contained an undocumented candidate-based pass
discount. The latter was **not active in E1**, whose sealed jobs used loss4;
it cannot explain this experiment. Neither observation by itself proves the
cause of the learned cancellation behavior.

Commit `2ca3238a4` preserves settlement supervision whenever the permitted view
contains `casting`, `announced_cast` or `owed`. Ordinary idle-pass weighting and
source eligibility are unchanged. New conversions declare
`idle_pass_weighting_revision=2`; missing metadata means legacy revision 1.
Dataset fingerprints distinguish the revisions, so old data and caches are
not silently relabeled. Real engine fixtures exercise paid Vitalizing Cascade
in Encoder7/8 on and off turn, alongside ordinary waiting behavior. The shared
predicate extraction is a separate behavior-preserving commit (`ed50c066d`).

Commit `a414cf194` removes loss5's hidden pass discount and declares
`categorical_weighting="symmetric_v1"`. Exact resume rejects ambiguous old
loss5 checkpoints before optimizer or recurrent-state restoration; a fresh
weights-only initialization remains supported. Loss4's contract, values and
gradients are preserved. Twenty focused PyTorch tests, five native contract
tests and strict Clippy passed. Native release helpers were rebuilt for the
new contract boundary; new conversions must use the rebuilt converter too.

Evidence under the investigation root:

- `e1-gameplay-admission-recovery-01a187f9-f940-75ae-815f-ef697db3195b/`:
  frozen gameplay plan, native jobs and all report/record outputs.
- `e1-gameplay-observer-01a18809-debd-74fa-ad0e-ee613dede236/`:
  finalized ledger (SHA-256 `5347aa4808aeeefd7b2780915df6c53d05f99d8f461c61146664738c7b59ab94`),
  completion receipt and repeated-view replay.
- `e1-gameplay-independent-summary-20261029/summary.json`: independent totals.
- `e1-payment-menus-01a18817-e565-762d-a083-c37d95524baa/` and
  `e1-payment-counterfactual-01a1881f-0018-7778-81c3-5bc3148ed66b/`:
  all menus, exact branches, source and guarded execution receipts.

## E2 prerequisite: real-history depth sensitivity

The bounded same-state probe completed in
`e2-statewise-depth-datafix-01a18832-06c5-7d97-95ff-a062b42d48a1/`.
R6 and the already preselected repaired candidate each replayed all 811 frames
of the first complete development seat history. Measurement used its first
32 genuine decisions (31 categorical, one subset), with K1/2/4/8 starting from
the same causal H for each state. No pair/scalar cases were substituted and no
optimizer updates were made. Positive K preserves H bitwise; K0 observations
advance it. The full chronological replay follows the native data contract.

All 31 categorical states and the single subset state changed their probability
outputs at K2, K4 and K8 relative to K1. Mean categorical KL(K1 || K), at
temperature 1, was:

| Model | K2 | K4 | K8 |
| --- | ---: | ---: | ---: |
| R6 | 0.00003172 | 0.00008647 | 0.00020716 |
| Repaired candidate | 0.00004514 | 0.00012909 | 0.00033166 |

Latent step-delta traces were exactly equal between the models, consistent
with E1 freezing the trunk and thinking block. Their readout changes differed.
This establishes execution sensitivity, not useful learned iterative reasoning.
Constrained actions were not decoded; a subset logit or Bernoulli threshold
must not be reported as an actual engine action. The fixed-K versus mixed-K
learning experiment and playing-strength comparison remain outstanding.

Worker time was 33.72 seconds; native pipeline time was 44.96 seconds. Peak
allocated GPU memory stayed below 0.425 GiB, reserved memory below 1.147 GiB,
and host PSS below 2.63 GiB. Instrumented joint K1/2/4/8 measurements averaged
53.6/57.8 ms per measured state for R6/candidate. Hooks synchronize with the
host; these are diagnostic costs, not serving latencies or isolated per-K
benchmarks. All 263 sealed inputs were independently rehashed; no GPU child
remained afterward. Report SHA-256:
`86815c1554a4698b7823a4eefb341ec389d5bfacd72575d8ecfeb3d922fb7d8a`.

Two preceding setup failures remain preserved: the first working directory
lacked a Git identity, and the next dataset used symlinks escaping its root.
Neither measured a model state. The successful preparation uses verified
independent dataset copies, an isolated Git snapshot and an actual CPU data
preflight loading the complete history in its final location. The generic
process-guard smoke check alone had not tested the dataset contract.

## Pre-E3: complete outcome collection and exact behavior reference

A separate technical collection completed all eight predeclared training-deck
games in 232.32 seconds: four tables, both learner-seat assignments, two tables
against Sharp and two against Steady. R6 used raw FP32, K2 and temperature 0.7.
There were two wins and six losses, with no caps, refusals, stalls or house
fallbacks. Outcomes did not select the data. Eight games are not a strength
comparison, and this does not satisfy the larger mixed-opponent E3 design.

The corrected converter retained all eight games and all 13,655 seat frames,
with no rejected games or unencoded decisions. Native admission certified
eight learner histories and 1,575 genuine stochastic learner choices, no
excluded games and no validation shards. Preprocessing revision is 2. The
house opponents' actions are observations, never neural PPO targets.

The first conversion pipeline had an incorrect expected output list: its
manifest expected shard IDs 0–7 while the converter emitted IDs 1–8. Conversion
itself completed successfully. The failed namespace remains preserved; a fresh
admission-only continuation sealed and verified the exact completed dataset
without reconversion, changed bytes or replacement games.

Independent review found an additional shared PPO target issue. The target
builder checked replayed log probabilities against recorded behavior values,
then cached the replayed values as the denominator and behavior-KL reference.
That silently erased accepted numerical replay differences. Commit `09a30de36`
keeps the original recorded values after the same parity check. A regression
injects an accepted 0.0005 difference and verifies that both head-only and
recurrent PPO preserve it and report positive sampled KL. Above-tolerance
differences still fail. All 26 focused target/training/recurrent CPU tests pass.
This is a reference-contract repair, not evidence that the tiny numerical
difference caused earlier poor playing results.

Commit `569d1645b` also versions full-PPO resume as contract 2 with
`behavior_probability_reference=recorded_executed_action`. Legacy or differently
marked optimizer continuations are rejected before initialization/restoration;
weights-only initialization remains possible. Twenty focused CPU tests and one
tiny CUDA transaction/resume test passed, including exact next-update weights
and Adam state. The CUDA test ran alone through explicit agent coordination;
its lock path was accidentally different from the canonical experiment lock,
so the receipt is not claimed as a test of cross-controller lock exclusion.

Full recorded-policy replay and head-gradient accumulation passed on CPU and
CUDA. Every replay covered 6,626 learner frames and all 1,575 choices; passive
observations advanced memory but were not actor targets. All source tensors
remained bit-identical, and none of these probes constructed or stepped an
optimizer. Explicit zero moment buffers only estimated prospective Adam storage.

| FP32 execution | Complete replay | Head forward/backward | Worker total |
| --- | ---: | ---: | ---: |
| CPU, B1 / board chunk 1 | 237.98 s | 226.43 s | 471.14 s |
| CUDA, B1 / board chunk 1 | 88.46 s | 79.80 s | 175.35 s |
| CUDA, B8 / board chunk 4 | 14.61 s | 9.24 s | 30.84 s |

All used U32, four host threads, deterministic algorithms and disabled TF32.
These are measured configurations, not a claim that four threads maximize CPU
performance. The additional B8 probe preserved the full normalized objective;
its gradient relative L2 error was `8.23e-6` against CPU and `5.39e-6` against
CUDA B1, below the predeclared `1e-3` bound. Recorded-logprob error was at most
`1.26e-5`, below the unchanged `0.001` admission tolerance. CUDA reserved
770 MB and native peak group PSS was 2.34 GB. GPU numbers describe the Torch
allocator, not whole-device usage. These head-only probes do not certify
full-model PPO capacity or Commander performance.

The single-update experiment therefore selects CUDA B8 / board chunk 4 before
launch. Its learning settings remain one epoch, at most one update, all 1,575
decisions in one normalized group, learning rate `2e-5`, target KL `0.02` and
U32. There is no discarded tail, relaxed KL limit, automatic retry or live
promotion. The real update additionally performs optimizer work and post-update
KL checks; its timing is not inferred from the no-step probe. An accepted
technical update would not establish better play or learned reasoning.

Evidence under the same investigation root:

- `pre-e3-outcome-01a18836-4a7c-768d-8c34-0a0caf0faf38/`: original plan,
  all eight outcomes and whole-collection quality receipt.
- `pre-e3-conversion-admission-01a1884b-fb13-776b-8e14-47079f3a6013/dataset/`:
  complete converted records, unchanged after successful conversion.
- `pre-e3-admission-recovery-01a1884d-10f6-76ae-a347-18aa5aa3c4ed/`:
  admission, independent verification, source seals and process cleanup.
- `pre-e3-resource-probe-01a1884a-89dd-742b-9b9a-83127022be15/`:
  complete CPU/CUDA B1 replay and gradient probes.
- `pre-e3-cuda-b8-resource-01a1885e-586e-7f89-bdf8-e1327437beef/`:
  B8 probe and independently rehashed comparison against both B1 runs.
- `pre-e3-one-update-01a1884f-89af-72ff-aa6e-88eda782fccb/`:
  separately sealed single-update plan and tensor-publication audit.

Dataset fingerprint:
`6af73310519f4a131377b672112475f2bd77a492c3bf159bb65f9ffa60a00592`.

The first actual update launch failed after 2.09 seconds while importing Torch,
before loading weights or constructing an optimizer. The native guard had
canonicalized the virtual-environment interpreter symlink and executed the
base interpreter. The successful resource probes used a Rust exec adapter and
therefore had not exposed that direct-invocation bug. Commit `97ef8e5ad` repairs
the guard itself: execution preserves the invocation path while provenance
binds both that path and the canonical binary target/hash. Twenty targeted
native tests and focused strict Clippy passed. Failed artifacts remain intact;
a new attempt required an actual guarded Torch/data-loader preflight with the
corrected binary. There were zero training steps in the failed attempt.

That recovery passed the actual direct-venv preflight in 6.97 seconds, including
Torch import, the native dataset constructor, all eight learner histories and
the expected 1,575 choices. The isolated update then completed in 53.18 guarded
seconds and accepted exactly one step with no discarded tail. All 24 policy
tensors changed; all 542 other tensors remained bit-identical. The full-rollout
sampled behavior KL was `3.8877511166e-6`, below the unchanged `0.02` limit.
All 400 sealed inputs were reverified, the publication audit passed, and all
owned processes exited. No optimizer state or persistent match memory is
exported with this head-only candidate. This establishes a technically complete
outcome-based update, not stronger play, Full-PPO or better reasoning.

Recovery evidence:
`pre-e3-one-update-recovery-01a18871-31b2-7691-be6a-522189b20c10/`.
Checkpoint SHA-256:
`5bf79ef5c932aa715331a77408eed0326270f49037eefa3da5d0ef5e07fca9d2`.
It remains isolated and was not promoted or used to initialize E3 automatically.

## E3 first-four integration screen

The mixed collection predeclares 64 TRAIN tables, both learner seats, with
16 tables each against Sharp, Steady, a pinned historical candidate and exact
R6 self-play. A native zero-game preflight accepted all 128 fixed deck entries
in exact order and verified loaded model identities. The first historical and
self-play tables were reserved as a four-case integration screen inside this
128-case design; outcomes never selected them.

All four games completed in a 151.43-second pipeline. The two self-play games
were clean; each historical game used one `repeated_view` house fallback. No
caps, refusals or stalls occurred. The collection therefore fails the declared
clean-data gate. The remaining 124 cases were not started, and none of these
four games was converted, admitted or used for an optimizer update. All four
outcomes and both fallbacks remain visible; the clean self-play pair was not
selected afterward as a replacement training set.

Deterministic replay verified both historical records and every post-input
snapshot. In each game the affected seat announced Fireball three times,
received the forced payable choice X=0 and selected multiple targets. Each
additional target costs one
generic mana; with only one red mana available the announcement was unpayable
and automatically unwound to the same priority question. Target selections
varied, so this does not establish identical deterministic neural output.
The fourth visit triggered the documented fallback. These are actual action
cycles, with no demonstrated guard/hash false positive. Sidecars show six
stochastic choices per affected seat (announcement and target set, three times)
and three forced numeric choices with no policy logprob. Only the second game
involves the designated learner; neither whole game is admitted. Head-PPO can
change announcement and target-set logits, but cannot change a forced X answer.

The static observation audit found the full `extra_target_cost=1` field in the
actual structural catalog, a source-card relation at the target question, and
the visible mana pool. This is not evidence of missing Fireball rules. Explicit
chosen X and a running selected-target count/budget are absent from the target
scorer inputs; independent subset sampling enforces cardinality bounds but no
conditional affordability rule. Neither the guard threshold nor admission
rule was relaxed.

Eight deterministic announcement branches now confirm the distinction. With
X=0 and one red mana, one target commits, while two or three automatically
unwind. Adding two colorless mana makes all three target counts payable.
Separately, with sufficient mana, X=0 and X=1 produce identical authorized
target views, pending questions and Encoder-7/8 target tensors. The complete
ten-frame observation prefixes also match with the actual knowledge and passive
encoding paths. The runtime does not feed an executed `ChooseNumber` back into
H. Identical initial memory therefore cannot distinguish these choices from
those observations. This is an input-alias proof, not a measured neural-memory
experiment: no inference or optimizer ran. It establishes an observation gap,
but does not establish that the gap caused the actual X=0 failures, where the
visible mana and target surcharge already suffice to identify excess targets.

Fixture evidence: `e3-fireball-fixture-01a18873-b4b2-748c-b018-079eaef05e78/`;
summary SHA-256
`f8bbc1c42cd17e334985ba8ba8fe0b3f448e11070ed6d5bfce29a368358e26da`.
Its source and dependencies are sealed. Existing Encoder-7/8 observations and
sampling remain unchanged; an explicit observation extension needs its own
version and matching collection/replay path. A separate training-design review
must also address the selection bias of excluding every game containing a
learner recovery: clean-game admission alone cannot teach the failures it
systematically removes. This is not permission to attribute house-assisted
terminal returns to the raw learner or to silently admit the failed four games.

The native observation foundation is implemented in `1ee48685b`:
`Engine::announced_spell_context` and separate opt-in Session announcement views
expose `PlayerView.announced_x` only to the resource owner or its authorized
controller. An unsettled X remains distinct from settled zero. At that milestone
no encoder consumed the field; old views, hashes, sampling and model inputs
were unchanged. A regression confirms that the new authorized target views distinguish
X=0/1 while the existing payment projections remain identical. Commit, cancel,
automatic reversal, nested mana, miracle and controlled-player visibility are
covered. Fifty-seven targeted Rust tests, strict Clippy on the affected
engine/view/gamehost/train libraries and tests, and formatting passed.

Independent review caught an effect-payment branch that precedes the X choice.
Reading the object's stored X there would falsely report a selected zero or a
stale previous value. The final accessor returns an unsettled value until the
actual cast chooses X; its regression starts with stale stored X=9 and verifies
the full transition to selected X=1 and commit. There is no engine-state,
journal/hash or rules mutation in this observation change. The original fixture's
static explanation also needed correction: `actions.rs` already writes chosen X
onto the object before targets, not only at commit. The empirical observation
alias remains valid. An additive `selected-x-storage-correction.json` beside
the sealed fixture records this without replacing its original artifacts.

Evidence: `e3-mixed-collection-preparation-01a18860-9237-7420-b8df-03b22d156d51/`
contains the fixed plan, all four outcomes, quality report and verified process
cleanup. `e3-repeat-review-01a1886b-6d07-774a-91e2-31f79677d54e/` contains the Rust
replay helper and exact intervening actions/resources. No live model changed.

## Four-case transfer after the accepted head-PPO step

The unchanged published one-step candidate passed export parity at K0/1/2/4/8
over 47 frames, with maximum absolute PyTorch/CPU-ORT error `8.58307e-6`.
The native zero-deal loader and exact deck/preset checks also passed. This is
numerical integration evidence, not evidence of better decisions.

All four predeclared E3 cases were then completed with the same frozen runtime,
deal/policy seeds, decks and opponent checkpoints. In the formerly identical
R6 selfplay pair only the learner changed; the opponent remained R6, explicitly
classified as historical. The historical pair and the R6-opponent pair have
separate producer receipts because an incorrect required-output check stopped
the first pipeline after its two games. A continuation ran only the two
unstarted cases, reusing the verified export and preserving the first results.

| Deal | R6 / updated utility | R6 / updated fallbacks | R6 / updated Fireball reversals |
| --- | ---: | ---: | ---: |
| 64 | 1 / 1 | 1 / 1 | 5 / 5 |
| 65 | 0 / 0 | 1 / 1 | 3 / 3 |
| 96 | 0 / 0 | 0 / 0 | 0 / 0 |
| 97 | 0 / 0 | 0 / 0 | 0 / 0 |

Only deal65 has an action divergence: at input413, from the exact same prior
state hash, the candidate selects three targets rather than four. Both choices
remain unaffordable and reverse to the same state. The other three action
traces are unchanged. The one-step update therefore has a measurable policy
effect but has not fixed the loop or improved these outcomes. These known
trigger cases are a development diagnostic, not a strength holdout.

Evidence: `step-ok-four-transfer-01a1888b-4937-76e2-95ce-5d2d48d6b9e9/` and
`step-transfer-continuation-01a18897-7151-7411-a2ea-730b7dfe0784/final-review.json`.
The combined comparison SHA-256 is
`337cbbf335a3fde8889776ba5541f45ffe643fb4c6ea68c423b6b1efc3f87c23`.
All 7,033 continuation input hashes remained unchanged and owned processes
exited. No data admission, further weight updates or promotion occurred.

## Full-PPO audit execution

The complete-collection KL audit previously reconstructed every segment's
history from the start. For a history of S segments this repeated S(S+1)/2
segment forwards even though no weights change during the audit. The audit
now streams each history once under the current weights, resetting H on lane
reuse and keeping all valid stochastic decisions. It neither reads behavior H
nor changes differentiable training unrolls or their current-policy burn-in.
The execution contract records `collection_audit=current_policy_streaming_v1`.

Twelve focused PyTorch tests passed, including existing update rollback/resume
and disk/export integration. The new tests compare every action logprob and
the aggregate divergence against full-prefix reconstruction, with unequal
history lengths, padding, lane reuse and changed recurrent/input weights.
The fixture reduces 42 network forwards to 21 at batch1. Commit `b71d9685d`
contains this execution change and its tests.

A separately bounded production-model probe then replayed the first eight
segments of the first learner history (game 1, seat 0) from the sealed Pre-E3
dataset. This is 256 frames and 60 stochastic decisions out of a 25-segment
history, explicitly not a complete episode or admitted learning rollout.
After separate warmups, repeated-prefix inference took 15.096 s and 1,152
forward calls; streaming took 3.588 s and 256 calls, about 4.21 times faster
for this prefix. Every action logprob was bit-identical between paths;
reference, stream and actual collection divergence were all
`7.556918054281898e-15`. Maximum error against recorded behavior logprobs was
`6.258487701416016e-7`. Peak allocated CUDA memory was 369.506/368.720 MB and
reserved memory 392.167 MB for both paths. No optimizer was constructed or
gradient computed; source tensors and all 243 sealed inputs remained unchanged.
Owned processes exited and the GPU lock was released. These measurements do
not establish full-episode speedup, backward capacity or playing strength.

Evidence: `full-ppo-audit-prefix-probe-01a1889f-0e6a-7496-a446-8734811d9418/review.json`
under the investigation root. Its nested preparation audit inherits a generic
terminal-return description from a reused helper; this does not certify the
incomplete prefix as a complete return or authorize training on it.

## Encoder 9 end-to-end integration

Commits `336599702` (Torch/native tooling) and `891d57afc` (native player and
converter) connect the opted-in X observation to a separate encoder contract.
The global width is explicitly 82: all 80 Encoder-8 values followed by X
presence and raw selected X. Entity width remains 128, sampling revision 2.
Default Encoder 7 and archived Encoder-7/8 identities remain unchanged.
Runtime and converter use the same authorized announcement view and versioned
passive/decision hashes; sidecars require a matching producer identity.
Unsupported live, legacy-arena, shadow and LLM conversion paths reject E9.

Config, checkpoint, export, cache and native-loader checks bind the global
width and model-5/relational-2 contract. Every loaded depth graph is checked
against the declared width. An independent review found an empty-entity
fallback incorrectly using the global width and explicit legacy-width
metadata that was insufficiently checked; both are corrected and covered.
The actual Fireball regression distinguishes unset/zero/one, preserves the
Encoder-8 prefix and verifies privacy and complete converter/runtime clocks.
Those two tests pass both with default features and with ONNX enabled.
Encoder-contract tests, converter/evaluator/live-seat tests, formatting and
strict Clippy also pass.

The explicit `--encoder-warmstart-to9` transition copies weights and zero-pads
the two input columns, with fresh optimizer/scheduler/sampler/H. A tiny
supervised test learned those columns, resumed correctly, and rejected a
wrong-width resume. Frozen-trunk warmstarts are refused: freezing new zero
columns cannot teach the network to use X. E7-to-E9 also changes E8 semantics,
so it is not a clean isolated test of X. Raw f32 encoding does not preserve
every large u32 exactly; hashes bind its exact integer separately. X changes
can introduce new passive observation ticks, so zero-padding an E8 projection
also does not prove full-history recurrent behavior equivalence.

All five K0/1/2/4/8 graphs pass tiny-model CPU ONNX parity. A first synthetic
demo catalog was correctly rejected by the native compiled-rule check; that
failure remains preserved. A separate fixture exported the actual full
compiled catalog (149,014 DAG nodes, 36,170 roots), with no relaxed checks or
removed nodes. Its five-graph/15-frame parity error was at most
`5.960464477539062e-7`. Native `eval4 --deals 0` then accepted the E9 model and
completed in 8.98 s: zero games, answers, fallbacks and refusals. The fixture
uses d16/one layer/structure width 8; it is not a trained candidate or a
large-model resource result. The receipt pins the exact dirty-build ELF used,
not a retrospective claim that the binary came from the later clean commit.

Evidence: `e9-tiny-compiled-cpu-export-01a188a3-779f-7345-b733-b651cdea3eaa/README.md`.
Native report SHA256:
`1607df0a653df866c5b165cd381d4cfc18ffe530b74e2f6a0525c421d30eb7e7`.
No GPU training or live promotion occurred. The information path now works;
its playing-strength benefit and the failure-learning contract remain open.

## Failure-boundary feasibility

An offline Rust investigation replayed all four original E3 integration
records, without new games or model inference. Every canonical snapshot hash
and all 19 sealed inputs were verified. In case 64 the first recovery belongs
to the historical opponent; it cannot become a learner failure or win target.
In case 65 it belongs to the learner. Exact prefixes stop before canonical
inputs 303 and 417 respectively, at record byte offsets 57,742 and 72,601.
The corresponding evaluator decision sequences are 301 and 415: these are
different clocks, not interchangeable indices. A first helper attempt that
incorrectly equated them failed before publication and remains preserved.

The retained prefix bytes and provenance are exact original slices. Recovery
and all subsequent actions/outcomes are excluded from the hypothetical
failure target. The learner prefix in case 65 contains 127 stochastic Net
probability samples; its three forced X=0 choices retain no actor probability.
It alone does not meet the unchanged planned minimum of 128 genuine choices.
Cases 96/97 have no recovery and remain unchanged negative controls. All four
cases remain in the denominator; none is newly admitted for learning.

This makes an explicit research termination before house recovery technically
plausible. It does not implement or validate the proposed collector, target,
admission or resume contract. An eventual training utility of zero for a
certified learner announcement loop would be a declared task penalty, not a
fabricated MTG loss. Legal cancellations, opponent failures and technical
aborts need distinct handling. The protocol and required fixtures are in
`failed-announcement-next-protocol-01a188a9-e814-7508-b0fe-7a88e06cbedc/protocol.md`.
The successful replay took 0.518 s and 44.2 MB peak PSS, without GPU work.
Evidence: `failed-announcement-gate-a-01a188ae-1c79-7780-a732-dd380715ffdf/review.json`;
raw report SHA256
`5b954c630e246577175618f66a370557d941e3e5e852534cda9ac85776bf9c29`.

## Verification status

Targeted native and PyTorch checks cover the new contracts, rollback, complete
history, native metric parity and relative-path metadata publication. The CPU
PyTorch suite completed 226 tests (six GPU-specific tests skipped); all six
were then run explicitly under the exclusive GPU lock and passed. The native
release binaries also build successfully. The late native execution-admission
change passed its targeted Rust and Torch boundary tests separately.

The first full default gate ran 23,906 tests: 23,905 passed and one exposed an
obsolete expectation that games survive loss of their owner agent. The feature
gate found the same expectation. Its investigation uncovered a real deployment
ordering issue: stopping the agent to drain now ends its engines. The fix holds
admission and drains before stopping/replacing the owner, with explicit force
behavior. Reboot also requires a fresh held-admission count and preserves foreign
holds. Old staged releases without pinned owner binaries are refused before
stopping the owner. The first upgrade must install the corrected ops executable
before using it: an already-running old deployment command still has its old
stop-before-drain behavior. No live deployment was performed here.

The corrected local-agent/lifecycle integration suite passes all seven tests.
The reboot command fixtures and the touched deploy health fixture now use a
non-shipped Rust executable, including the new admission-race behavior; no new
shell implementation remains in those tests. The ops package passes all 57 tests
and strict Clippy. A test that inspected the moved deploy module was also updated
to its new path and passed independently.

The full feature gate completed successfully on the corrected lifecycle source
(`328f137ea`); the subsequent test-fixture/path changes passed their targeted
checks. The final default gate completed on `31e6a78c0`: all 23,916 tests passed,
27 were skipped, and formatting, strict Clippy and API checks passed. The optional
catalog validation step was skipped because this checkout has no
`data/scryfall-cache`; no full code generation was run.

Those full gates predate the payment-weighting fixes above. The later fixes
passed their focused engine/converter, objective, resume and Clippy checks;
the full gates must be rerun before pushing this newer source. No push or live
model promotion was performed. After all builds stopped, only the regenerable
`target/debug/incremental` directory in this worktree was removed to recover
disk space (about 104 GiB free afterward); models, records, source and pinned
binaries were retained.

Receipts under the investigation root:

- `gate-full-native-fixtures-20261029.log`: final default gate, exit 0.
- `gate-features-after-lifecycle-20261029.log`: feature gate, exit 0.
- `torch-tests-complete-20261029.log`: 226-test suite, six GPU cases skipped.
- `cuda-contracts-01a187cc-aa57-79ce-9c52-7cff640ce6b6/job.json`: all six GPU
  cases passed under the exclusive GPU lock.

Earlier failed gate receipts remain preserved; they are not counted as passes.
The branch is committed locally; this report does not claim a push or live model
deployment.
