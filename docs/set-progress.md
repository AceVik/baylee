# Set progress — 2026-10-04

## Scope and evidence

Use first-printing Oracle identities from the generated set index, then read
**every corresponding card file's coverage declaration**. Current print-set
headers are reprints and cannot inventory an original set. A passing cast-only
smoke test is not proof that a card's rules text works. `Implemented` is a
maintainer claim; behavioral tests and remaining Oracle clauses still need review.

The earlier feedback handoff's "seven Alpha Partials" tracked only seven cards
whose cast-only tests were backfilled, not the entire set. Its subsequent "one
remaining" claim was incorrect. This complete inventory supersedes those counts.

## Limited Edition Alpha

290 distinct Oracle identities in `set_lea.rs`: **271 Implemented, 15 Partial,
4 explicitly excluded** by the existing owner scope in `data/unplayable.tsv`.
Thus 271 of 286 in-scope cards are marked Implemented; **Alpha is not complete**.
Gloom, Cyclopean Tomb, Creature Bond, Consecrate Land, Animate Artifact,
Nether Shadow, Sunglasses of Urza, Sengir Vampire, Earthbind, Personal Incarnation,
Channel and Guardian Angel are
included in the Implemented count. Their dedicated
behavioral tests replace the earlier cast-only evidence; milestone validation
and native screenshots are in `docs/feedback-fixes-2026-10-01.md`.

Excluded cards: Chaos Orb (dexterity); Contract from Below, Darkpact and Demonic
Attorney (ante). These are explicit scope exclusions, never counted as implemented.

### Remaining cards

| Card | Remaining implementation or acceptance work |
| --- | --- |
| [Animate Dead](../crates/baylee-cards/src/cards/enchantments/auras/mv_2/animate_dead.rs) | Aura reanimation, restricted attachment and the leave sacrifice are implemented and regression-tested; final live acceptance is pending |
| [Camouflage](../crates/baylee-cards/src/cards/instants/mv_1/camouflage.rs) | defending players putting their creatures into piles assigned to attackers at random, instead of declaring blockers, is not in the engine |
| [Drain Power](../crates/baylee-cards/src/cards/sorceries/mv_2/drain_power.rs) | forced land activations and exact mana transfer are implemented; final live acceptance is pending |
| [Illusionary Mask](../crates/baylee-cards/src/cards/artifacts/mv_2/illusionary_mask.rs) | spent-mana casting and face-up replacement events are implemented; final selector and live acceptance are pending |
| [Kudzu](../crates/baylee-cards/src/cards/enchantments/auras/mv_3/kudzu.rs) | tap-triggered destruction and controller-selected Aura relocation are implemented and regression-tested; final live acceptance is pending |
| [Lich](../crates/baylee-cards/src/cards/enchantments/mv_4/lich.rs) | zero-life protection, life-gain draws, damage sacrifices and leave-game loss are implemented and regression-tested; final live acceptance is pending |
| [Magical Hack](../crates/baylee-cards/src/cards/instants/mv_1/magical_hack.rs) | semantic text changes are implemented and independently tested; final batch acceptance is pending |
| [Sleight of Mind](../crates/baylee-cards/src/cards/instants/mv_1/sleight_of_mind.rs) | semantic text changes are implemented and independently tested; final batch acceptance is pending |
| [Time Vault](../crates/baylee-cards/src/cards/artifacts/mv_2/time_vault.rs) | turn-skip untapping and the extra-turn ability are implemented and regression-tested; Client/AI/Seat consumers are integrated, final live acceptance is pending |
| [Vesuvan Doppelganger](../crates/baylee-cards/src/cards/creatures/mv_5/vesuvan_doppelganger.rs) | copiable upkeep behavior and the full Oracle stack-dialog fallback are implemented and regression-tested; final live acceptance is pending |

### Interrupted-session takeover — 2026-10-04

The owner requested complete Alpha acceptance, not only persistence of the
interrupted milestone. Engine implementation and review are reserved for Astra
at medium or higher, or Claude Opus 5.5 at high or higher. Client integration,
live acceptance and the final Git milestone remain separate responsibilities.

The inherited intrinsic-mana migration had a missing client match arm, old
codegen expectations, unmigrated handwritten duals and verification readers.
Those integration gaps were repaired, with explicit client tests for changed
land colours and missing/wrong-index host projections. The all-card ability
sentence gate also exposed six library-moving mana abilities; its reader now
agrees with the current non-mana classification, and the generated line table
was regenerated normally. The 30 sentence-reader tests and the complete native
sentence-assignment test pass. A subsequent full Workspace run was stopped by
the 300-second execution limit during Gamehost tests; it is not a completed
Workspace gate. Logs are in `scratchpad/junie-alpha-*.log`.

No card is promoted solely on these integration checks. The accepted Alpha
count above remains unchanged until the remaining rules and live checks pass.

#### Native Engine checkpoints and Client integration

The owner's native `engine` session confirms Claude Opus 5.5/high and owns rules
implementation. Shared handoff/status/inbox files under `scratchpad/` coordinate
explicit source and Cargo pauses; there are no further automated continuations
of another session. Milestone 1 validates inherited Time Vault, Aura-binding and
constrained-cast work and completes Animate Dead. Milestone 2 completes Lich.
Each checkpoint regenerates the tables through two regular xtask passes.

