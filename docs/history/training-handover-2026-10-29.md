# Training takeover pointer — 29 October 2026

Operational snapshot after the completed full-model pilot, initial comparison
and larger-exposure training run. Read the
[training runbook](../training-runbook.md) for commands, contracts, research
findings and handover procedure. This is a pointer to durable evidence; actual
status can advance after this note.

The old continuous BC recipe remains stopped. The **bounded full-model E7/R6
FP32 PPO pilot completed successfully**: eight attempted and eight accepted
updates, 311 of 566 changed tensors, all published tensors finite, final
whole-collection KL `0.00004005224838611874`. Changes include the trunk,
relational module, recurrent memory, thinking block and action heads.
It used original R6, fresh Adam and the unchanged eight admitted games.

The differentiated update groups contained **131 actor choices in 512 frames**;
all 1,575 choices in 6,626 learner frames were replayed for correctness audits.
This is a bounded prefix dose, not a full epoch or evidence of better play.
B2/U32, K2, temperature 0.7, LR `2e-6`, target KL 0.02; at most eight attempts
and 1,800 update seconds. Actual guarded training took 988.33 seconds. Peak
allocated/reserved CUDA bytes: 2,274,077,184 / 2,696,937,472, with zero allocator
retries; peak host PSS 3,579,414,528 bytes. Controller and worker exited.

Current machine-readable pointer:
`/home/ace/baylee-data/v5/ops-codex-20261028/active-training.json`.
The experiment directory is
`/home/ace/baylee-data/v5/ops-codex-20261028/refactor-investigations-20261029/full-model-ppo-pilot-01a188cc-2d9c-7328-9639-aaf5b5af02f7/`.

| Evidence | Current binding |
| --- | --- |
| Plan | `plan.json` in the experiment directory |
| Native plan | `pilot.pipeline.json`, SHA-256 `c7cef67bab35f7655dda64363e241c63284c47314ff1a30cce2706dc87338e97` |
| Controller state | `execution/status.json`, `complete`; `review.json` independently reverified pipeline seals and records the result |
| Former controller identity | PID `2282235`, start ticks `12728937`, boot ID `ebf55ae5-05a2-41fc-a994-e5da27268ced`; verified exited, never signal a reused PID |
| Systemd unit | `baylee-full-ppo-01a188cc.service`, no automatic restart |
| Guard receipt | `execution/jobs/full_model_ppo/job.json`; complete, return code 0 |
| Candidate | `publication/net.pt`, SHA256 `132ddc3c8f4e791f48c95d2d2ebfddc89281e2909d367581083d9ff8313c44b7`; `publication/audit.json` contains changed tensors, progress and training contract |
| Tensor source | Frozen source revision `07e6b6dcb7830ffd566dd1a0a4b049b846435db3`, copied with `git show` into a separate Git-identified trainer snapshot; working checkout changes do not change these sealed inputs |
| Original R6 checkpoint | `e1-paired-gameplay-preparation-01a187df-d6ab-75fd-bda4-ebf073f2cf38/models/r6/net.pt` under the investigation root; SHA-256 `dac99a0c2bfa0612728e7909b56d75a6ec0b50cbb7538316cd8604c391f42827` |
| Dataset | `pre-e3-conversion-admission-01a1884b-fb13-776b-8e14-47079f3a6013/dataset` under the investigation root |
| Admission | `pre-e3-admission-recovery-01a1884d-10f6-76ae-a347-18aa5aa3c4ed/admission.json` under the investigation root |
| Resource boundaries | Exclusive `/home/ace/baylee-data/v5/experiments/gpu-work.lock`; bounded CUDA allocation at 11 GiB and host memory at 20 GiB; inspect actual receipts for measured peaks |
| Watcher | `baylee-full-ppo-01a188cc-watch.timer`, 30-second cadence, `execution/watchdog/config.json` |
| Owning chat | `01a16325-d29f-7ad1-9625-cbb5bf2cb1a4`; success requires phase `complete` and `execution/result.json` |

