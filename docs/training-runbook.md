# Training operations and agent handover

Operational entry point for BayleeAI training. Read [AGENTS.md](../AGENTS.md),
[trained-ai](trained-ai.md), the [current research plan](proposals/trained-ai-v5-refactor-training.md)
and the [29 October evidence report](history/trained-ai-v5-rust-refactor-2026-10-29.md)
before changing a contract. This document explains how to continue work; dated
measurements and failed attempts belong in the evidence report and run artifacts.
The historical `tools/trainer/HANDOVER.md` is not a restart recipe.

## Current operational pointer

Last reviewed: 30 October 2026. Working checkout:
`/home/ace/Projects/baylee-ai-payment`, branch
`codex/payment-context-v5-20261028`. This is local research, with no automatic
promotion or push. Always check actual status before acting on this snapshot.

| Item | Authoritative pointer and interpretation |
| --- | --- |
| Research collection and downstream work | `/home/ace/baylee-data/v5/ops-codex-20261028/active-research-collection.json` now names `failure-terminal-breadth-01a18b06-115f-7be2-944c-0e6a4ac95fea/`, running after both zero-game CPU loader checks passed. Sixteen fresh attempts form eight seat-paired tables, alternating historical neural opponents and frozen-R6 self-play. No retry/refill or automatic PPO. The earlier four-attempt pilot remains complete and reviewed: three target histories, 1,713 frames, 372 genuine choices. Follow actual stage receipts before launching anything |
| Current learning phase | `/home/ace/baylee-data/v5/ops-codex-20261028/active-training.json` names the latest research phase under `research-learning-recovery-01a18ad4-ea35-701e-952c-1e0191d0b9fb/`. CPU preflight, CUDA initialization and full numerical replay passed; exactly one update was accepted (61 choices/256 frames, 301 changed tensors, final KL 0.00007725546). Follow `execution_root` and actual receipts; no automatic repeat or promotion. Earlier coverage training remains completed and reviewed |
| Current behavior comparison | `/home/ace/baylee-data/v5/ops-codex-20261028/active-cpu-followup.json` follows `research-update-paired-gameplay-01a18ae5-3467-7853-ac5e-3d42fc1d3913/`, now `complete-reviewed`. All eight attempts are retained; four matched cases per arm produced identical canonical actions and outcomes. The learner loop persists, with no demonstrated improvement. Owned processes exited and its watcher stopped. Earlier coverage-candidate comparison receipts remain historical |
| Completed performance probe | `/home/ace/baylee-data/v5/ops-codex-20261028/active-performance-probe.json` is `complete-reviewed`; its fixed-weight CPU-hint probe performed no updates or model writes |
| Operational decisions | `/home/ace/baylee-data/v5/ops-codex-20261028/next-actions.json`; read the newest named experiment and its receipts, rather than historical PID fields elsewhere in that file |
| Evidence root | `/home/ace/baylee-data/v5/ops-codex-20261028/refactor-investigations-20261029/` |
| Old continuous BC controller | `baylee-continuous-v5-training.service` intentionally stopped; `stop-complete.json` under the evidence root certifies cleanup. Do not resume this recipe because of a delayed monitor event |
| Preserved interrupted BC state | `preserved-round11-last.pt` and `checkpoint-audit.json` under the evidence root: round 11, step 1800, incomplete round; SHA-256 `ee976bc3e03b783c112a20da6314b5f324dc7527f628e2e8f2a9d51beb2c655c` |
| Current research direction | Separate failure-terminal collection → conversion/cache → native input admission → Torch complete-history probability replay → one bounded PPO update and matched gameplay completed. Replay checked all 372 choices; update dose was only 61. No executed behavior changed on four reused cases. A broader predeclared fresh CPU collection is running; review all outcomes before scheduling another learning step. Native input admission does not certify numerical replay or playing strength |
| GPU exclusion | `/home/ace/baylee-data/v5/experiments/gpu-work.lock`; every CUDA stage, including probes/export, uses this exact shared lock |
| User game service | Preserve the existing gateway on `127.0.0.1:28766`, its agent, engines, live model and active games; training does not require restarting it |
| Quota-limited external collection | The Flash hold/probe is recorded in `next-actions.json`; a clean bounded probe permits technical review, not automatic restart of the large collector |

