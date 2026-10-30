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

The next predeclared collection is
`failure-terminal-breadth-01a18b06-115f-7be2-944c-0e6a4ac95fea/` under the
same investigation root. It uses original R6 as learner, with four historical
candidate1 tables alternating with four frozen-R6 self-play tables. Both focal
seats are retained per table: 16 planned attempts, eight correlated table
clusters, not 16 independent tables. Cases 160–175 select 16 distinct adapted
train decks at indices 32–47 of the unchanged 128-deck manifest, spanning ARC,
ATH, AVR, BFZ and BNG. Actual opening seeds are 902610300080–902610300087,
with base policy seed 902610300999. The overlap receipt checked 147 prior native
rows in this investigation root: no reused opening seed or full tuple matched.
This is fresh training collection, not a holdout or a playing-strength test.

The collection retains the proven E7/K2/temperature-0.7 raw FP32 runtime without
rebuilding or copying the models. It has no House fallback, outcome selection,
automatic refill, retry, PPO stage or promotion. Record every planned attempt,
including unknown and unstarted outcomes, and keep each seat pair in one table
cluster. Each two-attempt group has a 240-second guard and 270-second outer
stage limit; the pipeline is bounded to 2,280 seconds and its service to 2,340
seconds, with two CPU equivalents and 20 GiB host memory. The separate loader
preflight has zero games and a 150-second pipeline limit. Root reviewed the
preparation and released this preflight, followed by collection only if it passes.

The sealed collection manifest SHA-256 is
`eccdb0b845533fb05ab7c9af58225ecdf30e95dd55e7b45be036736c6dab5cc0`;
its launch map SHA-256 is
`fb98c979e283e325a08c53f2554fe88a72c08550610602d8297902fdafbdc3ac`.
The separate preflight manifest SHA-256 is
`4a1786e10531ad640d94391f638ed08ab9e32cd66c6552099f69050e7efbefc3`.
`ready.json` records the schedule, command, overlap and input-seal hashes;
20,193 frozen input files were rehashed during preparation. Preparation and
release alone are not proof of a running collector or usable training data;
follow the actual execution receipts and canonical collection pointer.

Both final-map CPU loaders passed in 18.443 and 18.447 seconds, at peak PSS
2,291,756,032 and 2,317,804,544 bytes respectively. The verifier took 3.111
seconds. Preflight cleanup confirmed all owned identities/groups exited and
the cgroup was empty; its watcher was stopped before emitting a terminal wake.
`preflight-verified.json` SHA-256 is
`a4f07b96c876fb26ed28c55b500ffa8d8a8a66b13057fda76745d915cc58e978`;
the separate cleanup receipt SHA-256 is
`f8702be685046c6db945390f946bcfdb8e78fea80235cd08c964ef9b526913f5`.
Collection then started exactly once under
`baylee-breadth-01a18b06-collection.service`: controller PID 2442449/start ticks
16553759 on the unchanged boot, with actual ELF/argv/cgroup identity checked.
The canonical collection pointer is running and its durable watcher active.
`collection-launcher-complete.json` SHA-256 is
`92d34c6edcc711816908dc3e95c7c135bf07d62a991d90a9fc674ea899edf9e8`.
This is an actual CPU collection launch, not another GPU learning update;
results still require review. The user gateway is unchanged.

The collection subsequently stopped at table 082. Its inner guard reached the
240-second deadline and finished cleanup after 243.499 seconds, with peak PSS
3,974,852,608 bytes. There was no OOM or native traceback. Case 164 had already
published a complete record; case 165 was assigned but had no completion and
remains `operational_unknown/process_deadline`, with null utility and no
bootstrap. The collector correctly finalized this ledger and returned failure;
the generic pipeline then stopped before starting tables 083–087.

The final original denominator is 16 planned, six assigned, five recorded, one
operational unknown and ten unstarted. Recorded cases are 160 natural loss,
161 learner announcement loop with utility zero and unknown MTG outcome,
162 natural win, 163 natural loss and 164 natural loss. Thus four natural
outcomes include one win and three losses; these exploratory training games
are not a strength estimate. Two of eight paired table clusters are fully
recorded, one partial and five unstarted. No conversion, input admission or
training was performed on this collection.

`collection-review.json` SHA-256 is
`c18c525b99a9d5f1e1800dd4d9f32184d084a3f67af464df9cb4c1d06b9ed448`.
Review rehashed 20,224 sealed inputs/source files and confirmed the controller
and all six outer/inner guard identities and groups had exited, with an empty
cgroup. Only the owned collection watcher was stopped. Event
`991344e21a106d3a03403769` is handled and the canonical pointer is
`failed-reviewed`; the GPU is empty and user gateway unchanged.