After the pilot, its reviewed state was `complete-reviewed`, `training=false`.
The current training pointer now refers to the reviewed coverage experiment below.
The current CPU comparison is recorded in
`/home/ace/baylee-data/v5/ops-codex-20261028/active-cpu-followup.json`. Inspect
its actual `execution/status.json`, controller identity and stage receipts
before taking any new action. The bounded follow-up compared candidate/R6
on 16 fresh-seed development cases
per model, eight seat-swapped tables, against Sharp with matched Steady baseline.
It includes CPU ONNX parity, zero-deal identity/deck checks and native paired
comparison, two evaluator threads, no GPU and a 30-minute overall bound.
All caps, refusals and interventions remain reported. No training admission or
promotion; fresh seeds on known families are not an untouched strength holdout.

First follow-up attempt `full-model-followup-eval-01a188dc-5fe2-7cde-8fe1-c77bcf65ab44`
failed with `Permission denied` during the export guard's preparation, before
any worker or gameplay started. Its exited controller and original artifacts
are preserved. All 7,006 input hashes passed; the enforcing AppArmor profile
denied writable `/dev/null` access needed to spawn Git for source identity.
Adding `bwrap --dev /dev` provides private standard devices while keeping the
read-only source/input mounts and enforcing profile. Exact-map diagnostics
verified the cause and fix before another attempt.

The fresh recovery is
`full-model-followup-eval-01a188e2-a5ee-79f8-b683-a38d787724fd` under the same
investigation root. Its 16 cases, seeds and candidate bytes match the preserved
attempt; all 7,013 sealed recovery inputs passed the final mount-map preflight.
It was launched as `baylee-full-ppo-eval-01a188e2.service`, no restart,
20 GiB cgroup limit and 1,830-second outer bound. Its `eval.pipeline.json`
SHA256 is `ef3a3e532f10149e4e2bf4b7b844e4fcf387fbbf01775007645d544d84bac7f9`.
The recovery **completed**. Both models won the same four cases and lost the
same twelve: paired better/equal/worse = 0/16/0. Each needed one repeated-view
House recovery in case 8; there were no caps or refused answers. The matched
Steady baseline had mean utility 0.5; both neural arms had mean utility 0.25.
All cases remain included, with interventions reported separately. Eight table
pairs do not establish equivalence or general strength, and the bootstrap's
degenerate zero interval must not be interpreted as certainty.

The canonical records also show changed behaviour: 7/16 input sequences
diverge, while 9/16 are identical. Initial headers match and each decoded
record digest was verified. `action-comparison.json` records first divergence;
this is structural comparison, not fresh engine replay or evidence of improved
play. Shard index offsets/lengths address compressed gzip members, not decoded
bytes; decode each indexed member before checking its canonical record hash.

Former inner controller identity: PID `2297908`, start ticks `12884769`, boot ID
`ebf55ae5-05a2-41fc-a994-e5da27268ced`; verified exited. The outer service PID
was not the pipeline owner. The watch event is `a3f6616e85ec69e59c679c21`.
`review.json`, `paired.json` and `reviewed-events/` contain the final report
and operator verification. The old failed namespace attempt is still retained.

The next experiment tests **actual learning exposure**, separately from new
observation/reward contracts: original R6 and the same eight admitted games,
fresh AdamW, B4/U64, one epoch / at most 32 attempts / 3,600 GPU-stage seconds.
It is a new bounded experiment, not an exact optimizer resume of B2/U32.
The wider batch/unroll needs its own measured memory result; the small pilot's
peak does not certify it. No automatic shape fallback, KL relaxation or retry.
It **started at 19:57 CET on 29 October** in
`refactor-investigations-20261029/full-model-ppo-coverage-01a18905-7895-7f03-a935-b6da0046fc79`.
The CPU loader and native schedule preflight passed: 108 unique segments,
27 update groups, 6,626 valid frames and 1,575 actor choices over eight learner
histories. This is the scheduled dose; actual retained exposure is reported
after training and may be smaller after rejection or timeout.

