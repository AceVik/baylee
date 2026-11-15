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
| Equipment, upgrades, cards, consumables and gold reset for every new run; resuming preserves them | Starter choice and initial budget are restored; no permanent power bonuses |
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

## Compiled native setup adapter — twelfth working milestone

Core now defines a bounded, strictly decoded rule-pack manifest containing only
pack/revision, seats and immutable definition IDs. Ordinary presets omit it.
`NativeLookup<Registry, Pack>` admits exactly its compiled pack, checks every
definition before deck construction, and materializes ordinary native sources
once. Plain registries reject these manifests. The marker has zero storage and
adds no per-event callbacks; no service may submit effects, scripts or mutations.
An exact setup manifest participates in the continuation hash, including entries
that only affect opening resources. Starting sources do not invent entry events.

Evidence: the three core manifest/legacy tests passed in
`/tmp/baylee-native-pack-tests-20261114-r2.log`. Five focused engine pack tests
passed in `/tmp/baylee-native-pack-tests-20261114-r3.log`: wrong/missing pack and
revision, unknown definition, unsupported presentation identity before any card
lookup, policy refusal, actual native upkeep/stack execution and paired replay
hashes, setup-only identity hashing, zero-size adapter and source removal with
its pending trigger retained. The first removal fixture reached an unrelated
cleanup discard; its explicit empty hands now isolate the intended two-upkeep
scenario. A missing exhaustive setup-error arm in the fork path was also fixed.

This is the engine adapter, not yet the hosted Tower pack/catalog or its AI.
No release-artifact throughput or binary-size improvement is claimed.
Strict all-target core/engine Clippy passed in
`/tmp/baylee-native-pack-clippy-20261114-r2.log`.

## Static registry across host boundaries — thirteenth working milestone

`Session<D, Registry>` now preserves its compiled registry through live play,
recording, resumption, exact replay, perspective projection and fair AI forks.
Existing callers retain the ordinary registry by default. Native setup cannot
fall back to that registry; public ordinary table setup cannot select a pack.
No runtime pack selector or virtual dispatch was added to these paths.

The two focused host tests passed in
`/tmp/baylee-host-native-registry-tests-20261114-r2.log`: paired continued inputs
produce identical hashes after resume, exact replay agrees, projection retains
the pack, simulation leaves the live game untouched, and an ordinary projector
refuses before emitting a moment. Strict all-target host Clippy passed in
`/tmp/baylee-host-native-registry-clippy-20261114.log`. The first test build ran
out of disk space; removing this worktree's regenerable incremental cache made
the retry possible. This is not yet the Tower catalog or network authority.
The three existing static-policy host tests also passed in
`/tmp/baylee-host-native-registry-standard-tests-20261114.log`, including the
128-step paired view/Expert-answer comparison, using the already-built binary.

## Compiled registry at the server edge — fourteenth working milestone

`EngineRunner<D, Registry>` and compressed replay projection now preserve the
same static registry as their sessions. The executable has one registry alias
for both paths; its current default remains ordinary until the real Tower pack
is wired in. Game construction/readiness moved into `startup.rs` in a separate
move-only commit before this change.

Two focused tests passed in
`/tmp/baylee-server-native-registry-tests-20261114.log`: unknown native IDs and
ordinary-host fallback are refused without seat frames, and a native game sends
its compressed record before the flush acknowledgement. That actual record
projects with the matching registry and produces only header/refusal with the
ordinary registry. Strict all-target server Clippy passed in
`/tmp/baylee-server-native-registry-clippy-20261114-r2.log` after correcting two
test-fixture field/path errors and replacing the extracted wildcard import.
The same built executable also passed the two variant admission, fourteen
readiness-barrier and eleven existing projection tests, without another build:
`/tmp/baylee-server-native-registry-{variants,curtain,projection}-20261114.log`.

## Owned equipment and exact fusion — fifteenth working milestone

The separate `baylee-tower::v2` domain now has monotonic run-local instance IDs,
three active/six reserve slots, two separate Mythic materials, inherited paid
basis and atomic buy/move/swap/fuse/sell transitions. A full inventory admits
buy-and-fuse only when the final state fits. The recipes are exactly three
Common, three Uncommon, or one Rare plus one Mythic material. A failed operation
cannot spend currency, consume an input or change the instance sequence.
Sales are bounded both by half the actual basis and by a catalog cap. The
complete active loadout must fit opening-card, hand-limit and mulligan bounds;
no purchased bonus is silently clamped. Legacy revision-one types are unchanged.

Seven focused tests passed in `/tmp/baylee-tower-v2-inventory-tests-20261114.log`:
all four families' full nine-Common-to-Mythic paths, the direct Rare path,
the recipe/rarity/material matrix, full-bag fusion, duplicate and spent IDs,
wrong families, occupied outputs, swaps/caps, free-grant resale, forged basis,
wallet/ID overflow and stable definition encoding. Strict all-target Clippy
passed in `/tmp/baylee-tower-v2-inventory-clippy-20261114-r2.log` after replacing
one wildcard import.

These are pure transitions, not shop authorization, durable deduplication,
calibrated balance or a finished Tower release. The first four-family catalog
is a vertical-slice hypothesis; its native Grove definitions and service/client
integration follow before any offer can be published.

## Native Tower pack and derived resources — sixteenth working milestone

The `tower` artifact now binds its live and replay registry to the actual native
revision-two pack. Plain registries reject Tower hosting identities; native
registries reject ordinary hosting identities. Tower refuses the unauthenticated
listening harness and requires its existing authenticated gateway attachment.
The new internal `baylee-tower` dependency adds no external dependency version.

The authority-side resource derivation is shared with pack admission. Admission
requires exactly two canonical seats, the frozen Expert policy, no privileged
capabilities/fixed hands/outside-game cards, and exactly the life/opening/house
rules justified by active definitions. This validates rules consistency, not
ownership or the honesty of an operator. The service must still prove acquisition
and bind the reservation and record.

All sixteen initial family/rarity definitions are now executable. Heartwood,
Satchel and Compass derive bounded opening resources. Grove is a removable
starting artifact whose own-upkeep ability creates one/two/three/four original
Plants through the ordinary stack. Setup invents no entry or upkeep event.
Four original token identities were appended by `cargo xtask codegen --tables`;
existing IDs were preserved. Their native text faces and original sentences are
covered by the documented asset fallback, with no external image identity.

Targeted evidence:
- `/tmp/baylee-tower-v2-loadout-tests-20261114-r2.log`: exact setup and nineteen
  forged configuration variants. The first fixture accidentally gave an AI an
  empty deck and was corrected to a structurally valid synthetic deck.
- `/tmp/baylee-native-tower-pack-tests-20261114-r2.log`: all sixteen definitions,
  each Grove rarity's real upkeep/stack/token ownership, resource-forgery refusal,
  wrong executor rejection and 32-step fixed-Expert resume/exact replay. The first
  acceptance fixture included tournament sideboards; the fixture now explicitly
  removes them instead of weakening Tower admission.
- `/tmp/baylee-native-tower-artifact-tests-20261114.log`: those three cases also
  passed in the separately compiled `--no-default-features --features tower`
  artifact, including a compile-time assertion of its actual session alias.
- `/tmp/baylee-native-tower-artifact-identity-20261114.log`: compiled identity and
  detector policy agree.
- `/tmp/baylee-native-tower-token-tests-20261114.log`: eight existing token-ledger,
  identity and art-fallback tests passed.
- Strict all-target server Clippy passed for StandardLoops, Standard Off and
  Tower in the `native-tower-pack-clippy-...-r4`, `native-tower-standard-off-clippy`
  and `native-tower-artifact-clippy` logs under `/tmp`, dated `20261114`.

The first catalog is still a balance hypothesis. AI synergy evaluation, the
remaining families/combo, global authority, shops, persistent awards and the
five actual Tower UI refinement passes remain work; these tests do not stand in
for those acceptance criteria or a release-performance measurement.
The second `codegen --tables --check` was up to date
(`/tmp/baylee-tower-native-tables-check-20261114.log`); workspace formatting and
diff whitespace checks also passed. No full corpus regeneration was run.

## Separate table charges — seventeenth working milestone

Revision two freezes healing at three gold and mana at five, with two combined
table charges. Purchase and verified settlement are atomic economy operations;
settlement consumes only the verified quantities and grants twelve gold. Mythic
materials are not table charges. A technical interruption must not settle, and
the future run/service transition must bind the terminal reservation exactly
once; the pure economy operation is not that authorization boundary.
The focused charge/cap/over-consumption test and strict all-target Tower Clippy
passed in `/tmp/baylee-tower-v2-charges-{tests,clippy}-20261114.log`.

## Persisted shop domain — eighteenth working milestone

Revision two now models eight quantity-one offers with distinct monotonic
run-local IDs. Two pins occupy existing slots across refreshes and visits;
purchases clear their pin and keep the sold offer. Three refreshes cost 2/3/4
gold. Cards, relics, table charges and crafting materials share one atomic
purchase result, including buy-and-fuse. Reconnect serialization preserves the
visit. Uncommon/Rare/material eligibility begins at stages 2/4/5; stage seven
has one explicit full-price material replacement, including acknowledged
replacement of an inherited utility pin.

Generation is bounded and independent of catalog iteration order. New visits
offer an affordable legal choice when one exists, considering the entire owned
pool, inactive equipment, charges and legal full-bag fusion. A self-review found
that testing only the first consumed fusion slot could miss a legal reserve
output when equipping the result exceeded the mulligan cap. Both freed slots are
now considered; the regression specifically requires that reserve output.
No candidate is invented for a zero-gold/full-capacity state.

Five focused tests passed in `/tmp/baylee-tower-v2-shop-tests-20261114-r2.log`,
covering sixteen seeds across all twelve visits, catalog permutations and saved
state, pin/price/stock identity, stale purchases, the one-time material choice,
32 full-bag acquisition seeds, the reserve-only fusion case and zero gold.
Strict all-target Tower Clippy passed in
`/tmp/baylee-tower-v2-shop-clippy-20261114-r3.log`.
Phase/revision gating, account/cohort seed allocation, durable idempotency and
the service/client are still required before these shops can be offered live.

Follow-up integration finding: the existing PGO artifact selection still gives
Tower the ordinary detector-free `engine_perf` session. With the actual native
registry now connected, training/proof must also carry that registry and a
strictly admitted two-seat Tower preset. `docs/releasing.md` now explicitly marks
Tower release acceptance open; no native release performance/proof is claimed.
Resolve this before staging, along with the authority/record durability work.

## Native profiling workload — nineteenth working milestone

The follow-up above is corrected: the Tower performance artifact links the real
compiled native pack without server default features and uses that registry for
sessions, forks and projection. Its two-seat fixtures include native Grove
rarities and charges, a human seat and the frozen Expert policy. Tower trains
and proves `frames`; other modes/four seats fail before execution. The PGO plan
refuses a mismatch between Tower server and workload features before touching
build directories. Standard variants retain their existing shapes.

