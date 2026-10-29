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
The next check compares the original seed-33002 pair in predeclared, seat-swapped
games against Sharp. Choosing that pair before gameplay avoids selecting the
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

Receipts under the investigation root:

- `gate-full-native-fixtures-20261029.log`: final default gate, exit 0.
- `gate-features-after-lifecycle-20261029.log`: feature gate, exit 0.
- `torch-tests-complete-20261029.log`: 226-test suite, six GPU cases skipped.
- `cuda-contracts-01a187cc-aa57-79ce-9c52-7cff640ce6b6/job.json`: all six GPU
  cases passed under the exclusive GPU lock.

Earlier failed gate receipts remain preserved; they are not counted as passes.
The branch is committed locally; this report does not claim a push or live model
deployment.