The coverage run **completed**, with all 27 attempted updates retained and all
1,575 actor choices / 6,626 learner frames / 108 segments covered once. All
eight learner histories have full retained exposure. No update was rejected
and no final rollback occurred. The final whole-collection KL is
`0.00007969814658532071`, with 311 changed tensors and finite publication.
The GPU stage took 2,638.27 seconds; total pipeline time was 2,666.73 seconds.
Peak CUDA allocated/reserved bytes were 3,488,394,752 / 4,347,396,096, with zero
allocator retries; peak host PSS was 3,591,371,776 bytes. This certifies the
observed B4/U64 experiment, not larger batches or Commander workloads.

Former native controller: PID `2310385`, start ticks `13105998`, boot ID
`ebf55ae5-05a2-41fc-a994-e5da27268ced`. The controller, GPU worker `2311533`
and all stage children were identity-checked as exited. The service was
`baylee-full-ppo-coverage-01a18905.service`; its watcher timer is stopped after
review. The manifest SHA256 is
`dd49a21075193a333cd4be3f3f42c1057175c50c10f5c1f4c21bef967bec8191`.
Candidate `publication/net.pt` SHA256 is
`b92023fd82fd2bac9574e64e4018d219019cec5a0fa7aef08aefd7af82059c74`.
`coverage-review.json` distinguishes retained coverage, attempted dose,
per-history coverage and final rollback. The event
`eeeb4947cfd354609d789ab8` is deduplicated in `reviewed-events/`; the independent
operator review reverified every pipeline input/output seal and actual update
counts. `active-training.json` now says `complete-reviewed`, `training=false`.
These are technical learning results; the comparison below found no improved
playing strength on its small development suite. Follow actual pointers
before any new operation.

The separate coverage-candidate CPU comparison completed under
`full-model-coverage-eval-01a18931-d9af-72e9-9a84-de9606a4b345/`. Its former
inner controller was PID `2330494`, start ticks `13431510`, on the same boot.
The controller and all stage children are identity-checked as exited; the unit
`baylee-full-coverage-eval-01a18931.service` completed successfully and its
watcher timer was stopped after review. The immutable
manifest SHA256 is
`45e0cae4466fc8b5d6b3208a5c4ce3bc9073d5aa5bc891052da6cd76cf82d506`.
The launch receipt seals all 7,231 inputs and binds the actual controller's
executable, argv and cgroup. Limits: two CPU workers, 200% CPU quota, 20 GiB
service memory, 1,800-second pipeline / 1,830-second service deadline; no retry
or GPU use. Export parity and the zero-control check passed. Export verified 47 actual frames with maximum error
`1.53e-5` and produced K=0/1/2/4/8 variants.

Only 16 candidate cases are played anew. Exact R6 and initial-pilot references
are reused from the preceding comparison after verifying engine, decks,
seeds, sampling, record bytes and settings. These are repeated development
cases, not a fresh holdout or proof of general strength. The native comparison
retains outcomes, caps, interventions and canonical action differences for
all three arms. No deployment change follows automatically.

The completed comparison has 4 wins / 12 losses for **each** of R6, tiny pilot
and coverage candidate, with no better/worse paired outcomes. All three have
the same single House intervention in case 8 (seed `902610299104`, focal seat
0): repeated Priority view, decision 264 / record input 265, accepted
`PassPriority`. There are no caps, refusals or stalls. The coverage candidate's
canonical sequences differ from R6 in cases 7/9/10/11 and from tiny in eight
cases; this demonstrates changed behavior without a measured outcome gain.
The bootstrap `[0,0]` interval is degenerate for identical observed differences
and does not prove equivalence on eight independent tables. Future broader
strength comparisons need a frozen candidate, a predeclared sample budget and
fresh independent tables, retaining a separate final seed/deck holdout.

