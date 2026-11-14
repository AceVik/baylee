# Tower Echoes v2 — implementation target

Status: implementation started after the verified handoff, 14 November 2026.
Owner review is planned for Tuesday,
17 November 2026, with Fable 5.2; that is a review target, not a claim that
implementation or acceptance is complete. The owner subsequently authorized
landing the existing foundation on main and starting full implementation;
Baylee main now contains `0d9cee40d82e746ed17964e81950846e87223a73`.
No production deployment is authorized by that merge.
Working branch: `codex/tower-echoes`, PR base `main` in Baylee. It preserves the
existing Daily/Weekly and Tower foundation; it is not an orphan history or a PR
stack that requires the old feature branch to merge first.

**Owner sequencing constraint, updated 14 November:** preparation tools and
fixtures may proceed while the Antigravity/Carbon bugfix swarm works. Engine
refactoring starts only after the final `agy/pre-refactor-handoff` tip has the
exact `Astra-Handoff: READY-2026-11-14` trailer. The owner explicitly replaces the
earlier wait-for-main/rebase sequence with a fresh verification and a `--no-ff`
merge of that handoff, followed by remote/local handoff cleanup. The review date
does not override this gate. See the exact sequence and current evidence in
[the preparation checklist](tower-echoes-preparation.md). Do not mutate the
swarm's worktree or cherry-pick an unfinished subset.

This sequencing gate is now satisfied: handoff `3fc9412c58b07c446c5f7e0c062965d6e4d2b984`
was freshly verified and merged as `aa7edb78d2b571af47ed45be93a6d7798e9c8f9e`.
The handoff refs and temporary watcher were cleaned up. The first kernel
extractions preserve existing behavior; the three execution variants, changed
new-table default and Tower v2 remain implementation/acceptance work.

This is the successor to [the initial proposal](tower-echoes.md). Code still
implements the older short run, directly bought ranks and narrow supplies seam.
Values below are explicit starting hypotheses, not measured game balance.
Owner requirements are distinguished from implementation defaults so calibration
can change numbers without silently changing the requested game.

## 1. Experience and owner requirements

The promise is: discover a promising build, survive while assembling it, then
make a spectacular combination work against another player's historical build.
Manual Magic play remains the duel. Items add ordinary abilities, starting
attributes and effects; they do not replace priority, the stack or core rules.

Binding requirements:

- One gateway-independent Tower service in the Baylee repository owns runs,
  shops, gold, snapshots, matchmaking and Tower rankings. Gateways host duels.
  Central owns identity and confirmed persistent accomplishments; its existing
  global Daily/Weekly tasks remain separate.
- Echoes come from the same duel ordinal, irrespective of real calendar date.
  Display the owner's name and the fact that Expert house AI pilots the echo.
- Native, compiled rule extensions and a matching AI extension; performance and
  maintainability matter. No interpreter or network callback in duel execution.
- Three identical Common items become one Uncommon; three identical Uncommons
  become one Rare; **one Rare plus one Mythic Potion becomes one Mythic Rare**.
  Three Rares are never required for the final upgrade.
- Powerful interactions and difficult-to-assemble infinite combos are intended.
  A technical work limit must not masquerade as a rules outcome.
- Owner clarification during this review: loop detection is **off by default
  for new ordinary tables**, optional at table creation, and **off in Tower**.
  This explicitly supersedes the earlier proposed mandatory-loop auto-draw.
- Ornate but readable native UX, localized text/timestamps, original imagery,
  distinctive rarity/tier treatment, keyboard access and reduced motion.
- Preserve solid existing modes. Test the changed surface economically, then
  pass the repository's required gates before publishing code for review.

No real-money power, permanent account stat advantages or mandatory daily
attendance. Long-term progression records discoveries and mastery cosmetically.

## 2. Run rhythm and recoverable sessions

Initial calibration after the pacing review: eight victories conquer a run; five
unsuccessful duels end it, giving at most twelve duels. A rules draw caused by
ordinary game rules still consumes one unsuccessful-duel allowance and is shown
as a draw. These run counters are distinct from duel life.
Starting life and deck state reset for each duel; purchased equipment and unspent
consumables persist within the run. Damage does not silently become persistent
injury. An echo spends its frozen charges only in this duel, never in its owner's
live inventory.

Target ordinary duel duration is approximately 10–20 minutes, with longer
interactive duels allowed. A run spans several sessions. Save after every shop
command and verified duel; make a one-duel session satisfying. No speed ranking,
real-time item cooldown, real-day stage or attendance streak penalty.

Flow: choose a legal starter archetype → initial shop → edit/equip → freeze build
and reserve opponent → manual duel → verify result → gold and next shop, or final
summary. Begin with three comparably funded starters that support different
strategies and allow pivots. The player's external collection supplies no power.
Basic lands are free, non-sellable and available to the owned-pool deck builder.