Continuation is restricted to the ten originally unstarted cases 166–175,
under a fresh namespace and the same original seeds/decks/opponents. Case 165
must not be retried or replaced. The original snapshot remains immutable;
a combined review must count 16 logical attempts exactly once, not add the
continuation's ten cases as a new denominator. Future independent collection
stages need an explicit accounting mode that can retain a verified, cleaned-up
deadline and proceed; generic pipeline and default collector failure behavior
remain unchanged. Missing/corrupt evidence, changed inputs, interruption,
worker errors and memory-limit failures still stop the collection.

Commit `8ca6d1b73` implements this narrow opt-in as
`collect-research --retain-deadline`. Sixteen research-accounting tests and the
CLI parser test passed, including a real guarded deadline followed by the next
independent pipeline command. The default deadline path still fails. Targeted
strict Clippy and the release build passed. The release wrapper SHA-256 is
`3dde481790c8ddab4543b7ca72b1950595cfdba8f08a743201a860e17a3dcfdc`.
Every opted-in stage must require its immutable `collection-accounting.json`;
`deadline_accounted` means the accounting is complete, not that its games
completed or were admitted to training.

The continuation was frozen under
`failure-terminal-breadth-continuation-01a18b20-6cb0-713c-af18-0a8d80e1ba10/`.
Its ready receipt SHA-256 is
`1892d99d89b2e2a549c01a458ab21dd7f942f9e7899e1823443191a3d125467d`,
pipeline SHA-256
`540f94b35a9ad2f2758c7811f78295fc7c754249db75c154eb5f32ba9ef5b4e6`
and literal launch-map SHA-256
`b9119666492152bdcaa324caf9e8887f1a38704cc0e9221e9cf3c55a86059d37`.
All 20,389 inherited/new input seals were rehashed. The unchanged model loader
checks were reused; the new wrapper's CLI passed inside the final CPU map in
0.081 seconds, with cleanup verified. Bounds are 600 seconds per inner
two-game collection, 660 per outer stage, 3,420 for the pipeline and 3,480 for
the service, with two CPU equivalents and 20 GiB. There are five independent
tables, three frozen-R6 self-play and two historical-opponent pairs, using
only original cases 166–175. Each requires explicit collection accounting;
no evaluator success summary is required after a retained deadline.

The continuation launched once after final ownership, input and unstarted-case
checks. Actual controller identity is PID 2452410/start ticks 16706331 on boot
`ebf55ae5-05a2-41fc-a994-e5da27268ced`; service
`baylee-breadth-cont-01a18b20-collection` and its `-watch.timer` were verified
active. The first observed stage was `table-083-selfplay`. Launch receipt
`collection-launcher-complete.json` has SHA-256
`752c6a386018687d096b6e3b65b11d5c7906b5c1a5b2b89ef1cb5181a1378a81`.
The canonical collection pointer names this running continuation and preserves
the original logical denominator. This is CPU data collection; no further
gradient update, model promotion or gateway change occurred.

The continuation subsequently completed all five tables in 1,342.663 seconds.
Every stage published `collection_complete=true`, with no further deadlines.
Combined accounting of original tables 080–082 and continuation tables 083–087
contains exactly 16 planned/assigned attempts: 15 recorded, one operational
unknown and none unstarted. The natural outcomes are four wins and nine losses;
one additional learner announcement loop has research utility zero. Case 170
ended in an opponent failure and remains target-free, as does original case
165's deadline. Thus 14 attempts have known research utility, including positive
returns absent from the earlier pilot; native frame/choice counts and input
admission are still separate checks. These training cases are not a strength
comparison or holdout.

`combined-collection-review.json` SHA-256 is
`15eb8b1b29e2123b703709eda72e0c22c42effb4d285f0a4d3d2f62b9f190a14`.
It rehashed 20,471 input/source seals and verified all 16 original/continuation
outer/inner guards and their owned groups had exited, along with the current
controller and cgroup. Only the continuation's owned watcher was stopped;
GPU empty and user gateway unchanged. Event `f3b1a65f5cbc32bc29ca5936` is
deduplicated in local and central receipts. The canonical pointer is
`complete-reviewed`: accounting complete, but combined collection incomplete
because case 165 remains unknown. Conversion must use the eight native table
roots named in the review, not duplicate the old aggregate's ten unstarted
placeholder rows.

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