Total pipeline wall time was 803.95 seconds, of which candidate gameplay took
576.00 seconds. Operator review revalidated all pipeline input/output seals,
48 arm-case rows, unchanged denominators and paired counts. Event
`0350274d30e2f5f574e7fbbf` is deduplicated in this run's `reviewed-events/`;
`active-cpu-followup.json` now says `complete-reviewed`. No longer repetition
of the same eight-game recipe or model promotion follows from this result.

The [performance review](trained-ai-full-ppo-performance-2026-10-29.md)
documents repeated full-collection audits and prefix reconstruction. Their
first partial runtime measurements are retained. Its fixed-weight probe under
`full-ppo-execution-probe-recovery-01a1895c-fd11-7234-9dc3-47486276272f/`
stopped at the unchanged hidden/value parity gate. Controller PID `2351100`,
start ticks `13706570`, and worker `2351118` have exited; their owned group
is empty, and only `baylee-ppo-execution-probe-01a1895c-watch.timer` was stopped.
Frozen trainer commit is `0d3be58fa62f4606f38c5ff2e53cf74abb5faf60`; manifest SHA is
`f72270b2a8a8c9911853452c4a8b5faf5f48ae5d304ed5d4b8498d73b74cebdb`.
It compares independent audit lanes and board encoding chunks, preserving
all 1,575 recorded decisions and 6,626 recurrent observations. It constructs
no optimizer and does not change the B4/U64 optimizer contract. It does not
yet measure a full update transaction or establish backward/Adam capacity.
Limits are 900 seconds for the GPU stage, 930 for the pipeline and 960 for the
service, with 20-GiB host memory and an 11-GiB Torch allocator cap. This is a
planned eight-configuration diagnostic with zero warm-ups and one pass per
configuration; only source B4/chunk0 and B4/chunk4 were executed. Baseline
passed in 38.454 seconds; chunk4 retained a failed comparison in 22.329 seconds.
Both covered all histories/frames/choices. All action LPs passed (maximum delta
`7.62939453125e-6`), KL delta was `1.8162543779359863e-14`, and acceptance
was unchanged. Hidden/value parity failed in histories (2,1) and (8,1).
The report lacks the exact failing frame/coordinate, so it cannot distinguish
a local numerical/layout difference from accumulated recurrent drift. No
candidate or lane8 result exists, and no optimizer was constructed.

Event `1a77bb8a3064b6c296d734ce` is reviewed/deduplicated; all 271 input seals
remain unchanged. Full review is in the sibling operations root
`perf-failure-review-01a18964-1a34-7002-befe-3e99dd49938d/operator-review.json`.
The rejected variant's cold, instrumented timings are not a validated speedup.
Keep chunk4 rejected and the original tolerances unchanged. A separate bounded
source-only diagnostic compared per-frame/chunked boards using common
reference H versus independently evolving H and retains the first exact
violation. Its root is
`recurrent-chunk-diagnostic-01a18967-f3e9-7745-9b23-259cd24c198a/`, frozen
from reviewed commit `556812a7b7d534da4fce0845564ff6d59d51aa5f`; 41 focused
CPU tests pass. Its complete CPU preflight took 23.65 seconds and its exact
CUDA prerequisite took 2.18 seconds; all prerequisite owners exited and
groups were empty before launch. Root independently checked the seals and
started it once. Controller `2356879`, start ticks `13833716`, boot
`ebf55ae5-05a2-41fc-a994-e5da27268ced` completed; it and both guarded
children exited with empty owned groups. Only watcher timer
`baylee-recurrent-chunk-diagnostic-01a18967-watch.timer` was stopped.
Manifest SHA256:
`3315ec3e17ed485ba664e02f67509ab3c0f3dc0d65efeb3401525ad2ccde86e1`.
Bounds are 300 seconds GPU stage / 330 pipeline / 360 service, 20-GiB host,
11-GiB allocator, four threads. Every receipt keeps `diagnostic_only=true`,
`parity_accepted=false` and zero updates/model writes. Its first violating
coordinate is game 2 / seat 1 / cursor 405 / original segment 52 / t21 /
lane 1 / H[6,535], after a compared prefix of 1,622 frames and 384 choices.
Identical incoming H still fails by 1.35505 times the allowed difference;
contiguous board storage gives the same result. Independently carried H
fails by 1.38913 times the allowance. Accumulation and storage stride alone
therefore do not explain the problem. The exact encoder/kernel cause remains
unisolated. All original recorded probabilities passed separately (8 histories,
6,626 frames, 1,575 choices); source model state is unchanged.

