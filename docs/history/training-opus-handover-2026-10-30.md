# Opus 5.5 takeover: training research and Terra collection

Owner authorization: push the PyTorch 2.15 migration as a PR, then use the
remaining current Codex weekly allowance for training-data games. Maintain a
queue of **ten simultaneous games**. The owner will close the Codex chat;
collection must not depend on this chat remaining open.

## First checks

Working checkout: `/home/ace/Projects/baylee-ai-payment`, branch
`codex/pytorch-215-migration`, rebased on `origin/main` at `5bae245c0`.
The previous large research/refactoring change is already on main. Migration
commit: `50132ab50`; collector startup fix: `60ca48901`.
Pull request: [#405](https://github.com/AceVik/baylee/pull/405).

Read `AGENTS.md`, [training-runbook](../training-runbook.md),
[research plan](../proposals/trained-ai-v5-refactor-training.md), and
[30 October learning report](trained-ai-research-learning-pilot-2026-10-30.md).
All orchestration, data work and helpers are Rust. Python is allowed only for
PyTorch and its directly associated tests; TypeScript only for web work.

The queue's authoritative local pointer is
`/home/ace/baylee-data/v5/ops-codex-20261028/active-terra-weekly-queue.json`.
It is written only after the dedicated host and collector have launched.
The recovery run for this handover is
`/home/ace/baylee-data/v5/ops-codex-20261028/terra-weekly-queue-20261030/runs/llm-teacher-terra-weekly-01a18cf6-dc18-76f9-b4b9-f4c3960b7934/`.
Two earlier preparation-only namespaces were superseded before any game was
started; their review notes explain the fixes. They are not additional queues.
The first actual startup, `llm-teacher-terra-weekly-01a18ce5-b5dc-744e-a326-64f4b5831e64`
under `/home/ace/baylee-data/v5/`, failed before assigning any games: simultaneous
account setup saturated the gateway's password-hashing workers (HTTP 429).
The collector now serializes account setup, then releases that permit before
gameplay; ten game slots remain concurrent. Sol reviewed that fix with no
blocking finding. The failed run and its database are retained for diagnosis.

The recovery lives under a separate `runs/` parent because the collector
deliberately combines sibling `llm-teacher-lane*` directories for scheduling
and statistics. The failed root's 150 attempts / 118 completed games were
historical sibling totals, **not new Terra games**. Count the recovery root's
own receipts; its parent contains no historical lanes.
Do not infer that collection is running from this document. Inspect:

```text
systemctl --user status baylee-terra-weekly-20261030.service
journalctl --user -u baylee-terra-weekly-20261030.service -n 40 --no-pager
jq . /home/ace/baylee-data/v5/ops-codex-20261028/active-terra-weekly-queue.json
```

The pointer names `root`. Under that root read `queue-status.json`,
`quota-latest.json`, collector `status.json`, `stats.json`, and
`collector.stdout.log` / `collector.stderr.log`. `queue-status.json` contains
PID, boot ID and start ticks. Recheck all three before signaling a process.
The systemd unit owns the entire process tree and is independent of Codex chat.
User lingering was enabled for `ace`, so logging out does not terminate the
user service manager. The queue has no reboot restart or automatic restart.
Never start a second controller to increase its apparent throughput.

## What the queue does

- Terra low (`codex:gpt-6.1-terra`) versus Terra low, ten games concurrently,
  through the signed-in Codex CLI subscription. No API-key or provider fallback.
- Human house decks **Allytifact** and **Victory**, frozen from the previously
  verified house manifest. Seat order rotates. Both have already been designated
  as training/development material; do not claim untouched deck holdouts.
- A fresh dedicated database `baylee_terra_weekly_recovery_20261030`, loopback gateway
  **28788**, admin **28789**, one agent with capacity 12. Preserve user gateway
  **28766**, arena gateways **28776/28777**, their agents and active games.
- Each game has a 90-minute deadline, 90-second model-call deadline, 1,500-call
  and 50-million-token seat caps. These are runaway bounds, not spending targets.
  New attempts replace finished attempts until a stop condition is observed.
- Only technical admission: official complete record, matching inputs and
  model/transcript attribution, terminal replay proof and owned-session cleanup.
  No semantic move-quality filter and no winning-seat-only selection.
- This produces **BC demonstration candidates**, not on-policy PPO experience.
  Preserve complete histories, but never label standing or House fallback actions
  as Terra actions. Preserve incomplete/error attempts in operational denominators.
- Statistics include deck wins/losses/draws, model, opponent, seat, model answers,
  standing answers and fallback counts. Wins use completed games only. Self-play
  deck win rates do not isolate intrinsic deck or model strength.

The native supervisor source, lockfile, frozen configuration and binaries are
retained with the run: `supervisor-source/`, `queue.json`, `profiles.json`,
`pairings.json`, `deck-manifest.json`, `decks/`, and `bin/`. SHA-256 seals in
`queue.json` are checked before launch. Its editable operational source is
`/home/ace/baylee-data/v5/ops-codex-20261028/terra-weekly-queue-20261030/`.
It is Rust, not a shell/Python launcher. To reproduce its build/tests use
`cargo test --locked`, `cargo clippy --locked --all-targets -- -D warnings` and
`cargo build --locked` in that directory. The collector itself is built from
the repository with `cargo build -p baylee-train --features llm-collector --bin llm_collector4`.

## Quota and stop behavior

Preparation observed approximately **13–14% remaining**. This is a timestamped
snapshot, not a promise about current availability. The subscription's current
weekly reset was reported around **4 November 2026, 14:26 Europe/Berlin**.
`queue.json` freezes the actual timestamp. The supervisor stops two minutes
before it; it must not spend the next week's allocation automatically.

Every 30 seconds it reads the official Codex app-server
`account/rateLimits/read` endpoint through the existing login. Only sanitized
usage/window fields are retained, never account identifiers or credentials.
The endpoint's reset timestamp can round by a second between reads; up to 60
seconds of rounding is accepted without moving the frozen cutoff. See the
[official app-server protocol](https://developers.openai.com/codex/app-server).

Stop conditions: exhausted subscription window, denied ordinary usage, changed
weekly window, explicit quota failure in a model-failure transcript, unreadable
quota status, ten most recent finished attempts failing admission, less than
8 GiB free disk, lost collector/host, operator stop, or the frozen reset cutoff.
No automatic restart or retry into another allowance. A stop at quota can leave
partial games; retain them as incomplete, not completed training examples.

Use the absolute `root` from the pointer in place of `ROOT`:

```text
ROOT/bin/terra-weekly-queue drain ROOT
ROOT/bin/terra-weekly-queue stop ROOT
systemctl --user stop baylee-terra-weekly-20261030.service
```

`drain` stops new assignments and lets current games finish, within their bounds.
`stop` requests immediate collector cleanup. `systemctl stop` also terminates
the owned service tree if graceful cleanup fails. Do not remove `STOP`, `DRAIN`,
quota evidence or `queue-started.json` to force a restart. The supervisor locks
both the run and the shared queue location and refuses an already-started root.
After failure or a deliberate restart, inspect receipts, verify cleanup, and
prepare a fresh UUID-v7 namespace. A later week's spend requires owner direction.

## Replays and training admission

Under `attempts/attempt-*`, `receipt.json` says `admitted`, `error`, `timeout`
or `incomplete`. An admitted attempt includes `technical-result.json`, the
official anonymized compressed record, `public-replay.jsonl.gz`,
`replay-receipt.json`, input receipts and per-seat transcripts. Follow the
artifact paths and hashes in the receipt, rather than guessing filenames.
`stats.json` and `stats.csv` aggregate these results.

Raw transcript files can contain private seat information. Keep them local.
Use the official anonymized record and the native verified conversion/admission
path described in the runbook for training. Do not upload local records or
transcripts into a public PR. The new main's engine/card catalog must remain
identified with these games; do not attach them to old sealed manifests or
silently reuse an older catalog/encoder cache.

## Research decisions to preserve

1. The bounded full-model PPO paths demonstrably change weights and behavior,
   but their paired games have **not demonstrated a net strength improvement**.
   The breadth comparison was one better, one worse, nine equal, five unresolved.
   A lower aggregate imitation loss or successful optimizer step is insufficient.
2. Source-state diagnosis passed all 726 audited choices / 2,915 frames, maximum
   log-probability error `0.0000131131`. The inspected divergence/loop points were
   outside update loss windows. More of the same short-prefix updates is not a
   supported fix; record actual differentiated training exposure.
3. Next bounded BC comparison: fixed data, weights, seeds, encoder, K and update
   budget; compare Loss5 with and without `--outcome-weighting-off`, both with
   `categorical_weighting=symmetric_v1` and newly converted
   `idle_pass_weighting_revision=2` data. Include both winning and losing teachers.
   Fresh optimizer, explicit weights-only start; no silent old-state resume.
4. Disabling policy outcome weighting does not remove shared-trunk value
   gradients. Test `--detach-value-input` separately afterwards; auxiliary
   gradients are another isolated ablation. Measure genuine non-pass choices,
   payment completion, cancellation/loops and per-head/per-source performance.
5. Colors, mana symbols and ordered rule trees already enter the model.
   Card-learning tasks should test applied consequences: affordability, X and
   extra/alternative costs, target restrictions and immediate effects, with
   engine-verified public-state labels and card/deck holdouts. Copying visible
   card attributes alone does not establish understanding.
6. New main adds cards/mechanics. Freeze catalog, DSL, engine, converter and
   export identities together. An encoder/tag-space change is a separate
   migration, not an exact resume or part of the first outcome-weighting test.
7. The old continuous BC service stays stopped. No unattended GPU optimization
   or live-model promotion is attached to this queue. All CUDA stages use
   `/home/ace/baylee-data/v5/experiments/gpu-work.lock` and bounded native guards.

## PyTorch migration and review

The active checkout's `tools/trainer/.venv` is PyTorch **2.15.0+cu132**,
Triton 3.9.0, CUDA toolkit 13.2.2, Python 3.13.15. The old checkout's 2.14
environment remains for historical sealed runs. Explicit cuRAND installation
is necessary for the native ONNX CUDA provider after the PyTorch upgrade.

The runbook records CPU tests, 18 selected existing CUDA tests, regional
FP32/BF16 compiler validation, full-sized checkpoint forward comparison and
native CUDA inference with CPU fallback disabled. Sol reviewed the migration
read-only and found no blocking issue. These checks establish compatibility
within the tested scope, not improved strength, throughput, all real-state
compiled training, macOS/MPS support or cross-version optimizer continuation.

Gate and migration evidence:
`/home/ace/baylee-data/v5/ops-codex-20261028/torch-215-migration-01a18cc3-f303-76d0-82bd-f4df25dcf719/`.
`pr-gate/` and `pr-gate-features/` retain the initial pre-push gate receipts.
`pr-gate-collector-fix/` and `pr-gate-features-collector-fix/` retain the
follow-up checks for the account-startup fix. Read their final `job.json`
receipts instead of assuming a launched check has passed.
The PR is separate from model deployment; do not promote a checkpoint because
the software migration passed.