Time Vault's new yes/no is wired through Client Core, AI and Seat: the question
names the whole-turn cost, cannot be automated, is answerable only by the correct
seat, and the narrator binds a reply to its question. The conservative house AI
declines this turn trade without declining free optional effects.

Live Sleight acceptance exposed a mixed-target browser that covered a legal
battlefield answer while displaying only its off-board alternatives. Its
reproducer failed before the fix. The browser now uses its existing obscured-
battlefield path for `ChooseTargets` as well as `ChooseCards`; mandatory/private
sheet ownership remains unchanged. Tests require exactly the offered rows,
forbid duplicate rows and check reset to an off-board-only question. All 54
browser tests pass.

Checkpoint 2 gates pass for Cards (130 tests), Client Core (1210), Client (1194
unit tests plus integration targets), AI (220), Seat, Codegen and all Gamehost
targets (209 active unit tests plus integration targets). Existing ignored tests
are unchanged; none were added or bypassed. Workspace Clippy with `-D warnings`,
the formatting check and the `dev-control` Client build pass. The first combined
240-second run spent 130 seconds compiling and ended before two Gamehost tests;
the subsequent complete isolated Gamehost run passes in 70.20 seconds of unit
test execution. The earlier full Workspace timeout is still not a complete
Workspace gate; the final current-source gate remains due.

Milestone-1 live evidence confirms Mask's zero/two-blue receipts and actual
face-down cast, Doppelganger's full upkeep Oracle and second blue copy, Hack's
actual Island-to-Mountain red production and Drain Power's exact mixed-colour
transfer including an already-tapped land counterexample. These observations
do not certify later source changes; see
`scratchpad/junie-alpha-milestone1-live.md` for the bounded evidence.

Checkpoint 3 wires Island Sanctuary's own draw-step draw and attack restriction,
but its additional draw-step draws remain a known completion blocker; coverage
stays Partial. Its two table-generation passes and Cards/Client Core/AI/Seat
gates pass. The coordinator added read-only browser row/control coordinates and
panel state to the feature-gated loopback diagnostic, with all 19 diagnostic
tests, feature-enabled Clippy, format check and the Client build passing. This
allows actual mixed-target selection without guessed screenshot coordinates.
Time Vault's milestone-2 live No path is confirmed: the next own turn occurs and
the Vault remains tapped; its Yes/live extra-turn path is still pending.

## Damage rules extension

The original fixed prevention order is replaced by resumable damage work shared
by combat, spells and abilities. Affected players now choose the next applicable
replacement/prevention effect under CR 616.1 and allocate a limited shield across
simultaneous sources under CR 615.7. Pending decisions have stable identities;
invalid and stale answers fail atomically. Life gain and counter removal are
committed with the completed damage event, rather than exposed during an open
choice. Replay, concession, LKI and fixed-width fingerprints are tested.

Power Leak is now Implemented after independent Oracle review and real-card
regressions. Its former ignored ordering test is active and passes both outcomes.
The full rules gate passes 7,913 tests with ten existing skips; native client
passes 1,230 tests with two existing ignores. Client-Core, AI, Seat and Train
checks, Clippy, Wasm release check and validation of all 2,955 cards pass.
The final live allocation check passes for both 1+2 and 0+3 across two sources,
with different lifelink outcomes and correct draft reset at a new choice ID. Individual
coverage flags still do not establish complete Alpha acceptance while the
separate self-effect and remaining-card acceptance work remains.

### Exact source selection — accepted

Source decisions now carry `DamageSourceRef` (object plus zone-change version)
and a stable `SourceChoiceId`. Current and earlier incarnations are distinct;
eligibility comes from live stack, shield, replacement and delayed references,
or the public permanent/spell/command-zone rules, not from every saved snapshot.
Copies, retargeted reference slots, paid sacrifices, linked exile cards and
actual permanent-spell resolution preserve the relevant exact identity.
Historical projections keep entitled card/rules identities separate from the
current object and feed the view's print/catalog iterators. Death triggers preserve
the battlefield source separately from the new-zone event object; only actual
effect references make that event object eligible. Granted triggers, simultaneous
deaths, repeated entries and copies have dedicated regressions. View49/Protocol16
describe the new contract.

Thirteen independent real-card scenarios cover the five Circles, Forcefield,
Reverse Damage, Jade Monolith, Sacrifice, Endless Sands and Omnath, including three
incarnations with distinct outcomes. Additional tests cover delayed cleanup,
source-choice concession/replay, ceased tokens, privacy, copy/retarget context
and archived divided damage. The final broad gate, after the death-trigger and
printed-symbol fixes, passes **7,954 tests**, with ten existing skips; Clippy and
all 2,955 card metadata validations pass. Engine fuzz Clippy and both footprint
checks pass; GameObject remains 312 bytes.
Logs: `/private/tmp/baylee-source-rules-final-gate.log` and
`/private/tmp/baylee-source-wasm-final-check.log`. Final native tests pass
1,235 cases with two existing ignores; native dev-control Clippy/build and Wasm
release checks also pass. Live screenshot acceptance passes at 960 logical
pixels: current source yields 34/38 life, historical source 37/38, with correct
keyboard selection and reset on a new decision. Root inspected the final source,
Abilities, reset and result screenshots; evidence is in the feedback log.