The completed initial failure-terminal pilot is
`failure-terminal-real-e2e-01a18a9d-d0e0-7db4-b154-2cbae46d83fd/` under the
evidence root. It retains all four fixed known development attempts, with no
refill, holdout, strength claim or promotion. Raw E7/R6 FP32 K2/temperature 0.7
collection stops before House recovery: learner loop has research utility zero
and unknown MTG outcome; opponent failure, operational unknown and cap remain
target-free. The separate worker uses uncentered `A = training_utility - V_source`
with frozen source values and a utility critic, without ordinary outcome labels.
Complete-history replay must meet the unchanged maximum log-probability error
of `0.001` before any update. See the
[separate research path](#separate-failure-terminal-research-path) for contracts
and commands, implemented in `599cfe70a`, `5090f40a1` and `7ba44fc23`. The
[real-data pilot report](history/trained-ai-research-learning-pilot-2026-10-30.md)
records native admission, numerical replay and the accepted bounded update.

The new breadth collection reuses the same frozen R6/native runtime, with fresh
opening seeds and 16 distinct adapted train decks. Its eight paired tables are
correlated within each pair; retain all 16 planned attempts, including unknown
and unstarted outcomes. Service `baylee-breadth-01a18b06-collection` has a
2,340-second limit, two CPU equivalents and 20 GiB memory. The durable
`baylee-breadth-01a18b06-collection-watch.timer` observes completion or failure.
The actual controller at launch was PID 2442449/start ticks 16553759 on boot
`ebf55ae5-05a2-41fc-a994-e5da27268ced`; recheck identity instead of reusing that
PID. Both preflight loaders and cleanup passed before this single launch.
This CPU collection has no GPU or automatic downstream training stage.

The completed initial pilot used B2/U32 and eight updates, differentiating only
131 actor choices. Its successor used B4/U64, K2, temperature 0.7, LR `2e-6`,
fresh AdamW from original R6 and one full epoch of the same eight games. All
27 updates were retained, covering 1,575 actor choices and 6,626 learner frames;
the final whole-collection KL was `0.00007969814658532071`. Its frozen manifest
is the authority for limits and source identity. Both candidates' 16-case
comparisons showed no outcome change: each model won 4/16 with the same one
House intervention. The coverage candidate changed four canonical sequences
relative to R6; no strength gain or equivalence is established by eight
repeated development tables. Preparation is not proof of a
running job, accepted updates, full dataset exposure, resource readiness for a
long run, or stronger play. Do not substitute the head-only candidate for its
original R6 source. Keep E9 observation learning and a new failure reward as
separate experiments so the cause of any change can be measured.

## What runs where

All operations that can be Rust are Rust: orchestration, process supervision,
recording/replay, conversion, packing/cache administration, provenance,
admission, metrics, reports, monitoring and publication. Python entry points
below are PyTorch computation and tensor/model adapters only. Do not add a
Python or shell controller, data-preparation helper or monitor. Invoke existing
CLIs directly. New one-off operational logic must also be maintainable Rust.

| Tool | Responsibility |
| --- | --- |
| `train_ops pipeline` | Ordered immutable experiment, input/output seals, per-root controller lock, time/PSS limits, shared GPU stage lock and terminal receipts |
| `train_ops run` | One bounded owned process-group job; has no `--gpu-lock`, so use a pipeline for a CUDA stage |
| `train_ops collect-research`, `finalize-research` | CPU research collector supervision and durable planned/assigned/recorded/unknown counts after verified owned-group cleanup |
| `eval4` | Native CPU/CUDA actor, complete game records and sidecars; ordinary evaluation and on-policy collection are distinct purposes |
| `convert4` | Deterministic full replay, versioned observation/target conversion and dataset publication |
| `convert4 research` | Separate failure-terminal research features/returns; conversion-only output, rejected by ordinary training/cache/admission |
| `trajectory_cache4` | Native packed trajectory cache, source identities, seals and tensor I/O boundaries; no Python fallback |
| `trajectory_cache4 research-build/research-verify/research-read` | Separate lossless research JSON-frame storage and diagnostic reads; not a tensor cache or PPO admission |
| `research_tensor4` | Strict native research tensor packing, pinned source/cache/conversion verification and dedicated input admission; numerical probability replay remains the Torch worker's separate gate |
| `train_ops admit-policy`, `verify-admission` | Learner-only whole-collection certification and actual-content revalidation before either PPO worker |
| `train5.py` | Supervised v5 PyTorch training; BC labels are imitation, not on-policy returns |
| `train_policy4.py` | Isolated action-head PPO; backbone, thinking, memory and critic remain frozen |
| `train_rl4.py` | Full-model FP32 recurrent PPO, current-policy history reconstruction, transactional KL rollback and strict optimizer continuation |
| `train_research_ppo4.py` | Separate PyTorch failure-terminal utility objective, complete-history sampled-probability replay, bounded PPO update and strict research checkpoint/resume contract |
| `probe_recurrent_ppo4.py` | Fixed-weight PyTorch execution comparisons with complete-history parity; opt-in `--cpu-execution-hints --audit-lanes 4 --board-chunks 0` compares host metadata without changing board batches or training defaults |
| `diagnose_recurrent_chunk4.py` | Source-only PyTorch localization of chunk4 numerical differences; successful diagnosis explicitly does not accept the rejected execution variant |
| `export_onnx4.py` | PyTorch export and actual numerical parity for bounded-depth native graphs |
| `paired_eval4`, `summarize_eval4`, `report_experiment4` | Native verified gameplay comparison and reports |
| `watch_run4`, `monitor_ops4` | Durable terminal-event observation and delivery; neither starts a controller |
| `stop_job4` | Identity-bound stop of a native guard supervisor and verification that owned descendants exited |
| `seat4`, `publish_live_model4`, `live_play_maintenance4` | Live preview, verified development-model selection and completed official-record import; separate from training |

Source directories are `crates/baylee-train-ops/src/`,
`crates/baylee-train/src/bin/{eval4,convert4,seat4}/` and `tools/trainer/`.
Build only required binaries when source changed; a new build is not a reason
to replace a pinned runtime in an existing experiment:

```text
cargo build --release -p baylee-train-ops --bins
cargo build --profile selfplay -p baylee-train --features onnx --bin eval4
cargo build --profile selfplay -p baylee-train --bin convert4
```

New research-worker reports include `timings.phase_seconds` for checkpoint
loading, policy construction, the native tensor handshake, replay, update
transactions, final verification and publication. These are host wall times
without added CUDA synchronization, not kernel timings; a phase can include
previous asynchronous GPU work. Imports, the final report's own write and
subsequent cleanup are excluded. Use native guard receipts for total worker
time and cleanup. The already frozen one-update pilot predates these fields;
do not rewrite its report or infer missing phase measurements.

New exports retain the value objective in `critic_semantics`, and the native
loader binds it into model identity. Research-utility models remain usable by
offline evaluation, but the live adapter refuses them because its draw rule
assumes MTG side-outcome utility. An older unmarked research export cannot be
recognized by architecture or value range; retain its external provenance and
re-export from the original checkpoint into a fresh directory when needed.
Never relabel a frozen graph or treat a successful export as live promotion.

Native `train_ops`, `watch_run4` and `stop_job4` syntax below was checked against
release `--help` and source. Evaluator/converter and PyTorch options were checked
against their CLI definitions. Use the frozen run's binaries and worker tree
for actual continuation; current `target/` contents can differ from them.

## Take over without starting a duplicate

1. Read every canonical pointer above and its current phase, beginning with
   `active-research-collection.json`; completed training/comparison/probe
   pointers do not authorize a restart. Follow the active root's actual
   execution directory (`collection-execution/` for this collector). Read the
   newest plan, `execution/status.json`,
   `execution/controller.json`, active `execution/jobs/STAGE/job.json`, and
   `execution/watchdog/{config,state,delivered}.json` if present. A queue event
   can be late or delivered more than once: deduplicate its event ID and review
   current state before acting.
2. Check the named systemd unit and actual process identity. Native identities
   bind **PID, kernel `start_ticks` and `boot_id`**. A live PID alone is not
   ownership; a stale receipt is not an instruction to signal a reused PID.
   The native guard/watcher use the identity implementation in
   `crates/baylee-train-ops/src/process.rs`.
3. A running valid owner keeps the work. Observe it and retain the same run
   pointer; do not launch a second controller because the previous chat/model
   reached its quota. Changing chat ownership does not require changing the
   trainer, seeds, source data or process.
4. `pipeline` holds `ROOT/controller.lock`, and individual jobs hold their guard
   lock. This prevents duplicate owners **of that root**, not a second pipeline
   in a different root. Review all active run pointers and the shared resource
   locks before creating another experiment. Never delete a lock file to
   bypass exclusion: that can create two independently locked inodes.
5. If complete, verify receipts, hashes and expected outputs before interpreting
   it. If failed/interrupted/quarantined, inspect failure and cleanup evidence;
   recover explicitly into a fresh UUIDv7 attempt. Do not edit the old manifest,
   erase its status, overwrite records or count a partial stage as complete.

Useful direct read-only invocations (replace `UNIT`/paths with the named run):

```text
systemctl --user status baylee-continuous-v5-training.service
systemctl --user show UNIT --property=MainPID --property=ActiveState --property=SubState
cat /proc/sys/kernel/random/boot_id
cat /proc/PID/stat
ps -p PID -o pid,ppid,pgid,stat,args
nvidia-smi
df -h /home/ace/baylee-data
```

`/proc/PID/stat` field 22 is the creation tick count; use the native identity
parser for automation because the parenthesized process name can contain spaces.
Do not add fragile shell parsing or use name-pattern kills.

## Plan, freeze and supervise an attempt

Choose a fresh UUIDv7 directory; `uuidgen --time-v7` is available on this host.
Write a Rust preparation command when plan construction needs program logic.
Freeze the source checkpoint, dataset seals, deck order, deal and policy seeds,
encoder/architecture/candidate/sampling contracts, loss, precision, K schedule,
budgets and required outputs before launching. Record why the chosen experiment
answers the next question. A useful artifact layout is:

```text
RUN/
  plan.json                 # research question, scope, dose, gates and budgets
  pipeline.json             # native executable plan; immutable after launch
  input-seals.json           # source/runtime/data receipts
  trainer/                  # frozen Git-identified worker snapshot
  bin/                      # pinned native runtime and required provider libraries
  execution/                # status/controller/inputs/result; jobs/STAGE/job.json
  publication/              # candidate and worker report, when successfully written
  review.json               # interpretation, coverage, cleanup and next decision
```

The pipeline schema is exactly version 1: absolute `root` and Git-identified
`cwd`, finite positive `total_timeout_seconds`, and ordered stages with `name`,
literal `command` array, finite `timeout_seconds` and `memory_gib`, `inputs`,
distinct absolute `required_outputs`, and optional absolute `gpu_lock`.
Unknown fields are rejected. A downstream generated input must be a declared
output of an earlier stage. Every stage needs at least one real required file.
Use the actual run's reviewed manifest; this structural example is not a recipe:

```json
{
  "version": 1,
  "root": "/absolute/new-uuidv7-run/execution",
  "cwd": "/absolute/new-uuidv7-run/trainer",
  "total_timeout_seconds": 1800,
  "stages": [{
    "name": "bounded_update",
    "command": ["/absolute/pinned-worker", "--its-reviewed-arguments"],
    "timeout_seconds": 1700,
    "memory_gib": 16,
    "gpu_lock": "/home/ace/baylee-data/v5/experiments/gpu-work.lock",
    "inputs": ["/absolute/sealed-input"],
    "required_outputs": ["/absolute/new-uuidv7-run/publication/net.pt"]
  }]
}
```

Include executable/worker files, imported modules, configs, provider libraries,
source checkpoint, dataset metadata/index/shards/catalog and admission in seals.
Tracked Git identity alone does not seal untracked experiment helpers. An
isolated source snapshot needs a real Git commit/receipt; create it without
changing the main checkout's branch. Dataset members must resolve inside their
declared root: symlinks out to another dataset are rejected. Copy and verify
when isolation requires a new dataset root.

Before reserving CUDA, use the **final guarded invocation and environment** to
import Torch, construct the native-backed dataset and load complete histories.
The guard preserves virtual-environment invocation semantics and records both
invocation path and canonical executable hash. Replacing a venv Python path by
its resolved interpreter can lose Torch. Pin `BAYLEE_TRAIN_OPS` to the correct
native binary for the worker and rebuild/pin `trajectory_cache4` with matching
contracts. A generic guard smoke or `--help` is not a loader preflight.

Launch the reviewed native plan directly, or through one named nonrestarting
systemd service with bounded cgroup memory and `KillMode=control-group`:

```text
/absolute/pinned/train_ops pipeline --manifest /absolute/RUN/pipeline.json
```

`pipeline` seals external inputs before its first stage, rechecks them and
earlier outputs before consumers, and verifies them again before completion.
It writes atomic `status.json`, stage logs/receipts, and final `result.json`.
The guard isolates child process groups, sums proportional memory (PSS),
enforces deadlines/memory bounds, and cleans owned descendants before reaping
their leader. A terminal file proves only its declared stage contract; inspect
the worker's acceptance and coverage report too.

For an isolated CPU follow-up using `bwrap`, test the actual mount/environment
map before launching the pipeline. On this host, a read-only bind of the host
root passed all input hashes but AppArmor denied writable `/dev/null` access:
even Git source-identity subprocess startup failed before export. A private
standard device mount (`--dev /dev`) fixed it while retaining read-only inputs
and the enforcing profile, and also omitted host GPU device nodes. This is a
CPU-only recipe, not a CUDA launch configuration. Preserve the failed attempt
and test the narrow correction in a fresh UUIDv7 run with the same cases.

There is one GPU owner at a time. Keep CPU collection/conversion threads and
RAM finite so the gateway/input remain responsive; initial native collection
used two threads and single-threaded BLAS, not all 24 hardware threads.
On the 12-GB GPU, measure representative actual large-board forward,
backward **and Adam state** before increasing batch/K or planning a long run.
Head-only probes, tiny catalogs and inference prefixes do not establish this
capacity. An optional CPU exclusion lock must be shared and explicitly acquired
by a Rust controller; `pipeline` currently exposes only the generic stage lock
named `gpu_lock`, with no independent CPU-lock or CPU-core-limit field. A memory
limit does not reserve CPU cores. Record actual budgets and resource readings.

## Record, replay and admit PPO data

Ordinary `eval4` reports are not self-certified PPO data. Initial supported
collection is 1v1, no teams, **Raw/FP32, K2, temperature 0.7**; explicit
`--collection-purpose on-policy-training` binds records, completion indices,
checkpoint identities and learner/opponent roles. Each neural actor has its own
memory and RNG. House, opponent, forced and fallback answers remain distinct;
only genuine learner stochastic actions have behavior-probability targets.

Example CLI shape, inside a bounded CPU or GPU pipeline stage with frozen
runtime, model/decks and predeclared seeds:

```text
/absolute/pinned/eval4 --model /absolute/r6-export --collection-purpose on-policy-training --against sharp --seats 2 --raw-policy --thinking-steps 2 --temperature 0.7 --no-baseline --deals 2 --threads 2 --generated 0 --decks-manifest /absolute/train-decks.json --deck-split train --first-seed 10000 --policy-seed 31431 --record-out /absolute/NEW/records --out /absolute/NEW/eval.json
```

For a neural opponent use `--opponent-model PATH --opponent-role historical`,
or `self-play` only for the exact frozen source policy. Preserve the fixed deck
order and rotations; a zero-deal preflight checks model/deck acceptance but
plays no game. Pin CUDA evaluator **and** adjacent
`libonnxruntime_providers_shared.so`/`libonnxruntime_providers_cuda.so`. Copying
only the executable has failed. Exercise an actual short native CUDA match
when that provider is intended; export parity does not exercise it.

Convert complete original records with the matching producer/converter build:

```text
/absolute/pinned/convert4 --runs /absolute/collection/records --out /absolute/NEW/dataset --encoder-version 7 --include-passive-obs --val-every 0 --threads 2
/absolute/pinned/trajectory_cache4 build --data-dir /absolute/NEW/dataset --split train
/absolute/pinned/train_ops admit-policy --data /absolute/NEW/dataset --source-checkpoint /absolute/r6/net.pt --collection-manifest /absolute/NEW/dataset/collection.json --out /absolute/NEW/admission.json
/absolute/pinned/train_ops verify-admission --data /absolute/NEW/dataset --source-checkpoint /absolute/r6/net.pt --admission /absolute/NEW/admission.json
```

`convert4` writes `dataset/collection.json` and names it in metadata as
`on_policy_collection_manifest`; this binds the source records/roles to actual
converted dataset seals. Do not fabricate it. Admission binds full histories/content and
selects learner histories only. The worker then replays every recorded behavior
probability before updates. Cache reuse independently checks source/data/index
digests, including before its first lazy mmap. Staged, imported or partially
converted files are not automatically admitted.

Complete losing games remain in the dataset. The initial strict contract
rejects caps, fallback, refusal, unencoded frames and incomplete histories;
retain every failed case and its reason in the denominator. Caps/technical
aborts are neither MTG draws nor manufactured losses. Preserve table pairs,
seeds and attempt limits; do not refill until a favorable clean set appears.
The planned E3/E4 design is at most 160 attempts for 128 complete games,
collection at most six hours and updates at most 32 attempts/one pass/30
minutes; smaller pilots have their own tighter limits. These are research
budgets, not guarantees of sufficient sample size.

### Separate failure-terminal research path

Use freshly built and pinned tools containing the research commands; an older
experiment's frozen binary does not acquire new commands when this checkout
changes. This path currently supports unteamed 1v1 neural self-play or a frozen
historical neural opponent, Raw/FP32 K2 and temperature 0.7. It stops before
House recovery. It has a separate native tensor/input-admission and PyTorch
research worker; live Terra games still need the executed-action/export join.

The bounded CPU wrapper receives the evaluator command literally after `--`:

```text
/absolute/pinned/train_ops collect-research --job /absolute/NEW/guard --run /absolute/NEW/records --timeout 600 --memory-gib 20 --input /absolute/train-decks.json -- /absolute/pinned/eval4 --model /absolute/r6-export --collection-purpose failure_terminal_research_v1 --opponent-role self-play --seats 2 --raw-policy --thinking-steps 2 --temperature 0.7 --no-baseline --deals 2 --threads 2 --generated 0 --decks-manifest /absolute/train-decks.json --deck-split train --first-seed 10000 --policy-seed 31431 --record-out /absolute/NEW/records --out /absolute/NEW/eval.json
/absolute/pinned/train_ops finalize-research --run /absolute/NEW/records --job /absolute/NEW/guard/job.json
```

Also bind the actual model/runtime inputs through the enclosing immutable
experiment manifest and the wrapper's repeated `--input` options. The wrapper
finalizes automatically after guard cleanup, including failed child jobs; its
exit remains unsuccessful for a failed collector. The second command is for
preserved recovery only, after the guard recorded verified cleanup and the
exact producer PID/start-ticks/boot identity is gone. It never kills a process,
retries a game or turns a technical failure into a game loss. A dead PID alone
without the required cleanup receipt is insufficient.

`attempts.plan.json` fixes the planned deals before workers run. Each worker
durably claims a deal before entering the engine, then publishes a completion
receipt only after syncing and verifying the record/index. The final ledger
distinguishes not-started deals, assigned operational unknowns and recorded
games. Observation errors, worker panics and bounded process termination retain
unknown utility. Missing completion-seal recovery may seal only a byte-identical
recomputed ledger; changed evidence is rejected and preserved.

For a fully indexed, completed research collection:

```text
/absolute/pinned/convert4 research --runs /absolute/NEW/records --out /absolute/NEW/research-features
```

For an already finalized operational attempt ledger, including collections
with failed workers or unstarted deals, use the explicit Linux adapter:

```text
/absolute/pinned/convert4 research --attempt-ledger-runs /absolute/NEW/records --out /absolute/NEW/research-features
```

The two input options are mutually exclusive. The finalized-ledger adapter
requires the original absolute source/guard paths and both existing final
ledger seals; it does not repair evidence. It recomputes the canonical ledger,
verifies producer exit and guard cleanup, and rechecks it before publication.
Manifest v2 preserves planned, assigned, recorded, operational-unknown and
not-started counts separately. An index row without a published game receipt
remains unknown; torn tails and orphan shards remain sealed evidence. An
unstarted deal is neither a played game nor an unknown outcome.

The converter verifies original canonical replay, controlled-seat roles,
sampling provenance and exact declared termination. It writes
`research.manifest.json`, `research.complete.json`, `attempts.jsonl` and separate
compressed research episodes. Known research returns are explicitly separated
from MTG outcomes; unknown attempts emit no target rows. Ordinary `Step4`
outcome, terminal and final-state auxiliary labels are absent. The original
`--runs` adapter keeps its complete-index-v2 requirement and manifest v1;
`--attempt-ledger-runs` emits manifest v2 without changing the frame/return
contract. Operational unknowns and not-started entries receive no episode
file or target frames. Fourteen converter and eleven operational-ledger tests
plus focused Clippy passed for this integration.
Do not feed these files to ordinary trajectory caches or PPO admission.
The separate lossless storage adapter accepts manifest v2 from the
finalized-ledger converter:

```text
/absolute/pinned/trajectory_cache4 research-build --data-dir /absolute/NEW/research-features --out /absolute/NEW/research-cache --expected-manifest-sha256 FROZEN_SHA256
/absolute/pinned/trajectory_cache4 research-verify --cache-dir /absolute/NEW/research-cache --expected-manifest-sha256 FROZEN_SHA256
/absolute/pinned/trajectory_cache4 research-read --cache-dir /absolute/NEW/research-cache --expected-manifest-sha256 FROZEN_SHA256 --row 0
```

Supply the conversion-manifest SHA from externally frozen inputs. Verification
checks the original finalized ledger, catalogs, converted episodes and exact
cached frame bytes. It preserves attempt denominators, passive observations,
original masks/pick sequences and null MTG outcomes. Unknown and unstarted
attempts retain accounting but receive no target frames. `research-read` is a
single-frame diagnostic; bulk consumers must reuse the Rust `Reader` session,
not spawn a process and reverify the entire collection per frame. A completion
receipt alone does not replace verification. These artifacts explicitly remain
`admitted=false`, `ppo_eligible=false`, `tensor_cache=false` and
`producer_binary_certified=false`.

The separate `research_tensor4` executable performs strict model-input packing
in Rust. It retains every represented entity, relation, candidate, pair, mask,
pick sequence and passive observation, and rejects invalid references instead
of truncating or repairing them. Its persistent process verifies source evidence
once at opening and serves typed little-endian arrays for contiguous history
ranges. Real empty menus retain the native evaluator's false masks. Only the
Torch loader adds masked temporal padding; ordinary `Dataset4` is not used to
encode research frames.

Freeze a new `failure_terminal_research_input_pins_v1` JSON containing:

- `cache_dir`, `conversion_manifest_sha256`, `cache_manifest_sha256`;
- `source_checkpoint: {path, sha256}`, the exact `learner_model_identity` and
  `catalog` from the externally checked collection/conversion;
- one entry per source in `producers`, containing `source_run_sha256`,
  `guard_job`, `guard_job_sha256`, `executable`, `executable_sha256` and the
  exact original `collection_contract`.

Paths are canonical absolute paths. The original schema-3 guard must seal the
actual producer executable and all learner/opponent model metadata/depth graphs.
Freezing arbitrary new pins does not repair missing original guard evidence.

```text
/absolute/pinned/research_tensor4 admit --pins /absolute/NEW/pins.json --expected-pins-sha256 FROZEN_PINS_SHA --out /absolute/NEW/admission.json
/absolute/pinned/research_tensor4 verify --pins /absolute/NEW/pins.json --expected-pins-sha256 FROZEN_PINS_SHA --admission /absolute/NEW/admission.json
```

This grants only `research_input_admitted=true`; the receipt still declares
`ordinary_ppo_admitted=false` and `numerical_replay_required=true`. The lossless
storage receipt remains storage-only. The dedicated Torch entry point is:

```text
/absolute/torch-venv/bin/python /absolute/frozen/trainer/train_research_ppo4.py --init /absolute/r6/net.pt --cache-dir /absolute/NEW/research-cache --conversion-sha256 FROZEN_CONVERSION_SHA --cache-sha256 FROZEN_CACHE_SHA --pins /absolute/NEW/pins.json --pins-sha256 FROZEN_PINS_SHA --admission /absolute/NEW/admission.json --out /absolute/NEW/replay-only --device cuda --batch-size 4 --unroll 64 --audit-only
```

Use the pinned `research_tensor4` through `BAYLEE_RESEARCH_TENSOR` and pinned
`train_ops` through `BAYLEE_TRAIN_OPS`. Also place a pinned `trajectory_cache4`
on the isolated process PATH: the checkpoint loader delegates encoder-contract
validation to it, even for research tensors. Do not rely on a development
checkout's `target/` fallback. The example is the worker invocation;
run it under the usual native guard, GPU lock and immutable pipeline manifest.
The worker closes and reaps its owned native tensor server on success or error.
Its dataset must not be shared with multiprocessing DataLoader workers.
Before CUDA, use a separate output directory with `--preflight-only --device cpu`
instead of `--audit-only`. This loads the actual checkpoint and materializes and
collates every admitted segment, checking counts, dtypes and finite CPU tensors.
It performs no model forward, optimizer construction, target creation or model
write. It is not a substitute for the numerical replay. Keep the existing exact
CUDA initialization/cap/allocation prerequisite under the final launch mappings.

For the bounded learning trial, use a new output directory, omit `--audit-only`
and explicitly set `--epochs 1 --max-steps 1 --lr 0.00002 --target-kl 0.02`.
The entire input collection is replayed before any optimizer construction,
with unchanged recorded behavior log probabilities and maximum error 0.001.
At least 128 genuine stochastic choices are required for a learning run.
All known-return learner histories remain present, including passive frames;
unknown attempts stay in the admission denominators and receive no targets.
The initial worker restricts this experiment to the approved E7 observation
contract, FP32, recorded K2/T0.7, without a BC anchor or auxiliary losses.
It explicitly disables TF32, fixes deterministic cuDNN behavior and bounds the
CUDA allocator at 11 GiB before loading model tensors. Precision flags are
part of the resume contract and are rechecked before publication.

The objective marker is
`failure_terminal_utility_minus_frozen_source_value_uncentered_v1`:
`A = training_utility - V_source`, without centering or rescaling. Source values
are frozen before updates and recomputed from the same source on resume. This
avoids turning an all-zero-return sample into positive advantages by centering;
it does not establish that the penalty improves play. The critic learns the
declared research utility, never an invented MTG outcome. Current-policy hidden
state is reconstructed from each complete history prefix. Each update checks
the whole collection against recorded behavior and restores weights, Adam and
RNG together on rejection. Full-model graph, recurrence and thinking parameters
remain trainable.

Research checkpoints use `research_ppo_contract` and `research_ppo_progress`;
ordinary PPO continuations are rejected. `--resume /absolute/RUN/last.pt` needs
the identical experiment contract. `--max-steps` is a cumulative attempted-batch
budget and may increase for exact continuation; it is not a full-epoch claim.
`probability-replay.json` and `report.json` record coverage and gates. Check
accepted updates, final gate and changed weights rather than process exit alone.
No publication to the user gateway is automatic.

Before calling the new path learning-ready, retain a real, original-probability
producer→converter→native-admission→Torch-replay→update receipt. Synthetic
format fixtures cannot provide that evidence or a playing-strength claim.
Implementation evidence and exact validations are in the
[attempt-ledger report](history/trained-ai-research-attempt-ledger-2026-10-29.md)
and [research-conversion report](history/trained-ai-failure-terminal-research-conversion-2026-10-29.md),
with native protocol and pin details in the
[research-tensor report](history/trained-ai-failure-terminal-research-tensors-2026-10-30.md).

## Update and recover

The first outcome contract uses win/loss/actual draw utility 1/0/0.5,
gamma=lambda=1. Segment boundaries are not terminal events. Both PPO workers
require the native admission receipt; BC/live/LLM/shadow labels do not become
PPO behavior data. Greedy moves have no PPO sampling likelihood.

Full-model worker argument shape for the bounded E7 pilot (the manifest must
also freeze source/runtime/precision, admission and external time/memory limits):

```text
/absolute/pinned-venv/bin/python /absolute/frozen-trainer/train_rl4.py --init /absolute/r6/net.pt --rollout-data /absolute/admitted-dataset --admission /absolute/admission.json --out /absolute/NEW/publication --device cuda --thinking-steps 2 --batch-size 2 --unroll 32 --epochs 1 --max-steps 8 --lr 2e-6 --target-kl 0.02 --seed 31431
```

`--audit-only` performs the native admission/probability replay with no optimizer
update. `--max-steps` counts attempted groups, not complete exposure. Inspect
accepted steps, consumed histories/choices, KL, terminal flags and final gate.
A run can publish unchanged source weights after rejection; a `net.pt` file
alone does not prove learning. Do not raise KL or temperature merely to get an
observable change. Optional supervised anchors change the objective and need
their own sealed contract.

Full-model PPO reconstructs H using the **current** policy over each complete
preceding history, detaches at an unroll boundary, then differentiates within
the unroll. The complete-collection audit streams each history under fixed
current weights. Both retain recorded executed-action probabilities as the PPO
denominator; a small replay discrepancy does not license replacing it with a
freshly computed behavior probability. Above-limit updates roll back model,
Adam and RNG together. Resume contract v2 binds
`behavior_probability_reference=recorded_executed_action` and
`collection_audit=current_policy_streaming_v1`.

Workers can support strict `--resume`, but native failed-attempt directories
remain immutable. A recovery uses a fresh UUIDv7 root, explicitly pins the old
checkpoint and receipts, and preserves the exact compatible optimizer/objective,
sampler, recurrent and RNG contract. A weights-only `--init` is a **new
experiment**, with fresh optimizer/scheduler/sampler/H where required; do not
call it an exact resume. Incompatible model/encoder/objective contracts must
fail, not be bypassed by changing labels. Never reuse optimizer state from a
head-only publication that did not export it.

Stop only the owned native job, using its actual guard receipt:

```text
/absolute/pinned/stop_job4 --job /absolute/RUN/execution/jobs/STAGE/job.json --timeout 15 --out /absolute/NEW-stop-receipt.json
```

This verifies guard/controller/child identity, signals the pinned supervisor,
and checks process-group cleanup. It rejects stale/foreign identities and does
not blindly escalate. `--job` is a **file**, not a directory or `controller.json`.
Keep the receipt and inspect final pipeline status/owned-process exit. A stale
receipt or expired stop deadline needs ownership investigation, not `pkill -f`.

## Monitor and transfer to another agent

Arm a watcher only after the actual controller starts; for a pipeline supply
its PID explicitly, since `pipeline` writes `controller.json`, not
`controller.pid`:

```text
/absolute/pinned/watch_run4 arm --root /absolute/RUN/execution --thread CHAT-UUID --controller-pid PID --success-phase complete --require-artifact result.json
/absolute/pinned/watch_run4 check --root /absolute/RUN/execution
/absolute/pinned/monitor_ops4 --root /home/ace/baylee-data/v5/ops-codex-20261028 --check
```

The checks are one-shot; an existing user timer supplies cadence. Watcher
configuration/delivery state is durable and immutable once armed. Its
at-least-once event IDs must be deduplicated. A wake requests review; it does
not authorize a restart, promotion or quota bypass. Neither tool repairs a
trainer. `watch_run4 check` can enqueue a wake to the configured owning chat;
use it only in the established authorized monitoring workflow.

If this agent's weekly quota expires, leave the live controller running within
its existing bounds and transfer the pointer and ownership information. Another
agent reads the same repository/run artifacts and continues the authorized
bounded work; it does not need to replay an old chat or restart a healthy job.
The watcher is bound to a chat UUID, not to whichever model still has quota.
If that chat cannot act, another agent must inspect the run directly; automatic
cross-chat reassignment is not implemented. Do not rewrite an armed watch in
place or erase its delivery ledger to force a second wake.

Paste this concise template into the handover and store its operational pointer
in `next-actions.json` after reviewing actual state:

```text
Reviewed at / checkout / branch / source commit or dirty-source receipt:
Owner instruction and experiment question:
RUN / immutable plan / pipeline SHA256 / pinned tool and worker receipts:
State: prepared | running STAGE | complete | failed | interrupted:
Controller: unit, PID, boot_id, start_ticks; stage job.json; owned-child cleanup:
Watcher: chat UUID, timer, config, event ID, delivered/reviewed status:
Source checkpoint hash / dataset+admission / encoder+architecture+candidate contract:
Bounds: remaining time, update/attempt budget, CPU/RAM, exact GPU/CPU lock paths:
Verified result: accepted updates, consumed choices/histories, KL, tensor/resource audit:
Artifacts preserved / failed attempts / holdouts and exclusions:
Next authorized bounded action and its entry criteria; open blocker/hypothesis:
Gateway28766/live selection preserved; promotion=false; push/gate status:
```

## Findings the next agent must retain

Detailed receipts and limitations are in the
[29 October report](history/trained-ai-v5-rust-refactor-2026-10-29.md) and
[research plan](proposals/trained-ai-v5-refactor-training.md). Findings below
are evidence, not permission to reinterpret old data.

| Finding | Operational consequence and remaining question |
| --- | --- |
| The old candidate context added the same shift to categorical logits, so softmax cancelled it | A versioned multiplicative gate now changes relative scores. Preserve candidate-context revision and matched controls; successful plumbing is not stronger play |
| Payment-settlement passes were discounted as idle passes in live/LLM conversion; a separate hidden discount existed in opt-in loss5 | New conversions use `idle_pass_weighting_revision=2`; corrected loss5 declares `categorical_weighting=symmetric_v1`. Do not relabel caches or exactly resume ambiguous old loss5. E1 used loss4, so the loss5 bug cannot explain E1 |
| E1 supervised improvements were dominated by pass behavior and reused development data | Per-head/source active-choice diagnosis and paired games are required. In 16 cases each, R6 won 6 and both trained candidates won 4; fewer interventions did not establish a strength gain |
| E7/E8 target observations could alias selected X=0 and X=1; X was already stored on the object before targets | E9 adds explicit authorized X presence/value and matching clocks/hashes. This is an observation gap, not proof it caused the observed X=0 multi-target affordability loop |
| Original eight complete TRAIN games provide 1,575 genuine learner stochastic choices; one isolated head-PPO step was accepted | All 24 policy tensors changed and 542 others stayed identical; KL was `3.8877511166e-6`. Four known transfer cases had unchanged outcomes/fallbacks/reversals. This proves one technical head update, neither Full-PPO nor improvement |
| The subsequent full-model E7/R6 pilot accepted all eight attempted updates | 311 of 566 tensors changed, including trunk, relational processing and recurrent memory; final whole-collection KL `4.00522484e-5`. The update groups contained only 131 actor choices / 512 frames; all 1,575 choices / 6,626 learner frames were audited. This is a bounded prefix dose, not a full epoch or evidence of stronger play |
| Its fresh-seed paired follow-up had identical outcomes on all 16 cases, with different canonical action sequences in seven | Both R6 and candidate won 4/16 and required one House recovery in the same case; no caps/refusals. Behaviour changed, but improvement or equivalence is not established. Distinguish audited exposure from differentiated training dose before deciding PPO failed |
| The next B4/U64 experiment retained a full epoch: 27/27 updates, all 1,575 choices / 6,626 learner frames | Final whole-collection KL `7.96981466e-5`, 311 changed tensors, 2,638.27 seconds GPU-stage wall time and zero allocator retries. The separate completed comparison again has 4/16 wins per arm and identical outcomes/interventions; technical learning succeeded, but a strength gain is not demonstrated |
| Device-file access and CPU preflight did not certify CUDA initialization in the frozen mount map | A read-only `/proc` caused CUDA error 304; adding private `--proc /proc` resolved it. A second pre-model failure exposed the allocator API's explicit device-index requirement. Before a GPU run, exercise the actual frozen initialization helper, cap, tiny allocation and synchronization under the final mounts; preserve failed attempts and use fresh run namespaces |
| The full-history source B4/chunk4 probe passed every action-likelihood check but failed hidden/value parity in two histories | Keep chunk4 rejected and the original tolerances unchanged. The 38.454 versus 22.329 second instrumented pass times do not certify a speedup. A separate diagnostic found a failing local update even with identical incoming H; contiguous board storage did not repair it. Accumulated recurrence is not necessary for the first violation. Exact encoder/kernel cause remains unisolated; see the [performance report](history/trained-ai-full-ppo-performance-2026-10-29.md) |
| CPU execution hints at B4/U64/chunk0 matched source and candidate H/value/action log-probabilities/KL exactly across all eight histories | Keep the path opt-in. Single instrumented passes took 38.923→38.067 seconds for R6 and 38.501→38.221 for the candidate; these do not prove repeatable throughput gains, backward equivalence or stronger play. Prioritize the missing research admission/tensor contract and learning objective before further small execution probes |
| Behavior replay can differ slightly from recorded FP32 likelihoods | Keep the recorded executed-action denominator after passing tolerance checks. Replacing it would erase real PPO drift; above-tolerance differences still fail |
| Strictly retaining only clean terminal games censors learner failures that trigger house recovery | The original four E3 cases are not admitted. A new research failure-terminal contract needs collector/converter/target/cache/admission/resume agreement. Unknown MTG results must stay unknown |
| Streaming full-collection KL auditing eliminated repeated fixed-weight history inference | A 256-frame/60-choice prefix measured 4.21x speedup with identical logprobs; this is not a complete-episode or backward/Adam capacity result |
| v5 relational processing and E9 integration pass targeted technical checks | Neither v5 superiority to v4, human-level strength, useful extra K, E9 gameplay benefit nor a need for a larger model has been established. First repair learning signals/contracts and measure full-model learning with the current model |

The [full-model performance review](history/trained-ai-full-ppo-performance-2026-10-29.md)
identifies repeated history inference and synchronization as measurement
candidates. Its first partial audit-lane/spatial-chunk measurement failed parity;
it is not an accepted optimization. Preserve optimizer groups, recorded likelihoods, complete
collection audits and transactional rollback when testing scheduling changes.

The original learner failure prefix has 127 stochastic samples, below the
planned minimum of 128. Case 64's first recovery belongs to the **opponent**;
case 65's belongs to the learner; 96/97 are no-recovery controls. Never equate
record input numbers with evaluator decision sequence numbers. A future
`failure_terminal_research_v1` utility of zero before the first house recovery
would be an explicit task penalty with no bootstrap, not a fabricated MTG loss.
At this runbook's baseline it is **not certified end to end or admitted as
training data**. Native collection now stops before recovery; its separate
read-only Gate A v2 passes all four archived cases, with the original provenance
and canonical hashes verified. See the [native boundary report](history/trained-ai-failure-terminal-native-boundary-2026-10-29.md).
The first audit failed because legitimate scalar Net frames lack categorical
log-probabilities; the correction preserves those frames without inventing an
actor likelihood. The separate conversion-only path and durable operational
ledger are now joined through the explicit finalized-ledger adapter above. A
lossless research storage layer now preserves the converted bytes and all
attempt denominators. A strict tensor boundary, dedicated research admission,
numerical recurrent replay and Torch target/resume identity remain required.
A new flag is not completed training
validation. Opponent failure, legal cancellation, cap and engine fault need
distinct handling.
Track pass/cancel rates and full outcomes because a penalty can encourage
excessive passing. Do not combine this reward change with E9 in one purported
causal test.

The full-model pilot took 988.33 seconds in its guarded training stage, with
2,274,077,184 peak allocated CUDA bytes, 2,696,937,472 peak reserved bytes and
zero allocator retries. This measures B2/U32 on the actual eight-game catalog;
it is not a capacity guarantee for larger boards, batches or another encoder.
Before optimizing throughput, separate history reconstruction, differentiated
updates and repeated whole-collection auditing in a bounded profile. Do not
remove the correctness audit merely to increase utilization. The published
checkpoint and complete evidence are in the dated takeover receipt.

## Encoder and live boundaries

| Contract | Inputs and compatibility | Supported operational boundary |
| --- | --- | --- |
| E7 | Default; entity 128, seat 64, global 80, candidate 48 | Existing datasets/checkpoints/live path retain their identities |
| E8 | Same widths; persistent announced source and numeric prompt context; v5 relational revision 2 | Explicit offline conversion/eval only until the live wire supplies its required context; same widths do not mean same semantics |
| E9 | E8 prefix plus X presence/value: global 82, entity 128; model5/relational2/candidate revision1 or2 | Explicit offline conversion/eval/export/cache contract; live, legacy arena, shadow/LLM conversion reject unsupported paths |

`--encoder-warmstart-from7` (E8) and `--encoder-warmstart-to9` (E9) are explicit
weights-only transitions with fresh training state. E9 retains the first 80
global projection columns and zeros the added columns; a frozen trunk cannot
learn them and is rejected. E7-to-E9 includes E8 changes, so only an E8 control
isolates X. New passive ticks can change full-history H even with zero new
columns. Large raw u32 X values lose f32 precision; hashes retain the exact
integer and presence. Do not relabel E7/E8 probabilities, caches or checkpoints.
Live wire work requires its own audited capability and provenance, not removal
of an encoder rejection. Check the current `seat4` docs/source before claiming
that ongoing implementation is deployable.

The E5–E7 live foundation now supports explicit stochastic raw play and local
decision audits (`631ddec8f`); see the [native/live tooling description](trained-ai.md#native-experiment-control-and-ppo-admission).
For a separate authorized playtest, the CLI shape is:

```text
/absolute/pinned/seat4 --model /absolute/frozen-export --raw-policy --temperature 0.7 --policy-seed 77 --thinking-steps 2 --decision-audits /absolute/NEW/audits
```

It preserves model/depth-graph identity, an independent policy seed, runtime
provenance, semantic observation/history hashes, and distinct generated/sent
records. Draw-rule overrides clear the replaced probability. Exact wire-view
hashes are separate diagnostics because socket timing fields cannot be
reconstructed by ordinary offline replay. The default preview remains greedy.
These audit files are **not admitted training data**. A next agent must still
obtain a complete official anonymous game export, establish the audit/export
join and exact executed actions/authorized observation history, replay behavior
probabilities, validate full source/runtime identity and secure explicit native
admission. Maintain forced/house/clock/teacher/opponent roles and terminal
failure distinctions. E8/E9 wire collection remains blocked; adding stochastic
sampling does not supply persistent announcement/X context or relax the
existing converter/admission contract.

## Evaluate and decide separately from training

Export a fresh isolated candidate with actual PyTorch/CPU-ORT parity at every
required K graph (0/1/2/4/8), preserve its report and model hashes, then run a
zero-deal native load and actual bounded games. `export_onnx4.py --model DIR
--data DATA --check 32` is the PyTorch worker's CLI shape. Numerical parity is
not gameplay validity. Record raw/tactical mode, latency, caps, fallback
reasons, refusals and exact actor/deck/runtime identities.

Reserve fresh deal seeds before tuning, pair seats/decks and report all cases
with uncertainty at table-cluster level. Reused E1/E3 trigger cases and the
256-case reference are development diagnostics. Fresh seeds alone do not
establish unseen-card/deck generalization; require separate data lineage.
Do not turn evaluation fixtures, branch counterfactuals or held-out diagnostics
into training labels. Compare current source and candidate with the same
validator/runtime before attributing a metric difference to learning.

```text
/absolute/pinned/paired_eval4 /absolute/candidate/eval.json /absolute/reference/eval.json --candidate-job /absolute/candidate/job.json --reference-job /absolute/reference/job.json --out /absolute/NEW/paired.json
```

The native comparator verifies completed jobs and matched evaluation inputs.
Changing K requires explicit `--allow-thinking-difference` and a declared
experiment. A development mean utility ranking or better imitation NLL does
not establish strength promotion. `publish_live_model4 --experiment DIR --out
MANIFEST` verifies a particular completed experiment layout before atomic live
selection; do not invoke it merely because training finished. A live selection
is frozen for the whole game. Preserve gateway 28766 until evidence and the
owner's current deployment scope support a change. Run both repository gates
with `DATABASE_URL` before pushing/landing; a historical green gate does not
cover subsequent contract changes.
