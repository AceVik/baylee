# Tower Echoes — design readiness and implementation evidence

Date: 14 November 2026. This records sequential self-review of the design, not
an independent code audit, measured balance study or passing runtime test suite.
Implementation remains paused while the design is being prepared.

Target: [Tower Echoes v2](tower-echoes-v2.md). Historical implemented foundation:
[initial proposal and test evidence](tower-echoes.md).

## Pass 1 — player experience and progression

Findings:

- The old five-win/three-loss limit gives at most seven duels. It does not support
  the intended long-build progression or a meaningful late Mythic opportunity.
- Three Rares for Mythic conflicts with the owner's corrected recipe.
- Directly purchased ranks and one copy per family prevent collecting triples.
- Persistent defeat allowance and duel life can be mistaken for the same thing.
- Matching by calendar date would compare different numbers of economic steps.

Decisions incorporated:

- The first draft used ten wins/four unsuccessful duels, at most thirteen, with
  inter-duel saving. Pass 6 below revises these pacing hypotheses; this records
  the actual intermediate decision, not the final defaults.
- Exactly one Rare plus a Mythic Potion; nine Commons if built from scratch.
- Three active and six reserve slots, distinct instances, deliberate fusion and
  an explicit full-bag buy-and-fuse transaction.
- Duel life resets; run allowance and unspent charges persist as separate values.
- Exact duel ordinal including losses/draws; no real-day requirement.

Evidence: inspected current run/shop/items/matching implementations, existing
proposal, engine conventions and repository Game Design/MMO Architecture/Game UX
skills. No code behavior changed and no runtime tests were run for this pass.

## Pass 2 — economy and adversarial commands

Findings and corrections:

- A vague promise of a useful affordable offer fails at zero gold/full capacity.
  The guarantee now applies at generation only if an eligible candidate exists;
  bounded generation has a deterministic fallback and an honest empty case.
- Two pinned utility offers conflict with the then-stage-nine guaranteed potion. The
  replacement is now a persisted explicit pre-purchase choice, never extra stock.
- A fusion can violate combined setup caps even if every input is valid alone.
  Validate the resulting entire loadout; allow the result to go to reserve.
- Rarity alone cannot identify interchangeable inputs. Freeze family, definition
  version and rarity; no first-release affix ambiguity or potion refill by fusion.
- Fresh free starting shops reward abandonment spam. Reuse the initial sequence
  until a verified duel; new runs cannot be used to roll it for free.
- A fusion-count badge encourages buying starting triples and abandoning.
  Credit distinct family/tier discoveries only after verified use.

Arithmetic inspected: nine Commons at four gold cost 36; Mythic upgrade adds 24,
total 60. Direct Rare plus potion costs 26 + 24 = 50. Before stage 5 the gross
base budget is 66; before stage 7, 90. These establish feasibility only: deck
upgrades, rerolls, scarcity and competing purchases consume the same budget.
No acquisition-probability or win-rate result has been measured.

Implementation checks required: stale/duplicate command; same idempotency key
with different payload; two concurrent buys/fusions; non-owned/reused input;
full-bag net fusion; sold pinned slot; refresh after purchase; guaranteed-offer choice
retry; free-grant basis; multi-generation resale conservation; overflow; failed
transaction leaves neither debit nor output. No runtime tests executed here.

## Pass 3 — distributed authority, recovery and matching

Findings and corrections:

- A valid replay cannot establish operator secrecy or human input. Ranked
  execution now requires an explicitly trusted operator; gateway access is not
  blanket authority to issue results or Central awards.
- Distinct service audience and worker/seat credentials are required. Existing
  Central trust APIs need an explicit compatibility check during implementation.
- A crash after displaying a draw can grant information if recovery rewinds it.
  Defined a durable-prefix/acknowledgement barrier outside the core, same-prefix
  recovery, fenced execution claims and quarantine for irrecoverable history.
- Returning an old worker's result after reassignment is not ordinary retry.
  Bind results to the current reservation/fencing generation and exact package.
- Separately updating outcome, charges, gold and next shop can duplicate rewards.
  Commit them together with an idempotent accomplishment outbox.
- Winner-only echoes bias the pool. Admit authorized pre-duel builds after a
  verified terminal game of any outcome; exclude self and duplicate weighting.
