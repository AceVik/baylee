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
| Contextual counters | 2 Sagas, 2 suspend, 2 undying, 2 persist | Lore, suspend and the ±1/±1 payoff are done (#73, 23.09.2026): `tactics::clock_score` reads the sign off the card the counter lands on, proved by `a_lore_counter_goes_on_my_own_saga_and_never_the_opponents` and `a_time_counter_delays_the_suspended_card_that_is_about_to_cast`. Three cases stay open and are scored 0 rather than guessed: vanishing, where a time counter runs the opposite way from suspend and which no card here prints; the lore counter that would reach a Saga's final chapter, which is that chapter and the Saga's death at once; and custom counters, which mean whatever their own card says. Neither clock is reachable from an effect a player **targets** — both are advanced by the engine — so there is no real-engine game to write until a card prints one, which `only_the_named_cards_put_a_lore_or_time_counter_on_anything` waits for. **Trenzalore Clocktower arrived on 20.09.2026 and does not change that sentence** (#166). Its counter is inside `{T}: Add {U}. Put a time counter on Trenzalore Clocktower.` — one mana ability, no target — so nothing ever asks `clock_score` about it, and the worry the ticket was opened on does not hold either: the `Time` arm answers 0 for this card because it asks whether the card prints `Suspend` before it reads the count, which `a_time_counter_is_not_a_delay_on_a_card_that_is_not_counting_down` pins with the Clocktower's three counters losing to a suspended card's four. What the card did expose is **#170** — `mana_shape` matches a one-element `[Effect::AddMana]`, so a mana ability with a second sentence is invisible to the planner and the agent never taps this land at all (`a_mana_land_that_also_counts_is_invisible_to_the_planner`). **The ±1/±1 half arrived with undying and persist on 23.09.2026** and is the counter whose side of the table the card decides: undying asks whether the creature had a +1/+1 counter on it (CR 702.93a) and persist whether it had a −1/−1 one (CR 702.79a), so the counter that helps everywhere else is the one that switches the return off on exactly that creature. `tactics::denies_a_return` reads the pair off the view's projected keywords — a granted undying is undying — and only the exact `+1/+1` and `−1/−1` kinds, because CR 122.1a makes each pair a counter kind of its own and a +2/+2 counter denies nothing. It is an exception inside the sign rather than a second `clock`: the benefit still decides the side of the table everywhere else. Both halves are pinned, and one alone would be satisfied by a second fixed sign — `a_plus_one_counter_does_not_go_on_my_own_undying_creature` takes the *smaller* of two of my own creatures, and `a_minus_one_counter_prefers_the_persist_creature_it_keeps_down` takes the *smaller* of two of theirs. |
| Proliferate and replacement effects | 0 infect/wither; 6 replacement abilities | Select friendly benefits and hostile poison; account for counter doublers, prevention and replacement ordering. |
| Planeswalker survival | 10 loyalty faces | Blocking to keep a walker is done (#75): a creature worth less chumps when the block brings the damage below loyalty, never when the walker survives or dies anyway, and the seat's life first — `a_walker_that_would_die_is_chumped_for_and_one_that_would_not_is_not`, and through the engine `a_creature_blocks_to_keep_a_walker_the_attack_would_kill`. Open: split attacks across players and walkers (`aim_at` sends the whole attack at one defender), loyalty-based prevention, static walker abilities, team blocks and multicolor protection. |
| Modal and multi-target effects | 10 modal cards | Choosing the mode is done (#93): a modal spell whose every effect sits under a mode is offered no normal cast (CR 700.2a), so the agent used to take the first printed one at every table. `filter::cast_mode` ranks the offered modes by what they reach, proved by `a_modal_spell_takes_the_mode_that_reaches_something` against Sheoldred's Edict and pinned the other way by `a_normal_cast_is_not_traded_for_a_mode`. It reads only what the engine has not already decided — a mode that needs targets was offered because they exist (CR 700.2a, CR 700.2b) — and only the three untargeted board sentences (`SacrificeFilter`, `DestroyChosenForPlayers`, `DestroyAll`); everything else is `None` and keeps the printed order. A fight's two targets are done (23.09.2026): each instance of "target" (CR 115.3) is its own question, marked by `DecisionContext::second_instance`, and `fight::fight_targets` answers it by what the fight would do — kill and survive, trade, or nothing — instead of by the spell's overall sign, which read Bridgeworks Battle's pump as a benefit and declined its "up to one" foe. Proved by `a_fight_names_the_creature_its_fighter_kills_and_survives` (including the decline against a 6/6) and `a_fight_names_the_fighter_with_a_fight_worth_having`. Open: other clauses served by different targets, and sacrifice and reanimation ownership. |
| Alternate resource engines | 11 treasure, 43 restricted mana, 11 alternative costs, 430 cards with a sacrifice or discard activation cost (24.09.2026) | Treasure is spent (#223): a registry token's abilities come from its definition (`a_treasure_pays_for_a_spell_when_nothing_else_can`). Restricted mana is spent (#223) on a spell whose filter the view reads as a match: one restricted tap per spell, sent last, and its colour named for a spell it may pay for (`restricted_mana_pays_for_the_spells_it_names_and_no_others` through the engine; `restricted_mana_is_named_for_a_spell_it_may_pay_for`, `restricted_mana_is_not_counted_as_held_up`). The 5 whose filter names a chosen type or the commander's types are never tapped. The 4 free-with-commander alternative costs are taken over a printed cost the pool already covers (#223, `a_free_alternative_cost_is_taken_over_the_printed_one`); a pitch or an evoke keeps the printed cost when it is affordable. A cost that sacrifices, discards or exiles from hand another card is no longer paid for a whitelisted gain (#223, `a_card_is_not_given_up_for_a_small_gain`: Zuran Orb was fed four Forests on turn 1); weighing such a trade is open. Open: a second restricted source for one spell, restricted mana for an X or kicker price, a changeling read against a subtype filter (the hand card's printed subtypes), Path of Ancestry and Boseiju (#232: the engine restricts mana the card doesn't); sacrifice/discard/exile costs whose payoff is worth the card; mana loops, life payments and untap engines. Prove progress and prohibit zero-cost repetitions. |
| Variable costs | 26 X costs, 1 cost reduction, 3 convoke/delve | Convoke and delve are done (#74), and each half was checked by removing it rather than by reading it. Convoke is answered from the engine's own offer (`convoke_pays_with_the_offered_permanents_instead_of_targeting_one`), with only as many taps as the floating pool leaves unpaid, the least useful first (#224: `a_convoke_answer_taps_only_what_the_pool_leaves_unpaid`, through the engine `a_convoke_cast_the_pool_already_pays_taps_no_creature`); delve is a **price** before it is a question, and `policy::spell_cost` subtracts the graveyard the same way `casting::can_cast` does (CR 702.66a) so the agent reaches for the spell at all — that subtraction had no assertion and now has `a_delve_spell_is_priced_with_the_graveyard_before_the_first_tap`. Kicker and "you may waterbend" (2 cards) are done (#74): one `policy::kicked_price` floats the kicked price before the cast and answers yes only from the pool, proved by `a_kicker_and_a_waterbend_are_paid_when_the_mana_is_there`; the waterbend's help stops at its own `{6}` (#229, CR 701.67b), as the engine's question does. Open (#246): `spell_cost` subtracts no convoke help, so the planner never casts a convoke spell its lands cannot pay alone, and casts one they could as soon as the engine offers it, with creatures paying what lands could have; multiple X symbols, variable target counts, printed cost reductions, life-X sweeps and draw-X near deck-out. |
| Stack strategy | 35 counterspells, 5 copy, 3 redirect, 3 ward | Ward is done (#74), in all three places it arrives. It is **not** `Effect::PayCostOrLoseLater` — that variant is Pact of Negation, and the refusal beside it has nothing to do with ward; ward is synthesised as a keyword trigger carrying `Effect::PlayerMayPayOr` (`trigger.rs`, CR 702.21). So it is priced when a target is chosen (`tactics::ward_priced`), answered by what refusing it would do (`policy::pays_tax`), and paid inside the window the yes opens (`policy::pay_owed`, CR 605.3a), with `ai_ward.rs` walking all three in a real game. A spell that can't be countered is no reason to cast a counter, and never the counter's choice while another is up (#243, `a_counterspell_is_not_spent_on_a_spell_that_cannot_be_countered`); the view marks it on the stack whether the words or a Cavern of Souls made it so. Open: counter wars, protecting a combo, redirect/copy effects and stack-value assessment (every counterable spell still weighs a flat +1500, #226). |
| Hidden-zone decisions | 87 library searches, 1 wish | Scout refresh after shuffle, reveal, wish and sideboarding; never reuse a prior hidden object handle to issue an action. |
| Transformation and alternate zones | 120 compiled two-face cards (see #115), of which 82 print a land behind a non-land front; 2 adventure, 1 disturb; 3 granting flashback, 13 entering as a copy (24.09.2026) | Part-answered (#76, #227). The **land** face is read: `policy::plays_as_land` asks the engine's own rule for the land drop (`CardDef::land_faces_from_hand`: a modal back, CR 712.12, never a transforming one, CR 712.8a) rather than the face that is up, so a hand of modal double-faced cards is no longer mulliganed as landless and one is no longer discarded as a spare spell. The **cast** faces are not: `filter::cast_mode` takes the `Normal` option whenever one is offered, so an adventure or an MDFC back is cast only when the engine offers nothing else — pinned by `an_adventure_is_offered_and_the_agent_takes_the_printed_front`, because choosing between them needs a value model this crate has not got. A **clone** copies the candidate with the most copiable value, whoever controls it, and not a legendary one of its own unless the copy stops being legendary (#227, `a_clone_copies_what_is_worth_having_twice`); lands tie on that value. A card **given** flashback is tapped for and cast from the graveyard, at the price `PublicObject::flashback` names (#242, `a_graveyard_card_with_flashback_is_tapped_for_and_cast`). Open: each playable face chosen on purpose, disturb, printed flashback (no face has it written: `no_face_that_prints_flashback_has_it_written_yet`), which card a grant goes to, madness, meld and copied abilities, once available in the relevant deck. |
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

**A row closes by injection, not by reading.** #74 was five behaviours that
all looked covered. Each was removed in turn and the suite re-run, and four of
the five were caught by a named test within seconds. The fifth — the delve
discount in `policy::spell_cost` — was removed and **the whole suite stayed
green**: the spell was priced at its printed eight, no mana plan existed for
it over two Islands, and the agent passed a turn it could have dug on, with
nothing to say so. The test beside it, `the_delve_question_is_a_cost_and_the
_agent_pays_it`, covers the *answer* and reads as though it covered the price.
That is the shape to expect here: a green row where one clause of it is
load-bearing and unasserted, hidden behind a neighbour with a similar name.

The sweep that followed took the rest of the rows the same way: 17 injections
over two batches, 3 of them controls with a known answer so that "nothing went
red" could not be a broken harness, and all 3 fired. Ten of the fourteen
behaviours were caught by a named test. The four that were not are worth
keeping written down, because they are three different things and only one of
them is a missing test:

- `filter::modal_modes`' refusal of two mode lists is **unreachable**, held
  by `no_pool_face_states_two_mode_lists`, and its own header now says so.
- The granted mana ability is read in **two** places — `policy::sources`
  turns the engine's offer into a tap, `policy::remaining_sources`
  manufactures that offer out of the battlefield so a colour can be priced
  before anything is tapped — and neither read was asserted. They are now one
  test asked from both ends, because two independent assertions both stay
  green while drifting apart, and the drift is the defect: a land the planner
  counts on and the engine then refuses.
- `search`'s refusal of a menace block with exactly one creature stayed green
  for a third reason again: the rule is live and no test, and no obvious hand
  scenario, puts it in the position of *deciding*. Against a 4/4 menace with
  two 2/2 blockers the search picks the gang block on its own merits with the
  rule removed. A 2/6 menace against a 6/6 and a 2/2, where one blocker
  strictly dominates, is the scene that separates them. #157 carries it,
  together with the finding beside it: `combat::choose_blocks`, the
  `lookahead == 0` path, has no menace rule at all.

  That sentence is as true after #156 as before it, and what changed is the
  **cost**. `combat::can_block` used to ask `state.combat.blockers_of(attacker)`
  before anything was recorded, so menace read as plain unblockable and the
  engine offered such an attacker to nobody — the missing rule was dormant,
  and an injection removing it stayed green because no game reached it.
  `cef30070` fixed that reading, and the offer now names a menace attacker
  wherever two creatures could legally block it. The same missing rule is
  now three of the five profiles (`NOVICE`, `CASUAL`, `STEADY`) answering
  with one blocker, which `Engine::declare_blockers` refuses **whole** rather
  than per pair: every other block in that declaration is lost with it, and
  for an AI chair nothing answers on its behalf — `Session::pump` passes
  priority only when the refusal came at a `Pending::Priority`, and a
  `ChooseBlockers` is not one, so it returns without advancing that question.
  No clock expires either; that one is for seats answering over a socket.
  Whether a later `pump` recovers is unmeasured (#180). #157's
  `enforce_menace` is what stands between that and a stalled table, and it is
  load-bearing rather than defensive as of that commit — a statement about
  code that exists, since the shallow path already reads menace off the
  view's projected keywords at `combat.rs:315`.

So a row that survives its injection has three readings and they point
opposite ways — the rule is dead, the test is missing, or nobody ever built
the scene where the rule decides. Name which one before writing anything.

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
