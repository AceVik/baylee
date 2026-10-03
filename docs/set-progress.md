# Set progress — 2026-10-03

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

290 distinct Oracle identities in `set_lea.rs`: **268 Implemented, 18 Partial,
4 explicitly excluded** by the existing owner scope in `data/unplayable.tsv`.
Thus 268 of 286 in-scope cards are marked Implemented; **Alpha is not complete**.
Gloom, Cyclopean Tomb, Creature Bond, Consecrate Land, Animate Artifact,
Nether Shadow, Sunglasses of Urza, Sengir Vampire and Earthbind are
included in the Implemented count. Their dedicated
behavioral tests replace the earlier cast-only evidence; milestone validation
and native screenshots are in `docs/feedback-fixes-2026-10-01.md`.

Excluded cards: Chaos Orb (dexterity); Contract from Below, Darkpact and Demonic
Attorney (ante). These are explicit scope exclusions, never counted as implemented.

### Remaining cards

| Card | Explicitly unsupported behavior from its implementation |
| --- | --- |
| [Animate Dead](../crates/baylee-cards/src/cards/enchantments/auras/mv_2/animate_dead.rs) | an Aura that enchants a creature card in a graveyard and returns it is not in the engine |
| [Camouflage](../crates/baylee-cards/src/cards/instants/mv_1/camouflage.rs) | defending players putting their creatures into piles assigned to attackers at random, instead of declaring blockers, is not in the engine |
| [Channel](../crates/baylee-cards/src/cards/sorceries/mv_2/channel.rs) | paying life for {C} any time a mana ability could be activated is not in the engine |
| [Drain Power](../crates/baylee-cards/src/cards/sorceries/mv_2/drain_power.rs) | making a player activate a mana ability of each land they control is not in the engine |
| [False Orders](../crates/baylee-cards/src/cards/instants/mv_1/false_orders.rs) | removing a blocker from combat and having it block again is not in the engine |
| [Guardian Angel](../crates/baylee-cards/src/cards/instants/mv_1/guardian_angel.rs) | paying {1} any time until end of turn for another shield is not in the engine; the first shield only |
| [Illusionary Mask](../crates/baylee-cards/src/cards/artifacts/mv_2/illusionary_mask.rs) | casting a creature card face down for the mana spent on {X}, and turning it face up instead of dealing or being dealt damage, are not in the engine |
| [Island Sanctuary](../crates/baylee-cards/src/cards/enchantments/mv_2/island_sanctuary.rs) | skipping a draw in exchange for an attack restriction until your next turn is not in the engine |
| [Kudzu](../crates/baylee-cards/src/cards/enchantments/auras/mv_3/kudzu.rs) | destroying the enchanted land when it becomes tapped and moving the Aura to another land are not in the DSL; it only enchants a land |
| [Library of Leng](../crates/baylee-cards/src/cards/artifacts/mv_1/library_of_leng.rs) | discarding a card onto the top of the library instead of into the graveyard is not in the engine; you have no maximum hand size |
| [Lich](../crates/baylee-cards/src/cards/enchantments/mv_4/lich.rs) | not losing the game at 0 life, life gain as draws and damage as sacrifices are not in the engine |
| [Magical Hack](../crates/baylee-cards/src/cards/instants/mv_1/magical_hack.rs) | text-changing effects (CR 612) are not in the engine |
| [Personal Incarnation](../crates/baylee-cards/src/cards/creatures/mv_6/personal_incarnation.rs) | redirecting damage to its owner, activation by its owner only, and losing half the owner’s life rounded up |
| [Raging River](../crates/baylee-cards/src/cards/enchantments/mv_2/raging_river.rs) | Left and right piles that restrict blockers |
| [Sleight of Mind](../crates/baylee-cards/src/cards/instants/mv_1/sleight_of_mind.rs) | text-changing effects (CR 612) are not in the engine |
| [Time Vault](../crates/baylee-cards/src/cards/artifacts/mv_2/time_vault.rs) | skipping a turn to untap it is not in the engine; it enters tapped, does not untap and takes an extra turn |
| [Vesuvan Doppelganger](../crates/baylee-cards/src/cards/creatures/mv_5/vesuvan_doppelganger.rs) | the copied upkeep ability that copies again is not in the DSL; it enters as a blue copy |
| [Word of Command](../crates/baylee-cards/src/cards/instants/mv_2/word_of_command.rs) | looking at an opponent's hand, controlling that player and making them play a card are not in the engine |

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
separate source-selection blocker remains.

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

### Separate target-incarnation gap

The read-only source audit also found an untargeted self-effect risk: Shivan
Dragon's `PumpFilter(Filter::This)` binds a bare source id and may pump a new
incarnation after a blink. This still needs a behavioral regression and a shared
self-effect audit; the fixed counter and damage paths do not establish that all
self-effects respect versions.

The existing `target_legality` implementation in Engine `progress.rs` explicitly
documents issue #117: ordinary targets are bare ObjectIds, so a target that leaves
and returns can be treated as legal despite being a new object. The exact-source
work now retains target-slot versions, but source-menu correctness alone does
not prove that ordinary target revalidation uses them. This known shared rules
gap must be tested and addressed before claiming complete Alpha acceptance.
Keep its outcome separate from the source-selection milestone.
The follow-up must cover retarget identity (`Retarget.was`/`Aim`) and duplicate
choices as well as first/second target groups, partial fizzle and copied spells.
A resolution-time version predicate alone would not finish that work.
The agreed next milestone keeps exact pairs when compacting surviving target
groups, stages retarget choices by exact object/player identity, and avoids
labeling an old target through its newer hidden incarnation. Independent cases
must include Bolt/blink fizzle, partially legal Fireball, both groups of a fight,
Fork preserving an old illegal target versus explicitly selecting the returned
one, and atomic duplicate/swap handling. No #117 production change is included
in the source-selection milestone.

The multiplayer departure code also has an existing general substitute-chooser
limitation under current CR 800.4g. New source-choice concession tests establish
fresh questions for a surviving chooser and continuation when no source remains;
they do not establish general replacement of a departed non-cost decision maker.

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
The source-incarnation selection design remains the next separate engine job.

Damage milestone committed as `81409297`. The source-selection extension is
now in progress with Astra xhigh; it is not accepted yet. Sol medium authors
independent real-card regressions, and Astra high owns client integration and
the display audit. Protocol16 is reserved for the new exact-source contract.

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
