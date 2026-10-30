# Failure-terminal research learning pilot — 30 October 2026

The first real producer-to-update trial has completed CPU collection,
research conversion, native cache verification, native input admission,
full CPU tensor preflight, complete-history Torch probability replay and
exactly one accepted full-model PPO update. This is a four-attempt development contract test, with no strength
claim, holdout result, promotion or push. Read the
[training runbook](../training-runbook.md#separate-failure-terminal-research-path)
for the operational contracts and takeover checks.

The evidence root is
`/home/ace/baylee-data/v5/ops-codex-20261028/refactor-investigations-20261029/failure-terminal-real-e2e-01a18a9d-d0e0-7db4-b154-2cbae46d83fd/`.
`collection-execution/status.json` is `complete`; the canonical
`active-research-collection.json` is `complete-reviewed`. The operator review
verified owned process cleanup, stopped its watcher and deduplicated event
`775f9819e24eade089bad75f`. `collection.pipeline.json` SHA-256 is
`5e130600bc6710a005143398296f1a6755c715181c4e9c41157b7e64b0ff666a`.
The pinned `research-features/research.manifest.json` SHA-256 is
`d3ff2f0da5b180048170cc4eb3dbd22b4d0777ce37c361942829c5354a1810b9`,
recorded in `conversion-manifest-pin.json`. The conversion manifest itself is
`conversion_only_unadmitted`; neither conversion nor a native input-admission
receipt certifies numerical policy replay.

Implemented milestones are `599cfe70a` (strict Rust research tensors and
dedicated input admission), `5090f40a1` (separate admitted full-history PyTorch
research PPO worker) and `7ba44fc23` (pinned Torch interop execution threads).
Operations, preparation, conversion, cache and admission remain Rust; Python
contains PyTorch computation and its associated tests only. Native verification
passed 23 tests plus two CLI tests and strict Clippy. The 12 research worker
tests passed in 2.185 seconds. A combined 21-test research/precision suite also
passed before the later preflight addition; these overlapping suites are not
an additive test total.

Collection used raw E7/R6 FP32, K2, temperature 0.7, a frozen historical neural
opponent for table 32 and neural self-play for table 48. These are four fixed
known development cases. All attempts remain in the denominator: 4 planned,
4 assigned, 4 recorded, 0 operational unknown and 0 not started. There were no
game retries or refills. Collection stopped before House recovery.

| Case | Observed classification | Research target |
| --- | --- | --- |
| 64 | Opponent failure; MTG outcome unknown | None |
| 65 | Learner announcement loop; MTG outcome null | Utility 0 |
| 96 | Natural terminal learner loss | Utility 0 |
| 97 | Natural terminal learner loss | Utility 0 |

Native admission verified three learner target histories with 1,713 frames and
372 genuine choices. The cache manifest SHA-256 is
`d6b8c9305c352e1fba0c47ae7b06463b2e727a558a657b56715ea46d56891915`;
`research-input-admission.json` SHA-256 is
`2b5cbe08963701733050fd1e770a572e127c699d01f54a328fd7bb551d69ef1b`.
An independent native verify also passed. Opponent failures, operational
unknowns and caps remain target-free, without invented game outcomes.

This zero-return pilot makes the objective distinction concrete: use
uncentered `A = training_utility - V_source`, with frozen source values, and
train the critic against research utility. Centering within this small batch
could make a failure appear relatively positive; ordinary MTG outcome labels
cannot replace the separate utility contract. The worker must reconstruct
complete recurrent histories and reproduce original sampled probabilities at
maximum log-probability error `0.001` before an update. Successful native
packing/input admission alone cannot pass that numerical gate.

Preparation failures remain evidence. The first build was quarantined because
source changed during the build; a fresh stable build passed in 63.177 seconds
with approximately 1.53 GB peak memory. Before any games, an external Rust
helper's prelaunch receipt-name mismatch was corrected. Neither event caused
a game retry or discarded attempt. The first learning preflight under
`research-learning-preparation-01a18ab2-d4b6-77ea-b983-59f2ea5fac58/`
then failed before model forward because `trajectory_cache4` was absent from
the isolated runtime PATH. Model loading uses its native `training-contract`
validator even though research tensors come from `research_tensor4`. Preserve
the failed receipt and include this additional pinned dependency in recovery;
the existing weights, data and numerical thresholds do not need changing.
The failed worker took 9.660 seconds with 1,129,421,824 bytes peak host PSS and
verified cleanup. Runtime sealing previously took 367 seconds with a
debug-linked preparation helper; subsequent launch checks use a derivative
linked to the optimized Rust library. This is preparation overhead, not GPU
training time or a measured model speedup.

The corrected runtime is
`research-learning-recovery-01a18ad4-ea35-701e-952c-1e0191d0b9fb/` under the same
investigation root. It reuses the original frozen trainer, runtime and data;
it adds the pinned `trajectory_cache4` executable to PATH and uses fresh phase
outputs. `cpu-loader-verified.json` SHA-256 is
`1564b288b0056f7e0ad4a4ce46539acf56e404b81ba530cd0012b2f322640696`.
All 28 segments, 1,713 frames, 372 choices and three histories passed; there
were zero forwards, optimizer steps or checkpoint writes. The worker took
125.030 seconds and peak host PSS was 2,432,882,688 bytes. Its owned processes
exited and the native guard verified cleanup.

CUDA initialization then passed in 8.254 seconds with 687,463,424 bytes peak
host PSS. The complete-history audit checked all 372 genuine choices in all
1,713 frames, three histories and 28 segments. Maximum log-probability error
was `0.000002384185791015625`, below the unchanged `0.001` tolerance. The
frozen source values ranged from `0.0019812833052128553` to
`0.8093113303184509`; mean uncentered advantage was `-0.3581425519779034`.
No bootstrap or advantage centering was used. The audit took 136.376 seconds
with 3,059,859,456 bytes peak host PSS. Its independently verified receipt is
`audit-verified.json`, SHA-256
`84ef1675940113c3eaea6cc135c6b5c5fdf3a6f0d682280811055b64e9073c16`.

The frozen `update.pipeline.json` SHA-256 is
`30fcb79d31859d97fdb957ecbfe43e507abbcc6e11b9864f63e86ef4bf09697e`.
Its worker completed with exactly one attempted and accepted update, using
**61 actor choices in 256 frames**. Full replay coverage is not the update
dose: this is one minibatch, not a full epoch. B4/U64, K2, temperature 0.7,
LR `2e-5`, fresh AdamW, FP32 and the target KL `0.02` were unchanged.
Final whole-collection behavior KL was `0.0000772554598370577`; the final
gate accepted the checkpoint. Gradient norm was `1.1445891857147217`,
minibatch loss `0.4461709856987008`, and 301 parameter tensors changed,
including policy components, the relational module and the value head.
The worker took 178.610 seconds with 3,812,177,920 bytes peak host PSS.
The native pipeline completed in 198.774 seconds, including its surrounding
checks. The report contains no numeric policy/value parameter-delta metric
beyond changed tensor names/count and policy KL.

Published research artifacts under `update/output/`:

| Artifact | SHA-256 |
| --- | --- |
| `net.pt` | `368b72d86b03a8a18f01c56378dd247f3ff8520e14dbf6e70d128288c6943127` |
| `last.pt` | `2fc683e3f767c25f89ea13446725defaac4d603f1d34f920eb41bbb2f5bb985a` |
| `report.json` | `8fcda476e93b2ef8df37fe578e8da451596f632b6cf5c3934061988bc0d8b76c` |

`update-verified.json` SHA-256 is
`deb4ce485399c18700365b24c38441751529909d432869eb32856926233f5884`.
It verifies original inputs, prerequisites, outputs and owned-process cleanup.
Controller PID 2427096/start ticks 16219480 and worker PID 2427142/start ticks
16221247 exited; identities belong to boot
`ebf55ae5-05a2-41fc-a994-e5da27268ced` and must never be treated as reusable
signal targets. The GPU is empty and its shared lock is released. All four
phase watcher timers are inactive; event `ae1095cb47a2bebeefac316a` was handled
in both central and local ledgers. The active training pointer reports
completion with `training=false`.

This establishes an actual bounded weight update from the separate research
utility contract. It does not establish better play or a suitable long-run
training mix. All three target histories have utility zero. Repeating this
batch alone could encourage excessive passing or other unhelpful avoidance;
next inspect matched gameplay against fixed opponents before increasing dose.
The value head now learns research utility, not solely MTG win probability;
retain that provenance in any export/comparison. Raw policy evaluation does
not validate that value head for live draw-offer decisions or model promotion.

The follow-up was prepared and started under
`research-update-paired-gameplay-01a18ae5-3467-7853-ac5e-3d42fc1d3913/`.
It uses fresh source and candidate attempts on cases 64/65/96/97, with the
same historical candidate1 opponent for 64/65 and frozen original R6 for 96/97.
Both arms use the historical-opponent provenance role; candidate-versus-candidate
would change the opponent and would not isolate the learner update. The fixed
plan is eight attempts, raw FP32 K2, temperature 0.7 and original seeds/decks.
Every attempt remains reported, with research utility distinct from MTG outcome.

The CPU export pipeline completed. All five ONNX depth graphs passed 47 parity
frames; maximum absolute difference was `0.000041544437408447266` against the
`0.001` limit. Export manifest SHA-256 is
`1130bf06be4805fe996e8f3ef0124458fa2ae76e6c6a7340c4292dcfcc25102c`;
the K2 graph SHA-256 is
`1a99b5189d124296812248905bc784de5e32ee9dcbb58a4903995913fae3a8cf`.
The exporter worker took 65.768 seconds and about 4.84 GB peak host PSS;
the complete export pipeline took 155.359 seconds. These are distinct
measurements: the difference includes native supervision, sealing and the
additional verification stage, not just model computation. Final operator
review rehashed 41,778 seals and verified owned-controller/worker/cgroup exit,
stopped the export watcher and handled event `ae6badce4982438e170d2c59`.
Its review SHA-256 is
`12b380c10461eab7bb0a53e9cd73ca934af8cc7d67949baa0dd5fe35ec3e7ed0`.
The final launch map explicitly pins the approved `trajectory_cache4` on PATH;
the earlier preparation draft is retained. `export-verified.json` retains the
research-utility critic semantics omitted by ordinary export metadata.
Export success alone does not establish a gameplay result. The bounded CPU
comparison started once under `baylee-research-gameplay-01a18ae5.service`;
controller PID 2433058/start ticks 16369208 belongs to the same boot above.
Its gameplay manifest SHA-256 is
`71bbbb6614cdc371429528d40e1c66302cec26a1940ea726bcd60ee2b35b69c1`.
The initial observer attach window elapsed during native input verification;
the existing service remained active and was attached without relaunching it.
The canonical CPU pointer and durable watcher track this owner. The pipeline
has a 1,260-second limit, its service 1,320 seconds, two CPU equivalents and
20 GiB memory, with no GPU stage. Each of the four two-attempt groups is bounded
to 240 seconds; retain incomplete or unstarted planned attempts without retries
or refills.

The comparison and its separate bounded report completed. All eight attempts
were assigned and recorded, with zero operational unknown or unstarted cases.
Each arm retained one opponent failure with unknown MTG outcome and no utility
target, one learner announcement loop with utility zero, and two natural MTG
losses with utility zero. All four candidate canonical input/action/state-hash
sequences were identical to the fresh source; all four source records also
reproduced the original data exactly. Each arm executed 816 learner inputs,
including 611 `PassPriority`, 20 `CastSpell` and zero `CancelCast`. These are
executed-input counts, including forced actions, not genuine-choice rates or
evidence that passing was optimal. The learned probabilities changed but this
bounded update did not alter executed behavior or fix the loop on these cases.

The gameplay pipeline took 662.289 seconds; reporting took 25.685 seconds.
`paired-review.json` SHA-256 is
`ff320ea9dd2752a16f3abc5d69a9b4615d34bc1acc5a438bd567418b2d6ba9b2`;
the action comparison SHA-256 is
`3a4e01143232a495339fd797e598b49edd5f288fbdb97809743f2851b1bcd0c3`.
Final review rehashed 41,908 seals and confirmed all owned controller, guard,
worker and report identities/groups exited and the cgroup was empty. The
gameplay watcher was stopped and event `a513a6a1bff9b7e06cb82b04` handled.
`reviewed-events/a513a6a1bff9b7e06cb82b04-gameplay-review.json` SHA-256 is
`daf77c2146ea2a8e6d38549cab1abaf7ce459cc1b094036b6de93f6f2bb3e4e6`.
The CPU pointer is `complete-reviewed`, GPU empty and user gateway unchanged.
These four reused development cases establish neither improved strength nor
equivalence. Do not increase dose by repeating only the three zero-target
histories. The next collection should predeclare a broader opponent/deck/seed
schedule and retain its outcomes without selecting wins or replacing failures.

The export review also identified a provenance bug: ordinary ONNX metadata
dropped the research critic's target semantics. Fix `0fb67c18f` preserves an
explicit `critic_semantics` marker, rejects malformed/conflicting checkpoint
objectives and parses the marker into native model identity. Research critics
remain usable offline; the live adapter refuses them before initialization
because its draw rule uses an MTG outcome scale. Thirty native identity/live
tests, 23 Torch export/worker tests and strict targeted Clippy passed. An orphan
research-progress rejection was also checked by its targeted test. Existing
unmarked archival exports remain compatible; this already frozen research
comparison retains external provenance and must stay offline. This fix changes
future exports, not the measured graph or the running user gateway.
Follow-up `2ace2ab9b` also refuses incompatible critics in the preview publisher
before replacing the live selection. All seven publication tests and strict
Clippy passed; the new regression uses correctly resealed evaluation metadata
and verifies the existing selection's bytes stay unchanged on refusal.

The frozen failure-terminal collector supports only historical neural and
neural self-play opponents. Its producer verification requires a model identity
for every seat. House and external LLM opponents therefore need an explicit
role/provenance contract extension; they cannot be admitted by relabeling a
House seat or adding an evaluator flag. Broader collection with the currently
supported neural roles is separate from that later extension.

Sol's read-only performance review identified hypotheses to measure after this
trial, not demonstrated speedups. Starting the tensor server and final native
verification each reconstruct admission, scan and pack frames; original
producer inputs can be hashed repeatedly across roles. Segment requests also
read/decompress/pack frames again. A dummy lane currently requests segment zero
before producing zero tensors. Commit `980427a96` adds worker host phase timings
for future runs, with 14 passing CPU tests; it does not modify this frozen
trial. Unique/repeated byte and request counters remain future measurements
around admission, reader validation, handshake, tensor loop, final verification
and checkpoint hashing. Preserve final content revalidation
when considering reuse; the aggregate 125-second measurement does not tell us
which component dominates.

The earlier coverage PPO experiment remains a technical success with 27
accepted updates and 311 changed tensors. Its 16-case development comparison
still has 4 wins/12 losses, matching original R6 without measured strength gain.
The old continuous BC controller stays intentionally stopped, and the user
gateway on port 28766 is preserved.
