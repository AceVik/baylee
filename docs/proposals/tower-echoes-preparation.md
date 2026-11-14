# Tower Echoes — preparation and handoff checklist

14 November 2026. Active Baylee branch: `codex/tower-echoes`. The
[design](tower-echoes-v2.md) defines the target; the [review history](tower-echoes-readiness.md)
records why it changed. This shorter checklist distinguishes working preparation
from production functionality. Nothing here establishes that Tower v2 is playable.

## Fixed requirements versus calibration

| Fixed by the owner | Initial calibration, still to playtest |
| --- | --- |
| Three Commons → Uncommon; three Uncommons → Rare; **one Rare + Mythic Potion → Mythic** | Prices 4/10/26 and potion 24; no claim that rarity alone determines strength |
| Same-stage player builds, manually played against Expert AI | Eight wins/five unsuccessful duels; 10–20 minute ordinary duels |
| Global Baylee Tower authority, duels hosted on gateways | Three active relics, six reserve slots, two duel charges |
| Central identity/permanent achievements; Daily/Weekly stay Central | 18 starting gold, 12 per verified terminal duel |
| Three compiled engine variants; detection absent from standard default and Tower | Higher-rarity opportunities at stages 2/4/5, full-price potion offer at 7 |
| Difficult, powerful combos allowed; no automatic Tower loop adjudication | Exact item effects, rarity distribution, matching preference and badge thresholds |
| Careful substantial refactoring permitted after the complete handoff | First diagnostic samples, then targeted expansion based on findings |

Keep acquisition probability separate from duel win probability. A toy estimate
of reaching stage 7 says nothing about whether a particular Rare appeared or
whether the player saved enough gold. Paid basis follows actual ingredients;
rarity's catalog price must not overwrite it after fusion. The nine-Common Mythic
path costs 60 at these prices; three directly bought Uncommons plus potion cost
54; a direct Rare plus potion costs 50. All compete with cards and refreshes.

## Offline tools prepared now

Implemented in the existing small `baylee-dev` binary, which does not depend on
the game engine or card pool. No new library versions or external dependency
graph were adopted; `serde` is now a direct dependency at its existing lock entry.

```text
cargo dev tower check
cargo dev tower fixtures
cargo dev tower pacing --wins 8 --defeats 5 --win-per-mille 500
cargo dev tower compare /absolute/path/to/adapter-observations.json
```

- `check`: compare manually specified expectations with small independent
  reference oracles. Revision 1 has 15 fusion, 5 resale and 18 execution cases.
- `fixtures`: export the complete versioned inputs and expectations as JSON for
  future test adapters. The fixture schema is a dev/test contract, not the future
  public Tower API or an authenticated executable manifest.
- `pacing`: bounded analytical calculation for independent duel outcomes, including
  terminal probability, expected duel count, stage reach and gross gold before
  spending. No real game, RNG, shop generator or telemetry is involved.
- `compare`: require every known case exactly once, reject unknown IDs/versions,
  mismatched results and malformed input. Input is bounded to one MiB; payload
  fields are not reflected in errors. Matching caller-provided observations does
  not authenticate the producer or prove that a binary omitted detector code.

The reference oracles are only for recipe arithmetic and declared execution
metadata. They do **not** implement an inventory, database transaction, agent
launcher, game engine or production authorization boundary. Production must not
depend on `baylee-dev`; future adapters invoke the actual implementation and
translate its outputs into the observation schema. Until then the command reports
`production_adapter_connected: false` and never claims an engine test passed.

Validation of this preparation milestone:

```text
cargo test --offline --locked --target-dir target/tower-prep -p baylee-dev --bin baylee-dev tower::
cargo clippy --offline --locked --target-dir target/tower-prep -p baylee-dev --all-targets -- -D warnings
```

Eleven focused tests pass, including the 120 small ingredient-count combinations,
multi-generation paid basis, recipe overflow, metadata order independence, wrong
contracts, missing/duplicate/extra observations, version/schema errors, bounded
redacted decoding and independent probability/terminal checks. Strict Clippy
passes for the changed developer crate. CLI `check`, `fixtures` and `pacing` also
ran; passing the fixture export itself to `compare` was correctly rejected as
the wrong observation schema. No production adapter is connected.