Two native workload tests and two artifact/PGO-selection tests passed in
`/tmp/baylee-native-tower-{perf,pgo}-tests-20261114.log`; the ordinary workload
test passed in `...perf-standard-tests-20261114.log`. Strict Clippy passed for
the native workload and PGO crate. Ordinary Clippy required a narrowly documented
allowance for the shared fallible preparation signature, whose native admission
can fail; the ordinary implementation has no failure path.
The actual release PGO builds, count coverage and semantic comparison are still
release acceptance work, not demonstrated by these focused wiring tests.

## Fenced record-head transitions — twentieth working milestone

Pure execution metadata now freezes the package and a bounded technical record
budget. Renewable worker generations fence expired/revoked writers; every append
requires the exact previous head. The ordered SHA-256 content commitment binds
package, predecessor, chunk index, length and bytes. Sealing freezes that head
for verification. Recovery retains the package and full prefix; it cannot reset
either, reopen a sealed record or manufacture a rules result.

Three focused tests cover lease boundaries, stale writers, rewind/duplicate
appends, exact bounds, sealing/recovery, serialization and commitment separation.
They and strict all-target Tower Clippy passed in
`/tmp/baylee-tower-v2-execution-{tests,clippy}-20261114.log`.
This is a domain contract, not durable ACK or authentication: the service must
atomically persist chunk plus head, bind operator credentials and release visible
consequences only after that commit. A 256 KiB chunk ceiling and configurable
record budget up to 512 MiB are technical storage bounds, not calibrated rule
limits. The lease ceiling is two minutes with renewal, not a player clock.

## Saved run state machine — twenty-first working milestone

Revision-two owner commands now bind optimistic revision, saved offers, owned
instances and exact phase. A reserved/playing/verifying/quarantined duel cannot
shop or abandon into a fresh opponent. Recovery preserves the execution package
and prefix; sealed recovery returns to verification. Operator retirement records
an explicit technical interruption with no outcome or income.

Settlements bind the current sealed package/generation/prefix, consume verified
charges and grant twelve gold once. History derives wins/defeats, ends at eight
wins/five unsuccessful duels, and preserves the final visit without opening a
thirteenth shop. Zero-turn concessions and long games are allowed; there is no
legacy 500-turn rule. A financial ledger checks exact income/debits/sales and
surviving paid basis. All successful shop commands close the stage-seven choice;
failed commands leave it intact. Pins survive advancement.

Five focused domain tests and strict all-target Clippy passed in
`/tmp/baylee-tower-v2-run-tests-20261114.log` and
`/tmp/baylee-tower-v2-run-clippy-20261114-r2.log`. Cases include both twelve-duel
terminal boundaries, wrong/duplicate results, stale purchases, a bought/fused/
moved/sold chain, recovery and stale verification, forged finances and revision
exhaustion. Records in these state-machine tests are deliberately synthetic;
the tests do not stand in for replay adjudication or transactional service tests.

## Exact native private packages — twenty-second working milestone

The revision-two package now freezes canonical owned decks, active definitions
in slot order and unspent charges, excluding reserves and wallets. Reservation
requires the same duel ordinal and full cohort for the selected echo. The cohort
binds catalog/prices, economy/pack revision, shop generator and every executable
identity field. The exact native preset is derived without incoming overrides,
fixed hands, capabilities or scripts. SHA-256 content commitments bind the
private package to the run and its record chain; they are not credentials.

Attribution stores account/snapshot references, never a copied display name;
the public projection must resolve current visibility. House fallback is an
explicit distinct origin. Private build/package/opponent values omit `Debug`.
Four focused package tests and strict all-target Clippy passed in
`/tmp/baylee-tower-v2-package-tests-20261114.log` and
`/tmp/baylee-tower-v2-package-clippy-20261114-r2.log`. They cover actual owned
active/reserve separation, asymmetric resources, full identity skew, same-stage
reservation, bounded strict decoding and content commitment changes. Admission
still relies on the authority for historical eligibility and acquisition proof.

## Global indexed matching policy — twenty-third working milestone

The new sampler addresses the full dense eligible population, excludes self,
prefers unseen accounts and falls back to repeated non-self accounts only when
necessary. It maps a uniformly sampled complement rank across at most thirteen
exclusions, then requires one indexed lookup under the same partition generation.
There is no first-512 truncation. A bounded rejection step removes modulo bias;
the seed must come independently from the authority. Constant-work swap deletion
preserves one slot/weight per remaining account.

Four focused tests and strict all-target Clippy passed in
`/tmp/baylee-tower-v2-matching-{tests,clippy}-20261114.log`: exhaustive small-set
rank bijections, self/unseen/empty behavior, million-entry index coverage,
deterministic selection, signed-bigint bounds and dense withdrawals. This is the
selection/index contract; persistence must still enforce uniqueness, eligibility,
partition locking and revisions transactionally. It is not a proof of operator
honesty or resistance to people creating multiple accounts.

## Transactional owner persistence — twenty-fourth working milestone

Added the separate `baylee-tower-store` adapter with explicit, digest-checked
PostgreSQL 18+ migration, immutable cohort publication, explicit versioned
enrollment consent, saved initial seeds and owner-bound reads/commands. Database
IDs use `DEFAULT uuidv7()`/`RETURNING`. Account locking serializes request receipts
with run state; exact retries return the original committed response, while a
different request under the same key refuses. A unique active-run index backs
the domain check. Early abandonment cannot sample another starting offer sequence.
Reordered catalog/deck lists normalize to the same publication; changed starter
content cannot be republished under the same exact cohort contract.

Three real PostgreSQL scenarios passed in
`/tmp/baylee-tower-store-postgres-20261114-r3.log`: eight concurrent identical
purchases charge once, eight distinct same-revision requests accept one, wrong
owners and reused keys refuse, and an injected receipt constraint failure rolls
back wallet, inventory, stock and revision. Migration repeats but rejects changed
history. The fixture originally rejected PostgreSQL's non-special URL host form
before any connection; literal loopback IP parsing was corrected without allowing
remote hosts or query overrides. All owned schemas/pools were removed/closed even
when the test body fails. No test service or GUI was started.

Strict all-target Clippy passed in
`/tmp/baylee-tower-store-clippy-20261114-r2.log`. The new crate forbids first-party
unsafe. The lock diff adds only the workspace crate and edges to existing pinned
packages; `url` 2.5.8 is reused for fixture isolation. No external dependency
version changed. This is not service security/release completion: authentication,
operator grants, record durability, global pool transactions, verified outbox,
account-wide quotas, retention and runtime privilege checks remain to integrate.

## Persistence correction pass — twenty-fifth working milestone

Migration two records consent version/time and backfills exact receipt-byte/run
counts for existing rows. Account limits (64 MiB retained responses, 1,024 retained
runs) complement per-run count/body limits. Quota charges, run creation and
receipts commit or roll back together. An idempotent response remains readable at
the limit and explicitly releases its transaction before returning. Migration
validation now accepts only a known contiguous prefix and applies missing steps;
runtime readiness requires the exact full history.

The new upgrade/quota case constructs the prior schema around valid saved data,
checks backfilled counters/consent and unchanged runs, then exercises full quotas,
unchanged failed purchases and retrieval of a committed response at the limit.
It and the three affected PostgreSQL scenarios passed in
`/tmp/baylee-tower-store-bounds-tests-20261114.log`; strict Clippy passed in
`/tmp/baylee-tower-store-bounds-clippy-20261114.log`. All fixture schemas/pools were
closed. Retention/deletion must decrement counters transactionally; its workflow,
record-storage quotas and live transport are still required before deployment.

## Dedicated operator identities — twenty-sixth working milestone

Migration three separates approved hosts, verifiers, publishers and technical
supervisors. Dedicated token digests resolve to a private-field principal, and
operations recheck enabled state/role; gateway registration does not supply any
role. Only hosts can advertise bounded exact executable identities. Only a live
publisher can publish a house fallback, which is immutable per cohort/stage/
archetype and explicitly distinct from a historical echo. Structural build
validation still does not certify difficulty or acquisition fairness.

The new PostgreSQL scenario covers unknown credentials, all wrong-role paths,
exact identity mismatch, duplicate inventories, immutable content conflicts,
concurrent inventory advertisements and a principal retained after revocation.
It and the four affected migration/owner/quota cases passed in
`/tmp/baylee-tower-store-operators-tests-20261114.log`; strict Clippy passed after
a move-only test-helper extraction in
`/tmp/baylee-tower-store-operators-clippy-20261114-r2.log`. Fixture cleanup completed.
No HTTP credential transport or Central token-scope change is implemented by
this milestone; operator checks must still bind actual claims, record append,
independent verification and recovery. Offline cohort/operator provisioning must
not be exposed as an unguarded service endpoint.

## Durable reservation and execution — twenty-seventh working milestone

Migration four freezes one package/assigned host per run ordinal, retains bounded
worker-claim history and stores ordered raw record chunks. Owner reservation and
receipt commit together; an exact retry retains the first entropy/package. Claims
require the approved host's exact advertised artifact. Every append commits bytes,
account quota and the run's content commitment together with synchronous commit
enabled before returning an acknowledgement. Repeated live-worker appends return
the original head, while conflicting content, expired/revoked claims and foreign
hosts refuse. Read/retry paths recompute each stored chunk's commitment.

Storage has independent byte and row bounds: 512 MiB / 65,536 chunks maximum per
reservation, two GiB / 262,144 retained chunks per account and 64 execution claims
per reservation. These are technical ceilings, never game outcomes. Supervisor
quarantine revokes the generation without changing seeds/opponent/economy. Recovery
on another compatible host retains every acknowledged byte. A sealed record can
only return to verification; explicit retirement creates no win/loss/income.

The five existing database cases and the initial four execution cases passed in
`/tmp/baylee-tower-store-execution-tests-20261114.log`. After adding commitment
read checks, the five execution scenarios passed in
`/tmp/baylee-tower-store-execution-tests-20261114-r2.log`: competing reservations
and appends, lost replies, injected SQL failure with complete quota/head rollback,
foreign roles, expiry/reassignment, sealed recovery, corruption and quota limits.
All isolated schemas/pools were cleaned. Four domain execution cases passed in
`/tmp/baylee-tower-record-limits-tests-20261114.log`; strict all-target Clippy passed
in `/tmp/baylee-tower-store-execution-clippy-20261114-r3.log`.

This milestone persists bootstrap house reservations; global eligible-pool
transactions, independent adjudication/outbox, retention accounting and the live
gateway's reveal-after-ACK barrier are still integration work. A storage ACK test
does not prove that the live engine transport already enforces that barrier.
Operator PostgreSQL configuration must retain fsync/WAL durability; synchronous
commit does not by itself certify failover replication or storage hardware.

## Independent adjudication and global echoes — twenty-eighth working milestone

Migrations five/six add an independently fenced verifier queue, exact settlement
receipts, an atomic accomplishment outbox and immutable pre-duel snapshots. A host
cannot claim/read/settle verifier work. Expiry/reassignment and technical quarantine
invalidate the former verifier generation. Settlement binds the saved package,
execution fence and sealed byte prefix; consumed charges, income, next shop,
snapshot/index, future starting seed and outbox commit together. Exact authorized
retries return the original response, even after subsequent owner commands.