The diagnostic took 76.34 seconds and is `complete-reviewed`. All 284 input
seals were rechecked; event `a038b872a495d0ec97fc1786` is deduplicated at
`reviews/a038b872a495d0ec97fc1786/operator-review.json` under the parent ops
root. See the [performance report](trained-ai-full-ppo-performance-2026-10-29.md)
for exact tolerances, outputs and report hash. The subsequent probe uses
CPU-computed C/P/recorded-K hints with unchanged per-frame board arithmetic;
training defaults remain unchanged pending full source/candidate parity.
Consult `active-performance-probe.json` for current ownership; training and
gameplay comparison remain completed.

The subsequent CPU-execution-hint probe was launched once at
`full-ppo-cpu-hints-probe-01a1897a-6810-797c-928e-e4fade5119e6/`, using
reviewed worker commit `051a814f76b3e25bebf48d0ebaea7e0150b748b2` after
45 passing affected tests. Full CPU preflight and the exact CUDA
initialization/allocation prerequisite passed; their owned groups were empty.
Controller `2360605` / start ticks `13902010` on the same boot completed
the GPU stage and native verification successfully.
Manifest SHA256 is
`8b35705ba43b56ffa6cb03de541d180af7d3e7e51a3c173a94d3ddbd9b3008bf`.
This fixed-weight experiment compares baseline/hints on both original R6 and
the coverage candidate at B4/U64/chunk0; it performs no training updates.
Budgets: 600/630/660 seconds for GPU stage/pipeline/service, 20-GiB host,
11-GiB Torch allocator, four threads. All four full-history comparisons passed
with zero H/value/action-logprob/KL differences and unchanged weights. Original
R6 audit times were 38.923 versus 38.067 seconds; candidate times were 38.501
versus 38.221 seconds. These single fixed-order measurements do not establish
a repeatable speedup; keep hints opt-in and prioritize the missing research
data/tensor/admission contract over more performance probes. Worker elapsed
218.96 seconds. Final operator review verified all 283 inputs and all native
seals, exited controller/children, empty groups/cgroup and empty GPU. Only the
owned watcher timer was stopped. The pointer is `complete-reviewed` and event
`6b983b9741589b41a3ec2e9a` is deduplicated; see parent ops
`reviews/6b983b9741589b41a3ec2e9a/operator-review.json`. The preserved
preparation reservation belongs to this completed root. No automatic restart,
promotion or training-mode change followed.

Two earlier pre-model failures remain intact and reviewed. CUDA needed a
private `/proc` mount to write its own thread name; then the allocator cap
needed an explicitly resolved CUDA device index. The final prerequisites now
exercise complete CPU loading/collation plus the actual frozen CUDA helper,
memory cap, tiny allocation, synchronization and cleanup. Both pass. The
failure event IDs `1cb76501b759543dd394978c` and `c8f2bfbe96d7bf4a7b2dc2cc`
are already deduplicated under their respective run roots; delayed delivery
must not restart those attempts or replace the active pointer.

If another agent takes over after a quota limit, inspect these pointers before
launching anything. Review acceptance, consumed dose, whole-collection KL,
changed tensors, memory/time and owned-process cleanup.
Keep a bounded prefix or rejected/unchanged publication explicitly identified.
Failure recovery requires a fresh UUIDv7 attempt and preserved original evidence;
no automatic retries, KL relaxation or winner filtering.