The default toy model reports conquest probability 0.19384765625 and expected
duel count 9.2294921875 at an assumed independent 50% win probability. These are
mathematical model outputs, not observed player or AI performance. Local logs:
`/tmp/baylee-tower-prep-tests-20261114.log` and
`/tmp/baylee-tower-prep-clippy-20261114.log`; exported reference/model JSON is in
the same temporary directory under the `baylee-tower-reference-v1-20261114` and
`baylee-tower-pacing-20261114` names. No test client, database or game server ran.

Observation schema 1, contract revision 1:

```json
{
  "schema": 1,
  "contract_revision": 1,
  "implementation": "actual-tested-build-id",
  "cases": [
    {
      "id": "fusion-mythic",
      "result": {"kind": "fusion", "value": {"rarity": "mythic", "gold_basis": 50}}
    }
  ]
}
```

The example is intentionally incomplete and will fail until **all** exported
cases are present. Fusion values are an output object or null for refusal;
resale values are integer gold; execution values are a variant ID or null.
Execution cases include feature fingerprint as well as variant/build/pack/cards/AI
identity, bad capabilities, absent capacity, duplicate entries and no fallback.

## Checks that must use actual production paths later

| Surface | Concrete evidence required | State |
| --- | --- | --- |
| Fusion ownership | Foreign/stale/consumed instance IDs, exact recipe, reserve/equip limits | Deferred: no v2 store adapter |
| Gold/atomicity | Concurrent buy/fuse/sell, duplicate request, failed transaction, inherited paid basis | Deferred: no v2 store adapter |
| Shop/recovery | Pinned stock, sold refresh, one-time guaranteed offer, gateway reconnect and private seeds | Deferred: no v2 shop adapter |
| Agent routing | Actual advertised variants, launch/Hello/setup mismatch, shared capacity and missing executable | Deferred until handoff/refactor |
| Compiled detection | Separately built artifacts, feature graph, code path and long finite combo profile | Deferred until handoff/refactor |
| Rules and all seats | Upkeep, delayed effects, source zones, replacement effects, costs, projections, choices | Deferred until handoff/refactor |
| Echo/result authority | Same-stage cohort, trusted host, exact replay, consumed charges, outbox deduplication | Deferred: no global service integration |
| Usability and fun | Real native DE/EN shop/table, five refinements, human pivots/Mythic/combo play | Deferred: no v2 playable client |

No ignored placeholder test counts as evidence for these rows. A reference
oracle self-check cannot close a production acceptance item. Keep the corpus
version explicit if its inputs or expectations change after adoption.

## Bugfix gate: exact owner-directed handoff

The swarm's stated scope is benchmark A1–A16/B1/C1; observed faults
#53/#67/#68/#71/#76/#77/#80/#81; and 51 `Coverage::Generated` cards. It is expected
to run `cargo dev gate`, `cargo dev gate features` and `cargo dev gate shipped-features`.
These are handoff requirements, **not independently verified completion claims**.

Remote branch: `agy/pre-refactor-handoff`. The initial base supplied by the owner
was `9bf1290c7`. Readiness exists only when the final tip's trimmed trailer value
is exactly `READY-2026-11-14` (subject `chore(handoff): pre-refactor bugfixes ready for Astra`).
An older READY ancestor, a substring or an unmarked new tip is not sufficient.

The temporary std-only Rust program under `/tmp/astra-handoff-watch` invokes
`git fetch --quiet origin refs/heads/agy/pre-refactor-handoff`, reads the exact
FETCH_HEAD trailer and sleeps with `std::thread::sleep` for 30 seconds until it
matches. It prints the pinned SHA/message and exits zero. It is not a shell loop,
service deployment or app automation. Keep the current task waiting for its exit.

After exit zero:

1. Save any completed independent preparation as a commit; require a clean tree.
2. Fresh-fetch `origin refs/heads/agy/pre-refactor-handoff`, resolve FETCH_HEAD and
   recheck its tip's exact trailer. If it is no longer READY, wait again.
3. `git merge --no-ff FETCH_HEAD -m "Merge agy/pre-refactor-handoff: pre-refactor bugfixes"`.
   Resolve real overlaps while preserving the fixes; verify successful ancestry.