Wins, losses, draws and deliberate concessions all produce the same base income
after verified completion. Concessions still spend a defeat allowance. Technical
failure grants neither income nor a victory. Abandonment ends the run without
completion credit. Final duel income can be recorded in the final ledger but
does not open another shop or transfer into another run.

## 3. Economy, luck and fusion

Start with 18 gold; each verified terminal duel grants 12. There is no interest
or win-streak income. Gold is run-local and never transfers between accounts.
Initial relic prices: Common 4, Uncommon 10, Rare 26; the Mythic Potion costs 24.
Individual exceptional items may have explicit, visible catalog prices instead.
Those are balance parameters frozen with the run, not caller-provided prices.

The baseline nine-Common path costs 36 gold plus the 24-gold potion, before shop
rerolls and other purchases. A directly purchased Rare plus potion costs 50.
Thus an early investment and a late lucky Rare can both matter. Base income before
duel n is `18 + 12 * (n - 1)`, before spending and eligible sales; verify ledger
conservation rather than assuming every player has that amount left.

Three equipped relic slots and six reserve slots make collecting possible while
keeping opportunity costs. Copies may occupy different active slots if their
definitions allow it. The reserve is inactive. Fusion consumes exactly the
selected instances and places its result in the chosen valid slot atomically.
It never activates an item from reserve or refreshes a used consumable.
Allow an explicitly previewed buy-and-fuse when the net resulting inventory fits;
a full bag must not prevent a valid three-to-one fusion.

Fusion requires the same family, rarity and frozen catalog definition; there
are no random affixes in the first release. Fusion is only for relics, never
ordinary consumable charges. There is no unfuse/refund path. Every command
selects owned instance IDs, expected revision and a request identity; the server
derives the recipe, cost and resulting instance. Validate the resulting loadout
as a whole, including combined opening-hand/mulligan caps, before committing.
An otherwise valid fusion may place its result in reserve instead of equipping
an illegal combination. Never silently clamp a purchased effect.

Mythic Potions are shop crafting materials, not table activations. They are
consumed during Rare-to-Mythic fusion, carry across shops and occupy a separate
allowance of at most two. Ordinary healing/mana potions occupy two duel
charges combined. All materials expire with the run. Persistent unlocks are
cosmetic; nothing is purchased with real money.

Proposed shop: eight offers, normally three cards, three relics and two utility
offers. Each has a stable offer identity, quantity and visible authoritative
price. A utility slot can contain a potion or crafting material. Duplicates have
different offer identities and may appear deliberately. Inventory capacity does
not hide useful future fusion targets.

Pin at most two offers across the next shop refresh; they consume offer slots
rather than creating extra stock. Up to three paid refreshes per visit cost
2/3/4 gold and replace unpinned slots with new identities, including sold slots.
The persisted offer list changes only through an accepted command or advancement;
reconnection and gateway changes do not reroll it. Show counts and next refresh
price before spending. No automatic fusion or silent inventory sale.

Later shops may offer higher rarities: Uncommon from stage 2, Rare from stage 4,
Mythic Potion from stage 5. At stage 7, before that visit's first transaction,
a reaching player can choose one guaranteed potion offer at its full price
instead of one normal utility offer. Replacing a pinned utility offer is an
explicit choice. This one-time choice is persisted and cannot be repeated by
refreshing; a bought potion is not restocked by the guarantee. This provides an
achievable plan without granting a free Mythic. Earlier discoveries remain rare.
Retaining a Rare and enough gold is a deliberate decision.

At visit creation, one unpinned offer is an affordable, legally usable card,
relic, valid fusion input or consumable if such a candidate exists at the player's
balance/capacity. If none exists, explain the constraint and keep selling/editing
available; do not invent an improvement or generate forever. The filter uses the
entire owned collection, not merely equipped items or knowledge of the next
opponent. Subsequent spending need not preserve affordability. Guarantee useful
choice coverage, not a complete winning combo or equally strong random offers.
Bound selection attempts and use a deterministic eligible-candidate fallback.
Freeze catalog, generator version and private seed per run.

Sales return at most half the recorded gold basis, additionally bounded by the
catalog resale value. Free grants have zero basis; fused basis is the sum of the
consumed items and potion. All descendants inherit that basis without duplication.
The ledger must prove that buy/fuse/sell sequences cannot create gold.

Persist shop randomness separately from public display IDs and duel shuffle
seeds. One active run per account/cohort. Repeatedly abandoning before the first
verified duel reuses that account/cohort's initial offer sequence; it does not
grant a fresh free sample. Normal completed play advances the start sequence.
Service rate limits bound requests and account abuse, but do not impose a daily
energy meter or paid refresh. This cannot prove that different accounts belong
to different people; do not claim Sybil resistance from account IDs alone.

## 4. Content and counterplay

The first full catalog should cover about twelve relic families, four rarities,
ordinary potions and the Mythic Potion. Freeze exact rules and prices before
admitting each family; a four-rarity label cannot stand in for unimplemented rules.
The first complete vertical slice uses a smaller fully supported subset.