The global index enforces one account weight per cohort/ordinal, uses the full
dense population sampler, excludes self and prefers unseen accounts. Only a
verified terminal completion admits its frozen pre-duel build; victory is not
required. The first still-eligible representative stays fixed regardless of later
outcomes. Empty eligible pools use explicitly labelled immutable house builds.
Withdrawal advances a consent generation so reenrollment cannot resurrect old
snapshots. Matching rechecks expiry/consent before reservation. Cleanup scans a
bounded number of indexed positions using a saved cursor, with constant-work
swap deletion; exhausted cleanup work refuses instead of pretending the pool is
empty. Local attribution checks additionally require current Central visibility
at the eventual service projection boundary.

Fourteen PostgreSQL cases passed in
`/tmp/baylee-tower-store-adjudication-tests-20261114-r2.log`. The first run exposed
an overly strict nullable-UUID constraint on an unclaimed verification job; the
unpublished migration was corrected and the affected suite passed. The added
same-stage unseen/repeat/cohort-isolation scenario passed separately in
`/tmp/baylee-tower-store-indexed-matching-tests-20261114.log`. Strict all-target
Clippy passed after test-helper extraction in
`/tmp/baylee-tower-store-adjudication-clippy-20261114-r5.log`. Fixture schemas and
pools were cleaned, including on the failing run; no service or client was started.

These are trusted-role persistence fixtures using explicitly synthetic bytes.
They do not certify legal MTG replays. The real native Tower verifier, service
authentication/transport, live reveal barrier, bounded retention with updated
receipt/record counters, Central delivery and native client remain to integrate.
An outbox row is not a delivered Central badge. No new external dependency was
introduced and no migration was applied to a production schema.

## Native replay adjudication — twenty-ninth working milestone

After a move-only extraction of the existing Tower verifier, legacy and native
adjudication share the same replay/charge-accounting logic. Native verification
uses `NoLoopDetection` and the compiled Tower registry, requires the exact current
artifact in both package and record header, and recomputes every frozen Expert
answer. Its technical limits have no 500-turn adjudication rule; the old
expedition keeps its existing acceptance contract. Input is read through one
bounded reusable JSON-line buffer. A Tower-only engine-server adapter also
reconstructs the exact acknowledged chunk commitment while streaming, refuses
trailing/truncated/changed framing, derives the canonical preset and returns
engine-derived outcome/turn/consumption bound to the saved reservation.

The three legacy Tower cases plus the bounded-line read case passed in
`/tmp/baylee-native-verifier-legacy-tests-20261114.log`. Three actual native-engine
cases passed in `/tmp/baylee-native-verifier-tests-20261114-r2.log`: Grove-equipped
games with/without real potion payments, arbitrary chunk boundaries, exact
commitments, forged Expert actions/results/hashes, missing artifact metadata,
truncation/trailing data and refusal before reading incompatible/over-budget work.
Strict all-target Clippy for gamehost and the Tower engine-server variant passed
in `/tmp/baylee-native-verifier-clippy-20261114-r2.log` after an internal field
rename. These short concession fixtures verify adjudication, not game balance or
long-run resource sizing. The native verifier still needs its supervised process
entry point and the service's lease/stream/settlement orchestration.

## Private native worker — thirtieth working milestone

The Tower executable now accepts a strict standalone `--tower-verify` command.
It creates no network listener or async runtime and reads no credentials. The
private input is a bounded request frame followed by retained chunks using the
existing big-endian frame codec; stdout returns one bounded result and stderr
contains only progress markers. An independent liveness writer terminates even a
blocked worker when its owner disappears. Liveness cannot renew work deadlines.
The ordinary executable explicitly refuses this command; its startup path still
creates the same runtime when actually hosting a game.

The actual compiled-process tests passed after fixing the test producer to close
stdin explicitly: verified record, altered stream and owner loss while input is
blocked (`/tmp/baylee-tower-verifier-worker-process-tests-20261114-r2.log`). Six
worker tests, four native verifier cases, strict CLI admission and changed-crate
all-target Clippy passed. Private input rejects extra executable/path fields and
over-budget requests. This is the child boundary; authority-side supervision,
durable input streaming and applying the independently verified result follow.

## Authority-to-worker integration — thirty-first working milestone

The separate `baylee-tower-service` library now probes operator-installed Tower
executables, bounds simultaneous workers, streams one retained database chunk at
a time, validates one complete private progress/result exchange, enforces time
and resident-memory limits, and reaps each owned child before releasing capacity.
Dropping a requesting future signals a separately owned cleanup task. Worker
environment is empty; database and operator credentials remain in the authority.
Only an exact bound native result reaches the transactional settlement API.
An exact committed receipt is recoverable after response loss without replaying
or awarding again. There are no new external versions in the dependency graph.

Three real PostgreSQL/native-process scenarios passed in
`/tmp/baylee-tower-authority-native-process-tests-20261114-r3.log`: persisted legal
play and exactly-once gold/progression, independently refused altered play and
budget exhaustion without a result, and explicit shutdown/dropped requests while
a retained-record read is blocked, followed by successful retry. A strict replay
rejection may close its input before all chunks arrive; the supervisor now retains
that rejection rather than misclassifying the broken input pipe as worker failure.
Three small private-pipe adversarial tests also pass. These tests use isolated
loopback schemas and owned child processes, which are removed/reaped afterwards.
Automatic queue handling, gateway transport/reveal barriers, Central identity and
award delivery, retention and the native Tower UI remain subsequent integration.

## Automatic verification queue — thirty-second working milestone

The service now consumes the durable ready queue automatically, taking a fresh
verifier generation and streaming the original retained evidence into the exact
local native artifact. An idle worker polls once per second. Shared admission
prevents a busy worker from claiming extra jobs. Shutdown waits for owned process
cleanup. Configuration/storage faults stop visibly; cancelled work receives no
result and remains recoverable after its lease expires.

Native replay refusal or resource/process failure uses a separate supervisor
credential to quarantine only that exact job generation. Migration seven retains
one closed reason, never raw logs or private record data. Quarantine/recovery
preserves the committed package/record and creates no game result or award. A
late failure cannot damage a replacement generation. A mismatched local artifact
is an operator configuration error, not evidence against the player's game.

Two real database/process queue tests passed in
`/tmp/baylee-tower-automatic-queue-tests-20261114.log`: automatic successful
settlement/graceful stop and rejected evidence with persisted failure, zero awards,
supervisor-only recovery and rejection of stale quarantine. The authority remains
a reusable service library at this milestone; authenticated HTTP deployment,
live gateway ACK/reveal integration and native UI are not claimed complete.

## Native record/reveal boundary — thirty-third working milestone

After a move-only extraction of the large table driver, record output is now a
compile-time transport parameter. Ordinary games retain their zero-state gzip
transport. The native Tower table captures raw canonical record pieces directly,
without compressing and decompressing them, and withholds all resulting frames
until their exact ordered commitments have been acknowledged. It admits one
Expert step at a time, refuses additional game work while waiting, bounds raw
operation output to eight MiB and player output to sixteen MiB/4,096 frames, and
cannot report finished until terminal output crosses the same barrier. These are
technical allocations, not rules losses/draws. The authority must authenticate
ACKs; a content hash is not authentication. The public table adapter exposes no
mutable session/driver that could bypass the boundary.

Three actual native-table tests pass in
`/tmp/baylee-durable-table-tests-20261114-r3.log`: held initial/terminal output,
partial/wrong-generation/reordered/duplicate ACKs, technical exhaustion, and an
independent native replay of the committed record. All ten existing ordinary
recording tests pass in `/tmp/baylee-legacy-record-transport-tests-20261114.log`,
including gzip continuity, empty final members, timed/manual flushing and final
snapshot hashes. Four record-commitment cases also pass. Strict Tower all-target
Clippy and the separate standard executable check pass. The new native adapter
still needs its authenticated gateway wire path and retained-prefix reconstruction
for live recovery; the existing attached transport has not silently opted into it.

## Atomic operation correction — thirty-fourth working milestone

Reviewing recovery exposed a hole in per-chunk acknowledgement: a large JSON line
could span several chunks, leaving an acknowledged but unreconstructable partial
operation after a crash. The new native table now offers one complete batch and
accepts only its final commitment; a partial head never advances its acknowledged
position. The authority commits every chunk, the exact batch receipt, quota and
run state in one transaction (`append_batch`, migration eight), ending at a whole
record line. Limits remain eight MiB/64 chunks. New writes use one bulk INSERT and
one account/run update rather than repeating those writes per chunk.

An actual PostgreSQL scenario injected failure into a later row, checked rollback
of all prior bytes/counters/state, raced four identical submissions, refused changed
batch boundaries and recovered the original receipt after later progress. It
passed in `/tmp/baylee-tower-atomic-record-batch-tests-20261114-r2.log`. The schema
upgrade/quota scenario and all three native barrier/replay tests also pass. This
correction supersedes milestone 33's incremental ACK interface before any public
route uses it. Retained-prefix reconstruction and authenticated live gateway
transport are still outstanding.

## Exact native recovery — thirty-fifth working milestone

Native recovery now uses the same strict, streaming reconstruction as independent
adjudication. It checks the exact artifact, preset, retained chunk commitment,
line sequence, nondecreasing timestamps, engine hashes and every frozen Expert
answer before returning a table. Terminal inputs without their End metadata are
refused. Continuation restores the recorder position without repeating its header,
the Expert's RNG/memory, paid potion charges and setting/declaration allowances.
Restoring display names deliberately does not call the normal initial reseeding
path. Automatic undeclarations on reconnect do not consume declaration allowance.

The restored table begins at its acknowledged prefix under the new fence, rejects
old ACKs and requires a fresh presentation-ready handshake. Its clock cannot move
backwards. A fully committed terminal prefix is ready for sealing even if the
old host died before sending that request; no second terminal move is invented.

Seven scoped gamehost tests pass in
`/tmp/baylee-tower-reconstruction-host-tests-20261114.log`. Native table scenarios
cover corrupt/truncated/excess evidence and paid-potion/terminal recovery in
`/tmp/baylee-tower-native-recovery-tests-20261114-r2.log`; the continuity case was
tightened to explicitly exercise both human and Expert decision boundaries and
passes in `/tmp/baylee-tower-native-recovery-continuity-20261114.log`. It compares
future record bytes against uninterrupted play across four reconstructions.
Strict Clippy for all targets of both changed crates passes in
`/tmp/baylee-tower-native-recovery-clippy-20261114.log`. Authenticated transport,
process-level recovery orchestration, native UI and full playable acceptance
remain outstanding; this is not an end-to-end completion claim.

## Concession correctness — thirty-sixth working milestone

After a move-only extraction of socket answer application, frozen-policy games
now admit concession while the Expert still owes a decision (CR 104.3a). Other
game-moving actions retain the serialization barrier. Live admission and strict
policy replay share the same predicate, so an accepted concession cannot later
be rejected solely for its timing. Six affected challenge tests pass in
`/tmp/baylee-challenge-concession-tests-20261114.log`, including the existing
simultaneous-mulligan race refusal. An actual native opening concession independently
verifies in `/tmp/baylee-tower-early-concession-tests-20261114-r2.log`; both changed
crates' strict all-target Clippy passes in
`/tmp/baylee-challenge-concession-clippy-20261114.log`.

## Scoped player access — thirty-seventh working milestone