### Exact target incarnations — accepted (`d95f741c`); self-effects rules-tested; live pending

The self-effect incarnation correction is rules-tested. Shivan Dragon, Frozen
Shade, Granite Gargoyle and Dragon Whelp bind pumps to the correct incarnation;
self-prevention, regeneration, sacrifice and untap follow the same contract.
Per-turn ability counts distinguish incarnations. Actual public moves caused by
costs or the resolving effect retain explicit successor provenance, including
nested/suspended instructions; damage attribution retains its original source.
Enduring Vitality and Rancor exercise permitted public-zone followups. The broad
gate caught Rancor's own-departure exception; the correction and two independent
positive/negative real-card regressions now pass.

Final Rules gate: **7,990 passed, ten existing skipped**, Clippy and all 2,955
metadata validations green (`/private/tmp/baylee-self-rules-gate.log`). Focused
checks include all ten independent self-effect card functions, Rancor's normal
and reentered-graveyard cases, shared counters, source choices, hash state and
footprint. GameObject remains 312 bytes; no generated, card-status or wire change.
The compiled-pool census found no resolving composite-`This` effect; existing
composite uses are static/condition/trigger filters, not additional acceptance
claims. Native live Shivan/Whelp verification passes: old pumps do not affect the
returned creature, a fresh pump works, and four fresh Whelp activations cause
the end-step sacrifice. Rancor returns to hand during the live blink scenario. The owner approved the
three proposed target-selection QoL changes and the finite-redirection display;
those changes remain uncommitted until tests and live acceptance. Any additional
UI change requires a new proposal and explicit confirmation. The Alpha count is unchanged.

Issue #117's Engine implementation now captures exact target incarnations before
announcement costs, checks both groups at resolution without substituting a
current incarnation for a missing reference, and narrows a resolution copy while
preserving the original announcement. Retargeting, duplicates, Ward events and
fixed divided-damage shares distinguish old and current versions at the same
arena id. Action replay and refusal preserve these bindings.

View50/Protocol17 carry exact `TargetRef` and separate historical target
projections; original names and privacy do not come from a newer card. The
independent seven real-card tests pass, including Bolt/Ephemerate, partially
legal Fireball, both Khalni Ambush groups, Fork keeping versus changing an old
target, atomic swaps, and Fury/Vantress Visions with distinct fixed shares.
Twelve focused Engine tests, targeted retarget/Ward/trigger tests, View shape,
Gamehost projection, actual footprint tests and Engine/fuzz Clippy pass.
GameObject remains 312 bytes. Consumer/native compilation passes, as do 1,239 native tests and 32 focused
consumer tests. The final broad Rules gate passes **7,972 tests**, ten existing
skips, Clippy and all 2,955 metadata validations
(`/private/tmp/baylee-target-rules-gate.log`). Live review found remaining text
wrapping and retarget-context layout defects; fixes, focused native regressions,
Clippy and rebuild now pass, as does the final Wasm release check. Both live
Keep/Change outcomes pass. Root accepted the final wrapped browser and compact
retarget screenshots: no clipped Oracle text, stale casting context or footer
collision. Keeping the old target leaves the returned Bears alive; choosing the
new incarnation destroys it. Both finish with an empty stack, life 40/40 and no
client error. The preceding source-selection milestone is `c6422ff1`.

The multiplayer departure code also has an existing general substitute-chooser
limitation under current CR 800.4g. New source-choice concession tests establish
fresh questions for a surviving chooser and continuation when no source remains;
they do not establish general replacement of a departed non-cost decision maker.

### Personal Incarnation — accepted

Owner-only activation, finite next-one redirection and rounded-up owner life loss
are implemented. Ten independent card tests include the real six-mana cast,
foreign control, repeated shields, blink/expiry, source-owner LKI, negative life,
resolution-time life and Swords' controller life gain. Generic finite redirection
also covers unpreventable damage and affected-player choices. View51/Protocol18
carry the new finite-redirection description.

Rules suite: 8,006 passed initially; its sole failure was the obsolete PI-only
no-ability placeholder. That test was replaced by a real cast/body/owner-ability
test and passed in a targeted rerun, with no production change. Thus all 8,007
cases are verified; ten existing skips remain. Native tests: 1,264 passed, two
existing ignores. Clippy, Wasm release check, two-pass table codegen/check and all
2,955 metadata validations pass. Logs: `/private/tmp/baylee-personal-rules-gate.log`,
`/private/tmp/baylee-personal-wasm-metadata.log` and
`/private/tmp/baylee-pi-qol-native-tests-final.log`.

Live: two combat sources deal two each; one point redirects to the owner and
Healing Salve prevents the other three (owner 40→39, creature damage0). After
real Control Magic, owner activation still works; Bolt leaves damage2 and owner39.
Terror then puts the creature in its owner's graveyard and changes owner39→19,
while the controller stays40. The abilities dialog remains.