Families support token growth, life conversion, spells/resources, and resilient
combat. Utility families add start life, bounded free mulligans, opening/maximum
hand size, or setup tokens. Keep the existing opening-hand limits explicit; do
not stack items beyond what the setup contract can represent.

Each family has one recognizable identity across rarities. Upgrade numbers only
where useful; some higher tiers add a synergy or remove a specific constraint.
Mythic can complete an exceptional engine but is not required for every good
build. Two or three unmerged copies may outperform one upgraded copy in a
particular build; the preview must explain the actual effects and freed slots.

First combo specimen: token entry gains life; life gain optionally pays mana to
create a token; token entry generates that mana. Require several dedicated slots,
a high-tier enabling piece and a legal starting event. The optional step gives
the controller an exit. Select the exact definitions and trigger order in the
engine fixture before balancing their gold costs.

Powerful recurring combo sources can be ordinary starting artifacts with legal
interaction, while modest setup bonuses can be player modifiers. Every relic
states its source zone and what happens if that source leaves. A destroyed source
does not delete the owned relic from the run; it returns on the next duel's setup.
Do not give every pivotal effect an untouchable emblem. The card/shop pool must
offer more than one practical answer to each supported combo archetype.

No blanket once-per-turn rule. Caps belong to explicit item text where balance
requires them. Assess complete builds, slot pressure, setup time, susceptibility
to disruption and acquisition cost rather than isolated damage/gold ratios.

## 5. Echo selection and rewards

Stage is the ordinal of the next duel, including preceding losses/draws. Match
exact stage, economy/catalog/rule-pack versions, engine build and AI policy.
Prefer a similar current win/loss record when the pool supports it; never widen
to a richer stage or inspect the requesting player's hidden deck to choose a
counter-build. Exclude own account and prefer unseen opponents. One account has
one sampling weight per cohort/stage, independent of submitted run count.

Freeze a legal pre-duel build; eligibility cannot depend on winning. A player's
echo is a historical build, not a replay of their decisions. Persist the chosen
opponent and shuffle/policy seeds with the reservation. Display public equipment
and the owner's permitted display name, with an explicit Echo/Expert label;
hidden deck contents stay hidden. Explain participation and identity use at run
enrollment, and honor future withdrawal/deletion.

If no suitable echo exists, offer waiting or a clearly named house opponent at
that stage. A run using fallback is marked for a separate expedition comparison,
not silently mixed into the all-player-echo competitive leaderboard. Explain this
before the player accepts fallback. Keep personal progress and ordinary mastery
rewards available. Do not invent human names or online status.

One personal best per season/cohort on the competitive board: wins descending,
then unsuccessful duels ascending; actual ties share rank. Stable IDs support
pagination only. Avoid speed, remaining life, wealth or combo spam as tiebreaks.
Histories retain other runs. Scores from different balance/policy versions are
never silently compared as if conditions were identical.

Persistent rewards: tiered original badges for verified victories, distinct
archetype completions, fusions and meaningful combo milestones; a collection book
with item discoveries and optional replay highlights. Count facts derived from
committed outcomes, not client claims or unlimited clicks. Track lifetime progress
without stat bonuses or a reset penalty for missing a day. Central receives only
confirmed idempotent accomplishment events; Tower retains its operational state.

Fusion discovery credits a distinct family/tier only after that result is used
in a verified duel. Repeatedly buying the same starting triple and abandoning
does not advance a cumulative fusion badge. Achievement facts have a bounded,
versioned definition; a combo medal requires a proved game event, not a submitted
combo name or event-count spam.

## 6. Rule-pack and AI contracts

The service freezes an admitted loadout. A native Tower module maps it to bounded
setup values and existing native ability/cost/effect definitions. The game engine
alone executes them. Versioned definition IDs and validated parameters replace
arbitrary scripts, callback URLs, raw state patches and administrative SetLife.

Reuse native triggered, activated, delayed and continuous-effect machinery.
Schedule by actual game events, including extra/skipped steps, never wall-clock
cron. Sources, controllers, durations, intervening conditions, counters and
pending delayed work are part of deterministic state. Outside a duel, normal
service scheduling may rotate seasons or retire expired reservations.

The owner refined this to three separately compiled artifacts backed by shared
engine code. Presence of loop detection is a compile-time decision in each:

| Variant ID | Rules and AI | Loop detection | Selection |
| --- | --- | --- | --- |
| `standard` | Existing ordinary formats/policies | Absent | Default new ordinary table |
| `standard-loops` | Same ordinary rules/policies | Compiled in and enabled | Explicit table-creation opt-in |
| `tower` | Native Tower rules and fixed Tower AI extension | Absent | Tower reservation only |

The table setting selects an artifact before launch; it does not install a
runtime enabled/disabled switch in the game engine. An enabled variant may retain
the explicit existing break/draw adjudication policy, frozen at setup. Do not
create an artifact per item, rarity or arbitrary combination of extensions.
Item choices remain validated data for the admitted Tower pack.

