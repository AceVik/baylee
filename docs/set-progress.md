# Set progress — 2026-10-02

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

290 distinct Oracle identities in `set_lea.rs`: **265 Implemented, 21 Partial,
4 explicitly excluded** by the existing owner scope in `data/unplayable.tsv`.
Thus 265 of 286 in-scope cards are marked Implemented; **Alpha is not complete**.
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
| [Demonic Hordes](../crates/baylee-cards/src/cards/creatures/mv_6/demonic_hordes.rs) | the upkeep payment and a land sacrificed by an opponent's choice are not in the engine; it destroys lands |
| [Drain Power](../crates/baylee-cards/src/cards/sorceries/mv_2/drain_power.rs) | making a player activate a mana ability of each land they control is not in the engine |
| [False Orders](../crates/baylee-cards/src/cards/instants/mv_1/false_orders.rs) | removing a blocker from combat and having it block again is not in the engine |
| [Fireball](../crates/baylee-cards/src/cards/sorceries/mv_1/fireball.rs) | damage divided evenly among any number of targets, and the cost of each target beyond the first, are not in the DSL; X damage to one target |
| [Guardian Angel](../crates/baylee-cards/src/cards/instants/mv_1/guardian_angel.rs) | paying {1} any time until end of turn for another shield is not in the engine; the first shield only |
| [Illusionary Mask](../crates/baylee-cards/src/cards/artifacts/mv_2/illusionary_mask.rs) | casting a creature card face down for the mana spent on {X}, and turning it face up instead of dealing or being dealt damage, are not in the engine |
| [Island Sanctuary](../crates/baylee-cards/src/cards/enchantments/mv_2/island_sanctuary.rs) | skipping a draw in exchange for an attack restriction until your next turn is not in the engine |
| [Kudzu](../crates/baylee-cards/src/cards/enchantments/auras/mv_3/kudzu.rs) | destroying the enchanted land when it becomes tapped and moving the Aura to another land are not in the DSL; it only enchants a land |
| [Library of Leng](../crates/baylee-cards/src/cards/artifacts/mv_1/library_of_leng.rs) | discarding a card onto the top of the library instead of into the graveyard is not in the engine; you have no maximum hand size |
| [Lich](../crates/baylee-cards/src/cards/enchantments/mv_4/lich.rs) | not losing the game at 0 life, life gain as draws and damage as sacrifices are not in the engine |
| [Magical Hack](../crates/baylee-cards/src/cards/instants/mv_1/magical_hack.rs) | text-changing effects (CR 612) are not in the engine |
| [Personal Incarnation](../crates/baylee-cards/src/cards/creatures/mv_6/personal_incarnation.rs) | redirecting damage to its owner, activation by its owner only, and losing half the owner’s life rounded up |
| [Power Leak](../crates/baylee-cards/src/cards/enchantments/auras/mv_2/power_leak.rs) | paying any amount of mana to prevent that much of the damage is not in the DSL; it only enchants an enchantment |
| [Raging River](../crates/baylee-cards/src/cards/enchantments/mv_2/raging_river.rs) | Left and right piles that restrict blockers |
| [Sleight of Mind](../crates/baylee-cards/src/cards/instants/mv_1/sleight_of_mind.rs) | text-changing effects (CR 612) are not in the engine |
| [Time Vault](../crates/baylee-cards/src/cards/artifacts/mv_2/time_vault.rs) | skipping a turn to untap it is not in the engine; it enters tapped, does not untap and takes an extra turn |
| [Vesuvan Doppelganger](../crates/baylee-cards/src/cards/creatures/mv_5/vesuvan_doppelganger.rs) | the copied upkeep ability that copies again is not in the DSL; it enters as a blue copy |
| [Word of Command](../crates/baylee-cards/src/cards/instants/mv_2/word_of_command.rs) | looking at an opponent's hand, controlling that player and making them play a card are not in the engine |

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
Alpha as a whole remains incomplete.

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

### Next batch prepared

Fireball, Demonic Hordes and Power Leak have 12 independently authored tests in
unregistered `*_batch_b.rs` files. They are not yet compiled or coverage evidence.
Remaining acceptance scenarios include all-illegal Fireball targets/surcharge
refusal, Hordes control changes/already-tapped state, and Power Leak controller
capture/unpreventable damage/redirection. No next-batch card is promoted yet.

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