4. Immediately delete the remote handoff branch, the same local branch if it
   exists, and prune origin. Remove only the owned `/tmp/astra-handoff-watch`
   directory once its process has exited. The explicit owner instruction permits
   this merge/deletion sequence; do not silently substitute a rebase.
5. Read `git --no-pager diff --stat HEAD^1..HEAD`, new `gemini-bugs-audit.md` and
   `docs/observed-faults.md` entries. Inventory fresh PROTOCOL_VERSION/VIEW_VERSION,
   PublicObject, LogEvent, delayed.rs, chosen.rs/for_each.rs and reach.rs changes.
6. Record the actual handoff SHA and start the planned refactoring from that
   baseline. Preserve the new regression cases and use targeted tests for overlaps.

The preparation milestone `735a6ff36` touches only developer tools, their direct dependency
declaration/lock entry and documentation. It changes no rules, AI, protocol,
agent, gateway, view, client renderer or generated card implementation.

## Handoff received and refactoring started

The Rust watcher exited **0** with `3fc9412c58b07c446c5f7e0c062965d6e4d2b984`.
A fresh fetch returned that same tip and the exact `READY-2026-11-14` trailer;
its subject was `chore(handoff): pre-refactor bugfixes ready for Astra`.
The tip also declares `Gates-Verified: gate, gate-features, gate-shipped-features`.
This records the swarm's gate attestation; it does not claim a second independent
full-gate execution on the merged feature branch.

The clean `codex/tower-echoes` branch merged it without conflicts as
`aa7edb78d2b571af47ed45be93a6d7798e9c8f9e` (`--no-ff`, owner-specified message).
Its second parent is the exact handoff SHA. The remote and local handoff refs
were deleted, origin pruned, remote absence checked, and the completed watcher's
owned directory moved to Trash; `/tmp/astra-handoff-watch` no longer exists.
Main and the swarm's checkout were not modified.

Reviewed the merge stat (268 files, 17,142 additions / 1,575 deletions), the
benchmark fault register A1–A16/B1/C1, and the new observed-fault entries. Contracts
to preserve through the refactor:

- **Protocol 47 / view 93**; read their constants instead of repeating them in
  callers. AI generation 19 adds chosen/linked-source filters.
- `PublicObject` now projects second subtype, chosen card type/number and
  **public** linked exile. Face-down identity remains hidden; grouping keys and
  all seat consumers must retain that distinction.
- `LogEvent::DidNotResolve::ability` distinguishes an ability from its source
  spell; `CardsChosen` and held APNAP choices preserve visible earlier choices
  without revealing private hand picks.
- `DelayedAction::AddMana` retains source incarnation and reaches the stack as
  a delayed triggered ability. Tower upkeep scheduling must preserve its
  response window instead of directly modifying the mana pool.
- `chosen.rs` / `for_each.rs` hold APNAP picks and carry out the batch together,
  preserving departure context and checking incarnation/control eligibility.
- Client `reach.rs` evaluates each offered face's timing, targets and price;
  the old front-face shortcut must not return through mode-aware UI work.
- The new card regression fixtures remain in their generated/testkit pipeline;
  no corpus regeneration was used for the handoff or initial kernel extraction.

The first extraction moves `on_loop_detected` and `loops_broken` unchanged into
`engine/loop_control.rs`. This prepares a single place for static loop control;
it does **not** yet disable detection or change the normal table default.

The second extraction puts the suppression latch, saved watch and diagnostic
counter into one `LoopControl` value. Checkpoints restore that value atomically;
simulation forks copy it only after the same hidden-signature/suppression guard.
The guard destructures its contents exhaustively, so adding detector state
requires an explicit fork decision. Snapshot hash contributions and individual
diagnostic fingerprint names/values are preserved. No speed or size improvement
is claimed before the variant measurements.

Focused validation on the merged baseline (14 November):

