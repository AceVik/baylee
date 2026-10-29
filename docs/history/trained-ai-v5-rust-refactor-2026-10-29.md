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