- A bounded query can still be biased: first-N database rows are not a sampling
  policy. Require an indexed defined sample over eligible account representatives.
- House fallback and player echoes have different competitive meaning. Give
  informed fallback choice and separate comparisons instead of fake players.
- Frozen display names can resurrect deleted attribution. Resolve current
  visibility and separate in-flight audit retention from public eligibility.

Failure matrix for implementation: Tower unavailable before reservation; worker
crash before/after accepted record append; gateway disconnect; duplicate terminal
delivery; result conflict; Central outbox failure; expired lease; wrong audience;
wrong account; stale revocation data; exact executor absent; withdrawn opponent;
mixed-version candidate; empty pool. In each case check gold, charges, stage,
opponent identity, public projection and replay continuity. No services started.

## Pass 4 — engine, AI, loops and performance

Concrete source findings:

- `baylee-engine/src/trigger.rs::collect_emblems` reads `own_abilities`; the
  existing supplies may obtain abilities from a token definition. A passive
  relic needs canonical lookup coverage, not just a registry entry.
- `LoopPolicy` defaults to `RunOnceThenBreak`. Initially this pass proposed
  productive/optional/mandatory classification. The owner's later clarification
  in pass 6 supersedes that: Tower detection is Off. `state/hash.rs` already
  includes life and mana progress; new state still needs correct replay hashes
  and compatibility with explicitly enabled policies outside Tower.
- `baylee-ai/src/activate.rs` rejects some free effects; `stall.rs` remembers
  repeated priority proposals. Preserve rejection recovery while enabling
  productive plans with a finite objective and an exit. Do not delete guards.
- `worth/mod.rs` has a first potion-specific valuation. Replace mode-specific
  identity checks with shared native semantics as the extension grows.
- Frozen Tower policy currently disables host-load-adaptive foresight. A new
  planner must have deterministic work and serialized/reconstructible plan state.
- The verifier bounds records, journal and turns. Big legal combos cannot rely
  on simply increasing all limits or accepting a tiny unbounded expansion claim.

Corrections: ordinary-action batch execution first; verified selective acceleration
as a separate optimization; opponent intervention and randomness retained;
checked arithmetic and object/record bounds; technical exhaustion separated from
rules outcomes; all seat consumers and AI forks included; unchanged Standard and
legacy challenge contracts compared separately. No zero-overhead promise.

Required focused fixtures: skipped/extra upkeep; cleanup versus end-step; two
simultaneous triggers and chosen order; intervening condition changes; source
destroyed/returned; replacement of life gain; countered paid potion; same input
after reconnect; productive loop; optional loop exit; interrupted
combo; numeric boundary; compressed-work bomb; differential fast/ordinary path;
AI assembles a winning line and responds to an opponent's visible combo without
seeing hidden cards. These are planned tests, not executed results.

## Pass 5 — UX, motivation and economical acceptance

Findings and corrections:

- Active relic icons can falsely imply a destroyed source still works. Tie
  projection, source highlight and inactive-for-this-duel state together.
- A generic three-item animation would misrepresent the corrected Mythic recipe.
  Specify separate three-to-one and Rare-plus-potion previews/animations.
- Missing error states invite a second start/purchase after timeout. Give each
  run phase one next action and handle stale projections without replaying input.
- Rarity color alone is inaccessible. Use names/materials/shapes, keyboard focus,
  reduced motion, complete DE/EN and typed localized timestamps.
- Long runs need meaningful one-duel sessions and pivots after losses. Save
  between duels, show progress, and avoid rewards for merely staying online.
- Perfect self-play does not prove player enjoyment, while tiny samples do not
  prove balance. Separate acquisition checks, paired diagnostic duels, held-out
  checks and actual human-played scenarios.
- The owner's five UI/implementation refinements cannot be satisfied by these
  paper reviews. Keep them as a separate post-implementation acceptance sequence.

Bounded first evidence plan: 64 pure synthetic acquisition paths; twelve selected
engine duels once stable, then only justified additions; three manual scenario
types; required repository gates once their scope is ready. No test/service was
run just to validate prose, and no existing historical result is relabeled v2.

## Pass 6 — owner clarification, pacing and cross-contract consistency

New binding owner decision: loop detection defaults Off for ordinary tables,
is opt-in at table creation, and is Off for Tower. This supersedes the earlier
proposal to automatically draw a mandatory Tower loop. The plan now specifies
actual detector bypass, including cycle-signature work, and independent technical
budgets with no invented victory/draw/defeat.