Personal Incarnation is now Implemented after the owner's display-fix approval
and final live inspection. The abilities dialog retains the complete unchanged
Oracle text and {0} Manafont symbol; the generic confirmation button says
“Fähigkeit aktivieren”, and the redirection label uses an arrow before the player.
No Scryfall/Oracle content file or per-card shortened text was introduced.
Additional checks: 55 Cardtext, one Core and 286 native HUD tests, native Clippy
and build, Wasm release check, all ten final card cases, metadata validation
and table-codegen reproducibility pass. Final PNG/JSON evidence is archived
outside the repository in `personal-qol-2026-10-03` under the task's visualization
archive, including `alpha-pi-final-ability` and `alpha-pi-final-effect-choice`.

## Following sets

Beta adds Circle of Protection: Black and Volcanic Island; both are currently
marked Implemented. Reprints inherit their original Oracle implementation.
Arabian Nights remains later work. Do not promote either set as fully verified
or start later card batches while Alpha still has the open clauses above.

Regenerate this inventory from the complete first-printing index after each
batch. Count exclusions separately and preserve any unsupported clause until a
behavioral test exercises its completed implementation.

## Completion pass requested 2026-10-02

The owner requested completion of all 25 remaining cards, with Astra xhigh
implementing engine changes and Sol medium independently reviewing Oracle
clauses and writing card behavior tests. The first batch is Fork, Power Sink,
Clockwork Beast and Drain Life; a batch is accepted only after implementation,
independent behavior tests and the relevant integration gates pass.

Fresh Oracle payloads and card-specific rulings for all 25 were fetched from
Scryfall on 2026-10-02 to the untracked local evidence directory
`/private/tmp/baylee-alpha-oracle-2026-10-02` (one pair of JSON files per Oracle
identity). This is source evidence, not test coverage or a completion claim.

Completion checks include all clauses, legal choices and refusals, responses,
timing, costs, target changes, incarnation isolation, and relevant replacement
and continuous effects. New questions and payment restrictions must also reach
the wire view and playable client; engine-only success is insufficient.

The owner reaffirmed that all engine extensions must be authored by Astra xhigh
and include tests; Astra medium/high may author those tests. Sol medium remains
the independent Oracle/card-behavior reviewer for this pass.

### First batch: Fork, Power Sink, Clockwork Beast, Drain Life

Astra xhigh authored the Engine changes, Sol medium authored independent card
scenarios and the final rules review, and Astra high completed client acceptance.
The first four declarations are Implemented and their acceptance checks pass.
Milestone commit: `b81591ec`. Alpha as a whole remains incomplete.

| Card | Behavior exercised independently |
| --- | --- |
| Fork | Keep/change player targets; target the resolving Fork with a copied Counterspell; choose both fight target groups; swap targets without changing cardinality; preserve sacrificed-creature information used by the copied spell. Existing card tests assert the red copy, legal spell types and independent original/copy resolution. Generic regressions cover fixed damage shares and retaining an unchanged illegal target. |
| Power Sink | Pay or decline; preserve spare lands/mana when paid; tap only mana-producing lands and empty all unspent mana when unpaid; perform the penalty even for an uncounterable spell; respect abilities granted to or removed from lands. |
| Clockwork Beast | Seven entering counters; attack/block history until combat end; nonparticipants and later combats; loss despite Fog; old trigger does not affect a new incarnation; own-upkeep-only recharge; choose zero or fewer than X; actual X payment; seven-counter cap without removing excess counters from other effects; numeric-prompt roundtrip and invalid-answer rejection. |
| Drain Life | Black mana reserved for X while other colors pay the fixed generic cost; life gain uses damage actually dealt; full creature toughness despite earlier damage; player life and planeswalker loyalty before damage; prevention and illegal-target resolution. Generic payment tests separately exercise actual mana versus spend-as-color permissions and reductions. |


Drain Life additionally has a real Rock Hydra regression: X=4 removes three
counters during prevention, deals one damage, and gains zero life because the
creature's current toughness is then zero. The old timing fails both the card
and generic regressions. Player life and planeswalker loyalty are captured
before damage; creature toughness is read afterward, with both caps applying to
a creature/planeswalker hybrid.

The former X=50 ceiling is removed. Immediate payments derive resource bounds
and search affordability logarithmically; deferred mana-ability windows allow
announced X before mana is generated and validate the final payment
transactionally. The full suite exposed and corrected the Miracle distinction.
A synthetic cast chooses X=61, then sacrifices a mana source for 63 black mana,
pays and resolves. Free casts retain X=0. Actual paid damage at X=32,767,
32,768, 65,534 and 393,208 emits one full damage event and exact life gain.
Damage events/logs use u32; sparse event amounts preserve the 312-byte
GameObject footprint. The view version is 46 and protocol version is 13.

Client acceptance covers complete ability sentences/costs, optional capped
counter choices, named and numbered retarget choices, and Mana-font symbols in
abilities, stack, preview rules, selection, payments, errors and confirmations.
Plain prompt/error and response-button paths were corrected. A screenshot
exposed a tap glyph wrapping below its activation button; the final rebuilt
client keeps it within a single line. Direct prose-button clicks still work.

Live checks at 1280×800 verify Clockwork's ability and refill, Drain Life's
black-only X and actual damage/gain, a four-green-mana payment prompt, and Fork
with two original targets. Keeping target 1 and changing target 2 produces
original `[1,2]` and red copy `[1,3]`, without an error. Root independently
viewed the final screenshots:

- `/private/tmp/alpha-clockwork-ability-final.png`
- `/private/tmp/alpha-fork-keep-target-final.png`
- `/private/tmp/alpha-fork-change-target-final.png`
- `/private/tmp/alpha-payment-mana-symbols.png`
- `/private/tmp/alpha-clockwork-counter-question.png`
- `/private/tmp/alpha-drain-life-black-only-x.png`
- `/private/tmp/alpha-power-sink-decline-question.png`

Validation already passed: 27 independent card tests, 13 generic damage tests,
large-X and deferred-payment regressions, Engine/View all-target Clippy, Engine
fuzz Clippy, 1,157 native client tests (2 existing ignored), native client Clippy,
and the final dev-control build. Full corpus-backed codegen check is reproducible;
validate accepts all 2,955 cards (oldest cached payload 12 days, with the fresh
25-card Oracle/rulings evidence above). Earlier broad failures were corrected:
legacy X-range assumptions retained their rollback assertions, the deferred
Miracle payment regression was fixed, and View46's schema fingerprint recorded.
The final broad no-fail-fast rules gate passes: **7,844 tests passed, 10 existing
skipped**, Clippy clean, and all 2,955 cards validated. Log:
`/private/tmp/baylee-alpha-rules-gate.log`.

### Second batch acceptance

Fireball, Demonic Hordes and Power Leak now have 24 active independent tests
(9/8/7 respectively), plus one explicitly ignored known-failure regression for
Power Leak's prevention-order choice. Together with the prior 27 tests, all 51
active independent tests pass. The 256-target Fireball scenario pays all 256
mana, and late surcharge payment/refusal, controller changes and source loss
are exercised. Astra xhigh implemented the engine changes; Astra high completed the payment/choice
UI and AI/seat consumers. Sol medium independently reviewed the cards and
identified the shared prevention-order gap. Fireball and Demonic Hordes passed
final independent review and live acceptance; Power Leak remains Partial.

Wasm release check passed for the client and its shared crates. Power Sink's
live decline prompt renders the generic-two mana glyph correctly; declining
counters the spell, taps the three remaining mana-producing lands, leaves the
creature untapped, and reports no error. The observed empty pool is not treated
as isolated proof of the drain clause because the client auto-advanced the
phase; independent card tests verify that clause without a phase transition.

Power Sink's paid branch also passes live: the payment window shows the Mana
font cost, a Forest supplies the missing mana, payment consumes exactly two,
the remaining two Forests and Sol Ring stay untapped, and Lightning Bolt resolves
for three damage while the main phase remains unchanged. Screenshot:
`/private/tmp/alpha-power-sink-payment-window.png`. Both branches report no error.
A next-batch wording improvement is recorded: the payment instruction currently
says to tap lands, although artifact and other mana abilities also work; the
new payment UI should describe activating mana abilities generally.

Batch B interface contract: fixed payments and optional arbitrary payments have
separate presentation states. Power Leak opens a mana-ability window before the
amount choice, permits zero and overpaying beyond two, and confines its
prevention to that one damage instruction. Hordes' opponent choice names the
player who will sacrifice and retains the upkeep trigger's controller even if
the permanent changes control. Fireball's surcharge is per extra target, while
resolution division counts only remaining legal targets. Protocol14 guards the
new payment/choice data; View47 records the new schema.

The same review found that Fireball's "any number" inherited a u8 target-choice
limit of 255. The compact DSL retains an explicit any-number sentinel, while
materialized target bounds and wire choices are widened to u32 and capped by
the actual legal options. The independent real-card regression with 256 targets now passes, including
actual payment. This also removes the old X-target clamp; the client consumer
changes also passed their final live acceptance.


Second-batch final validation: **7,881 rules tests passed, 11 skipped** (the
additional skip is the explicitly known Power Leak ordering regression),
workspace rules Clippy clean, 2,955 cards validated. Native client acceptance:
1,226 tests passed, 2 existing ignored; client-core 1,188, AI 207, narrated seat
147. Subsequent symbol/name regressions and 48 cardtext tests also passed,
with all-target Clippy and a rebuilt native client. Wasm release check passed
again after the symbol repair. Logs: `/private/tmp/baylee-alpha-batch-b-rules-gate.log`
and `/private/tmp/baylee-alpha-batch-b-wasm-check.log`.

Root independently viewed the final Hordes upkeep/preview/opponent-choice
screenshots and Power Leak's optional window/overpayment screenshots. Power
Leak paid three mana from Sol Ring plus Forest, reduced the pool from three to
zero while remaining in upkeep, and lost no life; on the next own upkeep,
payment zero caused exactly two damage. This ordinary live flow does not
resolve the separate prevention-order blocker. Final client findings and
screenshot paths are recorded in `docs/feedback-fixes-2026-10-01.md`.


Second-batch working milestone: `34f5bb65` (2026-10-03). Fresh inventory of
all 290 Alpha identities confirms 267 Implemented, 19 Partial and 4 excluded.
The next active work is the shared resumable damage procedure described above;
Astra xhigh owns its engine implementation and generic tests, with independent
Sol medium review and Astra high client integration once the wire contract is
ready. No additional card is claimed complete merely because this work started.