Central commit `913a3bed81b6187b30529d4114cd8334d15e30ce` adds the separate
five-minute identity capability, bound to the operator's sole Tower origin and
the original live native session. Real Central HTTP/PostgreSQL tests cover scope,
concurrent bounded refresh, terms, logout, revision changes and deletion; Central's
crypto-scope, dependency audit/deny, policy and secret checks pass. The branch is
published for reproducible SDK pins; this is not a main merge or final PR review.
Baylee's SDK and gateway-test pins advance together; the crypto source and manifests
are unchanged, and the exact-pin/all-feature consumer guard passes in
`/tmp/baylee-tower-identity-shipped-features-20261114-r2.log`.

The new `baylee-tower-api` exposes bounded owner choices and explicit projections;
the global service's `Players::router` checks live identity before every operation.
Pagination, store ownership, saved request receipts, revision locks and authority
randomness survive HTTP unchanged. The separate native account transport obtains
its destination from Central, refuses redirects, preserves request identities and
never sends the Tower credential to a gateway. Public projections omit private
execution/evidence fields even after reservation. This is low-frequency owner
traffic; no engine frame or kernel callback introspects Central.

A negative wire test found that Serde ignored extra fields on the old unit-shaped
Refresh/Abandon variants. Empty structure variants now reject them without changing
their JSON spelling or domain semantics. The targeted native player HTTP/PostgreSQL
scenario passes in `/tmp/baylee-tower-player-postgres-20261114.log` (one test, 0.30s):
repeated starts/purchases/reservations, foreign reads/writes, stale revisions,
malformed/oversized/browser requests, changed identity scope, logout/unavailability
and hidden-field projection. Owned listeners, pools and the PostgreSQL container
are closed even after scenario assertion failures. Central is an explicit synthetic
identity boundary in this Baylee test; the real identity implementation was tested
separately in Central, not silently replaced in a purported full-system test.

Two native transport tests, five affected run-transition regressions, the strict
wire test and bounded admission-counter regression pass. Strict all-target Clippy
for the five affected runtime crates passes in
`/tmp/baylee-tower-player-clippy-20261114-r5.log`. The standalone service launcher,
gateway execution transport, current opponent attribution, accomplishment delivery,
retention, item/AI playtesting and visible client integration remain outstanding.

## Private host transport — thirty-eighth working milestone

15 November 2026. A move-only commit extracted host DTOs from persistence into
the reusable API crate; the store preserves its public reexports. Host claims now
also bind the human Central account, so gateway seating cannot rely on knowledge
of a run ID. The separate host routes authenticate a dedicated Host credential,
then recheck role/assignment/fence through existing transactional methods.
Neither a player nor another operator role can use them. Claim/renew/read/seal
and advertised exact artifacts are available; no caller can submit a win or gold.

Private record submission uses bounded binary frames rather than expanding bytes
into JSON. The frame preserves whole-operation chunk boundaries, and the store
ACKs only the atomic final head. Sixteen control slots and four record slots bound
concurrent admission; large bodies are read only after host authentication. Shared
native header/error handling preserves the player route's behavior.

The expanded HTTP/PostgreSQL scenario passes in
`/tmp/baylee-tower-host-http-postgres-20261114.log` (0.45s): player/host scope
separation, wrong operator roles and host assignments, exact claim/batch/seal
retries, private chunk reads, changed/truncated batches, stale fences and no reward
on sealing. Its bytes are deliberately synthetic transport evidence, not a claim
of legal native gameplay. Three bounded protocol tests pass in
`/tmp/baylee-tower-host-framing-tests-20261114.log`, including every truncated
prefix, unknown metadata, independent byte/chunk limits and a maximum-size frame.
The launcher and actual gateway/agent/Table wiring are still outstanding.

## Async host adapter — thirty-ninth working milestone

`baylee-account` now has an explicit `tower-host` feature separate from its native
password/session SDK. The async connection uses a fixed operator-configured origin,
verified TLS, no redirect/proxy selection and bounded private responses. It rejects
wrong operation/claim/index replies, wrong seal bindings and incomplete/wrong-fence
batch acknowledgements. It does not retry or invent replacement operation IDs;
the native table retains responsibility for the content commitment/barrier.

The real HTTP/PostgreSQL scenario now exercises this adapter's claim and append
paths as well and passes in `/tmp/baylee-tower-host-sdk-postgres-20261115.log`
(0.46s). Negative transport cases pass in
`/tmp/baylee-tower-host-sdk-tests-20261115.log`: partial counts, wrong generation,
wrong response kind/version, oversized bodies and redirect credential traps.
Owned listeners and the isolated database fixture are closed. Gateway lifecycle
orchestration has not yet enabled this adapter; it is a tested integration building
block rather than a claim that native Tower duels are playable through gateways.
Strict all-target Clippy with both account feature sets enabled passes in
`/tmp/baylee-tower-host-sdk-clippy-20261115-r3.log`; the preceding host/API/store
milestone also passed `/tmp/baylee-tower-host-http-clippy-20261115.log`.

## Standalone authority — fortieth working milestone

The Tower service now has a native executable with bounded private stdin
configuration, explicit check/migration modes and no implicit schema changes.
Its loopback listener serves the existing authenticated player/host APIs. Exact
published cohort artifacts and distinct verifier/supervisor roles are checked
before readiness; configured queues run automatically. SIGTERM/Ctrl-C drains
owned work before database closure. Configuration, resource bounds, retained
cohort obligations and TLS proxy requirements are documented in the environment
runbook. This does not yet connect gateway games or the visible client.

The focused configuration test caught a real URL-parser mismatch: PostgreSQL
URLs retain literal IPv4 hosts as domains. Parsing those hosts strictly as IP
addresses admits loopback without DNS or numeric aliases; IPv6 and hostile
configuration regressions pass in
`/tmp/baylee-tower-bootstrap-config-tests-20261115-r2.log`.
The real standalone-process/PostgreSQL test passes in
`/tmp/baylee-tower-bootstrap-process-tests-20261115-r2.log` (0.35s), checking
configuration-only startup, idempotent explicit migration, redacted rejection,
HTTP readiness, authentication enforcement, graceful SIGTERM and verified child
exit/port closure. It uses zero queue workers deliberately: native independent
adjudication is covered by the earlier queue tests, not claimed by this transport
scenario. Strict all-target Clippy for service/store passes in
`/tmp/baylee-tower-bootstrap-clippy-20261115-r3.log`.

## Bounded private engine plane — forty-first working milestone

An initial move-only commit separates ordinary attachment from shared transport.
Protocol 49 then adds the private Tower frame: bounded typed control plus one raw
chunk, without raising socket limits. The pure assembler retains whole-operation
semantics across those frames and checks complete content commitments before
allowing further writes or sealing. Claims carry the original immutable record
budget. Live endpoint loops are the next milestone, not claimed here.

API/protocol unit tests pass in `/tmp/baylee-tower-engine-wire-tests-20261115.log`
(5 API and 51 small protocol tests, 0.05s total). The new adversarial cases cover
multipart protobuf-to-HTTP round trips, exact complete ACKs, retained-prefix
restart, reordering, changed counts, wrong generations, interleaving, partial
ACKs, incomplete record lines, the independent eight-MiB operation bound, unknown
controls/fields and oversized metadata/data. The updated real HTTP claim scenario
passes in `/tmp/baylee-tower-engine-claim-tests-20261115.log` (0.46s). A stale
protocol-version regression still expected 47 despite the earlier move to 48;
it now documents/checks the actual 49 contract.
Strict all-target Clippy for the three changed runtime crates passes in
`/tmp/baylee-tower-engine-wire-clippy-20261115-r3.log`.

## Native Tower attachment — forty-second working milestone

The Tower artifact now has its own compiled websocket loop around the durable
native Table, with no ordinary preset/foresight fallback. Initial and continuing
record batches wait for exact durable ACKs; final views also wait for sealing.
Bounded human frames arriving meanwhile are queued as unaccepted input. Recovery
pulls one chunk at a time into strict reconstruction, without a full-record
allocation or a private temporary recording file. Changed bytes or wrong game,
direction, index, generation or acknowledgement stop the attachment without a
manufactured outcome. Ordinary attachment remains separately compiled.

Three focused actual-native websocket tests pass in
`/tmp/baylee-tower-native-attachment-tests-20261115-r2.log` (3.19s). One connects
three successive native instances: interruption after a committed initial batch,
recovery and concession, then recovery of the already-terminal record. The final
retained bytes independently verify as a defeat; terminal recovery appends no
new header/end. Others refuse wrong game/ACK generation and corrupted recovery
bytes before any native view or replacement evidence escapes. These tests use
in-memory real websocket peers and join every owned task; the peer simulates
already-tested authority commits, so this is not yet a live gateway/database
end-to-end claim. Tower strict binary/test Clippy passes in
`/tmp/baylee-tower-native-attachment-clippy-20261115-r3.log`.
The ordinary binary's strict Clippy also passes in
`/tmp/baylee-tower-ordinary-attachment-clippy-20261115.log`.

## Owner check before host claim — forty-third working milestone

Gateway integration review found a denial-of-start boundary: checking the human
owner only after a private host claim could let a foreign run ID consume the
victim's initial worker generation. The host request now carries the verified
session's Central UUID, the authority checks immutable ownership before any claim
mutation, and the async adapter checks the same identity in the reply. Gateways
must derive this UUID from their authenticated session, never from the public body.
The expanded real HTTP/PostgreSQL test passes in
`/tmp/baylee-tower-preclaim-owner-tests-20261115.log` (0.48s), including unchanged
victim state/revision/generation after the foreign request. Strict all-target
Clippy for service/API/account passes in
`/tmp/baylee-tower-preclaim-owner-clippy-20261115-r2.log`.

## Gateway attachment — forty-fourth working milestone

Following a move-only extraction of gateway startup/state and ordinary engine
attachment, the gateway now discovers its authenticated Tower host identity,
claims only for its verified Central player, chooses an exact advertised agent
capability and attaches the native Tower protocol. Recovery reads verify the
original prefix; complete operations pass through the atomic authority batch
transport before an ACK can unlock views. Lease renewal stops before sealing,
and private Tower games refuse ordinary rematches and spectators.

Three focused gateway tests pass in
`/tmp/baylee-gateway-tower-unit-tests-20261115-r2.log`: strict configuration/body,
exact capability/capacity selection and refusal to copy private opponent builds
through ordinary rematches. Strict gateway binary/tests Clippy passes in
`/tmp/baylee-gateway-tower-integration-clippy-20261115-r4.log`.
The expanded host identity HTTP/storage test passes in
`/tmp/baylee-tower-gateway-host-identity-native-20261115-r2.log` (1.24s).
Docker's internal filesystem became read-only, so this run used an owned native
PostgreSQL 18.7 cluster with SCRAM authentication and loopback TCP. The Rust
fixture reaped its database process, proved port closure and removed the cluster.
It did not restart or change unrelated Docker services.

This evidence does not yet prove the complete gateway/native/authority journey.
That integration scenario, automatic expired-host reassignment, global reconnect
discovery, historic opponent names and the new visible client remain outstanding.

## Live gateway/native acceptance — forty-fifth working milestone