Source inspection found a compatibility trap: public `ProtoHouseRules.loop_policy`
is a uint32 and gamehost currently maps zero/unknown values to RunOnceThenBreak.
An enum default change would leave network tables enabled. Update/version the
creation contract and UI, preserve old serialized enabled variants and explicit
record policies, and reject unknown policies. Freeze Off into Tower packages.
Test normal new-table defaults, explicit enablement, old record replay, forged
Tower enablement and that long Tower repetitions emit no detector adjudication.

Pacing correction: under a deliberately simple independent 50% win model,
ten victories before four losses completes only `378 / 8192 ≈ 4.61%` of runs.
That is a mathematical scenario, not observed player performance. It risks making
the only named success too remote. The final starting hypothesis is eight wins
before five unsuccessful duels, at most twelve: `794 / 4096 ≈ 19.38%` under that
same toy model. Loss recovery and sub-goals still matter; actual skill, AI, draw
rate and matchmaking need playtests.

Move direct Uncommon/Rare/Mythic-Potion availability to stages 2/4/5, with the
guaranteed full-price potion opportunity at stage 7. Thus an undefeated eight-win
run has a chance to use its planned Mythic before ending, rather than missing a
stage-nine offer entirely. No free item or guaranteed complete combo is added.

Other consistency repairs:

- Changed new-table defaults are a deliberate Standard behavior change, while
  existing explicit policies/frozen challenge records retain their contracts.
- Stage-seven guaranteed stock remains a once-per-visit pre-purchase choice and
  cannot be multiplied by refresh/reconnect.
- Technical failure, mandatory-loop detection Off, draw caused by another game
  rule and player concession remain distinct outcomes.
- The branch includes the earlier feature foundation: future gate/PR scope must
  be computed against main, not just these documentation changes.
- Corrected a duplicated heading in the initial proposal and a duplicated word;
  the initial proposal now clearly links to its superseding design.

## Pass 7 — disabled-detection performance refinement

The owner specifically asked whether turning detection off can make the engine
faster. Inspection confirms that `LoopWatch` only starts expensive signature
sampling at 4096 iterations and then every 256. Ordinary-play savings therefore
need measurement; long progressing combinations are a more relevant second case.

The revised design requires a full Off fast path with no detector allocation,
signature calculation or state maintenance, plus a separately compiled Tower
variant with the detection implementation/nonempty watch state absent. Standard
retains the opt-in table setting. A small adapter centralizes the choice; shared
engine rules are not duplicated. A no-detection artifact rejects enabled presets.

Planned evidence: inspect independently built feature/artifact paths; compare a
short ordinary trace and a long finite combo past the sample threshold with
identical inputs; measure detector calls/allocations, timing, throughput and
binary/memory cost. Replay hashes and action validation remain. No benchmark
was executed and no measured speedup is claimed in this planning pass.

## Pass 8 — compile-time variants and agent routing

The owner's final refinement replaces the remaining runtime Standard toggle
with artifact selection: `standard` without detection (default),
`standard-loops` with detection (table opt-in), and `tower` without detection.
The setting chooses which executable to launch; it is fixed for the game.
This supersedes pass 7's possible runtime adapter branch in the ordinary binary.

Source inspection: agent Config currently names one `engine_bin`/`executor`,
AgentHello advertises one executor, and StartEngine contains no variant contract.
The selector therefore needs bounded installed-capability advertisement, an exact
reservation/start identity, local path mapping and independent engine handshake
validation. All variants share the agent's capacity. Missing capability cannot
fall back to a different rules/detection mode. Replay retains the same contract.

Preserve one shared kernel with static policy adapters/small entry points. Verify
the two detector-free release artifacts separately; a workspace invocation can
unify features and accidentally include detection. The Cargo primary documentation
was checked for that behavior. Include variant/feature identity in build metadata;
the same git commit is insufficient to distinguish these artifacts.

Additional acceptance cases: default table starts standard; enabled table starts
standard-loops; Tower starts tower; wrong/unknown variant, absent binary, stale
advertisement, mismatched Hello/preset, exhausted shared capacity and attempted
remote path injection are refused; old records select the retained compatible
executor. No executable was built or routing behavior changed during this pass.