```text
cargo nextest run --offline --locked --cargo-profile gate -p baylee-engine --lib --features fork \
  -E 'test(engine::loop_tests::) | test(engine::loop_false_positive_tests::) | test(engine::refusal_tests::a_refused_answer_moves_no_field_of_the_engine) | test(engine::simulation_tests::every_refusal_has_its_case)'
# Move-only extraction: 11 passed, 0 failed; 2,621 unrelated tests filtered out.

cargo nextest run --offline --locked --cargo-profile gate -p baylee-engine --lib --features fork \
  -E 'test(engine::loop_tests::) | test(engine::loop_false_positive_tests::) | test(engine::refusal_tests::a_refused_answer_moves_no_field_of_the_engine) | test(engine::simulation_tests::every_refusal_has_its_case) | test(engine::checkpoint_tests::) | test(engine::trigger_order_tests::)'
# State isolation: 32 passed, 0 failed; 2,600 unrelated tests filtered out.

cargo clippy --offline --locked -p baylee-engine --lib --features fork -- -D warnings
# Passed. The pre-existing Cargo workspace.package.description manifest warning remains.
```

`cargo fmt --check -p baylee-engine` and `git diff --check` pass. Local logs:
`/tmp/baylee-loop-extraction-20261114.log`, `/tmp/baylee-loop-state-20261114.log`
and `/tmp/baylee-loop-state-clippy-20261114.log`. The second run was justified by
the subsequent state-layout/checkpoint change; no broad game benchmark was run.
All owned test commands and the watcher have exited; no clients or services were
started. Subsequent publication follows the owner's changed-crate gate scope.

## Foundation landing and life-rule regressions — 14 November

The owner authorized the foundation merge and full implementation. Baylee
`origin/main` was fast-forwarded to `0d9cee40d82e746ed17964e81950846e87223a73`
after the combined snapshot passed both local gates (28,217 regular tests and
2,744 client-feature tests, plus the other scoped feature checks). Logs:
`/tmp/baylee-main-gate-20261114.log` and
`/tmp/baylee-main-features-20261114.log`. The feature branch is based on that
main. Central PR 66 was subsequently rebased into main after its protected checks
passed; main is `4ededee1887bd7446c934d3b6d8536cfbcd2314e`. Central work continues
on `codex/tower-achievements`. Engine implementation did not wait for that CI.

The owner's latest QA instruction limits subsequent gates to changed crates and
directly affected interfaces. Do not repeat the completed workspace runs for
each milestone. The owner also withdrew the proposed lethal-life-payment ban.

Checked against Wizards' [Comprehensive Rules, effective 6 November 2026](https://media.wizards.com/2026/downloads/MagicCompRules%2020261106.txt):
CR 119.4 allows paying exactly one's life total; 704.3/704.5a apply the loss
before the next priority window. CR 603.3b orders simultaneous triggers APNAP.
The full stack does not resolve before checking loss. CR 704.4 separately
excludes state-based actions during a single resolving spell/ability.

The existing payment implementation is correct. Added real-card regressions
using Windswept Heath at one life and Opportunity with two opposing Sheoldreds:
own-turn lethal loss, own-turn survival, opponent-turn gains first, both trigger
ordering settings, and Platinum Angel allowing intermediate zero life. No
house-rule prohibition and no card implementation changes were made.

Validated with:

```text
BAYLEE_BUILD_STAMP=none cargo nextest run --offline --locked --cargo-profile gate \
  -p baylee-engine --lib --features fork \
  -E 'test(engine::life_timing_tests::) | test(engine::life_cost_flow_tests::) | test(state::tests::the_last_point_of_life_is_still_payable)'
```

14 tests passed; 2,623 unrelated tests skipped. The initial new fixture used the
wrong answer shape for a player-only target; correcting it to `ChoosePlayer`
made all scenarios pass. Log: `/tmp/baylee-life-timing-20261114.log`.

## Static loop control — first working kernel milestone

`Engine<L, D>` now owns its sealed detection policy as a type. `DetectLoops`
preserves the existing detector and its break/draw behavior. `NoLoopDetection`
and its segment watch are zero-sized; they never invoke signature, reset or
stack-drainage closures. Both player-answer and decision-free segment paths use
the same statically dispatched seam. Ordinary numeric checks and replay hashes
remain independent. No vtable or runtime enabled/disabled flag was introduced.

`LoopPolicy::Off` is appended after the existing serialized variants. Construction
rejects a policy/type mismatch before creating the game. Checkpoints, suspended
payment rollback and fair simulation forks retain the same type. Legacy
`Engine::new` deliberately remains enabled during the staged host/protocol
migration; the new-table default has not yet changed in this milestone.