Independent next-phase acceptance plan (Sol medium): Power Leak paid one plus
Reverse Damage must allow life 21 or 22 according to the chosen order; with
Healing Salve, a later Bolt must distinguish remaining shield capacity. Real
Jade Monolith redirection must move the chooser with the recipient. A flying
Two-Headed Giant blocking Baleful Strix and Extraction Specialist gives a
behavioral simultaneous-allocation test: allocating protection to deathtouch
versus lifelink changes survival and gained life. Three-player Fireball with
multiple applicable shields tests APNAP choices starting from a nonzero active
player. Invalid, repeated and stale responses must preserve pending state,
fingerprint and journal atomically. These are planned scenarios, not passing
coverage yet.


Damage integration status (uncommitted, not accepted): the initial Engine
compile and six generic damage tests pass. The public contract uses
`ChooseDamageEffect` and `AllocatePrevention`, with stable batch/step choice
identities and explicit preventability. Native/core/AI/seat/train consumers and
regression tests are prepared but not yet compiled. Independent real-card tests
are being authored; source-incarnation LKI and legacy overlapping-shield tests
remain part of the ongoing audit. Protocol15 is reserved for this contract.


The subsequent full Engine run passes 4,562 tests with two pre-existing ignored
cases. The former ignored Power Leak ordering regression is active and passes
both legal results, along with six independent real-card scenarios. Review
caught and fixed premature life gain/counter removal during pending damage
choices. View48/Protocol15 describe the contract. This is not final acceptance:
consumer compilation/live checks and concession/replay continuation regressions
are still outstanding at this checkpoint.


Read-only next-step source-choice design: introduce an exact source reference
(object plus zone-change version), a separate stable source-choice ID, and a
one-source Pending/Action contract. Enumerate only sources allowed by current
CR 609.7a, using live stack/effect/delayed references; deduplicate exact versions.
Do not offer every archived graveyard object. Retain source snapshots while
referenced instead of clearing them at cleanup. Delayed triggers and captured
stack references (including cast/copy/retarget) need version propagation without
increasing GameObject's 312-byte footprint.

The view must project historical source labels and permitted identity separately
from ordinary ObjectId-based `looking_at`: current permanent versus earlier
incarnation with an ability on the stack, including tokens and hidden identity.
Consumers need explicit source wording, stale-answer rejection, legal AI/seat
fallback and training guards. Acceptance must cover all Circle colors, Reverse
Damage, Jade Monolith and Forcefield, multiple old incarnations, source references
across cleanup, projection privacy and live same-name rows. Merely making the
Artillery example selectable will not close the full source-selection blocker.
This remains a design, with no implementation started before the damage milestone.


Final damage rules gate: **7,913 passed, ten existing skipped**, Clippy and
validation of all 2,955 cards green. Wasm release check and full corpus-backed
codegen reproducibility check also pass. Four additional continuation tests
cover replay/stale responses and concessions; the central life-change guard
preserves departed players' CR 800.4i last-known life and emits no new life
changes for them. Power Leak's independent final review recommends Implemented;
the fresh 290-card inventory is now 268 Implemented, 18 Partial, four excluded.
Logs: `/private/tmp/baylee-damage-rules-gate.log`,
`/private/tmp/baylee-damage-wasm-check.log`,
`/private/tmp/baylee-damage-codegen-check.log`.


Damage client acceptance is complete on the final rebuilt binary: both Reverse
Damage orders and both finite-shield allocations pass live, with no errors.
Root reviewed final screenshots; detailed evidence is in the feedback log.
Source-incarnation selection was subsequently accepted as `c6422ff1`; exact
target incarnations were accepted as `d95f741c` (see the current sections above).

Damage milestone committed as `81409297`. The following source and target
milestones are accepted; Protocol17/View50 are the current contract. Astra xhigh
now owns the self-effect incarnation follow-up, with independent Sol medium
card tests and Astra high client checks. No additional card has been marked
Implemented by these shared-engine milestones.

### Remaining-card acceptance queue

The following is an implementation queue, not additional coverage. It was
checked against the fresh October 2 Oracle payloads. Each batch still needs an
independent behavioral review and any new interaction must be exercised in the
client before its cards count as complete.

