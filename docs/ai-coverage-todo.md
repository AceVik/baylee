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
| Contextual counters | Time can be good on vanishing and bad on suspend; lore advances chapters; custom counters need their printed payoff. Add examples before assigning a universal sign. |
| Proliferate and replacement effects | Select friendly benefits and hostile poison; account for counter doublers, prevention and replacement ordering. |
| Planeswalker survival | Split attacks across players and walkers, loyalty-based prevention, static walker abilities, team blocks and multicolor protection. |
| Modal and multi-target effects | Different targets serving different clauses; modal spells with independent target requirements; sacrifice and reanimation ownership. |
| Alternate resource engines | Sacrifice/discard/exile/counter costs with beneficial payoffs; mana loops, restricted mana, treasure, life payments and untap engines. Prove progress and prohibit zero-cost repetitions. |
| Variable costs | Multiple X symbols, variable target counts, cost reductions, convoke/delve, life-X sweeps and draw-X near deck-out. |
| Stack strategy | Counter wars, ward/taxes, uncounterable spells, protecting a combo, redirect/copy effects and stack-value assessment. |
| Hidden-zone decisions | Scout refresh after shuffle, reveal, wish and sideboarding; never reuse a prior hidden object handle to issue an action. |
| Transformation and alternate zones | Each playable face, adventure, disturb, flashback, madness, suspend, meld and copied abilities, once available in the relevant deck. |
| Full-game lookahead | Responses, combat triggers, replacements, alternative wins and combo sequencing. The current bounded combat search does not simulate these. |

Security tests must continue to reject scouting by a human, an invalid seat,
a human driving an AI chair and an AI standing in for a disconnected human.
Scouting must leave human views and print-table disclosure unchanged.
