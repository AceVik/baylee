# Full-model PPO performance review — 29 October 2026

The initial review below used source/artifacts only. Subsequent work implements
and CPU-tests an isolated fixed-weight probe. Its first GPU launch failed at
CUDA initialization; the diagnosis and fresh recovery are recorded at the end.
Gameplay comparison and failure-ledger integration remain separate.

## Evidence and scope

Frozen run root (all source line references below resolve inside this root):

```text
/home/ace/baylee-data/v5/ops-codex-20261028/refactor-investigations-20261029/full-model-ppo-coverage-01a18905-7895-7f03-a935-b6da0046fc79
```

`execution/jobs/full_model_ppo/job.json:51` records 2,638.268388161 seconds for
the entire guarded GPU stage. `publication/audit.json:410–419` records zero
allocator retries, 3,488,394,752 peak allocated bytes, 4,347,396,096 peak
reserved bytes, 27 accepted/attempted updates and final collection divergence
`7.969814658532071e-5`. `dose.json:1–71` and `coverage-review.json` establish
all eight histories, 6,626 differentiated frames, 1,575 actor choices and
108 segments in 27 B4/U64 groups. The source target replay's maximum
log-probability discrepancy was `7.62939453125e-6`, against tolerance `0.001`
(`execution/jobs/full_model_ppo/stdout.log:29`).

The preceding B2/U32 pilot took 988.33 seconds for eight updates containing
512 differentiated frames/131 actor choices, while auditing the same complete
collection (the [runbook](../training-runbook.md), findings section). These
stages have different differentiated dose and number of audits. Their wall
times do not isolate a batch-size speedup. Neither GPU utilization nor these
aggregate times identifies a dominant phase or supports doubling throughput.

The older `pre-e3-cuda-b8-resource-01a1885e-586e-7f89-bdf8-e1327437beef/`
`comparison.json` measured B8/U32, spatial chunk 4: 14.608 seconds replay and
9.240 seconds resource forward/backward, versus B1/chunk1's 88.459 and
79.804 seconds. This was **head-only**, with 24 gradient tensors, no optimizer
construction/step and only prospective moment buffers. It motivates a new
full-model probe but does not certify its capacity or speed; batch and chunk
also changed together in that comparison.

## Static repeated work, not phase timing

| Observation | Frozen source/evidence | Consequence to measure |
| --- | --- | --- |
| Every attempted update runs an entire current-weight collection audit; publication repeats it | `trainer/recurrent_ppo4/optimization.py:65`; `driver.py:202` | 28 whole-collection audits for 27 updates, plus initial target replay: 192,154 valid frame visits in those 29 passes. These are counts, not measured time. All checks must remain. |
| Burn-in starts separately from zero for every selected segment, lane by lane | `forward.py:49–62`; `dose.json:934` | 48,832 valid burn-in frames for the epoch, in addition to 6,626 differentiated frames. Stored behavior H or H from older weights cannot replace this computation. |
| PPO advance transfers each timestep and calls the full model without execution hints or spatial chunks | `forward.py:21–38`; `model4/player.py:270–300` | Candidate/pair extents and maximum K require CUDA-to-host shape/scalar decisions; many small transfers/kernel launches are plausible overhead. Existing CPU scheduling helpers are not used here. |
| Action likelihood branches inspect CUDA head masks; audit divergence crosses to Python per timestep | `policy_math4.py:61,76,83`; `optimization.py:13,24` | Synchronization is a concrete candidate, but its share of elapsed time is unknown. PPO also computes unused anchor-head KL at `forward.py:79` and `collection.py:45–46`. |
| Structure encodings are rebuilt for each burn-in segment and audit batch | `forward.py:55–59`; `collection.py:39–42` | Potential repeated fixed-weight work; no cache may survive a weight change or leak detached representations into the differentiated unroll. |
| Full parameter/Adam snapshots precede each update; a checkpoint follows each attempt | `optimization.py:48–50`; `driver.py:168–169,196` | Measure snapshot/copy and serialization cost separately; preserve transactional model/Adam/RNG rollback. |

The existing `training4/execution.py:69–100` spatial helper merges only the
batch axis for independent board encodings. Recurrence still runs in time
order; `model4/player.py:255` accepts the encoded board. PPO does not currently
use that helper, and target preparation explicitly passes chunk zero
(`driver.py:125`). Spatial chunking is mathematically equivalent in this
eval-mode FP32 computation, subject to floating-point parity and resource
validation. It is not established as bit-identical or faster for this run.

## Next bounded probe proposal

Retain **optimizer B4/U64, exact ordered segment groups, FP32, K2, temperature
0.7, LR `2e-6`, recorded executed-action denominators and KL limit `0.02`**.
Changing optimizer B to 8 changes loss grouping, Adam step count and trajectory;
that would be a new training experiment. Instead, decouple audit lane count
from optimizer batch size: `driver.py:187–188` currently passes the same value
to both. Audit lanes 4 versus 8 change only how fixed-weight complete histories
are scheduled. Spatial chunk 0 versus 4 changes independent board execution.

