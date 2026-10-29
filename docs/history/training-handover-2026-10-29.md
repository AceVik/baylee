# Training takeover pointer — 29 October 2026

Operational snapshot checked at 17:55 UTC. Read the
[training runbook](../training-runbook.md) for commands, contracts, research
findings and handover procedure. This is a pointer to durable evidence; actual
status can advance after this note.

The old continuous BC recipe remains stopped. A **new bounded full-model E7/R6
FP32 PPO pilot is running**, separate from the completed head-only update and
from E9/failure-reward experiments. It starts from original R6, fresh Adam,
the unchanged admitted eight games and 1,575 genuine learner stochastic
choices. Limits are B2/U32, K2, temperature 0.7, LR `2e-6`, target KL 0.02,
at most eight attempted update groups and at most 1,800 update seconds.
Eight updates do not cover all trajectory segments. The question is technical
learning/resource feasibility; no playing-strength or long-run claim follows.

Current machine-readable pointer:
`/home/ace/baylee-data/v5/ops-codex-20261028/active-training.json`.
The experiment directory is
`/home/ace/baylee-data/v5/ops-codex-20261028/refactor-investigations-20261029/full-model-ppo-pilot-01a188cc-2d9c-7328-9639-aaf5b5af02f7/`.

| Evidence | Current binding |
| --- | --- |
| Plan | `plan.json` in the experiment directory |
| Native plan | `pilot.pipeline.json`, SHA-256 `c7cef67bab35f7655dda64363e241c63284c47314ff1a30cce2706dc87338e97` |
| Controller state | `execution/status.json`, observed phase `full_model_ppo` after CPU preflight |
| Controller identity | PID `2282235`, start ticks `12728937`, boot ID `ebf55ae5-05a2-41fc-a994-e5da27268ced`; verify all three before treating it as the owner |
| Systemd unit | `baylee-full-ppo-01a188cc.service`, no automatic restart |
| Active guard receipt | `execution/jobs/full_model_ppo/job.json`; use this file with `stop_job4` only when a stop is needed |
| Tensor source | Frozen source revision `07e6b6dcb7830ffd566dd1a0a4b049b846435db3`, copied with `git show` into a separate Git-identified trainer snapshot; working checkout changes do not change these sealed inputs |
| Original R6 checkpoint | `e1-paired-gameplay-preparation-01a187df-d6ab-75fd-bda4-ebf073f2cf38/models/r6/net.pt` under the investigation root; SHA-256 `dac99a0c2bfa0612728e7909b56d75a6ec0b50cbb7538316cd8604c391f42827` |
| Dataset | `pre-e3-conversion-admission-01a1884b-fb13-776b-8e14-47079f3a6013/dataset` under the investigation root |
| Admission | `pre-e3-admission-recovery-01a1884d-10f6-76ae-a347-18aa5aa3c4ed/admission.json` under the investigation root |
| Resource boundaries | Exclusive `/home/ace/baylee-data/v5/experiments/gpu-work.lock`; bounded CUDA allocation at 11 GiB and host memory at 20 GiB; inspect actual receipts for measured peaks |
| Watcher | `baylee-full-ppo-01a188cc-watch.timer`, 30-second cadence, `execution/watchdog/config.json` |
| Owning chat | `01a16325-d29f-7ad1-9625-cbb5bf2cb1a4`; success requires phase `complete` and `execution/result.json` |

If another agent takes over after a quota limit, inspect this live controller
instead of launching another one. Review final acceptance, consumed dose,
whole-collection KL, changed tensors, memory/time and owned-process cleanup.
Keep a bounded prefix or rejected/unchanged publication explicitly identified.
Failure recovery requires a fresh UUIDv7 attempt and preserved original evidence;
no automatic retries, KL relaxation or winner filtering.

The user gateway on port 28766 and live selection are preserved. There is no
automatic promotion and no push. Both repository gates are required before a
later push; earlier gate results do not cover all subsequent source changes.
The separate failure-terminal and live-wire implementation work must present
its own fresh end-to-end receipts before its new paths can become training or
deployment contracts.