Strict Clippy for `baylee-core` and `baylee-engine` (all targets, fork) passed.
51 targeted engine tests passed, 2,591 unrelated tests skipped: existing loop
break/draw/false-positive cases, trigger ordering, payment/checkpoint rollback,
hidden-information fork refusal, the life-rule scenarios, and five new static
policy cases. The off case keeps a real repeating blink loop running for 10,240
answers until concession; the ordinary 256-answer comparison preserves decisions,
events and game state in both variants. Logs:
`/tmp/baylee-static-policy-clippy-20261114.log` and
`/tmp/baylee-static-policy-tests-20261114.log`.

This establishes kernel behavior, not yet the three installed server artifacts,
agent routing, release-code omission or measured speedup. Those need their own
build/host evidence before the feature is presented as complete.

## Static policy across the host — second working milestone

`Session<D>`, its table views and `SimulationSession<D>` now retain the kernel's
compiled policy. Typed construction, recorded construction and resumption reject
mismatches; legacy constructors preserve their existing enabled behavior.
Recorder/checker operations accept either type, and
`replay_exact_with_detection<D>` replays every hash with the matching policy.
There is no cross-policy fallback. This also keeps the policy through house-AI
foresight and its hidden-information forks.

Strict Clippy for gamehost, all targets with `simulation`, passed. 15 focused
tests passed, 427 unrelated tests skipped, including legacy resumption and loop
false-positive boundaries. New cases compare seat views and Expert play over
128 decisions in both variants using Allytifact versus Victory, and check an
Off record's exact replay, continuation/resumption, wrong-variant rejection and
simulation isolation. Logs: `/tmp/baylee-static-host-clippy-20261114.log` and
`/tmp/baylee-static-host-tests-20261114.log`. No client or network service was
started for these in-process tests.

## Static server artifacts — third working milestone

The server's transport driver, recorder and replay projection retain the compiled
loop type. Default builds currently retain the legacy `standard-loops` artifact;
`--no-default-features` builds `standard`, and adding `--features tower` builds
the prepared Tower artifact. Combining Tower and loop detection fails compilation.
The Tower item rule pack and global service are still being implemented; this
milestone alone does not make that mode playable.

An isolated `--metadata` command emits a bounded execution identity without
reading credentials or opening sockets. Variant, source, features, rules, card
registry and AI identity are explicit. Metadata contains no executable path or
download instruction and is compatibility evidence, not operator attestation.
The owner confirmed that the gateway selects this identity and the agent starts
only a locally configured matching artifact. Wire admission is the next step.

36 targeted protocol/host/server tests passed (697 unrelated tests skipped),
including both policy mismatch directions, strict metadata parsing, hostile
transport inputs, curtain/record boundaries and Off replay projection.
Strict all-target Clippy passed for default, Off and Tower configurations.
Logs: `/tmp/baylee-static-server-tests-20261114.log`,
`/tmp/baylee-static-server-clippy-20261114.log`,
`/tmp/baylee-static-server-off-clippy-20261114.log`, and
`/tmp/baylee-static-server-tower-clippy-20261114.log`.
Release-code omission and performance are not yet measured.

## Gateway-selected artifacts — fourth working milestone

Protocol 48 carries bounded execution identities in agent inventory, start order,
engine hello and game setup. The gateway reserves one compatible artifact and
one shared capacity slot atomically. The agent probes only operator-configured
paths, resolves exact identities locally and refuses unavailable variants and
duplicate game IDs requesting a different artifact. No path, environment or
download command arrives from the gateway. The per-game authenticated attach
checks every identity field before exposing the private preset; the executable
checks its own compiled identity again. New hosted record headers retain it.

Replay jobs select the recorded loop variant through a bounded metadata read,
and verify their reserved artifact before receiving private records. Legacy
headers retain their existing enabled policy. Legacy v1 challenge certification
still names a build; this does not pretend to replace it with a full-artifact
certificate or attest the honesty of a remote host.