| Shared work | Cards | Essential acceptance beyond the ordinary successful case |
| --- | --- | --- |
| Owner-only activation and finite redirection | Personal Incarnation | The owner can activate after control changes; the controller cannot if not the owner. Redirect only the next one damage per shield, preserve source identity, and calculate the owner's rounded-up half-life at death-trigger resolution. Fresh rulings confirm negative life totals stay unchanged and exile alone does not trigger life loss. |
| Reusable player permissions and payment windows | Channel, Guardian Angel | Channel works in mana-ability windows including payment; Guardian Angel uses instant timing. Preserve duration, legal payment, target incarnation and repeated use, including after the original spell leaves the stack. |
| Mana activation during resolution | Drain Power | Each land's controller chooses a legal mana ability where possible; tapped lands and abilities with extra costs are handled correctly. Transfer all unspent mana after the activations. |
| Attachment and linked-object transitions | Animate Dead, Kudzu | Graveyard enchant legality, intervening condition, return/reattach and delayed sacrifice for Animate Dead; destruction failure, land-controller choice, non-target attachment and source loss for Kudzu. |
| Copiable exceptions and recurring copy trigger | Vesuvan Doppelganger | Retain color through successive copies, carry the granted upkeep ability into copiable values, allow declining, check targets again, and handle a second copier copying the Doppelganger. |
| Draw/discard/turn replacement choices | Island Sanctuary, Library of Leng, Time Vault | Distinguish draw step from other draws, effect-caused discard from costs, and a skipped turn from an untap step. Test multiple eligible events, optional refusal, duration and interaction with other replacements. |
| Combat reassignment and pile choices | False Orders, Raging River, Camouflage | Multiplayer defending seats, legal blocking restrictions, formerly blocked status, extra-block capacity, empty piles and deterministic seeded random assignment; expose choices clearly in the client. |
| Life and damage replacement/trigger composition | Lich | Entry life loss, nonpositive-life loss exception, replacement draws, nontoken sacrifices after damage, insufficient permanents and the graveyard trigger all have distinct behavior. |
| Layer-three text changes | Magical Hack, Sleight of Mind | Change eligible rules words, including relevant keyword text; preserve names and reminder text, spell-to-permanent continuity and indefinite duration. Verify both rules behavior and displayed ability text. |
| Face-down casting and event replacement | Illusionary Mask | Track the actual colors/types of mana spent on X, legal hidden card selection, sorcery timing, and face-up replacement before assigning/dealing/receiving damage or tapping, without leaking hidden identity. |
| Controlling another player while playing/resolving | Word of Command | Private hand selection, legal land or spell play, constrained land mana abilities and spending, inability to play, and control during the chosen spell's later resolution. |


## Remaining Alpha completion batches — owner instruction 2026-10-03

The owner requested completion of all remaining cards as one continuing task,
with independent batches worked concurrently instead of a one-card queue.
Channel/Guardian passed native tests and live gameplay; their final compact
payment layout and historical-recipient display were visually accepted.
Do not promote a card merely because its shared primitive exists.

| Shared work | Cards | Required acceptance focus |
| --- | --- | --- |
| Constrained mana and nested playing | Drain Power, Illusionary Mask, Word of Command | Per-land mana decisions and pool transfer; actual spent mana quantities/types, private face-down cast and all four reveal replacements; decision actor versus resource owner, entitled hand visibility, constrained land mana and controlled resolution. |
| Resumable event replacement | Island Sanctuary, Library of Leng, Time Vault, Lich | Draw-step-only optional replacement and attack restriction duration; effect discard versus costs and hidden deck order; whole-turn skip including extra turns; all Lich clauses, non-token simultaneous sacrifices, insufficient sacrifice and independent loss conditions. |
| Zone-aware Aura attachment | Animate Dead, Kudzu | Graveyard enchantment then exact returned incarnation, failed/protected attachment and linked sacrifice; tapped-land destruction and optional non-targeting relocation chosen by land controller. |
| Combat partition/reassignment | Camouflage, Raging River, False Orders | Seeded pile assignment, empty piles/block capacity/multiplayer; side restrictions and later arrivals; legal timing, reblocking and blocked/unblocked status with actual combat damage. |
| Semantic text and copy provenance | Magical Hack, Sleight of Mind, Vesuvan Doppelganger | Real land/color-word changes across relevant rules and type line without changing names/mana symbols; exact incarnation and permanent-spell continuity; recurring copyable upkeep ability, retained color/damage and no synthetic ETB. |

Engine work uses Astra xhigh; independent card/test review uses Sol medium.
Central state/DSL/resolution files have one owner; a second Engine worker owns
the disjoint text/copy modules and supplies integration hooks. Cargo runs remain
serialized while implementation, independent review and client inspection may
proceed concurrently. Existing full Oracle text and abilities dialogs remain.
After the owner requested a detailed description, five UI additions were
explained: word replacement, creature groups, mandatory land-mana choices,
controlled-player context and private Mask selection/payment details. The owner
explicitly approved these with “Go!”. Client implementation may proceed within
that scope; full Oracle text, existing abilities dialogs and explicit choices
remain required. Further visual or interaction changes require a fresh proposal.


### Channel / Guardian Angel acceptance

Both cards are Implemented following 12 independent card cases, shared payment,
AI and view regressions, the broad rules gate plus corrected legacy-preferences
regression, and 1,267 native tests (two existing ignores). Wasm, metadata for
2,955 cards, reproducible codegen, Clippy, fuzz checks and footprint passed.
Live Fireball X=1 paid exactly one Channel life (40→39), then dealt one damage;
no payment occurred merely by opening/selecting. Two paid Guardian actions
prevented two of Bolt's three damage, leaving Bears alive with one damage.
After a blink, the old permission did not protect the returned Bears from Bolt.
All captured outcomes had empty stacks and no client error. Root inspected the
full Oracle, Manafont symbols, hourglass, historical recipient and corrected
960-pixel payment layout. Evidence is linked in the feedback log.

The general existing per-color u16 mana-pool ceiling remains: overflow refuses
atomically before charging life. This is not a claim of unlimited numeric mana.

## Arabian Nights — owner-ordered pass without engine changes (2026-10-05)

The owner directed the next set in release order to be implemented fully with
**no engine changes**: a card whose clause needs engine or DSL work is marked
`Coverage::Partial` and the missing piece is recorded in `TODO.md`
("Arabian Nights — engine gaps"). Shahrazad (subgame) and Jeweled Bird (ante)
joined `data/unplayable.tsv` and are never built.

