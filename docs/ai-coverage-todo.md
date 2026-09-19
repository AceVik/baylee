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

Every row's **Pool today** figure is read by
`crates/baylee-gamehost/tests/ai_coverage_guards.rs` out of
`baylee_cards::all()`, never by grepping card files, and the effect-level
ones descend through `Effect::walk` — so a count includes what a card does
inside a `Sequence`, in either half of a kicker clause and behind "unless you
pay". Measured 19.09.2026. The figures are prose here and assertions there:
a row printed by the pool asserts *more than none*, a row the pool cannot
print yet asserts *none*, and neither pins the number, because a row does not
stop being owed because a second card joined it.

**A figure above nought means the row is owed now, not that it is answered.**
Two rows are part-answered, and they say so in their own cell.

| TODO test | Pool today | Required scenario and assertion |
| --- | --- | --- |
| Full deck families | not a card shape | Paired seeds for aggro, control, ramp, combo, tokens, artifacts, enchantments, graveyard, mill and prison using complete implemented lists; report caps and refusals beside wins. |
| Commander pair rules | 1 plain Partner; 0 other pairings | Real Partner, Partner With, Friends Forever, Background, Doctor's Companion and other legal pairings as their rules land; each commander's tax and damage remain separate. |
| Commander identity changes | 3 controller changes, 4 token copies | Copy/control/zone changes preserve per-commander damage identity; a partner's damage never contributes to the other's 21. |
| Contextual counters | 2 Sagas, 2 suspend | Lore and suspend are done (#73): `tactics::clock_score` reads the sign off the card the counter lands on, proved by `a_lore_counter_goes_on_my_own_saga_and_never_the_opponents` and `a_time_counter_delays_the_suspended_card_that_is_about_to_cast`. Three cases stay open and are scored 0 rather than guessed: vanishing, where a time counter runs the opposite way from suspend and which no card here prints; the lore counter that would reach a Saga's final chapter, which is that chapter and the Saga's death at once; and custom counters, which mean whatever their own card says. Neither clock is reachable from an effect a player targets — both are advanced by the engine — so there is no real-engine game to write until a card prints one, which `no_pool_card_puts_a_lore_or_time_counter_on_anything` waits for. |
| Proliferate and replacement effects | 0 infect/wither; 6 replacement abilities | Select friendly benefits and hostile poison; account for counter doublers, prevention and replacement ordering. |
| Planeswalker survival | 10 loyalty faces | Split attacks across players and walkers, loyalty-based prevention, static walker abilities, team blocks and multicolor protection. |
| Modal and multi-target effects | 10 modal cards | Choosing the mode is done (#93): a modal spell whose every effect sits under a mode is offered no normal cast (CR 700.2a), so the agent used to take the first printed one at every table. `filter::cast_mode` ranks the offered modes by what they reach, proved by `a_modal_spell_takes_the_mode_that_reaches_something` against Sheoldred's Edict and pinned the other way by `a_normal_cast_is_not_traded_for_a_mode`. It reads only what the engine has not already decided — a mode that needs targets was offered because they exist (CR 700.2a, CR 700.2b) — and only the three untargeted board sentences (`SacrificeFilter`, `DestroyChosenForPlayers`, `DestroyAll`); everything else is `None` and keeps the printed order. Open: different targets serving different clauses, and sacrifice and reanimation ownership. |
| Alternate resource engines | 6 treasure, 12 restricted mana, 11 alternative costs, 361 cards with a sacrifice or discard activation cost | Sacrifice/discard/exile/counter costs with beneficial payoffs; mana loops, restricted mana, treasure, life payments and untap engines. Prove progress and prohibit zero-cost repetitions. |
| Variable costs | 26 X costs, 1 cost reduction, 4 convoke/delve | Multiple X symbols, variable target counts, cost reductions, convoke/delve, life-X sweeps and draw-X near deck-out. |
| Stack strategy | 35 counterspells, 5 copy, 3 redirect, 3 ward | Counter wars, ward/taxes, uncounterable spells, protecting a combo, redirect/copy effects and stack-value assessment. |
| Hidden-zone decisions | 87 library searches, 1 wish | Scout refresh after shuffle, reveal, wish and sideboarding; never reuse a prior hidden object handle to issue an action. |
| Transformation and alternate zones | 120 compiled two-face cards (see #115), 3 adventure/disturb, 3 granting flashback, 10 entering as a copy | Each playable face, adventure, disturb, flashback, madness, suspend, meld and copied abilities, once available in the relevant deck. |
| Full-game lookahead | not a card shape | Responses, combat triggers, replacements, alternative wins and combo sequencing. The current bounded combat search does not simulate these. |

Three mechanics the table names have **no figure and can never get one from
this pool**, and the reason is the DSL rather than the card list: there is no
`Effect` for proliferate, and no handle anywhere for madness or meld. A probe
for any of them would assert nought against a population that cannot change,
which is a green test measuring nothing. What has to move first is the
vocabulary, and the day it does the probe becomes writable — so they are
named here and in the test's own header rather than given a cell.

Infect and wither are the opposite case and worth keeping apart from them:
the DSL can say both, this pool prints neither, and that nought **is** a
finding, asserted in
`a_mechanic_the_pool_cannot_print_yet_has_no_ai_test_to_write`.

**Every cell names its instrument, and that is the harder half.** A figure
here answers whatever the probe behind it asks, which is not always the
question the row's own sentence asks — and a wrong answer of that kind
arrives with a green test behind it. Four of the sixteen rows written for
this column were caught doing it, and none of them was caught by measuring
more carefully:

- *treasure* read two of the seven `Effect` variants that carry a token and
  returned 5 where the pool makes **6**; the seventh arm was Fountainport's.
- *a sacrifice, discard or exile cost* asked `additional_costs` and returned
  3, of which exactly one prints a non-mana part at all — and it is a
  `PayLifeX`, so the cell had **no members of its own category** and still
  read as populated. The row's sentence is about an **activation** cost, and
  that is **361 cards** (382 abilities, 383 parts: the units differ and the
  row asserts on cards).
- *flashback* returned 3 and no card in this pool has flashback: those three
  grant it to somebody else's card, which is a different test to write.
- *a second face* counts `faces.len() >= 2`. The printed figure is 121 and
  what a client wants is the ~107 with a separate back image, because an
  adventure prints both halves on one physical face. #115 is open on the
  predicate; the cell says which number it is until then.

Every number above was measured rather than recalled, and that is not a
formality. Eleven figures in the first draft of this column were written from
memory and **eight of the eleven were wrong** — Sagas as 1 where the pool has
2, modal cards as 63 where it has 10, two-faced as 116 where it has 120,
adventure and disturb as 8 where it has 3. The three that happened to be
right are the reason this is worth saying out loud: a recalled count reads
exactly like a measured one, and being right about some of them is what makes
the rest believable.

Security tests must continue to reject scouting by a human, an invalid seat,
a human driving an AI chair and an AI standing in for a disconnected human.
Scouting must leave human views and print-table disclosure unchanged.