The agent advertises bounded locally installed variant/build capabilities, and
maps those IDs to operator-configured executables. The gateway/Tower reservation
selects an exact compatible contract: variant, build, pack, card pool, AI policy
and loop capability. StartEngine carries that identity, never a remote filesystem
path, shell command or download. Validate again at engine attachment and setup.
An unavailable variant waits or reports a specific capacity/incompatibility error;
there is no fallback that changes rules. Count all variants against the agent's
shared capacity, not three separate copies of the same available slots.

Store the same contract with the record and use it for replay/resume/verification.
A source commit alone cannot identify three differently compiled artifacts:
include the variant/feature fingerprint and verify the installed artifact against
operator-controlled build metadata. This is compatibility checking, not remote
attestation of an untrusted operator. Retain old executors for supported records.

Build release artifacts separately and inspect their resolved dependency features.
Cargo combines dependency features within a build; a combined feature-unified
workspace build is not proof that `standard` or `tower` omitted the detector.
See the [Cargo feature-unification reference](https://doc.rust-lang.org/cargo/reference/features.html#feature-unification),
checked 14 November 2026. Small entry points or static policy types can select
the shared implementation without copying the rules kernel.

The AI extension evaluates scarcity, synergy, threats, trigger ordering and
multi-step plans through the same authorized PlayerView/Pending interface.
It may propose/rank legal actions, never enact privileged effects. Use native
semantic descriptors shared with rules, not name matching or a separate rule
interpreter. Fixed deterministic work budgets and policy seeds make decisions
reproducible; machine load must not change recorded ranked decisions.

The echo pilot never shops for or alters its saved build. A separate offline
shop planner can help balance acquisition strategies. Existing Standard AI keeps
its behavior unless a separately justified shared bug fix is required. Human,
house, trained and language-model seats must all see and answer every new choice;
encoding a new feature is not evidence that an old learned model plays it well.

### Trigger and combo correctness

Every admitted effect declares source identity/incarnation, controller, trigger
zone, condition, duration and legal choices. Setup placement is not an invented
enter event; recurring effects begin at their specified actual game event. Keep
native replacements, state-based actions, trigger ordering and priority. A next
own upkeep fires at the next actual own upkeep; a skipped one does not consume
it, and an extra matching step counts. End-step triggers and cleanup expiry remain
different operations. Conditions and source removal use native rule semantics.

The emblem trigger and static collectors now use the canonical ability lookup,
including token-backed definitions, with targeted native/stored-emblem tests.
Simply adding a passive relic to a registry still does not complete its setup,
view, AI and authority integration. New runtime state must participate in
replay, resume, projections, AI forks and rules-relevant progress signatures.

Combos are legal sequences, not a privileged command that declares a win or
injects resources. A player may request a finite repeat count for a demonstrated
sequence. First support deterministic, bounded batches of ordinary legal actions
with all opponent decisions intact. Suspend at a changed condition, new choice,
opponent intervention or game end. Keep progress visible and let the controller
stop at a legal decision boundary; disconnect is not consent to keep acting.

Accelerated shortcuts are a separate, narrowly verified optimization. They must
preserve costs, triggers, replacement effects, source incarnations, target choices,
history counters, RNG and the exact endpoint. They cannot assume an opponent
passes, compress random outcomes or omit a threshold crossed halfway through.
Use scalar/aggregate acceleration only where equivalence to the ordinary path is
proved and tested; otherwise execute ordinary steps in batches. Individually
targetable tokens cannot be replaced with an indistinguishable count by assumption.
There is no promise of a universal solver for arbitrary card interactions.

The owner has now explicitly chosen disabled loop detection for Tower. Add an
explicit `Off` policy; neither the action-path cycle detector nor the internal
cycle-signature detector runs under it. Do not silently substitute
`RunOnceThenBreak`, `CompRulesDraw`, trigger suppression or an automatic loop draw.
The frozen Tower package requires Off; an attempted policy override is refused.
The player's finite repeat tool and the AI's tactical planning remain available:
they select legal actions and exits rather than adjudicating a loop outcome.

New ordinary table creation defaults to Off and exposes an opt-in control with
an explanation of the selected enabled policy. This chooses `standard` versus
`standard-loops` before launch. Existing enabled break/draw policies remain
explicit choices where supported; they must not become an invisible default.
The selected binary must match the frozen setting; it cannot toggle detection
mid-game. Off removes loop-signature work, not merely the final break action.
Technical memory/CPU/record bounds stay independent and never synthesize a rules
result. AI recovery from rejected actions must remain, but cannot become a hidden
Tower cycle detector that suppresses legal productive play.

Preserve the serialized identities of existing enabled policies and append the
new variant without renumbering them. The public protocol currently maps zero
and unknown values to break-on-loop; changing only the Rust default is therefore
insufficient. Version the table-creation contract, distinguish a missing new
setting from an explicit legacy policy, reject unsupported values and update
gateway validation, client controls, presets and all consumers together. Existing
records and frozen challenge packages retain their explicit policy; record decode
must never substitute the new-table default for historical behavior.

Counts and resources use exact checked arithmetic; no floating-point infinity,
wraparound or saturated values that change legality. Bound repeat commands,
records and intermediate objects before allocating. Cooperative execution yields
outside the core; an owned worker enforces memory and wall-time limits as a final
backstop. A budget failure is a technical state, never a fabricated win or draw.
Show unsupported numeric/work requests before consuming resources when possible.
Common admitted combo lines must fit the tested execution envelope before release.

The current verifier's 16 MiB/50,000-line/500,000-event/500-turn limits are legacy
technical bounds, not a design promise that every new combo fits. Add versioned
repeat records and an equally bounded verifier if batching changes record volume;
validate expansion work rather than trusting a tiny compressed request. Client
and verifier must agree on shortcut choices. The AI replay must reproduce the
same decision/plan state and fixed budget, including rejected-proposal recovery.

The Magic reference for voluntary shortcuts is
[Comprehensive Rules 733, Taking Shortcuts](https://media.wizards.com/2026/downloads/MagicCompRules%2020261106.txt),
checked 14 November 2026. Opponents can shorten a proposed sequence to intervene;
that interaction remains. The rules' automatic mandatory-loop draw is not enabled
in Tower, per the owner's explicit loop-detection setting. A runtime watchdog is
not an alternative path to that adjudication.

### Performance and isolation acceptance

Resolve definitions once at setup; share immutable definitions. Keep strings,
reflection, dynamic scripts, service calls and shop logic out of the decision
path. Prefer extending the existing event collector to a parallel event bus.
Profile before adding event indices or generic plugin machinery. Compile-time
specialization is an opportunity, not proof of zero overhead.

The owner's additional performance requirement makes disabled detection a
compile-time path in both `standard` and `tower`. A small static policy adapter
owns detector-only counters, signatures, comparisons and reset state; the disabled
implementation is zero-sized. The release artifacts omit the detection code and
nonempty watch state, including calls in both action and internal machine paths.
There is no per-action enabled/disabled branch, detector allocation or detector
state maintenance in these variants. `standard-loops` contains the real detector.
Keep the normal game rules in one implementation and prove this property on the
release artifacts instead of assuming inlining removed a runtime switch.

A binary without detection advertises only Off support and refuses an enabled
preset; `standard-loops` similarly refuses a request for Off rather than acting
as a universal switchable binary. Artifact selection happens in the agent.
Verify the separately built release artifact/call path and feature graph; a
combined feature-unified build cannot demonstrate this property. If a structural
specialization causes excessive code size or complexity, measure and refine the
adapter instead of duplicating the entire engine or claiming unproven savings.

Current code delays cycle-signature work until 4096 iterations and samples every
256 thereafter (`baylee-engine/src/loops.rs`). That limits its ordinary-case cost;
do not describe it as hashing the full state on every action. Measure a short
ordinary trace and a long, finite productive combo past that threshold, each
with identical actions under enabled and disabled detection. Count detector
allocations/signature calls and compare latency/throughput; separately measure
the specialized artifact. Use a progressing trace so enabled detection does not
intentionally change the outcome. Report observed gains, including negligible
ones, instead of assuming a universal speedup.

Replay/synchronization hashes, legal-action validation and deterministic AI
rejected-action recovery are different responsibilities and remain intact.
Disabling loop detection is not permission to remove verification or let a
worker consume unlimited machine resources. The resource supervisor does not
inspect repetition or silently turn detection back on.

Capture release-build Standard baselines before changes: decision latency,
simulation throughput, allocation/peak memory, binary size and cold start. Compare
unchanged semantic fixtures with the same machine/build configuration. An observed
regression over 5% is an investigation threshold, not a statistical conclusion or
an allowance to regress by 5%; rerun only when noise or a relevant change warrants
it. New Tower cost should scale with actual active effects, bounded search work
and produced objects. Use a quiet baseline, no giant repeated full-suite benchmark.

An unextended Standard preset with an explicit historical policy must preserve
behavior under its version contract. The deliberate new-table default change to
Off is separately tested and documented. Differential tests compare ordinary
execution against Tower-with-no-pack and accelerated against unaccelerated
supported combos. If serialized schemas
must change, version and migrate/refuse explicitly rather than silently ignoring
old supplies. Old Daily/Weekly frozen policies and records remain reproducible
through their retained executor; do not mutate their contract to add Tower AI.

## 7. Authority, recovery and privacy

Use one durable Tower authority with reusable domain/store/API libraries and a
small executable, all in Baylee. Its database/schema is separate from a gateway's
local tables. Store optimistic revisions plus transactional row ownership and
unique constraints; a Rust transition named `verified` is not proof of authority.
Do not let gateways write Tower tables or make arbitrary accomplishment claims.

Commands include account-bound session identity, run, expected revision and
idempotency identity. Persist the canonical request digest and committed response
with the mutation. Same key/same request returns that response; same key/different
request refuses. Validation includes ownership, phase, catalog, input instances,
resulting capacity and checked arithmetic. No client-supplied prices, result,
rarity, item definition, opponent, seed or executor filesystem path is accepted.

Authentication must reuse reviewed Central identity mechanisms with a distinct
Tower audience and purpose. A gateway admission token is not a Tower credential;
worker, gateway, agent and seat privileges are separate. Reuse reviewed trust and
revocation logic through an appropriate library rather than duplicating crypto.
Discovery must not permit arbitrary credential-bearing origins or callback URLs.
Adding Central scope/API support requires its own minimal reviewed change, not
moving the Tower economy into Central or extending a broad publisher privilege.

Run state: Shop → Reserved → Playing → Verifying → Shop/Conquered/Exhausted.
Abandonment is explicit; a technical recovery state cannot spend or earn. Freezing
the loadout, choosing the opponent and creating a unique outstanding reservation
are one transaction. A lease's increasing fencing generation binds the authorized
executor; expired workers cannot publish results into a newer claim. Gateway
switching between duels preserves the run. Mid-duel reconnection routes to the
existing reservation, not a second match.

Before a player can observe an accepted action's consequences, the execution
boundary must make the corresponding record prefix recoverable. Use ordered
durable append/chunks outside the synchronous rules engine, with bounded buffering
and backpressure; do not put a database write into each engine primitive. Define
the acknowledgement barrier and measure its latency. Recovery replays the same
acknowledged prefix, seeds and exact policy. It cannot restart from an earlier
decision after the player has seen new hidden information.

If a prefix or exact executor cannot be recovered, quarantine the reservation for
an explicit technical outcome. Never automatically restore an attractive earlier
position or issue a fresh opponent. A player's disconnect/concession does not
qualify as a service technical void. A trusted supervisor establishes failures;
the client cannot submit one to avoid a loss. A technical void earns no ranked
progress and triggers bounded operator-visible recovery, not unbounded retries.

The verification worker receives the authority's stored package and authenticated
record, then reconstructs rules, frozen house decisions, final result and spent
charges. The service atomically commits one outcome, economic ledger entries,
eligible snapshot status, next shop and accomplishment outbox. Conflicting records
for the same reservation are refused and investigated, not last-writer-wins.
Delivering an outbox message twice cannot duplicate a reward. Central being
temporarily unavailable leaves delivery pending; it cannot stop a completed duel
from being recorded or justify replaying it for another reward.

Replay proves a legal deterministic history, not that an operator kept hidden
cards secret or a human made the choices. Ranked hosting requires approved
execution operators with bounded workers and explicit trust; gateway registration
alone is insufficient. Untrusted hosts may access approved execution or run a
clearly separate unranked mode. Do not claim complete anti-cheat or bot detection.

Admit an echo only from an authorized legal pre-duel snapshot backed by a verified
completed game (win, loss, draw or concession). Publishing cannot depend on a win.
Use a bounded representative per account/stage/cohort; choose it independently
of strength or outcome. Match from an indexed eligible population with a defined
seeded sampling policy; taking the first 512 rows is not a uniform global sample.
Recheck eligibility when reserving and hide private metadata from projections.

Concrete indexing contract: maintain dense slots per cohort/duel ordinal, exactly
one row per eligible account, with a locked partition count and generation. Resolve
self/previous accounts to their current slots, sample a rank uniformly in the
remaining population and expand across those at most thirteen exclusions. Prefer
unseen accounts; only when none remain may a previous non-self account be chosen.
Read the one selected indexed slot and freeze it in the same transaction. Removal
swaps the last row into its hole and increments the generation; insertion and
representative replacement also increment it. Never reuse a selected slot across
generations or count expired/withdrawn rows as eligible. Bounded deterministic
rejection sampling avoids modulo bias without reading the entire global pool.

Enrollment explains historical build/name use. Withdrawal/deletion immediately
removes future eligibility and public attribution; resolve names through current
visibility instead of trusting an old copied display string. An in-flight private
package can complete under its reserved contract without republishing identity.
Initial retention proposal: 30 days of inactivity for resumable runs, 30 days for
new echo eligibility, 90 days for private adjudication records, with earlier
deletion where compatible with the explicit dispute policy. Show run expiry and
cohort retirement before enrollment; no loss of earned permanent cosmetics for
missing a day. Final operator/privacy documentation must approve these periods
before release. Remove or aggregate logs; never record credentials or hidden
card payloads in ordinary operational logs.

All request/record sizes, inventory, candidates, queues, pending leases, workers
and retries have explicit bounds. Archive incompatible old cohorts with their
versioned executors for the supported replay/retention period. Security retirement
may terminate a cohort explicitly; never silently upgrade an in-flight ruleset.

## 8. Native UX and durable motivation

Lobby: Daily, Weekly and Tower form a balanced, consistently sized entrance.
Tower shows current stage, record and one next action: continue shopping, return
to duel, reconnect, or wait for verification. Gateway diagnostics remain compact
and secondary. Daily/Weekly retain their own meanings and Central-backed data.
Normal table creation additionally exposes loop detection as an initially disabled
option; its behavior is explained when enabled. Tower has a fixed disabled policy.

Run overview: a readable stage track, next opponent status, remaining defeat
allowance, current build and saved state. Separate run counters from life/mana.
Show when a run is recoverable, when verification is pending and when switching
gateways is possible. Never offer a second start button for a reserved match.

Shop: stable offers in a centered grid, compact deck/build summary on one side
and equipped/reserve items on the other at desktop widths. Preserve hierarchy
when panels stack; inspect at 1280 and 1440 logical widths, enlarged text and
supported native scaling. Item details show effect, rarity, price, owned copies,
fusion progress and interaction/timing. Recommendations are optional explanations
of visible synergies, not promises of a winning strategy or opponent leaks.

Fusion: preview the exact consumed instances and before/after rules, affected
stats, freed slots and resulting placement. Common/Uncommon fusion animates three
items into one; Mythic animates one Rare plus one potion. Confirm against fresh
server state. A stale reply refreshes the projection and explains the conflict;
it never applies an old remembered offer index. No silent auto-fuse on purchase.

Table: echo owner, stage and explicit Expert AI label; equipment source strip;
separate potion charges; upcoming delayed effects where publicly knowable. Show
source identity and modified values from engine projections. A relic whose
battlefield source was destroyed remains owned but visibly inactive for this
duel; an icon must not imply a second active copy. Mana uses the existing color
choice and activation conventions. Other consumptions retain guarded activation.
Combo controls show the chosen finite repetition, progress, why execution stopped
and the next legal choice. Animation can group events without dropping their
rules/history or hiding an opponent response window.

Presentation: original illustrations/icons; rarity expressed by shape, label and
material as well as color. Common is restrained, Uncommon more detailed, Rare
ornate and Mythic distinctive, with restrained shader motion. Badges have their
own recognizable designs and visible tier progression, not recolored copies.
Reduced motion retains all information; no repeated full-screen celebration.
Respect existing asset/font licences and run the repository legal review before
adding player-visible assets. No Oaken/WotC art is copied into the repository.

German and English text must be complete. Store UTC timestamps; format them with
the user's locale/timezone and an absolute tooltip where a relative time is used.
Durations, gold and counts use their correct types. Keyboard focus, Escape,
confirmation, contrast and enlarged text are acceptance items, not later polish.
Keep interaction decisions in client-core and render projected state in Bevy.

The end-of-duel screen acknowledges one meaningful improvement, discovery or
milestone and offers the next saved step. The collection book shows attained and
next tiers, original item art and earned replay highlights. Notifications are
opt-in and infrequent. There is no streak loss, forced daily login or hidden
account power. A loss should explain what happened and leave a concrete pivot
when the run continues; it should not claim that the random shop guaranteed a win.

## 9. Validation and release acceptance

Planning passes are not the requested implementation/visual iterations. After a
playable slice exists, perform at least five observed refinement cycles:
layout/symmetry; choice clarity; feedback/pacing; accessibility/recovery; complete
human-played runs and opponent/combo understanding. Revisit any failed criterion;
five is the minimum, not permission to stop with a known broken flow.

Use cheap pure-domain fixtures first: fusion/ledger invariants, deterministic
shops and sampled acquisition paths. Start with a bounded 64 synthetic acquisition
paths across intended strategies, without playing engine games. These are not
proof of competitive balance or a replacement for a real shopper model.

Once mechanics stabilize, an initial duel sample is twelve games: three selected
contrasting matchups, two seeds and both seat orders. Record seed, build/policy,
rules, strategy and outcome. Add only samples needed by a changed effect or an
observed uncertainty; freeze candidate parameters before a separate small held-out
sample. Do not repeatedly rebuild/replay an unchanged broad suite. Small samples
diagnose failures; they do not establish precise win rates or universal fairness.

Human play must include an ordinary pivot after a loss, building and using a
Mythic, and executing/interacting with a combo. Use isolated synthetic accounts,
loopback services and owned process tracking; follow the client's dev-control
skill and close every agent-started process after the demo. Capture actual native
screens and timestamps, not only headless screenshots or fabricated mockups.

Observe affordability at each stage, dead offers, fusion completion, forced
discard/sale, purchase rate conditional on being offered, archetype/seat advantage,
duel length, viable pivots, combo assembly and disruption, AI mistakes, fallback
frequency and verification/reconnect failures. Manual clarity/fun observations
complement these metrics. Do not optimize retention by idle-online time.

Hard release criteria: no duplication/overspending or cross-account access;
deterministic legal results and consumption; same-stage matching; truthful fallback;
all new legal choices operable by every seat; recoverable acknowledged actions;
no silently truncated legal combo; no undocumented Standard regression; complete
native states/localization/accessibility; original/licensed assets; documented
retention and trusted-host policy. A missing primitive or consumer keeps the
affected content explicitly unavailable, never partially advertised as supported.

Run targeted unit/property/integration tests and strict checks for changed crates.
The owner's latest instruction, 14 November, explicitly limits local gates to
changed crates and directly affected contracts; do not repeat already-passed
workspace gates or wait for CI before continuing implementation. Apply that
scope in both repositories. Database, dependency and crypto checks are needed
when those surfaces change. Required remote branch protection still applies to
publication. The final PR review follows the implementation iterations and QA;
the planned Fable review remains an additional review target.

## 10. Implementation sequence and review handoff

The owner expressly permits careful larger refactorings for a clean, extensible,
maintainable and efficient architecture. Plan changes around responsibilities,
not a requirement to preserve the current file layout or Tower-specific seams.
Keep working rules semantics and the owner's new explicit defaults distinct from
structural compatibility. A broad rewrite is not a substitute for proving both.

Intended boundaries:

| Boundary | Responsibility |
| --- | --- |
| Shared rules kernel | Native Magic effects, choices, state transitions and replay |
| Compiled mode/loop adapters | Admitted setup/effect definitions and fixed detection capability |
| Shared AI plus mode strategy | Authorized observation, legal action proposals and deterministic planning |
| Lightweight execution contract | Variant/build capabilities shared by gateway, agent and executors without importing engine rules into the gateway |
| Tower domain and store | Runs, item instances, shop/fusion ledger, reservations and transactions |
| Tower service and workers | Authentication, routing, verification supervision and durable delivery |
| Client-core and rendering | Interaction state/projections versus presentation |

Use a crate boundary when it enforces a dependency/trust or reusable-library
boundary; do not create a separate crate for every small abstraction. Replace
scattered item-name checks and special-case fields with a small typed vocabulary.
Do not add a general plugin framework capable of rewriting priority or granting
raw state access. Avoid an extra event bus/interpreter, duplicated rules or a
second AI-only definition of an effect. Keep generated card sources in their
existing pipeline and retain the repository's corpus/codegen constraints.

Refactoring method: capture current contracts and meaningful reference fixtures,
extract/move modules in behavior-preserving commits, then add behavior in separate
commits. Preserve source/record identities or use explicit versioned conversion.
Temporary adapters must name a removal milestone; retire the old path once its
supported callers/records have a tested route. Measure allocation, throughput,
binary size and build cost where a structural change affects them. Prefer a
bounded working slice over carrying several unverified replacement subsystems.

Sequence, gated by the final bugfix handoff:

0. Wait for the exact final handoff trailer using the owner's temporary Rust
   watcher. Fresh-fetch and recheck the tip, merge it as explicitly authorized,
   clean up the handoff refs/watcher, then inspect its audit/fault entries and
   changed contracts. Record the exact merged commit and inherited regression
   fixtures. Check relevant overlaps; do not repeat already passed broad gates
   merely to poll readiness. A different commit on the branch is not completion.
1. Freeze contracts and planned adversarial fixtures. Make the necessary move-only
   extractions; build and verify the three static execution variants/agent contract
   against ordinary reference games before new Tower effects complicate the diff.
2. Native rule-pack slice and matching AI semantics: setup, one upkeep relic,
   one continuous effect and potions. Replace the narrow legacy supplies seam
   through the tested versioned route, with all seat consumers represented.
3. V2 economy and durable ownership: instance-based fusion, ledger and shops;
   global Tower reservations, verification workers, same-stage echoes and gateway
   switching. No service may treat a domain method name as result authentication.
4. Combo execution and selective verified shortcuts, with real opposing decisions,
   replay/resume, all seat consumers and numerical/resource limits.
5. Owned-pool shop/deckbuilder, rarity fusion, table equipment/potion UX, rankings
   and minimum idempotent Central accomplishment integration where required.
6. Complete supported catalog, targeted balance samples, manual play, at least
   five observed UX/refinement iterations, changed-surface gates and PR review.

Carbon's regression cases remain acceptance cases through these milestones.
If a refactor conflicts with a fix, understand the fixed invariant and preserve
it; resolving a textual merge conflict is not evidence that behavior survived.

Each working milestone gets its own reviewable commit and focused evidence.
Daily/Weekly remain Central-certified and keep their existing multi-solution
checks; neither their old proof graph nor the old expedition demo proves v2.

The review package must include the exact branch/head, PRs against main, changed
contracts, migrations/rollback behavior, targeted test commands and logs, local
demo recipes, visual evidence, balance measurements and candid known limitations.
The owner's later instruction authorizes the foundation merge; new work remains
on the root feature branch with a PR against main. If a Central change is needed,
use its own new codex branch and its stricter safety and dependency policy,
with the owner's changed-crate QA scope above.

See [the readiness review](tower-echoes-readiness.md) for passes, unresolved
validation and concrete completion criteria. This document authorizes no
production deployment or changes to real user data.