20 agent-library tests passed in the initial targeted run. Two fixture defects
then stopped that run: the old command-line control expected a nonexistent
executable to pass startup, and a new assertion compared a canonical macOS path
against its `/var` alias. Both were corrected. The remaining/affected selection
passed 63 tests (1,364 skipped), including real gateway socket admission, wrong
token isolation, wrong-artifact refusal before disclosure, transport lifecycle,
private/public replay perspectives, record boundaries, shared capacity and
5-second metadata timeout/reaping with a harmless Rust executable. The owned
PostgreSQL fixtures were removed and port closure verified on both success and
failure. No client or unrelated service was touched.

Strict all-target Clippy for protocol, agent, server, gamehost and gateway passed.
Off and Tower artifact configurations each passed their two metadata/command
tests. AsyncAPI was regenerated from the protobuf source. Evidence:
`/tmp/baylee-engine-routing-tests-20261114.log`,
`/tmp/baylee-engine-routing-tests-20261114-r2.log`,
`/tmp/baylee-engine-routing-clippy-final-20261114.log`,
`/tmp/baylee-engine-routing-off-20261114.log`,
`/tmp/baylee-engine-routing-tower-20261114.log`.
Linux-only SIGTERM coverage was updated to use a Rust fixture but was not run
on this macOS host. Release artifact packaging, new-table/client defaults and
the Tower v2 rules/service remain subsequent milestones.

## Local variant continuity — fifth working milestone

The native local host now keeps its static loop type through its session, house
search, recorder and background worker. Production installation selects it once
from the preset; replay and restart select the saved policy, and restart checks
the reconstructed hash before installing a worker. Legacy constructors retain
their enabled policy for existing integrations. The directory-module move also
needed its relative embedded-deck paths adjusted, caught by the first check.

Eight targeted client tests passed (2,603 skipped): actual background-host
installation for all three policies, play and restart, wrong restart hash,
an Off game projected through the client's real replay path, and the existing
worker responsiveness/streaming/resumption regressions. Evidence:
`/tmp/baylee-client-static-host-tests-20261114.log`. No graphical client was
started. This verifies type continuity, not containment of an unbounded kernel
operation: process supervision for that case remains separate required work.

## New-table policy and visible selection — sixth working milestone

New ordinary gateway, offline and practice tables now default to Off. The
legacy house-rule default and frozen challenge presets retain their original
policy. An optional room-setup field carries the explicit selection; unknown
JSON and protobuf policies are refused, and an absent old wire field retains
its legacy meaning. The client offers Off, interrupt-once (house rule), and
draw-on-mandatory-loop with localized explanations. Draft edits and template
changes preserve the selection. A real gateway/agent/seat integration test
reaches GameStatic through the appropriate concrete engine for each option.

Selective checks covered the changed core, client model, protobuf parser,
gateway reservation/connection and replay paths. Repairs to test fixtures
updated their default artifact and supplied a real routing header for replay
size-bound tests; a corrupt header is also refused. Evidence is split across
`/tmp/baylee-new-table-tests-20261114.log` and its `-r2` through `-r5`
logs (failed fixture assumptions are retained, not presented as clean runs).
All intended cases passed after those corrections. Strict affected-crate
Clippy passed, including the final gateway fixture and native client passes.
The API documents were regenerated with the existing generator.

Two native dev-control tests passed in English and German, at 1280×720 and
1440×900, with reduced motion, virtual input and the owner's additional left
screen. They exercise all three choices and assert actual unclipped label and
explanation geometry. The first run found that rebuilding the sheet reset
scroll position: the sheet now has its own retained scroll position, reset
only when opened. Later image assertions made the test scroll further after
shrinking the viewport. Final evidence:
`/tmp/baylee-loop-picker-ui-20261114-r5.log`,
`target/e2e/run-1794674679/`, and
`/tmp/baylee-new-table-ui-clippy-20261114.log`.
All owned clients exited and their loopback ports were verified closed.
This is a table-settings regression check, not the five required Tower UI
refinement passes, nor a claim that default-Off resource containment is finished.

## Separate artifact packaging — seventh working milestone

The shared PGO builder now has three explicit artifact selections. Training,
profile use and the unprofiled semantic comparison all retain the same package
and feature arguments, with separate directories per artifact. The previous
plain comparison discarded the shipping selection, which would compare a
detector-free artifact against the legacy detected session. The fixed workload
now instantiates the matching session type, including simulation forks and replay.