After a move-only extraction of the existing Central server fixture, an optional
Tower-only integration path now connects actual Central enrollment/native login,
the gateway process, the global Tower HTTP/store, a separately compiled native
Tower child and its independent automatic verification queue. Its synthetic agent
control peer starts the real executable with an environment-only engine token;
agent allowlist/path validation remains covered by the earlier agent tests.

The focused test passes in `/tmp/baylee-tower-gateway-native-e2e-20261115-r2.log`
(12.00s). It proves host discovery, exact artifact routing, private owner binding,
unchanged reservation after foreign-player/extra-field requests, repeated start
returning the same table with a renewed seat token, a real human concession,
durable terminal views, one independently verified defeat and exactly one duel
income. The ordinary rematch route cannot expose the private opponent build.
No private Tower record frame reaches the seat. The automatic queue reports one
settlement and zero quarantines. Every owned engine, gateway, authority, Central
and native PostgreSQL process/listener is stopped and the test cluster removed.
This is transport/economic evidence using Forest-only decks, not balance evidence.

Strict integration-test Clippy passes in
`/tmp/baylee-tower-gateway-native-e2e-clippy-20261115-r3.log`. The first execution
found overlong synthetic display names; the fixture now observes Central's real
16-character rule. Its copied runtime grants also include the pinned migration
30 Tower-scope table. No auth bypass or weakened production rule was added.

To repeat with the repository's owned Docker fixture, build/copy a separate Tower
artifact (`--no-default-features --features tower`) before building the gateway
test, then set `BAYLEE_TOWER_V2_ENGINE` to its absolute path and run
`cargo dev central integration`. Only the Tower scenario runs in that mode;
legacy challenge generation and unchanged Central scenarios are skipped. The
native PostgreSQL fallback used here is retained as the temporary Rust harness
`/tmp/astra-native-qa`, because this machine's Docker filesystem is read-only.

## Automatic exact-prefix recovery — forty-sixth working milestone

Migration 9 adds a bounded-query expiry index and preserves existing playing
states through a one-time backfill. Normal Rust transactions maintain that index
without scanning/deserializing all active runs. Supervisor discovery and actual
recovery are separate: a fresh locked check refuses live, renewed, superseded,
sealed or settled work. Up to three automatic retries retain the same reservation,
package, prefix and original allocation; a disabled/incompatible host or exhausted
budget remains quarantined without rewards. Configured cohort queues supervise
expiry alongside independent adjudication, with separate value-only counters.

Review found a transport race: reusing the reservation UUID as each successor's
game ID let late stop/status callbacks address that successor. The gateway now
uses the execution claim UUID for transport; recovery already restores its
presentation identity independently of the immutable record. An owner's repeated
start during one live execution still returns the same table.

All 13 affected authority execution tests pass in
`/tmp/baylee-tower-expiry-storage-tests-20261115.log` (5.57s), including migration
backfill, simultaneous supervisors, renewed hints, exhausted retries, disabled
hosts, preserved bytes, old-worker fencing and existing atomic settlement/matching.
The expanded live scenario passes in
`/tmp/baylee-tower-gateway-recovery-e2e-20261115.log` (16.06s): kill the owned native
child after an acknowledged visible decision, expire only that synthetic lease,
observe automatic restoration, launch a separate transport generation, send a
late predecessor failure, recover the exact same encoded decision and concede.
The automatic worker reports one recovery, one settlement and no quarantine;
currency is paid once. All owned processes/listeners and the temporary cluster
are closed. These tests do not claim representative human difficulty or complete
cross-gateway discovery/attribution/client acceptance.
Strict Clippy for the changed gateway binary/integration test, Tower store
library/execution tests and service library passes in
`/tmp/baylee-tower-expiry-clippy-20261115-r8.log`. The generated API documents
also match the gateway's routes, including the empty-body rejection status.

## Fresh owned gateway location — forty-seventh working milestone

Migration ten adds immutable offline approval of canonical gateway origins.
The new owner-only `locate` operation reads the current reservation/claim rather
than reusing the historical result of an idempotent command. It exposes only an
approved enabled host and the current transport game, hides quarantined/disabled
assignments, and never supplies a token, fence, private package or record prefix.
The native SDK binds the response to the requested run and transport environment;
gateway admission remains a separate Central capability.

Three SDK transport tests pass in
`/tmp/baylee-tower-location-sdk-tests-20261115.log` (0.08s). The three affected
PostgreSQL location/expiry tests pass in
`/tmp/baylee-tower-location-store-tests-20261115.log` (0.61s), including foreign
owners, changed/invalid origins, disabled hosts, explicit cross-host reassignment,
fresh execution identity and migration backfill. Strict SDK/store test Clippy
passes in `/tmp/baylee-tower-location-tests-clippy-20261115-r2.log`; gateway and
service runtime Clippy passed in `/tmp/baylee-tower-location-clippy-20261115-r2.log`.
The actual native crash/recovery scenario also checks location before launch and
after each claim. It passes in
`/tmp/baylee-tower-location-native-tests-20261115.log` (14.85s), with one recovery,
one settlement, no quarantine and verified cleanup of owned services/database.
This establishes fresh routing; global host availability and the new native
client remain subsequent work.

## Global exact-host availability — forty-eighth working milestone

Gateways now publish installed exact engines separately from their free subset.
Filling an agent therefore cannot silently revoke compatibility for recovery.
An owned 20-second task advertises even without player traffic; admission holds
withdraw only free capacity. Migration eleven atomically replaces at most sixteen
indexed availability entries per host and fixes their lifetime at sixty seconds.
The owner-only `hosts` API joins these with explicit origin approval and enabled
roles, matches the complete frozen executor and returns bounded cursor pages.
The SDK checks account-run binding, ordering, page size and every destination.

Four SDK tests pass in `/tmp/baylee-tower-discovery-sdk-tests-20261115.log` (0.09s).
Five affected real PostgreSQL tests pass in
`/tmp/baylee-tower-discovery-store-tests-20261115.log` (1.00s combined), including
atomic concurrent advertising, invalid subsets, expiry, exact build skew, foreign
ownership, revocation, pagination and the updated migration/recovery scenarios.
The gateway inventory/assignment test passes in
`/tmp/baylee-tower-discovery-gateway-tests-20261115.log`.
Strict all-target Clippy for the five affected crates passes in
`/tmp/baylee-tower-discovery-clippy-20261115.log`.

The live native acceptance passes in
`/tmp/baylee-tower-discovery-native-tests-20261115.log` (31.80s). It discovers the
gateway through the background heartbeat before any player gateway request,
fills its sole agent, observes empty public availability, then still recovers the
exact acknowledged decision after a native crash. The independent worker reports
one recovery, one settlement and no quarantine. Owned services and PostgreSQL are
stopped and their listeners checked. Cross-host supervision is available through
the existing explicit recovery path; automatic migration to another gateway and
the visible v2 client are not claimed by this milestone.

## Native account-to-seat handover — forty-ninth working milestone

`Account::play_tower` now reuses an existing Tower connection, checks that its
account and Central endpoint match, locates the owned execution, obtains a
separate gateway-audience admission and asks for the strict human seat. A second
authority location read must confirm the returned game and the same approved
host/origin. Canonical UUIDv7, human chair zero and a lowercase 64-hex seat token
are required; unexpected response fields are refused. The token wipes on drop.
A gateway's forbidden response cannot impersonate a Central terms requirement.

The first live integration exposed why the account worker should reuse its scoped
Tower connection: repeated fresh grants correctly evict the oldest of Central's
bounded two grants, invalidating a separately cached shop connection. The corrected
SDK reuses that connection instead of weakening Central's credential policy.
The focused HTTP test fixture also explicitly switches accepted sockets to
blocking I/O with deadlines, because macOS can inherit its polling listener's
nonblocking flag. No production timeout or authorization rule changed for that fix.

Six affected SDK tests pass in
`/tmp/baylee-tower-sdk-handover-unit-tests-20261115-r2.log` (0.10s), including stale
or redirected handovers and malformed/private-chair capabilities. Strict SDK and
gateway integration Clippy passes in
`/tmp/baylee-tower-sdk-handover-clippy-20261115-r3.log`.
The actual native scenario now uses this SDK handover both for an attached game
and after automatic crash recovery, and rejects a different account both before
transport and at the authority. It passes in
`/tmp/baylee-tower-sdk-handover-native-tests-20261115-r2.log` (31.90s): one recovery,
one verified settlement, no quarantines, the original shop connection still
usable, and all owned services/database closed. New lobby presentation and
representative gameplay/balance remain outstanding.

## Retry-safe client state — fiftieth working milestone

The new `client-core::echoes` module owns v2 enrollment, run inspection, host
selection, shop/deck commands and explicit play intent, without network or renderer
dependencies. It serializes worker operations and retains exact mutation requests
through ambiguous failures. A newer read does not clear uncertainty; an original
retry receipt cannot rewind a newer displayed revision. Reservation acknowledgement
and subsequent gateway admission are separate, so a failed attachment resumes the
reserved duel without another opponent selection. No local economy, loss, reward
or quarantine result is invented. Worker identities reject departed replies.

The four focused model tests pass in
`/tmp/baylee-tower-client-state-tests-20261115.log` (0.01s; 2,577 unrelated tests
filtered out), including byte-identical mutation retries, delayed old workers,
malformed success, explicit consent and admission failure after a committed
reservation. Strict library/test Clippy passes in
`/tmp/baylee-tower-client-state-clippy-20261115-r2.log`. No backend/native test was
rerun for this renderer-free change. The native v2 screen/worker and observed
visual refinement iterations remain outstanding.

## Retained run catalogs — fifty-first working milestone

An authenticated player can read a published cohort's immutable catalog even
after enrollment closes. The active enrollment list stays empty; this read
neither admits a new run nor exposes opponents or private execution packages.
The SDK binds the returned cohort to the requested UUID, and the client retains
the catalog separately from currently accepting cohorts so an existing shop can
still name its cards, equipment and authoritative prices.

The affected real PostgreSQL/HTTP player scenario passes in
`/tmp/baylee-tower-retained-catalog-http-tests-20261115.log` (0.68s), including
closed enrollment, the exact retained catalog and an unknown cohort. Owned
PostgreSQL exited and its listener and isolated cluster were removed. The focused
model regression passes in
`/tmp/baylee-tower-retained-catalog-model-test-20261115.log` (one test; 2,581
unrelated tests filtered out). Strict service, SDK and client-core library/test
Clippy passes in `/tmp/baylee-tower-retained-catalog-clippy-20261115.log`. No native
engine or renderer test was repeated for this read-only catalog path.

## Native Tower lobby and choices — fifty-second working milestone

The account home now opens the separate Echoes screen. Its adapter caches the
short-lived Tower connection on the serial account worker, renews it before
expiry and discards connection/mailbox state on account changes. It handles
enrollment, saved-run inspection, approved host discovery, exact mutation retry,
confirmed gateway handover and returning to authoritative result verification.
The ordinary multiplayer rematch control is suppressed for an Echoes table.

Shop, equipment/fusion and deck panels expose the saved offers, prices, pins,
refresh limits, stage-seven material choice, owned slots, exact fusion ingredients,
conserved sale refunds, consumable charges and verified history. Purchases can
fuse directly into a full inventory. Client validation reuses the pure domain
rules; a confirmation is bound to its displayed run/revision and rechecked before
submission. Unsaved deck changes stay local and block a duel or another trade.
Enrollment and future-echo withdrawal have explicit consent confirmations. UI
strings are translated, controls have icons and semantic keyboard focus, and
creation timestamps are formatted with an explicit UTC label. Malformed catalogs,
oversized decks and invalid inventory slots are refused before rendering.

