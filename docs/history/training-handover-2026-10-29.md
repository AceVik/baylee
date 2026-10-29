# Training takeover pointer — 29 October 2026

Operational snapshot after the completed full-model pilot and comparison,
with the larger-exposure run active. Read the
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
The current training pointer now refers to the active coverage experiment below.
The completed CPU comparison is recorded in
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

The native controller identity is PID `2310385`, start ticks `13105998`,
boot ID `ebf55ae5-05a2-41fc-a994-e5da27268ced`. At this snapshot its phase is
`full_model_ppo`, with GPU worker `2311533`; verify their live identities before
acting. The service is `baylee-full-ppo-coverage-01a18905.service` and its watcher
is `baylee-full-ppo-coverage-01a18905-watch.timer`, bound to the inner controller
and the same chat. The manifest SHA256 is
`dd49a21075193a333cd4be3f3f42c1057175c50c10f5c1f4c21bef967bec8191`.
`coverage-review.json` will distinguish retained coverage, attempted dose,
per-history coverage and final rollback. Follow `active-training.json` and
actual receipts for current state; never launch a second controller from this
dated snapshot. The completed pilot/comparison watchers have been stopped.

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

Next implementation work: join the finalized operational ledger to research
conversion, implement the versioned research cache/admission and Torch
target/resume handling, then numerical recurrent probability replay and a tiny
end-to-end test. The shared replay's Forced/Net classification now checks the
reconstructed menu rather than trusting the label; this closes a validation
gap and does not establish that the existing PPO dataset contained mislabeled
frames. Keep conversion-only artifacts unadmitted. For Terra/Sonnet live
opponents, join neural audits to official anonymous exports and prove exact
executed-action/observation correspondence. E9 additionally needs negotiated
authorized wire context; do not remove its guard to make a test start. Keep
reward, observation and learning-dose changes in separate experiments. The
new paths require their own end-to-end receipts before training admission.