State of the set's 77 Oracle identities: **41 Implemented, 34 Partial, 2
excluded**. The transcoder wrote 10 (Dandân, Fishliver Oil, Hasran Ogress,
Hurr Jackal, Junún Efreet, Khabál Ghoul, Kird Ape, Repentant Blacksmith,
Sandstorm, War Elephant); 11 were hand-implemented (Aladdin, Army of Allah,
Brass Man, Ebony Horse, Erg Raiders, Island Fish Jasconius, Metamorphosis,
Piety, Rukh Egg, Sorceress Queen, Unstable Mutation); the 34 Partial cards
each carry their exact reason in the file and are listed in `TODO.md`.
Two cards already in the pool were among them (Diamond Valley, Island of
Wak-Wak) and Desert kept its pre-existing Partial.

Gates for this pass: `codegen --check` green, `xtask validate` green (3,007
cards conform), the engine library suite green (4,780 passed, 2 ignored), the
workspace suite green except `baylee-catalog`'s PostgreSQL-backed search tests
(no `DATABASE_URL` on this machine), clippy `-D warnings` and `cargo fmt`
clean. One tool finding — `claim_tests` reading the auto-transcribed Sandstorm
as an unconditional promise — was fixed with one vocabulary word (`" attacking
"` in `CONDITIONS`); no rules behaviour changed.

A follow-up pass then wrote **35 rules-derived tests** for the 21
`Implemented` cards (from the Scryfall payloads and rulings saved under
`scratchpad/arn-tests/`, never from the implementation): every test names its
card through `card_index(...)`, every loggable ability fires in the suite,
the 17 permanents leave the battlefield clean under `BAYLEE_LEAVE_LOG`, and
every listed ability mutant was run individually and killed. The engine suite
is green at 4,815 passed. The cards therefore stand at L3 with the firing and
leave halves of L4 green; the formal L4/L5 stamp still needs an
`--coverage` (`cargo llvm-cov`) export, which is not installed on this
machine (owner's call). The 31 `Partial` cards have no tests.

## Antiquities — same protocol, next set (2026-10-05)

Owner-ordered continuation with no engine changes, same as Arabian Nights.
State of the set's 85 Oracle identities: **52 Implemented, 32 Partial, 1
excluded** (Bronze Tablet, ante, in `data/unplayable.tsv`). The transcoder
wrote 14; 16 were hand-implemented; 32 are Partial with their exact reason in
the file and the grouped engine work in `TODO.md` ("Antiquities — engine
gaps"). One card was demoted during testing: **The Rack** printed "3 minus
the number of cards in their hand" but the DSL can only spell count minus a
constant, and the hand implementation had mirrored Black Vise; its ability is
off the card and the missing amount is recorded.

Gates: `codegen --check` green, `xtask validate` green (3,069 cards conform),
engine library suite green at 4,857 passed (2 ignored), workspace suite green
except `baylee-catalog`'s PostgreSQL-backed search tests (no `DATABASE_URL`),
clippy `-D warnings` and `cargo fmt` clean. One tool finding: `validate`'s
`OFFERS_A_CHOICE` list lacked `PlayerMayPayCostOr`, a false positive on
Yawgmoth Demon, fixed with one line in `xtask/src/main.rs`.

A rules-derived test pass then wrote 42 tests for the newly Implemented cards
(from the Scryfall payloads and rulings under `scratchpad/atq-tests/`), each
with a `card_index(...)` helper; every loggable ability fires, all 47
permanents leave the battlefield clean under `BAYLEE_LEAVE_LOG`, and every
listed ability mutant was killed. It was this pass that caught The Rack. As
with Arabian Nights, the formal L4/L5 stamp still needs the `--coverage`
export (`cargo llvm-cov` is not installed); the 32 Partial cards have no
tests.

## Legends — third set, 24 batches (2026-10-05)

310 Oracle identities: **167 Implemented, 141 Partial, 2 excluded** (ante:
Rebirth, Tempest Efreet). 64 cards came out of the transcoder; 189 stubs were
hand-worked in three waves of eight parallel agents under the same no-engine
contract, each Partial carrying its own `// NOT SUPPORTED` reason. The engine
work this exposed is grouped in `TODO.md` ("Legends — engine gaps"); rampage,
one-sided block triggers and source-filtered prevention are the big recurring
families.

Gates: `codegen --check` green, `xtask validate` green (3,322 cards conform),
engine library suite green at 4,857 passed (2 ignored), `baylee-cards` lints
green (one fix: Cocoon's `Coverage::Partial` reason spelled `Custom(` inside
a string literal, which the counter-id lint reads as code), workspace suite
green except `baylee-catalog`'s PostgreSQL-backed tests, clippy `-D warnings`
and `cargo fmt` clean. One engine test-list fix: the event-reader census in
`this_object_tests.rs` did not know `IfEventObjectMatches`, which
`resolve/mod.rs` and `sources/event_readers.rs` both read — one spelling
added.

**The rules-derived test pass for Legends' 167 Implemented cards has not run
yet**: the 121 newly Implemented cards sit at L2 (validate-clean, no engine
test names them yet) and the 46 inherited ones keep their prior level. The
ARN (21) and ATQ (30) passes are the template. Nothing in this set was
demoted by testing because testing has not run.