The user gateway on port 28766 and live selection are preserved. There is no
automatic promotion and no push. Both repository gates are required before a
later push; earlier gate results do not cover all subsequent source changes.
Parallel Rust milestones are committed locally:

- `631ddec8f`: stochastic E5–E7 live decisions, durable generated/sent audits,
  independent policy RNG and probability/history/model identity. Twenty-one
  live-adapter and 17 seat tests plus focused Clippy pass. E8/E9 guards remain;
  local audits alone are not anonymous exports or admitted PPO data.
- `1d81843fe`: failure-terminal collection stops before house recovery, binds
  controlled-role replay and rejects research data in ordinary conversion.
  Gate A v2 verifies all four original cases: 64 opponent failure/unknown,
  65 learner loop with separate research utility zero and unknown MTG outcome,
  96/97 natural terminal controls. Seventeen evaluator and eight converter
  tests plus focused Clippy pass. See the [boundary report](trained-ai-failure-terminal-native-boundary-2026-10-29.md).
- `333d937b5`: durable research attempt plans/claims/completions and native
  `collect-research`/`finalize-research` supervision. Nine ledger tests cover
  assigned unknowns, incomplete evidence, immutable seal recovery and an actual
  bounded child timeout. Verified owned-process-group cleanup is required;
  escaped descendants and actual OOM termination were not certified by those
  tests. See the [attempt-ledger report](trained-ai-research-attempt-ledger-2026-10-29.md).
- `27e69301a`: separate `convert4 research` observations/returns with canonical
  proof and reconstructed forcedness checks. Twenty-two converter library,
  ten converter CLI and seventeen evaluator tests passed, plus focused final
  research regressions and combined Clippy with tests. Gate A v3 preserves the
  same four classifications. Complete index v2 inputs only; no operational
  abort-ledger ingestion, ordinary targets/cache or PPO admission. See the
  [conversion report](trained-ai-failure-terminal-research-conversion-2026-10-29.md).
- `7b4f7e936`: explicit `convert4 research --attempt-ledger-runs` adapter and
  shared read-only final-ledger verification. Fourteen converter and eleven
  operational-ledger tests plus both focused Clippy commands passed. A real
  failed Rust child fixture preserves 5 planned / 4 assigned / 3 recorded /
  1 operational unknown / 1 not started, including orphan/torn evidence, and
  emits only two known-return histories. All unknowns remain target-free.
  The original complete-index path remains intact; the new manifest is v2.
- `7235001dc`: lossless native research-frame storage with externally pinned
  conversion SHA, original producer/guard/catalog verification and exact random
  frame readback. Six cache tests, actual CLI test, six ordinary-cache tests,
  bounded actual producer/converter/cache integration and both focused Clippy
  commands passed. The integration preserves the same 5/4/3/1/1 accounting.
  This is storage only: no tensor packing, numerical policy replay, producer
  executable certification or PPO admission. Reuse the Rust reader for bulk
  access; the single-row CLI is diagnostic. Commands and boundaries are in the
  [runbook](../training-runbook.md#separate-failure-terminal-research-path).

Next implementation work: implement the strict Rust research tensor boundary,
dedicated admission and Torch target/loss/resume handling, then numerical
recurrent probability replay and a tiny
end-to-end test. The shared replay's Forced/Net classification now checks the
reconstructed menu rather than trusting the label; this closes a validation
gap and does not establish that the existing PPO dataset contained mislabeled
frames. Keep conversion-only artifacts unadmitted. For Terra/Sonnet live
opponents, join neural audits to official anonymous exports and prove exact
executed-action/observation correspondence. E9 additionally needs negotiated
authorized wire context; do not remove its guard to make a test start. Keep
reward, observation and learning-dose changes in separate experiments. The
new paths require their own end-to-end receipts before training admission.