Ten targeted client-core tests pass in
`/tmp/baylee-tower-client-draft-tests-20261115-r2.log` (0.00s), including full-bag
fusion, Rare plus Mythic Potion, unowned cards, deck size/draft preservation,
malformed projections and repeatable withdrawal. Three native worker tests pass
in `/tmp/baylee-tower-client-worker-tests-20261115.log` (0.01s), proving account
mailbox isolation, refusal to remain busy after failed enqueue and protection
against a stale control behind a confirmation. Their first native link took
1m50s and reported the existing large debug unwind-table warning; the tests
themselves started no graphical client or services. The language coverage test
passes in `/tmp/baylee-tower-client-lobby-language-test-20261115.log` (0.07s).
Strict library/test Clippy for both changed client crates passes in
`/tmp/baylee-tower-client-lobby-clippy-20261115-r5.log` (11.66s).

This is a functional native integration milestone, not its visual acceptance.
The real rendered iterations, restart restoration of Tower context, current
opponent attribution, permanent awards and a representative gameplay/balance demo
remain outstanding. No backend or engine suite was rerun for these client changes.

## Authenticated Tower seat recovery — fifty-third working milestone

The common gateway seat route now returns a distinct optional Tower run context,
only after the ordinary owner/seat authorization and token rotation. It cannot
be combined with a Daily/Weekly challenge marker. The client validates UUIDs and
human chair zero, accepts the marker only after its gateway epoch check, restores
the Tower end-screen behavior and reads that owned run on returning. A gateway
response arriving before the first Central worker retains only navigation intent;
account departure clears it. Back/Escape cancels a deferred read, and idle polling
does not dirty/rebuild the lobby. Event Debug output contains no seat credential.

Six targeted client regressions pass in
`/tmp/baylee-tower-seat-context-client-tests-20261115-r2.log` (0.29s). The existing
Daily/Weekly seat restoration regression passes in
`/tmp/baylee-tower-seat-context-legacy-test-20261115.log` (0.23s). The initial
native test exposed a genuine SDK incompatibility: an existing execution takes
the common seat route, so its strict decoder also needed the optional context.
It now binds that context to the requested run and still refuses foreign runs,
unrelated fields and challenge context. Two SDK regressions pass in
`/tmp/baylee-tower-seat-context-sdk-tests-20261115-r2.log` (with `client` enabled;
the earlier featureless invocation ran zero relevant tests and is not evidence).

The real Central/PostgreSQL/gateway/native-executor acceptance passes in
`/tmp/baylee-tower-seat-context-native-tests-20261115-r2.log` (31.80s): fresh and
existing SDK handovers, common seat recovery with token rotation, one exact-prefix
crash recovery, one independently verified outcome and one gold payment. Every
owned service, native child and isolated database was closed. Strict Clippy for
the four affected crates passes in
`/tmp/baylee-tower-seat-context-clippy-20261115-r5.log`; generated OpenAPI includes
the bounded optional metadata. The native fixture still uses a land-only deck,
so it establishes transport/recovery correctness, not gameplay balance.

## Playable native demo fixture — fifty-fourth working milestone

The opt-in `BAYLEE_TOWER_V2_DEMO=1` branch of the existing native fixture publishes
three legal 40-card starters (green creatures, red aggression, white creatures)
from cards marked Implemented, with a distinct legal collection and fixed prices.
All twelve ordinals have three explicit House fallback builds; gradual equipment
is a playtest hypothesis, not calibrated balance. A real synthetic Central account
owns a normally enrolled run. The native control peer keeps its advertised single
slot alive, launches only the probed Tower artifact, handles stops and reaps every
owned child. The default short acceptance scenario keeps its land-only package.
Tower shutdown now aborts and joins timed-out worker/server tasks before dropping
its schema, instead of detaching them.

The opt-in demo also requires `BAYLEE_CENTRAL_DEMO=1`, a per-run password passed
only through the parent process's environment, `BAYLEE_CENTRAL_UI_READY` and the
existing isolated PostgreSQL fixture configuration. Readiness contains loopback
origins and synthetic run IDs only. Its `.stop` sibling ends the bounded hold;
`BAYLEE_CENTRAL_UI_HOLD_SECS` is limited to 1–1200 seconds. No authentication,
shop, scoring or verifier bypass was added to shipped code.

The focused catalog test passes in
`/tmp/baylee-tower-playable-demo-catalog-20261115.log` (one test, 0.00s), and strict
gateway integration Clippy passes in `/tmp/baylee-tower-demo-clippy-20261115.log`.
The actual German 1440×900 client signed in, showed the owned run, loaded its
live shop/host and switched to equipment. Baseline images:
`/tmp/baylee-tower-ui-baseline-shop-20261115.png` and
`/tmp/baylee-tower-ui-baseline-equipment-20261115.png`.
The complete owned UI/backend/database cleanup is recorded in
`/tmp/baylee-tower-ui-iteration1b-20261115.log`.

The first diagnostic run clicked during the 1.65-second sign-in flight and was
closed after its timeout. The corrected driver waits for that flight. A first
capture of the occluded passive Mac window was black; two consecutive captures
produced the real rendered frame. Black frames are not visual acceptance.
These images establish the baseline, not five completed visual improvements:
they expose excessive page length, repeated descriptions, generic icons and
purchase details below all offers. No duel or balance result is claimed yet.

## Native layout refinement 1 — fifty-fifth working milestone

The overview and selected expedition now occupy separate pages. Three starter
decks are aligned in equal columns, selected shop details precede the offers,
repeated effect paragraphs are removed from the grid, and compact next-duel,
deck and navigation labels have distinct icons. The selected run remains bound
while its overview is open; an unsaved deck prevents switching runs.

The real German 1440×900 client was inspected at enrollment, shop and purchase
detail. A normally authorized compass purchase reduced gold from 18 to 14 and
marked its offer sold. The overview reopened the starter list. Evidence:
`/tmp/tower-refinement1-shop.png`, `/tmp/tower-refinement1-detail.png` and
`/tmp/baylee-tower-ui-refinement1-20261115.log` (all owned processes/listeners
closed). Strict client library/test Clippy passes in
`/tmp/baylee-tower-ui-refinement1-clippy-20261115-r3.log`; its feedback simplified
the page state into the existing enum. The observed controls still need stronger
selection, clearer equipment slots and item artwork. This is the first observed
improvement, not final visual acceptance.

Before introducing item shaders, the debug client baseline was saved to
`/tmp/tower-refinement1-perf.json`: 11,672 frames, 57.77 FPS, update p50 6.446 ms,
p95 7.276 ms, including startup and navigation. The 120-frame change sample is
`/tmp/tower-refinement1-changed.json`; it does not name all components in this
build and is not proof of a completely idle UI. These are diagnostic baselines,
not an isolated GPU benchmark or a release-performance claim.

## Item presentation refinement 2 — fifty-sixth working milestone

Tower objects now have original analytic portraits: carved heartwood, satchel,
compass, grove and distinct potion bottles. A bounded shared material cache adds
engraving, corner gemstones and a crown with rarity; animation reads the GPU
clock and honors reduced motion/ambient quality. It is separate from permanent
achievement medals. Selected controls have a checkmark and filled background.
Active and reserve destinations form separate aligned three-column grids with
occupancy labels, and the purchase button includes the actual gold price.

Nine focused Tower-client tests pass (shader frontend, shared material reuse and
idle uploads, account isolation and seat recovery) in
`/tmp/baylee-tower-ui-refinement2-tests-20261115.log`. Strict client library/test
Clippy passes in `/tmp/baylee-tower-ui-refinement2-clippy-20261115.log`. The language
test caught a missing English argument declaration; after correction the exact
cross-language argument test passes in `/tmp/baylee-tower-ui-language-20261115-r2.log`.

The live 1440×900 shop and equipment were inspected; purchasing a compass into
Active 1 reduced gold by four and changed free mulligans from one to two.
`/tmp/tower-refinement2-equipped.png` shows the result. A fresh client verified
the corrected price text at 1280×720 in `/tmp/tower-refinement2b-compact.png`.
Cleanup is recorded in `/tmp/baylee-tower-ui-refinement2-20261115.log` and its
`refinement2b` sibling. Higher rarity animation has not yet received full visual
acceptance. The shorter viewport also showed that the purchase button and
equipment actions still sit too far down: the next refinement addresses that.

## Visible decisions refinement 3 — fifty-seventh working milestone

Purchase review and the explicit destination now precede the slot grid. The
empty equipment page explains the active/reserve limit and links to the shop;
move/sale/fusion controls appear only for a selected item, above the inventory.
Selecting an item returns to those controls. Common satchels describe the actual
hand-limit benefit without advertising a zero-card opening-hand increase.
Inventory and destination rendering were extracted in a separate move-only commit.

The 1280×720 client now keeps the price, destination and purchase button visible:
`/tmp/tower-refinement3-purchase.png`. The empty state and current deck were also
inspected in `/tmp/tower-refinement3-empty.png` and `...-deck.png`. The deck remains
too text-heavy and shop cards lack useful rules previews; those are outstanding
presentation work. Strict changed-client Clippy and the language argument check
pass in `/tmp/baylee-tower-ui-refinement3-clippy-20261115.log` and
`/tmp/baylee-tower-ui-language-20261115-r3.log`. The owned client, backend and
PostgreSQL are closed (`/tmp/baylee-tower-ui-refinement3-20261115.log`).

## Informed card choices refinement 4 — fifty-eighth working milestone

Selected shop cards now show the existing English rules, mana symbols, type and
creature stats; every face is available without an image request. Deck card names
are explicit preview controls with their own keyboard stops. The preview is
read-only and clears when entering a different run or returning from a duel.
Prices and purchase review remain above potentially long rules text.

The native shop showed Goblin Arsonist's red mana cost, 1/1 and death trigger;
selecting Giant Growth in the deck showed its green cost and full effect.
`/tmp/tower-refinement4-deck-preview.png` records the latter. The English view at
1280×720 and XL text remains readable (`/tmp/tower-refinement4-xl-deck.png`);
Tab enters the Tower keyboard context. Strict client Clippy and language argument
coverage pass in `/tmp/baylee-tower-ui-refinement4-clippy-20261115-r2.log` and
`/tmp/baylee-tower-ui-language-20261115-r4.log`. All owned processes are closed
in `/tmp/baylee-tower-ui-refinement4-20261115.log`. The existing backend binary was
reused for these UI-only checks. Duplicate focus destinations on the empty
equipment shortcut and the ordinary shop tab need correction in the next pass.

## Clear progression refinement 5 — fifty-ninth working milestone

Four equal metric cards now distinguish victories, remaining lives, gold and the
current stage. Empty history no longer reserves a row. Enrollment has an explicit
title and separately formatted publication timestamp; the new expedition entry
has its own label. The equipment-to-shop shortcut has a distinct keyboard stop,
and read-only navigation allocates no mutation UUID. Confirmation guards still
block stale navigation. Reduced-motion coverage includes existing and newly
created portrait materials and verifies no idle material uploads.

