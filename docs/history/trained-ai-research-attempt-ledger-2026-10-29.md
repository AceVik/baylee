# Failure-terminal research: operational attempt accounting

29 October 2026. This is a CPU-only operational foundation. No game, model
inference, training update or GPU workload was launched to verify it.

## Contract and storage

The native failure-terminal collector now creates `attempts.plan.json`
(`failure_terminal_attempt_plan_v1`) before spawning its worker threads. The
plan binds the unchanged source `run.json` hash, the producer's Linux PID,
process-start ticks and boot ID, and every planned deal's seed, learner seat,
independent policy seed and ordered deck keys. Planning does not establish that
an attempt was assigned or played.

Before a worker enters the engine, it publishes the immutable
`attempts/<20-digit-deal>/claim.json`. The claim binds the plan/run hashes,
producer identity, worker and deal. Existing claims cannot be overwritten.
Each successfully returned episode remains in the existing gzip shard and
`games.jsonl`. Research-only publication syncs the shard and index, verifies
the indexed gzip member's decompressed record hash by reading it back, then
atomically publishes the exact index row as the attempt's `game.json`.

Research worker errors and Rust panics are handled after joining the workers;
the join loop drains every handle. An unfinished claimed attempt may receive
`abort.json` with `worker_error`, `observation_error` or `worker_panic`. None
receives a fabricated game outcome, record, sidecar or loss. Observation errors
are classified through the research-only observation error context. A panic
is classified as a worker panic without guessing the operation that caused it.

The final immutable `attempts.final.json` uses
`failure_terminal_attempt_ledger_v1`, sealed by `attempts.complete.json`.
Every planned deal has exactly one of these statuses:

| Status | Meaning |
| --- | --- |
| `recorded` | A durable claim and completed game receipt match the original index row and readback-verified record. Replay and training admission remain separate. |
| `operational_unknown` | The deal was durably assigned, but no completed game receipt exists. Utility is null, bootstrap is false, and no game/sidecar is invented. |
| `not_started` | No durable claim exists. It is a planned deal, never counted as played. |

Counts retain planned, assigned, recorded, operational-unknown and not-started
denominators separately. A kill between index publication and game-receipt
publication is conservatively unknown. A trailing torn index fragment and
partial/orphan shards remain unchanged as sealed evidence. Complete indexed
rows outside the plan or without matching claims are rejected; malformed
canonical completion receipts also prevent finalization.

## Native runner and recovery

The purpose-specific runner uses the existing process-group guard:

```text
/absolute/train_ops collect-research --run /absolute/NEW/records \
  --job /absolute/NEW/collector-job --timeout 60 --memory-gib 8 -- \
  /absolute/eval4 --collection-purpose failure_terminal_research_v1 \
  ...explicit frozen models, CPU collection arguments and record destination...
```

This wrapper rejects `BAYLEE_EP=cuda`; it grants no GPU ownership and performs
no automatic retry. Existing ordinary `train_ops run` and evaluation behavior
remain the same. The guard's new `cleanup_verified` receipt is true only after
its owned leader and live members of that process group have exited. It does
not claim to find descendants that escaped the group.

After the bounded child ends, the wrapper finalizes the ledger, including on
deadline, memory-limit or unsuccessful child exit. A standalone recovery is:

```text
/absolute/train_ops finalize-research --run /absolute/PRESERVED/records \
  --job /absolute/PRESERVED/collector-job/job.json
```

Recovery never signals or restarts a process. It requires the matching producer
identity to be dead, the producer's lifetime lock to be available, a terminal
guard receipt and explicit verified cleanup. An old or incomplete guard receipt
cannot substitute for cleanup proof. The finalized ledger binds SHA256 seals
for the plan, run, original index, all record shards (including partial ones),
claims, game/abort receipts and guard receipt. Evidence is rechecked before
publication; downstream consumers must revalidate these seals too. Existing
finalized files are immutable and idempotent recovery verifies their evidence.
If the final ledger was published but its completion seal was interrupted,
recovery recomputes the whole ledger after the same ownership checks and
requires byte-identical existing content before publishing only the missing
seal. Changed ledger bytes or source evidence are rejected.

If the wrapper itself is killed before it can finalize, durable claims remain
available for explicit recovery once the guard's cleanup proof exists. If that
proof is absent, finalization refuses. A process that fails before the plan is
published has assigned no attempts under this contract and retains only the
outer guard's failed-start artifacts. A disk failure can prevent receipts from
being published; no in-process code can guarantee a write after SIGKILL.

## Verification and limits

Focused Rust fixtures cover the fixed denominator, duplicate assignment,
record/hash tampering, changed sealed receipts, partial and truncated
publication, an observation-panic simulation, and live/recycled producer
identity. A separate bounded native fixture runs a Rust helper that claims one
attempt and parks. The actual guard terminates it at a one-second deadline;
the wrapper verifies cleanup and emits one null-utility operational unknown
and two not-started entries. It confirms that the original process identity is
dead. This fixture performs no game or model work.

Passed the nine focused `research_attempts` tests, the native CLI parsing test,
an incremental ONNX `eval4` check, and Clippy including the operational tests
with warnings denied. Cargo used two build jobs throughout:

```text
cargo test -p baylee-train-ops --lib research_attempts -j 2
cargo test -p baylee-train-ops --bin train_ops -j 2
cargo check -p baylee-train --features onnx --bin eval4 -j 2
cargo clippy -p baylee-train-ops --lib --bin train_ops --tests -j 2 -- -D warnings
```

Actual out-of-memory termination and a SIGKILL of the outer wrapper were not
exercised. Their recovery relies on the same durable claims and explicit guard
cleanup proof, with unknown outcomes retained conservatively. The current
research converter's first source adapter still declares
`attempt_denominator_contract = complete_index_v2`; it does not yet consume the
new final ledger. A future explicit adapter must retain recordless operational
unknowns and not-started entries without manufacturing training frames. Neither
ledger finalization nor a successful guarded command grants BC/PPO admission.
