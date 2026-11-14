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
started. Feature-branch publication remains subject to the full required gates.
