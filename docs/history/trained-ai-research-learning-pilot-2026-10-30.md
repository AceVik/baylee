# Failure-terminal research learning pilot — 30 October 2026

The first real producer-to-update trial has completed CPU collection,
research conversion, native cache verification and native input admission;
complete-history Torch probability replay and one bounded PPO update remain
pending. This is a four-attempt development contract test, with no strength
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

Later receipts must establish actual replay
coverage, numerical errors, accepted update dose, KL, changed tensors and
owned-process cleanup before this report can describe a learning result.

The earlier coverage PPO experiment remains a technical success with 27
accepted updates and 311 changed tensors. Its 16-case development comparison
still has 4 wins/12 losses, matching original R6 without measured strength gain.
The old continuous BC controller stays intentionally stopped, and the user
gateway on port 28766 is preserved.