1. Prepare an isolated, sealed Rust-owned pipeline and direct PyTorch tensor
   probe. Preserve old run files. Reserve the shared
   `/home/ace/baylee-data/v5/experiments/gpu-work.lock`; use four Torch/OMP/MKL
   threads, a 900-second total deadline, 20 GiB group-PSS limit and the existing
   11 GiB Torch allocator cap. Stop on OOM, allocator retry, parity failure or
   deadline; no automatic smaller-shape fallback or promotion.
2. Compare the fixed-weight whole-collection audit configurations
   `(lanes, spatial chunk) = (4,0), (8,0), (4,4), (8,4)` at the original R6
   source and completed candidate. Map results by immutable history/frame/action
   identity, not lane/output order. Every pass must contain exactly the same
   eight complete histories and 1,575 choices. Record valid and padded frames.
   At most one warm-up and two measured passes per configuration/checkpoint;
   the hard total deadline takes precedence over completing that maximum.
3. Record synchronized phase timings for data/collation, structure encoding,
   prefix reconstruction, differentiated forward/backward, snapshot/Adam,
   complete audit and save. Report warm-up separately, allocator peaks/retries,
   group PSS and all parity deltas. CUDA events can separate device execution
   from host/control time; whole-stage wall time remains the comparison metric.
4. Only after audit parity, compare one fixed B4/U64 full-model transaction
   from identical source/model/Adam/RNG snapshots with reference versus chunk4,
   keeping the audit complete. Choose a predeclared group with long burn-in
   and large boards. Restore the snapshots after both runs. This is a capacity
   and numerical probe, not another learning dose or candidate publication.

Predeclare proposed parity gates: actor log-probability maximum absolute delta
`2e-5`, hidden/value comparison `atol=2e-5, rtol=2e-4`, complete divergence
absolute delta `1e-7`, and concatenated full-model gradient relative L2
`1e-4`. These bounds are proposed, not validated full-model results; a failure
requires review rather than relaxing them after seeing the measurements.
Keep the original recorded-behavior replay tolerance `0.001` and the original
whole-collection `0.02` acceptance limit. Require identical acceptance
decisions; a decision within the numerical comparison tolerance of the limit
needs the reference calculation, never a relaxed limit. Check every parameter
and Adam tensor for finiteness and report their maximum/relative differences.
The injected rejection/exception tests must restore model, Adam and RNG exactly
(existing transaction coverage is in `test_recurrent_ppo5.py:88–103`).

If spatial or lane batching does not dominate, a later separately measured
execution-only change can move known mask/extents/K decisions to host metadata
or accumulate audited FP64 divergence on device with equivalent nonfinite
failure handling. Neither is implemented here. Sharing stale H, omitting
collection checks, weakening rollback, mixed precision, architecture changes
and reward changes are outside this proposal.

## Probe implementation, not performance results

`tools/trainer/probe_recurrent_ppo4.py` now implements the fixed-weight portion
through `recurrent_ppo_probe4/`. It creates no optimizer and never runs a
transaction. The production collection scheduler is shared through
`recurrent_ppo4.collection.stream_collection_batches`; optional board chunking
reuses `training4.execution.encode_boards`. Default training/audit callers
retain their per-frame execution and optimizer grouping.

The worker requires explicit source/candidate checkpoint paths, admitted dataset
and admission receipt, fresh report path, and expected history/frame/choice
counts. It fixes U64, four intra-op threads, one inter-op thread, deterministic
FP32 and TF32 off. The CUDA allocator is capped at 11 GiB (or physical device
capacity if smaller). The Rust launcher must enforce the total 900-second
deadline, 20-GiB group-PSS bound, shared GPU lock and immutable input/output
seals. No Python process controller or monitor is added.

Original recorded likelihoods are replayed with the existing admission/target
preparation before measurement. Candidate metadata must bind the same original
source, dataset, model identity, B4/U64 contract and unchanged KL limit `0.02`.
Each pass captures every valid H/value and every actor log-probability, keyed
by native `(game_id, seat, cursor)`. Numerical gates above apply to each action
and history, not just aggregate KL. Atomic reports retain preparation and every
completed pass; only all required configurations plus model-state/input checks
permit status `complete`. A failed later pass preserves prior evidence.

The first Rust-owned run is planned with zero warm-ups and one measured pass
for each source/candidate × lanes4/8 × chunks0/4 configuration. Instrumented
wall time includes parity copies and phase synchronization; phase sums omit
scheduler gaps. Fixed order and allocator/cache warm-up can affect the timings.
One cold-inclusive pass is diagnostic evidence, not a speedup estimate.

Targeted CPU verification passed 26 tests across `test_recurrent_ppo_probe4`,
`test_recurrent_collection5`, `test_recurrent_ppo5` and
`test_training_execution4`: actual lane/chunk parity, uneven history refill and
padding, production-KL agreement, model immutability, nonfinite/drift/identity
rejection, strict checkpoint contracts and atomic partial-progress retention.
This also exercises existing full-model transaction and grouped-board gradient
checks on tiny fixtures; it does not establish full-model CUDA capacity.

