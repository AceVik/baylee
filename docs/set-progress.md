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

290 distinct Oracle identities in `set_lea.rs`: **254 Implemented, 32 Partial,
4 explicitly excluded** by the existing owner scope in `data/unplayable.tsv`.
Thus 254 of 286 in-scope cards are marked Implemented; **Alpha is not complete**.
Gloom and Cyclopean Tomb are included in the Implemented count. Their dedicated
behavioral tests replace the earlier cast-only evidence; milestone validation
and native screenshots are in `docs/feedback-fixes-2026-10-01.md`.

Excluded cards: Chaos Orb (dexterity); Contract from Below, Darkpact and Demonic
Attorney (ante). These are explicit scope exclusions, never counted as implemented.

### Remaining cards

| Card | Explicitly unsupported behavior from its implementation |
| --- | --- |
| [Animate Artifact](../crates/baylee-cards/src/cards/enchantments/auras/mv_4/animate_artifact.rs) | power and toughness equal to the enchanted artifact's mana value are not in the DSL; the Aura attaches and does nothing |
| [Animate Dead](../crates/baylee-cards/src/cards/enchantments/auras/mv_2/animate_dead.rs) | an Aura that enchants a creature card in a graveyard and returns it is not in the engine |
| [Camouflage](../crates/baylee-cards/src/cards/instants/mv_1/camouflage.rs) | defending players putting their creatures into piles assigned to attackers at random, instead of declaring blockers, is not in the engine |
| [Channel](../crates/baylee-cards/src/cards/sorceries/mv_2/channel.rs) | paying life for {C} any time a mana ability could be activated is not in the engine |
| [Clockwork Beast](../crates/baylee-cards/src/cards/creatures/artifacts/mv_6/clockwork_beast.rs) | an end-of-combat trigger and putting up to X counters, at most seven in all, are not in the engine; it enters with seven +1/+0 counters |
| [Consecrate Land](../crates/baylee-cards/src/cards/enchantments/auras/mv_1/consecrate_land.rs) | refusing other Auras is not in the engine; the enchanted land has indestructible |
| [Creature Bond](../crates/baylee-cards/src/cards/enchantments/auras/mv_2/creature_bond.rs) | damage equal to the dying creature's toughness is not in the DSL; the Aura attaches and does nothing |
| [Demonic Hordes](../crates/baylee-cards/src/cards/creatures/mv_6/demonic_hordes.rs) | the upkeep payment and a land sacrificed by an opponent's choice are not in the engine; it destroys lands |
| [Drain Life](../crates/baylee-cards/src/cards/sorceries/mv_2/drain_life.rs) | black mana only for X and the capped life gain are not in the engine; it deals X damage |
| [Drain Power](../crates/baylee-cards/src/cards/sorceries/mv_2/drain_power.rs) | making a player activate a mana ability of each land they control is not in the engine |
| [Earthbind](../crates/baylee-cards/src/cards/enchantments/auras/mv_1/earthbind.rs) | the enter trigger's condition and the ability the Aura gains are not read; the Aura attaches and does nothing |
| [False Orders](../crates/baylee-cards/src/cards/instants/mv_1/false_orders.rs) | removing a blocker from combat and having it block again is not in the engine |
| [Fireball](../crates/baylee-cards/src/cards/sorceries/mv_1/fireball.rs) | damage divided evenly among any number of targets, and the cost of each target beyond the first, are not in the DSL; X damage to one target |
| [Fork](../crates/baylee-cards/src/cards/instants/mv_2/fork.rs) | a copy of a spell aimed at a player keeps that player: only object targets may be chosen anew |
| [Guardian Angel](../crates/baylee-cards/src/cards/instants/mv_1/guardian_angel.rs) | paying {1} any time until end of turn for another shield is not in the engine; the first shield only |
| [Illusionary Mask](../crates/baylee-cards/src/cards/artifacts/mv_2/illusionary_mask.rs) | casting a creature card face down for the mana spent on {X}, and turning it face up instead of dealing or being dealt damage, are not in the engine |
| [Island Sanctuary](../crates/baylee-cards/src/cards/enchantments/mv_2/island_sanctuary.rs) | skipping a draw in exchange for an attack restriction until your next turn is not in the engine |
| [Kudzu](../crates/baylee-cards/src/cards/enchantments/auras/mv_3/kudzu.rs) | destroying the enchanted land when it becomes tapped and moving the Aura to another land are not in the DSL; it only enchants a land |
| [Library of Leng](../crates/baylee-cards/src/cards/artifacts/mv_1/library_of_leng.rs) | discarding a card onto the top of the library instead of into the graveyard is not in the engine; you have no maximum hand size |
| [Lich](../crates/baylee-cards/src/cards/enchantments/mv_4/lich.rs) | not losing the game at 0 life, life gain as draws and damage as sacrifices are not in the engine |
| [Magical Hack](../crates/baylee-cards/src/cards/instants/mv_1/magical_hack.rs) | text-changing effects (CR 612) are not in the engine |
| [Nether Shadow](../crates/baylee-cards/src/cards/creatures/mv_2/nether_shadow.rs) | returning from the graveyard by the creature cards above it is not in the engine; it has haste |
| [Personal Incarnation](../crates/baylee-cards/src/cards/creatures/mv_6/personal_incarnation.rs) | redirecting damage to its owner, activation by its owner only, and losing half the owner’s life rounded up |
| [Power Leak](../crates/baylee-cards/src/cards/enchantments/auras/mv_2/power_leak.rs) | paying any amount of mana to prevent that much of the damage is not in the DSL; it only enchants an enchantment |
| [Power Sink](../crates/baylee-cards/src/cards/instants/mv_1/power_sink.rs) | tapping the lands and emptying the mana pool of a player who doesn't pay are not in the engine; it counters unless {X} is paid |
| [Raging River](../crates/baylee-cards/src/cards/enchantments/mv_2/raging_river.rs) | Left and right piles that restrict blockers |
| [Sengir Vampire](../crates/baylee-cards/src/cards/creatures/mv_5/sengir_vampire.rs) | the trigger on a creature it damaged this turn dying is not in the engine; it flies |
| [Sleight of Mind](../crates/baylee-cards/src/cards/instants/mv_1/sleight_of_mind.rs) | text-changing effects (CR 612) are not in the engine |
| [Sunglasses of Urza](../crates/baylee-cards/src/cards/artifacts/mv_3/sunglasses_of_urza.rs) | spending white mana as though it were red is not in the engine |
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