Both `cargo tools pgo engine-server` and server staging build the three artifacts
independently. Deployment retains and requires every one before stopping the
drained owner. Default agent startup probes only the two known siblings beside
its primary engine; an explicit primary or additional list disables that
discovery. Empty additional configuration deliberately means none. Paths and
command arguments still never come from the gateway.

Local evidence:
- `/tmp/baylee-pgo-variants-tests-20261114.log`: 16 PGO checks passed.
- `/tmp/baylee-artifact-packaging-tests-20261114.log` and `-r2.log`:
  seven targeted scenarios covered explicit/default inventory, identical build
  arguments, a failed profile, forced drain, and each missing staged variant.
  One assertion initially inspected a fixture after its cleanup; the existence
  check moved to the live fixture, and the two affected tests passed.
- `/tmp/baylee-workload-loops-tests-20261114.log` and
  `/tmp/baylee-workload-off-tests-20261114.log`: the workload constructs and
  records the actual selected session; Off plus simulation also compiles.
- `/tmp/baylee-artifact-packaging-clippy-20261114.log`: strict all-target
  Clippy passed for PGO, perf, tools, ops and agent.

The operations tests use isolated command stand-ins. No server was deployed and
no full three-variant PGO training or new performance benchmark has run yet.
The native Tower pack and its representative training scenarios are still pending.

## Owned hosted-worker containment — eighth working milestone

`baylee-worker` separates startup/operation deadlines and resident-memory
limits from game rules. The agent consumes a private progress pipe and samples
only the owned process; it kills through its Child handle and reports Exited
only after reaping. Idle human thinking is unlimited by the work watchdog.
Duplicate Busy markers cannot renew a deadline, and finite productive operations
can continue. The engine brackets outer driver calls and replay projection;
no progress or clock checks were inserted into the deterministic kernel.

An independent thread watches the empty ownership pipe inside an attached
engine. It exits the process if its owner vanishes even while the rules thread
is stuck. The agent keeps the pipe separate from Child::wait, which would
otherwise close stdin immediately. Logs use stderr; metadata-only probing
remains separate. Agent shutdown handlers are now installed before discovery,
and cancellation kills/reaps a metadata probe before returning.