Ten targeted Tower client tests pass in
`/tmp/baylee-tower-ui-refinement5-tests-20261115.log`; strict client library/test
Clippy and exact translation arguments pass in
`/tmp/baylee-tower-ui-refinement5-clippy-20261115-r3.log` and
`/tmp/baylee-tower-ui-language-20261115-r5.log`. The real German shop was inspected
at 1440×900 and equipment/return at 1280×720. A normal four-gold Heartwood purchase
into Active 1 raised starting life to 24, observed both in equipment and the real
native engine view. A free mulligan retained seven cards. Playing Forest and
confirming the automatic mana plan put Llanowar Elves on the stack.

The short manual test then deliberately conceded: the native end screen offered
return/replay without an ordinary rematch. Returning loaded the independently
verified defeat, stage 2, four remaining lives and exactly 12 gold (14 to 26).
Evidence: `/tmp/tower-refinement5-equipped.png`,
`/tmp/tower-native-duel-initial.png` and `/tmp/tower-refinement5-return.png`.
The owned client, backend, native engine and PostgreSQL were stopped; listener
closure and cluster removal are recorded in
`/tmp/baylee-tower-ui-refinement5-20261115.log`.

This completes five observed UI improvement passes, not final gameplay/balance
acceptance. The opponent still uses the house label, permanent award delivery
is outstanding and higher-rarity animation still needs direct visual inspection.
The two-second recording made with the portrait outside the viewport is not
animation evidence. No full engine suite was repeated for these client changes.

## Permanent accomplishment delivery — design pass

The verified-settlement transaction already writes one immutable award event per
reservation. Delivery must use that event's stable database UUID, account and
run/stage as its idempotency identity; it must never invent achievements from a
client score, gateway statement or failed technical execution. A separate Courier
operator role may claim and acknowledge outbox rows, but cannot adjudicate games,
publish content or change gold. Host, verifier, publisher and supervisor roles do
not inherit it. Every operation rechecks revocation inside its transaction.

Claims use database-generated UUIDv7 fences, a bounded lease and SKIP LOCKED.
Expired workers cannot acknowledge a replacement claim. A lost remote reply
causes at-least-once delivery of the same event, requiring Central's immutable
receipt and deduplication before local acknowledgment. Delivery failures retain
the event and use bounded exponential backoff, without storing remote response
text or credentials. Account deletion cascades pending events; Central must also
refuse deleted accounts during a request already in flight. Run retention must
preserve undelivered accomplishments. Central downtime must not change duel
outcomes, currency or the native shop flow.

The first implementation slice covers only the durable delivery queue. Central's
explicit issuer credential, ingestion receipt, account-owned badge projection and
the service delivery worker remain separate required integration work. No generic
gateway achievement permission or public outbox endpoint is introduced.

## Fenced accomplishment outbox — sixtieth working milestone

Migration twelve adds the separate Courier role and a bounded two-minute delivery
claim. Claims use database UUIDv7 fences and SKIP LOCKED; acknowledgment checks
the current database clock and exact courier/fence, preserves its first receipt
time and is idempotent after success. Deferred attempts retain the immutable event,
clear the fence and back off from five seconds to at most one hour. Revocation is
checked under the operator lock on every call. No remote error body is stored and
there is no externally callable delivery route yet.

Three focused real PostgreSQL scenarios pass in
`/tmp/baylee-tower-award-outbox-tests-20261115-r2.log` (0.23s): disjoint concurrent
claims, stable event/account/result across recovery, late/foreign acknowledgments,
wrong roles, live revocation, exact acknowledgment retry, backoff cap and removal
of an in-flight event after account deletion. The deletion fixture first withdraws
consent and prunes the dense echo index; its deliberate RESTRICT constraint must
not be weakened to leave index holes. This does not implement the cross-service
account-deletion worker.

The existing version-one upgrade fixture was stale after migrations nine through
eleven: its reverse setup omitted execution-watch and host-location/availability
tables. The fixture now removes those explicitly before reconstructing version
one. Both migration/data-preservation and role-isolation scenarios pass in
`/tmp/baylee-tower-award-migration-roles-20261115-r3.log` (0.17s each). All owned
PostgreSQL instances, listeners and temporary clusters were closed, including the
failed diagnostic runs. Strict store library/test Clippy passes in
`/tmp/baylee-tower-award-outbox-clippy-20261115.log`. No engine or graphical test
was repeated for this storage-only milestone. Central ingestion and the delivery
worker are still outstanding; this is not a claim of end-to-end badge delivery.

### Milestone 61 — permanent-fact delivery to Central

The separate background `Courier` consumes the durable outbox with its own local
operator role and a separately provisioned Central issuer secret. The fixed
Central transport uses verified TLS in production, no redirects, a five-second
total deadline and a 4 KiB response limit. It accepts only the exact event/account/
run/ordinal acknowledgment. A lost response or local acknowledgment repeats the
same event; remote failures defer with durable backoff. Local revocation/storage
failure pauses only this worker, never gameplay or verification. Production stdin
configuration requires the two separate credentials and refuses reuse with its
verification/recovery credentials. No public courier route was introduced.

Review found that concurrent couriers could otherwise deliver later run ordinals
before an earlier receipt and cause unnecessary Central conflicts/backoff. Claim
selection now fences later ordinals on earlier undelivered events, including an
active lease or backoff. It joins the indexed reservation run/stage and unique
outbox reservation keys; each run has at most twelve stages. Independent runs
continue, and Central still validates ordering independently.

The API/client and synthetic gateway fixture pin the same published Central
commit `4b14dcf05c770639deb49443e34d23e3536e8c74`; only ten first-party Central
source IDs changed in the lockfile, with no external package version/feature
update. The mirrored fixture grants match that commit. `Account` exposes permanent
progress and bounded receipt export through its original Central session, even
while the Tower service is down. Rendering those new badges remains separate work.

Targeted evidence:

- `/tmp/baylee-tower-award-courier-tests-20261115-r2.log`: real outbox plus bounded
  synthetic HTTP, 0.25s, exercises unavailable/wrong/oversized/redirected replies,
  cancellation after remote acceptance, lost local ACK, exact retry, absent account,
  live local revocation and unchanged gold/wins. The existing player identity HTTP
  scenario passes in the same run (0.18s) after the shared transport extraction.
- The standalone service migration/start/shutdown test passes in the first run;
  its courier test exposed a test-only lease-reset mistake (zero expiry with a
  non-null claim), corrected to an expired positive lease without weakening SQL
  constraints. Strict production config tests pass in
  `/tmp/baylee-tower-award-courier-config-20261115.log`.
- `/tmp/baylee-tower-awards-real-central-20261115.log`: real enrolled/restored account,
  real Central HTTP/PostgreSQL, automatic courier, permanent SDK reads/export and
  lost-ACK deduplication pass in 4.03s. The input fact comes from the explicitly
  synthetic trusted-verifier fixture; this is delivery evidence, not another native
  game-rules/replay test. Select it with `BAYLEE_CENTRAL_TOWER_AWARDS_ONLY=1` in the
  owned Central integration fixture, without `BAYLEE_TOWER_V2_ENGINE`.
- `/tmp/baylee-tower-award-ordering-tests-20261115.log`: four outbox scenarios pass
  (0.25s), including concurrent same-run claims, backoff ordering, another run's
  independent delivery, fencing, role revocation and deletion.

All owned databases/listeners/processes were stopped. The temporary native QA
helper now uses PostgreSQL fast shutdown; its earlier detached shared-memory
segments were attributed to exact own-run log timestamps, checked for no attached
processes, and removed. No unrelated process or kernel setting was changed.
Strict store/service/account library, binary and test Clippy passes in
`/tmp/baylee-tower-award-courier-clippy-20261115-r6.log`; the integration harness
passes strict Clippy in `/tmp/baylee-tower-award-gateway-clippy-20261115-r2.log`.
Touched-crate formatting and whitespace checks pass. No engine test suite was run.

### Milestone 62 — bounded offline operator administration

The standalone service now accepts `--check-admin` and `--admin` with a strict
196 KiB stdin-only envelope, independently of its HTTP configuration. The closed
operations provision/revoke operator credentials, approve immutable host origins,
publish immutable cohorts/house builds and pause enrollment. SQL schema ownership
is checked independently of application roles. House publication additionally
requires a live Publisher. Exact provisioning retries preserve identity; changed
roles/names and revoked secrets refuse. Closing enrollment preserves old runs and
cannot be undone by an exact re-publication. No migration or dependency changed.

Review passes examined (1) input/secret/SQL authority boundaries, (2) repeat and
revocation behavior, and (3) real process/DB evidence and cleanup. The operator
manual documents calibration and final runtime SQL grants as outstanding release
checks; neither structural publication nor this milestone claims balanced decks
or full production readiness. No public administrative route was added.

Evidence on 15 November 2026:

- `/tmp/baylee-tower-admin-tests-20261115.log`: four binary configuration/input
  tests pass; three integration scenarios pass in 1.25s with real child processes
  and PostgreSQL 18; existing standalone migration/start/HTTP/shutdown passes in
  0.18s. Broad runtime table grants plus a valid Host token still cannot execute
  any administration action. All expected refusals emit only the fixed public
  diagnostic. The synthetic all-basic-land publication is not calibration data.
- `/tmp/baylee-tower-admin-store-tests-20261115.log`: existing live role,
  immutable fallback and concurrent host advertisement regression passes (0.17s).
- `/tmp/baylee-tower-admin-clippy-20261115-r4.log`: strict library/binary/test
  Clippy passes for the two touched crates. Earlier compile checks corrected
  test imports, secret-string borrowing and test layout before execution.

Owned child commands, two private PostgreSQL clusters, login roles and listeners
were closed/removed. Touched-crate formatting and whitespace checks pass. No
engine, graphical or unrelated test suite was repeated.

### Milestone 63 — bounded, account-isolated gateway start retries

Gateway retry identities now have a two-slot per-Central-account limit alongside
the existing 32-slot process bound, and expire after ten minutes without extension
by retries. Successful attachment releases its pending slot. The cache never
grants a lease; every new identity still passes the global ownership, assignment,
live lease and recovery checks. Invalid/truncated HTTP replies retain the original
claim because a remote commit may already have happened.

The first real integration run exposed a regression in retaining ambiguous
refusals with the old run-only key: a foreign account's refused attempt could
occupy the owner's slot. The key now includes authenticated Central account and
run, including removal paths; both a focused regression and the existing real
foreign-account/start sequence cover this boundary. No shared-session identity,
rules, reward or durable execution format changed.

Evidence on 15 November 2026:

- `/tmp/baylee-tower-claim-tests-20261115-r2.log`: five targeted gateway tests pass
  in 0.01s. They cover both caps, exact retry, expiry without sleeps, foreign-slot
  isolation and real loopback HTTP malformed/unavailable/conflict replies.
- `/tmp/baylee-tower-claim-native-e2e-20261115-r2.log`: one real Central/gateway/
  retained native Tower integration passes in 31.79s. It verifies foreign-owner
  refusal, repeated seat reclaim, one actual engine crash, preserved prefix and
  fenced recovery, then deliberate concession and one independently verified
  defeat/gold payment. It is not a difficulty or successful-victory playtest.