The separate `--preflight-only --device cpu` mode now imports Torch, loads both
strict policies/catalogs on CPU, constructs the native-backed dataset, selects
the admitted learners and loads/collates **every** segment of every selected
history before CUDA reservation. It checks native `(game_id, seat, cursor)`
identity, complete frame/choice counts, finite recorded actor log-probabilities
and the K2/temperature contract without running a model forward. Its fresh
native JSON receipt uses mode `recurrent_ppo_tensor_preflight_v1`; completion
requires `all_histories_loaded_and_collated` and
`source_candidate_contract_compatible`, with actual history/frame/choice totals
and source/candidate/data identities. This CPU preflight deliberately performs
no numerical probability replay; the GPU worker retains that existing gate.
The owning Rust launcher must run it with the final frozen mounts/environment,
bounded guard and sealed inputs before allowing the CUDA stage.

The expanded 29-test CPU suite also checks complete short/padded-history
loading, changed segment identities, missing frames, invalid original actor
likelihood/K/temperature, source/candidate catalog mismatch and failed coverage.
The dispatch test rejects any attempted CUDA allocator/device access or audit
forward, and verifies that rejection cannot publish a complete receipt.

## First launch failure and initialization diagnosis

All following roots are under
`/home/ace/baylee-data/v5/ops-codex-20261028/refactor-investigations-20261029/`.
The first sealed launch,
`full-ppo-execution-probe-01a18945-2801-71c6-9b7e-6ee038105ff5`, passed its
complete CPU data preflight in 30.58 seconds, then stopped with CUDA error 304
before executing the model. Its controller/worker identities were checked as
exited and the owned group empty. Artifacts remain intact; the failure is
reviewed and its watcher stopped. This is neither a training failure nor a
numerical parity result. The PyTorch driver now writes an initial receipt and
catches initialization failures so even a zero-pass attempt retains a report;
11 focused tests pass, including both CUDA initialization failure points.

The bounded initialization-only diagnosis,
`cuda-map-diagnostic-01a18955-41c4-7e9c-815b-a00086214e24`, reproduced error
304 in 2.45 seconds. The CUDA thread's write to
`/proc/self/task/<tid>/comm` returned `EROFS` under the read-only root mount.
Adding only `bwrap --proc /proc` let the same initialization succeed in
2.65 seconds on the RTX 4070 Ti. This preserves the existing PID namespace,
read-only inputs, private device directory and selected device binds. No
model, forward pass or optimizer was used in either diagnostic.

The first recovery,
`full-ppo-execution-probe-recovery-01a18958-7f3f-7ed2-9bb9-f0fa0642cb73`,
freezes trainer commit `283a6cc90bf839bb82a01ab01ffa8d29fd0f08bb`. Its Rust
launcher requires both complete CPU preflight and a separately guarded
60-second CUDA initialization check under the actual final mount map before
allowing the full probe. CPU preflight again passed all eight histories,
6,626 frames and 1,575 choices in 31.14 seconds. The full probe retains the
same eight fixed-weight configurations, 900-second GPU-stage deadline,
930-second pipeline limit, 960-second service limit, 20-GiB host limit and
11-GiB allocator cap. No retry, numerical tolerance change, optimizer step or
deployment is implied.

That recovery passed the CUDA properties check, but its full launch stopped
before model loading at `set_per_process_memory_fraction`: this PyTorch build
requires an explicit device index, while the worker supplied `torch.device('cuda')`.
The retained report records zero completed configurations, optimizer steps and
parameter updates. The run was reviewed, its processes checked as exited, and
its watcher stopped; event `c8f2bfbe96d7bf4a7b2dc2cc` is deduplicated.

Commit `0d3be58fa62f4606f38c5ff2e53cf74abb5faf60` resolves indexless CUDA
through `torch.cuda.current_device()` before querying properties and applying
the cap. Explicit `cuda:N` is retained. Thirteen focused probe/preflight tests
pass, covering actual cap arguments, all three initialization failure points
and CPU preflight isolation. The next prerequisite must call this same frozen
PyTorch helper and perform a tiny allocation/synchronization, so checking only
device properties cannot miss the allocator setup again. Consult
`active-performance-probe.json` in the parent operations directory for live
ownership; preparation alone is not a launch.

The second recovery,
`full-ppo-execution-probe-recovery-01a1895c-fd11-7234-9dc3-47486276272f`,
passed both strengthened prerequisites: the guarded complete CPU preflight
took 33.80 seconds; the CUDA check took 2.70 seconds and verified `cuda:0`,
an actual 11-GiB allocator cap, a four-byte tensor, synchronization and zero
allocated bytes after cleanup. Owned prerequisite processes exited and their
groups were empty. Root then independently rechecked the frozen inputs and
launched the full probe once. At handover, controller `2351100` / start ticks
`13706570` on boot `ebf55ae5-05a2-41fc-a994-e5da27268ced` is active in
`fixed_weight_cuda_probe`; the service and owning-chat watcher are active.
Manifest SHA256 is
`f72270b2a8a8c9911853452c4a8b5faf5f48ae5d304ed5d4b8498d73b74cebdb`.
Completed parity/timing results still require review; no execution optimization
or learning configuration is certified by initialization alone.
