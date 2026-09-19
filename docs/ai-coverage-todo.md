# AI coverage as the card pool grows

The AI evaluates types, costs, projected characteristics and DSL effects. It
does not select a policy by card or deck name. This is not a claim that every
Magic deck works today: the implemented rules and card pool bound what can be
tested end to end. A stub's printed types are useful for deck analysis, but
its absent rules are not evidence of a working strategy.

Use synthetic engine fixtures to test an existing effect before a real card
implements it. Use real registered cards for casting, cost and interaction
tests as soon as they exist. A TODO is not a passing or ignored test.

For each newly implemented mechanic, add the smallest losing decision as a
regression first, then a real-engine game that reaches the resolution. Where
the client offers a new kind of choice, exercise it in the running client.

| TODO test | Required scenario and assertion |
| --- | --- |
| Full deck families | Paired seeds for aggro, control, ramp, combo, tokens, artifacts, enchantments, graveyard, mill and prison using complete implemented lists; report caps and refusals beside wins. |
| Commander pair rules | Real Partner, Partner With, Friends Forever, Background, Doctor's Companion and other legal pairings as their rules land; each commander's tax and damage remain separate. |
| Commander identity changes | Copy/control/zone changes preserve per-commander damage identity; a partner's damage never contributes to the other's 21. |
| Contextual counters | Lore and suspend are done (#73): `tactics::clock_score` reads the sign off the card the counter lands on, proved by `a_lore_counter_goes_on_my_own_saga_and_never_the_opponents` and `a_time_counter_delays_the_suspended_card_that_is_about_to_cast`. Three cases stay open and are scored 0 rather than guessed: vanishing, where a time counter runs the opposite way from suspend and which no card here prints; the lore counter that would reach a Saga's final chapter, which is that chapter and the Saga's death at once; and custom counters, which mean whatever their own card says. Neither clock is reachable from an effect a player targets — both are advanced by the engine — so there is no real-engine game to write until a card prints one, which `no_pool_card_puts_a_lore_or_time_counter_on_anything` waits for. |
| Proliferate and replacement effects | Select friendly benefits and hostile poison; account for counter doublers, prevention and replacement ordering. |
| Planeswalker survival | Split attacks across players and walkers, loyalty-based prevention, static walker abilities, team blocks and multicolor protection. |
| Modal and multi-target effects | Choosing the mode is done (#93): a modal spell whose every effect sits under a mode is offered no normal cast (CR 700.2a), so the agent used to take the first printed one at every table. `filter::cast_mode` ranks the offered modes by what they reach, proved by `a_modal_spell_takes_the_mode_that_reaches_something` against Sheoldred's Edict and pinned the other way by `a_normal_cast_is_not_traded_for_a_mode`. It reads only what the engine has not already decided — a mode that needs targets was offered because they exist (CR 700.2a, CR 700.2b) — and only the three untargeted board sentences (`SacrificeFilter`, `DestroyChosenForPlayers`, `DestroyAll`); everything else is `None` and keeps the printed order. Open: different targets serving different clauses, and sacrifice and reanimation ownership. |
| Alternate resource engines | Sacrifice/discard/exile/counter costs with beneficial payoffs; mana loops, restricted mana, treasure, life payments and untap engines. Prove progress and prohibit zero-cost repetitions. |
| Variable costs | Multiple X symbols, variable target counts, cost reductions, convoke/delve, life-X sweeps and draw-X near deck-out. |
| Stack strategy | Counter wars, ward/taxes, uncounterable spells, protecting a combo, redirect/copy effects and stack-value assessment. |
| Hidden-zone decisions | Scout refresh after shuffle, reveal, wish and sideboarding; never reuse a prior hidden object handle to issue an action. |
| Transformation and alternate zones | Each playable face, adventure, disturb, flashback, madness, suspend, meld and copied abilities, once available in the relevant deck. |
| Full-game lookahead | Responses, combat triggers, replacements, alternative wins and combo sequencing. The current bounded combat search does not simulate these. |

Security tests must continue to reject scouting by a human, an invalid seat,
a human driving an AI chair and an AI standing in for a disconnected human.
Scouting must leave human views and print-table disclosure unchanged.