- `/tmp/baylee-tower-claim-clippy-20261115-r4.log`: strict gateway binary/test
  Clippy passes. Formatting and whitespace checks pass. No unrelated tests or
  full engine suite ran.

Both the failed diagnostic and successful real run closed all owned services,
native children and PostgreSQL clusters/listeners. No graphical client opened.

### Milestone 64 — frozen offer generations before catalog expansion

Retained shops now carry their cohort's generation explicitly. The original
generation defaults from old JSON and is omitted again on serialization, keeping
old receipts byte-identical. Shop opening, refresh, run loading/listing, house
publication and native package admission bind to the saved generation instead of
the latest default. Unknown generations fail closed. Only generation one is
enabled in this milestone; additional items are not silently added to old runs.

Before changing production code, a golden commitment captured 1,536 original
shop states (32 seeds, all twelve visits and three paid refreshes per visit,
with carried pins) and the original catalog digest. Both remain identical.
This freezes the current algorithm and encoding; it is not proof that future
price/effect changes can ignore their own version contracts.

Evidence on 15 November 2026:

- `/tmp/baylee-tower-generation-domain-20261115.log`: all 57 Tower-domain tests
  pass in 0.17s, including the golden and unsupported-generation admission tests.
- `/tmp/baylee-tower-generation-postgres-20261115.log`: one retained-state test
  and three existing transaction regressions pass (0.13s + 0.19s). An old start
  receipt remains identical after a paid refresh; corrupt generations cannot be
  read/listed or mutated, and refusal leaves the saved bytes/revision unchanged.
- `/tmp/baylee-tower-generation-clippy-20261115-r3.log`: strict touched-library
  and test Clippy passes. The isolated PostgreSQL fixture exited and its owned
  listener/cluster were removed. No engine or unrelated test suite was run.

### Milestone 65 — original 3D-style inventory art, two presentation passes

The owner's requested fantasy pictures now ship for all four implemented relic
families and all three potion/material types. Seven separate built-in image
generation calls produced the original alpha PNGs, without external reference
art. Prompts, generation paths, hashes and use are recorded in
`art/baylee/tower/`. The client samples the full image inside its original rarity
setting; all tiers reuse the family texture. Loading is lazy, bounded to seven
images, with no network art request or per-frame image/material upload.

An initial native inspection found the detail illustration too small. The second
pass enlarged shop/equipment details to 128 logical pixels while keeping repeated
tiles at 84. The final German shop was inspected at 1440 × 900 and 1280 × 720;
name, actual effect, destination, price and purchase review remain visible. A
normal four-gold Grove purchase reduced gold from 18 to 14; selecting the owned
reserve item showed the same new portrait in equipment. This is presentation and
purchase-flow evidence, not an expanded item catalog or full-run balance test.

Evidence on 15 November 2026:

- `/tmp/baylee-tower-art-tests-20261115.log`: four targeted portrait tests pass
  in 0.59s, covering shader validation, shared family textures across tiers and
  repeated rebuilds, reduced motion/idle writes, and all seven shipped PNGs'
  genuine alpha and bounded square dimensions.
- `/tmp/baylee-tower-art-clippy-20261115-r2.log`: strict client library/test
  Clippy including `dev-control` passes. The native dev client builds; its linker
  reports the existing large compact-unwind-table warning, not a shader error.
- `/tmp/baylee-tower-art-before-interval-20261115.json` and
  `/tmp/baylee-tower-art-final-interval-20261115.json`: before/after passive
  1440 × 900 shop samples, excluding startup, show median frames of 16.649/16.614
  ms and 58.55/58.23 FPS under the 60-FPS cap. These are debug-client samples,
  with independently seeded fixture shops, not an isolated GPU benchmark or
  evidence of a speedup. Entity/system counts remain 1058/859. The intermediate
  image report showed only three item textures loaded, shared across the view.
- `/tmp/baylee-tower-portraits-compact-20261115-r3.png` and
  `/tmp/baylee-tower-portraits-equipment-20261115.png`: inspected final compact
  purchase and owned-item views. Earlier black captures after resizing an
  occluded passive window are not visual evidence; unpausing and capturing two
  consecutive frames produced the inspected result.

The baseline and both new-art client runs closed their owned clients, backend
services and PostgreSQL clusters/listeners. Cleanup is recorded in
`/tmp/baylee-tower-art-baseline-20261115.log`,
`/tmp/baylee-tower-art-native-20261115.log` and its `-r2` sibling. No unrelated or
full engine tests ran. Higher-rarity animation still needs direct visual
acceptance in a real inventory; its motion/cache tests pass, but the Common
screenshots here are not evidence of the higher-tier animation.

### Milestone 66 — run-local power, persistent history

The owner's clarification is now an explicit binding requirement: a new run
inherits no equipment, upgrades, gold, purchased cards, charges, crafting
materials or gameplay bonuses. Its regular starter and starting budget are
restored. Resuming the same run preserves its saved build. Permanent cosmetic
accomplishments and historical echo snapshots are distinct from owned power.
The existing authority already constructs fresh run state; no production logic
or schema change was necessary.

A focused PostgreSQL regression uses only ordinary Store operations: buy three
matching relics, a healing charge and a card, edit the deck, fuse an equipped
Uncommon, pin an offer, resume, abandon and enroll again with the same account.
The new run equals the original starting state under its frozen generation.
Exact old purchase retries return the original receipt, request reuse with the
new run conflicts, and attempts to move/sell the old item or import its purchased
card are refused without mutation. The old finished record remains unchanged.
This covers the abandonment transition, not an additional conquest playtest.

Evidence on 15 November 2026:

- `/tmp/baylee-tower-run-scope-tests-20261115.log`: the one targeted real
  PostgreSQL test passes in 0.16s. The owned database exited and its listener and
  temporary cluster were removed.
- `/tmp/baylee-tower-run-scope-clippy-20261115.log`: strict Clippy passes for the
  new store integration test. Formatting and whitespace checks pass. No engine,
  client or unrelated suites ran.

### Milestone 67 — shared native market and inventory

The existing eight offers now form one image-led grid beside a single inspector.
Three equipped and six reserve slots, charges and crafting-material counts stay
under the stock. Selecting an owned relic replaces the offer inspector instead
of requiring a page change. Its effect and current location are explicit. The
paid reroll sits beside the shop tabs; run metrics and the next duel occupy one
compact header. Expedition history remains available on the other run pages.

The inspector scrolls independently, remembers its own position, and resets when
its selected object changes. Its three-column destination picker uses numbered
controls with the shell's minimum hit size, an icon-font checkmark and full
location/occupant tooltips.
The inventory itself has one selection action per owned instance; empty slots
are placeholders, so no duplicate destination actions enter the keyboard order.
Sold stock keeps its place and height. Item titles wrap within their measured
width; German uses the natural compound spelling `Pfadfinder-Kompass`.

Nine native refinement passes corrected text-width measurement, duplicate
placement actions, the oversized run header, misaligned inventory rows,
overlapping destination buttons, sold-tile height changes, long compound words
and verbose narrow-layout headings. The final two passes use the shell's
pixel-aligned paragraph rendering and a small glyph-overhang gutter for long
effect text under desktop UI scaling. Each pass used an isolated synthetic
account and loopback backend/PostgreSQL, virtual input and a passive client.
Every owned client, backend and database was closed after its pass, with exit
and listener closure checked by the Rust owner harness.

Focused evidence on 15 November 2026:

- `/tmp/baylee-tower-market-tests-20261115-final.log`: three UI regression
  tests pass in 0.05s. They check unique offer/place/item actions, sold and
  confirmation fencing, and mutually exclusive inspections without changing
  authoritative run state.
- Native pass two purchases a Common relic (18 → 14 gold), pins a second
  matching offer, then refreshes (14 → 12); the pin survives and the next price
  becomes 3. Screenshots and semantic states are kept under
  `/tmp/baylee-tower-market2-{bought,final}.{png,json}`.
- Native pass five purchases into reserve and moves the actual instance into
  active slot one. The confirmation names both locations; the resulting owned
  selection is at the active dock position. Its Tab check exposes the visible
  `echoes/back` focus stop. Evidence: `/tmp/baylee-tower-market5-{owned,moving,moved,keyboard}.{png,json}`.
- Pass six checks DE 1280 × 720 with social open and EN 1440 × 900. The word-wrap
  fix passes on a newly rolled Compass; 1024 × 768 subsequently identifies the
  verbose inventory headings, replaced with short localized headings in pass
  seven. Menu-overlay overlap reports were inspected separately from page
  layout; they are intentional menu-over-background overlap.
- Final pass nine passes the layout check at DE 1024 × 768 with social closed,
  1280 × 720 with social open, and 1440 × 900 with social open. The long Grove
  effect and Compass name are both present. Evidence:
  `/tmp/baylee-tower-market9-{1024,720,1440}.{png,json}`. A selected final preview
  also lives in the chat's `tower-market-20261115/shop.png` visualization folder.
- `/tmp/baylee-tower-market-locales-20261115.log`: three targeted phrase checks
  pass in 0.08s, including every phrase and argument parity in both shipped
  languages. Strict client-library `dev-control` Clippy, formatting and diff
  whitespace checks pass; no broad engine suite or CI wait was added.

This milestone changes presentation and selection only. It retains the existing
revision-bound command/confirmation path, frozen stock generation, prices,
run-local ownership and gameplay rules. It does not claim the proposed 24-family
catalog, four-tier comparison, ready-fusion guidance, enlarged-text acceptance,
or streamlined purchase submission is complete. Those remain separate work;
new pictures or enum entries alone do not count as new playable families.

### Milestone 68 — original portraits for the first life-synergy families

Prepared four distinct transparent 3D-style illustrations for Quellkelch,
Herdstein, Blutbernstein and Lebensspule. Each has its own silhouette, materials
and visual identity; no existing-game imagery was provided as a reference.
The generated 1254 × 1254 PNGs retain alpha and are stored in the project, with
prompts, original output paths and SHA-256 records under `art/baylee/tower/`.
Visual inspection, dimension/alpha inspection and all portrait checksums pass.
No engine or client runtime test was needed for these inactive raster assets.

Admission is still pending. Do not count these images as implemented item
families or silently repurpose the original frozen generator. Before adding
families, preserve old cohort/AI/native identities, calibrate the larger offer
pool and implement native effects, four upgrade tiers, client rules and house-AI
handling with actual event/cost/targeting tests. The catalog target remains the
24-family design in `tower-echoes-market.md`.

### Milestone 69 — retain native and AI contracts across catalog upgrades

Generation one now explicitly owns native revision 2 and house generation 19.
Catalog commitments, cohort admission and reconstructed presets read that saved
contract rather than today's native/AI defaults. Authority-side preset derivation
can address a retained generation; the compiled executor still admits only its
own pack. Unknown generations and substituted rules/policies fail closed.

Focused verification: all 35 `baylee-tower` v2 tests pass (0.19s), including the
unchanged golden stream across 32 seeds, every stage, pins and paid refreshes,
its old catalog digest, and a new retained native/policy preset regression.
Library/test Clippy with warnings denied passes. This compatibility milestone
does not enable new equipment or alter generation-one prices and offers.