Evidence: `/tmp/baylee-worker-owned-tests-20261114.log` (six tests: pure bounds,
real stalled/oversized/invalid/idle workers and existing probe limits);
`/tmp/baylee-agent-probe-shutdown-tests-20261114.log` (two tests including
SIGTERM during the actual agent's metadata probe, on macOS);
`/tmp/baylee-engine-private-progress-tests-20261114-r2.log` (the actual
detector-free server constructs a game, emits private progress with info logging
enabled, and exits when its ownership pipe closes while its gateway socket stays
open); `/tmp/baylee-worker-host-clippy-20261114-r3.log` (strict all-target
Clippy for worker, agent and engine server). The initial real-server test exposed
a fixture missing its required deck header; it was corrected. Test children
were reaped and the synthetic listener was checked closed.

The native client's process boundary follows in milestones nine and ten.
Resident-memory sampling is not a
hard OS allocation limit; whole-host memory isolation/admission and measurements
under realistic finite combos remain release work. No performance gain or
complete Tower security certification is claimed by these checks.

## Owned desktop-worker containment — ninth working milestone

Desktop `LocalHost::install` and resume now use a private headless helper of the
installed client. The renderer never waits for rules work, reconstruction or
process exit. The helper instantiates the requested concrete loop policy once;
the full compatibility and view contract is checked before publishing state.
An explicit Tower record is refused by this ordinary local adapter. Hosted
Tower continues to use the gateway-selected artifact, not this helper.

The owner enforces startup, unresponsive-operation and sampled resident-memory
budgets and always reaps its Child after cancellation or failure. The child's
independent input thread observes EOF even while rules work cannot return.
Frames, queues and accumulated records are bounded; record deltas stream once,
and restart hashes are checked before opening a kept file for writing. Human
thinking and renderer backpressure are not charged as engine work. No detector,
resource meter, clock or IPC was inserted into the rules hot path.

Technical closures have a separate localized explanation, including an exit
before the first snapshot. The lobby cannot turn that closure into a draw,
deck loss or recent-game result. Administrative closure is still identified
separately. Existing mobile/browser transports remain experimental; local
replay projection receives its boundary in milestone ten. This process boundary
is not an OS sandbox or a hard memory reservation.

Targeted evidence:
- `/tmp/baylee-worker-frames-tests-20261114.log`: bounded framing refuses empty,
  oversized and truncated frames before reading their body; round-trip passed.
- `/tmp/baylee-local-process-tests-20261114-r4.log`: two actual-client subprocess
  scenarios passed, covering all three ordinary policies, restart/hash and
  incremental-record continuity, wrong-hash and wrong-pack refusal, queue
  overflow and cleanup.
- `/tmp/baylee-local-interruption-tests-20261114.log`: two renderer/lobby tests
  passed, including an interruption before the roster and no deck/result entry.
- `/tmp/baylee-local-supervisor-tests-20261114.log`: six synthetic process cases
  in one test passed: startup stall, busy stall, invalid framing, wrong version,
  memory excess, and idle thinking followed by cancellation. Every child reaped.
- `/tmp/baylee-local-interruption-i18n-20261114.log`: all phrases still have the
  same arguments and complete coverage in every shipped language.
- `/tmp/baylee-local-process-clippy-20261114-r3.log`: strict all-target Clippy
  passed for worker, client and client-core. The client passed again in
  `/tmp/baylee-local-process-clippy-20261114-r5.log` after the record-variant guard
  and non-blocking terminal-error delivery.

The UI and supervisor tests reused one compiled unit-test executable to avoid
another long native link. This is a containment milestone, not the five Tower
presentation iterations or a claim of calibrated Tower balance. No services,
real credentials or player files were used by these checks.

## Owned replay projection — tenth working milestone

Desktop local replay projection now uses the same private client subprocess and
supervisor as local games. The owner validates the recorded ordinary loop policy
and requested seat, then streams bounded tape frames from the child. Cancellation
terminates and reaps even a stuck rules operation before the projection job
returns; a cancelled or failed job cannot publish a partial tape. The helper
opens no settings store and writes no kept files. The deterministic projector
remains the shared implementation for the helper, explicit tests and experimental
mobile hosts; no clocks or IPC entered the rules kernel.

The worker waits after its final frame until the owner consumes it, preventing
process exit from racing the buffered result. Progress is monotonic, frames and
accumulated tape bytes are bounded, and the existing native operation/memory
budgets apply between progress frames. The parent validates the finished tape
through the same parser as hosted projections.

Targeted evidence:
- `/tmp/baylee-replay-process-integration-20261114.log`: three actual-client
  subprocess tests passed. The new case compares exact tapes for all three loop
  policies and all three supported perspectives (nine projections), including
  a completed game; the other two preserve local game/restart coverage.
- `/tmp/baylee-replay-process-supervisor-20261114.log`: both supervisor tests
  passed, including cancellation of a nonresponsive projection and proof that
  its owned process was reaped, plus the six existing resource/contract cases.
- `/tmp/baylee-replay-process-presentation-20261114.log`: all 21 existing replay
  presentation/transport tests passed using the just-built test executable.
- `/tmp/baylee-replay-process-clippy-final-20261114-r2.log`: strict all-target
  Clippy passed for client and client-core after moving a test-only import.
  Workspace formatting and the final diff whitespace check also passed.

These checks cover replay containment and equivalence, not full Tower balance or
the requested Tower presentation iterations. Experimental mobile execution still
uses the synchronous projection seam.

## Canonical passive sources — eleventh working milestone

Command-zone trigger collection and static-effect registration now read the
canonical object ability definition. Previously both silently skipped an emblem
whose abilities came from a native token definition instead of `own_abilities`.
Trigger collection captures the resolved list once and reuses it for context and
stack resolution; stored abilities retain precedence. The existing Magic event,
priority, stack and layer machinery still runs these effects.

`/tmp/baylee-native-emblems-tests-20261114.log`: five focused engine tests passed,
including native upkeep/static emblems under all three loop policies, real stack
resolution, controller-specific upkeep, no duplicate static registration and
stored-list precedence. Existing setup, eliminated-player and Tower potion
emblem cases passed in the same filtered run. This repairs a shared prerequisite;
it does not yet admit new paid Tower relics or expose an administrative API.
Strict all-target engine Clippy passed in
`/tmp/baylee-native-emblems-clippy-20261114.log`.