## Pass 9 — maintainable refactoring and Carbon dependency

The owner permits larger careful refactorings, but explicitly keeps this task
in planning until Carbon's engine fixes have landed. That is a sequencing gate,
not permission to start a parallel engine rewrite. Do not inspect or mutate
another chat's worktree as a substitute for integrating completed main commits.

The implementation sequence now begins with post-fix rebase, reference fixtures
and relevant checks. It separates structural extraction from semantic changes,
introduces explicit kernel/mode/AI/execution-contract/domain/service/client
boundaries, and records removal milestones for temporary compatibility paths.
The new compiled variants are proved against existing games before adding more
Tower-specific behavior. Generated card code keeps its established pipeline.

The architecture is prepared to begin after that dependency; neither Carbon's
fix completion nor runtime readiness has been verified here. All current edits
remain documentation-only. No Carbon message, branch operation, engine patch,
test run or background automation was initiated.

## Requirement traceability

The owner's subsequent preparation/handoff instruction is recorded in the
[current preparation checklist](tower-echoes-preparation.md). It permits offline
Rust tools/tests now and replaces waiting for main with the exact READY-trailer
handoff merge; earlier pass descriptions above remain chronological history.

| Requirement | Decision in v2 | Implementation evidence required |
| --- | --- | --- |
| Global Tower, games on gateways | Sections 1, 5, 7 | One run across two gateways; exact reservation and echo |
| Central boundaries | Sections 1, 7, 10 | Purpose-scoped identity/award delivery; no Tower economy there |
| Correct triple/Mythic recipes | Section 3 | All recipes, ownership, full inventory and duplicate races |
| Luck and combination skill | Sections 2–4, 8–9 | Acquisition samples, viable pivots, human combo/disruption play |
| Same-stage historical opponents | Section 5 | Losses advance ordinal; versions/self/repeat/fallback tests |
| Native extensions and speed | Section 6 | Standard baseline/differential checks, measured release builds |
| Faster disabled detection | Section 6 | Full Off bypass; detector-free Tower artifact; short/long paired traces |
| Compile-time variants and agent selection | Section 6 | Three isolated artifacts; exact routing/handshake/replay; no runtime detection toggle |
| Mode-aware house AI | Section 6 | Projected choices, plans, replay, no hidden-information access |
| Loop detection Off | Sections 1, 6, 8 | New-table Off/opt-in, Tower forced Off, retained old policies |
| Professional native UX | Section 8 | Actual DE/EN screens, errors, focus, reduced motion, scaling |
| Long-term cosmetic rewards | Sections 5, 8 | Verified deduplication, distinct discoveries, no power transfer |
| Sparse meaningful tests | Section 9 | Changed-surface evidence, bounded samples, required gates |
| New branch and later owner review | Header, section 10 | codex/tower-echoes; reviewed commits/PRs; no automatic merge |
| Careful larger refactoring | Section 10 | Clear dependency boundaries; move-only commits; retained behavior/performance evidence |
| Bugfix swarm before refactoring | Header, section 10, preparation checklist | Final tip READY trailer, fresh verification, explicit merge/cleanup, retained regressions |

## Readiness decision

The design has explicit ownership, transitions, recipes, default policies,
failure semantics and a bounded vertical-slice sequence. There is no known
unresolved architectural contradiction blocking the first implementation
milestone after these passes. Engine refactoring remains paused pending the
final swarm READY handoff and its verified merge/reference check. Offline
preparation tools/tests are authorized separately. This is not a
guarantee of no defects or of player enjoyment, and does not mean the feature is
implemented.

Still to establish through implementation/measurement:

- The exact supported item catalog and multi-rarity costs/effects, acquisition
  probabilities, win/loss rhythm and fun of counterplay.
- Central audience compatibility and the minimum separately reviewed API changes.
- Durable-prefix recovery latency and bounded retained-executor operation.
- Correct normal/Tower Off behavior and deterministic native rule-pack/AI paths.
- Which shortcuts can be accelerated equivalently; ordinary execution remains
  the reference, with no silent partial support or unbounded verification work.
- Real performance, bot strength, UI clarity and adversarial service behavior.
- Final operator retention and trusted-host policy before public release.

No expensive builds, game runs, production action or test services were necessary
for this planning milestone. Static documentation/branch checks are recorded in
the final commit handoff; earlier test results remain historical evidence only.
