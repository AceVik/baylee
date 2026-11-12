# House AI

The six names in `AIProfile::NAMED` drive both the offline picker and the
network lobby. Existing `novice`, `steady`, and `sharp` keys still work.
Every level but `seer` is fair: it plays from its seat's view and its own
decklist, at any table (§"Fair by default, and the one level that peeks").
Measurements name `sharp-2026-11`, a frozen copy of `sharp` that no player
is offered (§"The frozen reference").

| Profile | Opening hand | Spell selection and mana | Combat |
| --- | --- | --- | --- |
| novice | Keeps every hand | ±400 deterministic score noise (±800 before generation 16); taps out, and casts an instant at its moment (§"When an instant is cast", from generation 14) | Individual trades |
| casual | Also checks early plays and colour access, as steady (rejected only fewer than two or more than five lands before generation 17) | ±400 noise; taps out, and casts an instant at its moment (from generation 14) | Individual trades |
| steady | Also checks early plays and colour access; bottoms excess lands | ±100 noise; casts an instant at its moment and keeps mana up for a counterspell or, before combat, a trick (§"When an instant is cast") | Individual trades; a trick once blocks are declared |
| sharp | Same opening policy as steady | ±30 seeded score noise; also makes mana for an ability worth its price (§"Mana for an ability") | Searches whole attack sets and blocking assignments, including gang blocks |
| expert | Same opening policy as sharp | ±12 seeded noise | Also simulates retaliation by surviving opponents, and prices the damage of an attack by the life it leaves |
| seer (unfair) | As expert | As expert, and is told every opponent's decklist and hand and the next three cards of every library | As expert |

All levels plan coloured payments through the same renderer-free mana matcher
as the human client. Simple printed and granted mana abilities count, with one
source per permanent. A land's CR 305.6 shortcut counts with every colour its
basic land types give it, so one with two (Urborg and Yavimaya together, a
land Evil Presence made a Swamp under Yavimaya) makes either (NEW-09). A tap
written as one "if" (Gemstone Caverns, River of Tears) makes what the host says
the side that runs now makes (`BoardMana`, `mana_branched`), and a tap that adds
two (a bounce land's `{U}{R}`, `mana_bundle`) or one of each colour among your
permanents (Bloom Tender, `mana_of_each_color`) counts as one of each. A tap for
each counter on itself (City of Shadows) makes what `mana_poured` counts. The
agent does not tap toward an unaffordable spell, nor toward one the engine
would not cast once paid
(`LegalActions::castable_once_paid`: in neither `castable` nor `payable`, such
as Terror with no nonblack creature to point at, CR 601.2c).
Printed mana bonuses, tap penalties and coloured taxes count only while their
abilities are active; phased-out permanents contribute nothing. Coloured taxes
use the chosen spell face or prototype characteristics. Phased-out permanents
also cannot satisfy a casting condition or contribute a convoke payment.
A cast is priced as the engine will charge it, through `spell_price`: the
offer's `spell_increases`, then its `spell_reductions` (Goblin Warchief, a
card's affinity) off the generic part (CR 601.2f, 118.7a). The LLM seat's
reach and the client price through the same function.

Before announcing a mana-X activation, the house checks that its fixed price
and any printed minimum X can be reached through the ordinary view-only mana
planner. A permanent whose cost includes tapping itself cannot also be used
as a mana source for that same activation. Affordable X=0 remains useful when
the fixed effect matters, such as Kessig Wolf Run granting trample.
This closes a repeated rollback found in eight of the 17 capped Sharp-training
cases: announce Wolf Run, choose X=0, choose a creature, then pass its unpaid
`{R}{G}` window without the required colors. The engine correctly reverses that incomplete
activation, but the same unchanged priority offer used to make the house try
again thousands of times. Real-Session regressions in
`crates/baylee-gamehost/tests/ai_activation_payment.rs` prove the rollback and
the affordable activation's actual payment/tap. Card-defined X and printed
activation reductions remain outside this new precheck; it does not claim a
complete solver for every non-mana resource cost or all remaining caps. Frozen
experiment artifacts retain their original house behavior.

An X spell whose required graveyard target has a positive mana-value bound must
also have enough visible eligible targets at the planned, payable X. This avoids
repeatedly announcing Stir the Grave with X=1 when every creature in the caster's
graveyard costs more: the engine rolls that announcement back, and the same
priority offer otherwise causes another identical attempt. The check reads only
the seat's public graveyards and treats unknown filter clauses as possible,
not impossible. The regression is in `baylee-ai/src/tests/x_target_tests.rs`;
six-seat training seed 891015 now finishes instead of looping on turn 118.

An exact mana-value X target is not monotone in the amount paid. The house
planner and its subsequent number answer restrict ordinary object-targeting
X spells to matching visible mana values at or below the payable bound,
including zero. They do not repeatedly cast Detonate at X=6 when only an
artifact of mana value 3 is available. Shroud, opposing hexproof and phased-out
objects are excluded; the engine still makes the authoritative target check.
`x_target_tests.rs` covers affordable, zero, unaffordable and protected targets.
The reproduced training fault is duel seed 1400020 (`next-duels-0000019`).

**One source per permanent is a rule with a consequence**, and it is the
reason a tap's *price* is part of the ranking. Nothing under `manaplan::plan`
keys on `ObjectId`, so the dedup in `policy::sources` is the only thing
enforcing that a land taps once — which makes the entry that survives it the
only mode the agent will ever use for that permanent. The key ranked on mana
made, then colours reached, and on nothing else, so a permanent printing a
free tap beside a priced one kept the priced one: **26 of the 33 faces in this
pool that print both**, measured on 20.09.2026 (#168). The population is
counted per **face** and not per card, which is not pedantry: Havengul
Laboratory prints the free tap on one face and the priced one on the other,
and a card-level count reads that as a permanent choosing between them when
no permanent ever can. Havenwood Battleground sold itself for a green it
already had; Spire of Industry paid a life whenever colourless was the whole
of what the plan asked for; the five Vivid lands removed a charge
counter even when the colour asked for was the one their free tap makes. `priced` now sorts ahead of the
amount, so free wins first and the old order decides between free modes. A
permanent whose *only* mana ability is priced is untouched, because the dedup
keeps one entry per permanent whatever the key says: this changes which mode
survives and never how many. Between permanents the solver ranks the price a
second time: `manaplan::Source` carries `priced`, and the matcher reaches for
a priced tap only where no clean one fits the pip (#210). Among clean taps
it keeps, as the client does, a creature first, then a permanent used for
more than mana at no mana of its own (Karakas: `CardDef::use_beyond_mana`,
handed to `manaplan::prioritize`), then the colours its hand needs. The
client and the language-model seat also count what the abilities on their
board wait for (`baylee_ai::board_claims`); the house does not, in every
generation (`docs/observed-faults.md` entry 78).
Command-zone commanders participate in these plans, including their public
cast-count tax, and so does a graveyard card the view says the seat may
flash back, at the price it names (`PublicObject::flashback`, #242): the
engine lists a graveyard spell as castable only once that price floats, so a
card the plan does not walk is one it never taps for. Steady and harder levels choose a mana colour by the casts it
can complete with the
remaining visible sources; an expensive, uncastable card cannot drown out the
colour needed for an affordable play.

A visible madness question is answered from the exiled card's projected
identity, not from its name or a hidden-zone fact. `policy::madness` reads
`FaceDef::madness` and accepts when `can_pay` covers that cost with X at zero;
the engine has already refused illegal targets and impossible non-mana parts.
The ordinary owed-payment planner then taps for the actual complete price. An
X answer uses the remaining sources when
`DecisionContext::alternative_cost = Madness`, independently of
`DecisionContext::cast_mode`, so a modal madness cast does not lose its mana
opportunity. `a_free_madness_cast_is_accepted_without_mana` covers the policy,
and `gamehost/tests/ai_madness.rs` plays a discarded Basking Rootwalla through
the real exile view onto the battlefield.

Counterspells need an opposing stack entry they would take off, and never
name one the view marks `UNCOUNTERABLE` while there is another: such a spell
is a legal target, and the counter resolves and counters nothing (#243).
Nor one that only replaces itself (`tactics::only_replaces_itself`: every
effect draws, scries, surveils or puts back, netting one card at most). Opt
and Brainstorm resolve, and a counter traded for one would be a card for a
card its caster was replacing anyway (#226). Among the rest, a counter names
the spell worth most: its own material plus the material of what of this
seat's it is aimed at, so Path to Exile on this seat's creature outranks
Ancestral Recall at the same cost. Everything but a cantrip is still
countered on sight: the worth ranks what is up at once, and
nothing holds a counter back for a threat that has not been cast. So a
Counterspell is traded for a lone Dark Ritual whose caster has nothing left
to spend the mana on; `a_counterspell_is_not_spent_on_a_spell_that_only_replaces_itself`
pins that as a limitation.
Removal needs something opposing on the battlefield, and a permanent this
seat's own spell or ability on the stack already removes — by destruction,
exile, tucking, lethal damage or lethal toughness reduction — is nothing to
aim a second one at (`worth::answered`, the mirror of `doomed`): the house
used to cast Swords to Plowshares at the commander its own Path to Exile was
taking, and Path lost its target
(`removal_is_not_cast_at_what_this_seats_own_removal_is_taking`). A deferred
pay-or-lose
obligation (a pact) is cast only as described under "A pact is a counter paid
for next turn" below. An "unless" price
paid by naming an object — a sacrifice, a discard, a card exiled from the
graveyard — is the opposite case and is paid, with the least valuable card on
the menu: it asks with `min: 0` because naming nothing is the refusal, and
refusing gives up the permanent that carries the price.

**A tax is answered by what refusing it does, not by what it costs.** Ward
(CR 702.21) refuses into countering the spell the seat has just cast; a
Rhystic tax refuses into one card for an opponent. Same price, opposite
decisions, and a policy reading the price could never tell them apart. So a
ward tax is paid whenever the mana can be found and the other stays declined,
because spare mana and mana the curve needs look alike to a stateless policy
while a card is the cheaper thing to give up. What refusing does is read
through a sequence: Power Sink's "unless" counters the spell and then taps
the lands, and it is paid, `{0}` included, as a counter is (NEW-15). Ward
is read a second time at target choice, since CR 601.2 picks targets before
mana is paid: a warded creature is priced against the material scale when
the seat can cover the spell and the tax together, and sinks below every
other candidate when it cannot — going around a ward costs nothing, walking into one it cannot pay
costs the card. **The window that follows a yes is answered too**: the engine
hands the seat priority to make the mana (CR 605.3a), and because a window is
an ordinary priority round with nothing castable in it, every path in the
agent's ladder used to pass — so it paid for a spell and then lost it, which
is worse than refusing. `policy::pay_owed` reads `PlayerView::owed` before
the land drop and taps toward the price, and **taps nothing toward a price it
cannot reach**: a seat that taps two of the three lands it needs has lost the
mana and the spell, where one that taps none has lost only what it had
already agreed to lose. It reads `awaiting` beside `owed`, because both ride
in every view and a seat taking one without the other pays for its
opponent's window. `crates/baylee-gamehost/tests/ai_ward.rs` plays it out
against the real engine.

**A pact is a counter paid for next turn.** Pact of Negation's delayed
`Effect::PayManaOrLose` used to refuse the cast outright. `policy::spell_score`
now casts it as a counterspell on two further conditions: every mana source
this seat has, untapped again, makes the price (`pays_next_turn`, CR 502.3),
and the best opponent's spell it could counter (its material plus what of this
seat's it is aimed at) is worth more than the price at 150 a mana, which is
then taken off the counter's value. The price goes on the stack at that upkeep
and is asked as it resolves, from what is still untapped, so this seat spends
nothing there while a payment of its own is on the stack: a pact's, echo,
cumulative upkeep (`tactics::asks_its_controller_for_mana`,
`a_seat_spends_nothing_over_its_own_echo`). Nor on an empty stack, where
anything could as well wait a step
(`a_seat_spends_nothing_in_its_own_upkeep`: a fetchland is cracked in the draw
step instead). Echo itself is `policy::pays_tax`'s question, like any upkeep
that sacrifices its permanent unpaid. Inside the window a dual
land's colour question is answered for the price
(`a_colour_named_in_a_payment_window_keeps_the_price_payable`). It used to be
answered for the spells in hand, so under `{3}{U}{U}` owed a Hallowed Fountain
named white, the Islands left could not make both blue, and the seat lost the
game it had agreed to pay for. Before that fix, casting pacts cost SHARP,
EXPERT and STEADY four to five points on the allytifact–victory pairing
against never casting them; after it, every profile is within two points of
never casting them (360 games each, 29.09.2026).

**An optional additional cost is paid when the pool already holds it.**
Kicker (CR 702.33a) and "you may waterbend" (CR 701.67a) are one question,
`YesNoPrompt::Kicker`, and `policy::kicked_price` answers both. The engine pays
the kicked total from the floating pool alone and refuses the whole cast when
it is short (CR 601.2h), so the planner floats the kicked price before it casts
whenever it fits beside the reserve, and `policy::kicks` says yes only when the
pool covers it. Waterbend counts every untapped creature and artifact that is
not a planned mana source, up to the waterbend's own generic mana, which is all
CR 701.67b lets them pay. Kicked is the better half by design, so it is paid
whenever it can be, except when the kicked half would draw the library out.

**Buyback is paid when the card is worth more than what it takes.** The
question comes with the plain cast, and `policy::buys_back` says yes only when
the floating pool covers the spell and the buyback's mana and the parts are
worth it (`buyback_parts_worth`): life while ten stays, a land past the fifth,
a discard out of the lands in hand past the one this turn plays, a random
discard where it mostly finds one. `policy::buyback_price` floats the mana
before the cast, unless it would take what another spell in hand could be cast
with: a Capsize brought back is not a creature played. An X spell spends
itself on its X, and is bought back only with what is left.

**Multikicker is kicked as far as the pool reaches.** `policy::aim` floats the
most payments the budget and the sources cover, and `policy::multikicks`
answers the count out of the pool, not the engine's maximum, which is open
while a land is untapped. Each kick is the better half by design, unless the
spell draws for it and would draw the library out. A controlled player's
buyback is declined and their spell not kicked (`controlled/choices.rs`): both
would buy them something (CR 724.5).

**Where a resolved spell goes is judged by whether the seat can pay for it
again** (`endings.rs`, CR 616.1e). The card back in hand keeps it at its full
price, and rebound is one free cast and then the card is gone. So a seat with
the lands for the spell and its buyback takes the hand, and one without takes
the free cast. A counterspell always goes home, because the next upkeep has
nothing on the stack for it to answer. Rebound beats an adventure, the creature
free against the creature paid for, and an exile that gives nothing back comes
last. For a player this seat controls the same list is read from the other end:
plain exile if it is offered, else what they would not have taken.

**Replicate is paid as often as the pool covers.** The engine offers as many
payments as the floating pool pays for (`NumberPrompt::Replicate`, CR
702.56a), and each is a copy, so the agent answers `max`. `policy::aim` has
floated them first: after a kicker that fits, the most payments the budget
and the sources cover, and the spell is cast only once they float. Each
copy's new-target question is `redirect.rs`'s copy case, as a Dualcaster
Mage's is: a copied Lose Focus is aimed at another of the opponents' spells,
never at the Lose Focus it copies. `resolving_about` looks for an ability of
the source before the source itself, because the replicate trigger's source
is that spell, still on the stack below it; read the other way round, the
copy took the spell's own target as its subject and was aimed at the spell
it copies.

**A Room's door is not unlocked by the agent.** The unlock is a special
action under a reserved index (`choice::unlock_door`), offered only once the
half's mana cost floats, and `activate::printed` finds no ability at a
reserved index, so none of the agent's ability choosers takes it, as none
takes turning a permanent face up. A Room the agent casts keeps the half it
was cast as.

**A tap question is answered with what the pool leaves unpaid.** Convoke and a
paid waterbend ask which permanents to tap (`TargetPrompt::Convoke`).
`policy::convoke_taps` measures the cast's price (the context's cost with X,
plus the waterbend once paid) against the floating pool and taps the fewest
that leave it payable. With no price to measure it taps as many as it may,
because an underpaid cast is rolled back whole. It taps an artifact first, then
on its own turn a creature that cannot attack yet, then the smaller body
(#224). `policy::spell_cost` subtracts no convoke help (it does for delve), so
the planner aims only at a convoke spell its lands could pay whole (#246). It
still casts one the moment the engine offers it, and the engine counts the
creatures: over four Plains and two Clerics, Clever Concealment is cast off two
Plains, and both Clerics pay the `{2}`.

**Restricted mana pays for the spells it names.** Mana that may be spent only
on some spells (CR 106.6), such as Ancient Ziggurat's creature mana, is read off
the ability before the tap, because the view says how much restricted mana
floats and never what it may pay for. `restricted.rs` taps such a source for a
spell only when `filter_matches` reads its filter as a match. An unreadable
filter (the commander's types) is a no; a chosen type is read off the source
(Cavern of Souls, `Filter::MatchesChosenTypeOfSource`). It taps at most one
restricted source per spell (the seat, which sends its taps whole, as many as
it takes), and that tap comes last: once the mana floats, no
plan counts it, and only the engine's merge makes the spell castable. Its mana
is spent first, so it never counts as held up. Its colour is named for a spell
it may pay for. A price floated before the cast (an X, a kicker) is paid from
unrestricted sources only. The restriction's kind of payment (`ManaUse`) is
asked before its filter, as the engine asks it: mana that may be spent only to
cast spells pays for no activated ability, so Delighted Halfling's coloured
mana is never planned for Yawgmoth's proliferate, nor Mishra's Workshop's for
an artifact's ability (NEW-05).

`ManaUse::LastLinkedExile` is stricter still: the house admits the source only
for the exact object identity projected in `BoardMana::restricted_to`, never for
an activated ability, a same-named spell, or a missing projection. Repeated
colors in a noted activation mixture are one fixed bundle rather than a menu of
alternatives, so the offer preserves both the amount and distribution of mana.

Searches, bottoming,
and surveils at the skilled levels evaluate only identities actually visible
in `PlayerView`, including `looking_at` while a search is open.

An agent is never handed the game log (#262). The host sends log lines only
to a seat answered over a socket, never to an AI chair or to a chair the house
holds for an absent player
(`session::tests::a_seat_the_house_answers_is_never_told_the_log`). What an
agent knows of the past is what its view shows now.

**The house plays as a team, and teammates show each other their hands**
(owner, 10.11.2026). This replaced #265's rule that no agent is handed a
teammate's hand. What a house chair is shown is still only what a player in
its chair could be shown, by a share the table's `HandSharingRule` permits,
and `Table::show_shared_hands` reads the rule, the sides and `has_left`
again for every view it builds (`Table::house_view`, which `house_answer`
answers from):

- *House with house.* Two house chairs on one side show each other their
  hands without asking, where the rule is `Allowed` and
  `AiHandSharePolicy` shares continuously (`Table::house_shows_house`).
  Under `OnceOnly`, `Never` or `OnRequestOnce` they show none.
- *House with a person.* The person decides. The house asks each person on
  its side once, when the table is made (`Session::house_asks_its_teammates`,
  their `hand_requests`), and reads their hand only if they show it; it
  shows its own to a person who asks, as before.
- *A chair the house holds for an absent player* plays with the hands its
  player was shown, since the shares are its player's; it neither asks nor
  answers a request, which wait for the player
  (`view::tests::a_stand_in_plays_with_the_hands_its_player_was_shown`).
- Never an opponent's hand, nothing under `Disabled`, and a share ends when
  its owner leaves the game or loses (`session::tests::shared_hand`:
  `house_teammates_show_each_other_their_hands_and_never_an_opponents`,
  `the_tables_rule_holds_for_the_house_too`,
  `a_teammate_who_has_left_shows_no_hand`,
  `the_house_asks_a_person_on_its_side_and_reads_only_what_it_is_shown`).
- The frozen reference `sharp-2026-11` is shown no hand, shows none to a
  house teammate and asks nobody, so BayleeAI's benchmark tables play as
  they did (`the_frozen_reference_shares_no_hand_with_the_house`); `seer`
  peeks anyway.

What the house does with it, and with its side, at `AIProfile::TEAM_PLAY`
(`src/tests/team_tests.rs`, each test asking the frozen reference too):

- An ally's removal or counter on the stack is the side's: nothing is aimed
  at what it is already taking (`worth::answered`).
- An ally's board is the side's: a sweeper that takes an ally's creatures is
  priced with them as losses (`sweeper_net_worth`).
- An ally in a lower seat who holds a cheap instant (a counterspell, from
  `AIProfile::INSTANT_TIMING`) and the untapped lands for it holds the answer
  up for the side, and this seat develops instead of keeping its own mana up
  (`an_ally_holds_the_answer`); the lower seat keeps its own, so two allies
  who see each other never both tap out.

The audit found the rest already playing as a team: targets are chosen as
friendly or hostile by side (`HeuristicAgent::hostile`, and a counter weighs
what a spell is aimed at of the whole side), and the engine offers an attack
at opponents and their planeswalkers alone (`combat::defender_options`),
which `tests/team_play.rs` holds every level to on a two-against-two table.
The engine asks each defending player for the blocks of what attacks it
(`Engine::ask_blockers`), so in the formats it plays no block protects an
ally; a shared-life format would change that.

Measured on 10.11.2026 (commit a83eca936's selfplay build, CPU):

- *Exercised.* Over 40 two-against-two tables on the five house decks,
  every chair `sharp` (`session::tests::team_exercise`, run by hand), a
  teammate's hand was in view at 122,379 of 125,453 house decisions
  (97.5 %), and the answer differed from the frozen reference's in the same
  chair at 39 of them (0.03 %), every one with a hand in view. Per deck the
  share in view was 95.8–99.2 % (Victory 95.8 %), and the answers changed
  2 (Victory, maik) to 23 (schwarzrand) times.
- *Worth.* eval4 `--random-policy`, house against house on the campaign's
  test decks (`decks-20261107c`, constructed), 512 deals per format and run,
  `sharp` against `sharp-2026-11` and swapped, read by `house_duplicate`,
  which counts each game once: eval4 rotates the focal seat over a deal's
  seats with one seed, so a run repeats each game once per seat of a side.
  Two against two: 133 of 252 decided games (52.8 %, Wilson 95 % 46.6–58.9);
  team play turned 7 seeds (won from both sides) and lost none. Three against
  three: 85 of 168 (50.6 %, 43.1–58.1); 3 seeds turned for it, 2 against.
  Small, as the 0.03 % says it must be, and in its favour where it shows. No
  deck stands out; the constructed test decks hold no Victory (a Commander
  deck), so Victory's team play is measured above only by how often it is
  exercised.

**A mode is chosen by what it reaches, not by where it is printed.** A modal
spell whose every effect sits under a mode is offered no normal cast
(CR 700.2a), and the answer to a cast question used to be the position of a
`Normal` option — which for such a spell is not there, so the first printed
mode was taken at every table. What is left to read is narrow, because the
engine has already dropped every mode whose targets cannot be chosen
(CR 700.2a for a spell, CR 700.2b for a trigger): what it does not decide is
the mode that targets nothing and reaches nothing anyway, Sheoldred's Edict
asking an opponent who controls only a planeswalker to sacrifice a nontoken
creature. That is read from the `PlayerView` by the crate's own `Filter`
reader, and the reader is three-valued on purpose — `MatchesChosenTypeOfSource`,
`AttachedToBySource` and `SharesSubtypeWithCommander` name `GameState` fields
a projection has no counterpart for, and `IsToken` joins them for an object
carrying neither a printing nor a token handle, because a token copying a card
and a permanent the seat may not look at are one shape in a view and opposite
answers in the rules. Answering `false` for "cannot see" would make a guess
look like a reading. A mode that reaches something beats
one that reaches nothing, an unreadable mode sits between them, and the
printed order breaks every tie, so a table the agent cannot read is answered
exactly as before. A card that also offers a normal cast keeps it: overload
prints a mode that costs more than the card does. The exception is an
alternative cost that costs nothing at all, such as Deadly Rollick while you
control your commander. The engine offers `Normal` only when the pool already
covers it, so taking `Normal` there spent floating mana on a free spell. A pitch
(Force of Will) or an evoke (Mulldrifter) still costs a card or the creature,
so it keeps the printed cost when that can be paid.

**What a card says includes what it says behind a price.** Every reader here
that walks an effect list — `tactics::meaning`, `worth::effect_worth`,
`intelligence::sweeper` — asks
`baylee_cards_dsl::Effect::branches` which effects run another effect, rather
than listing them itself. Each of them used to carry its own list (there were
five readers then: `activate`'s `gains`, `harmless` and `draws` are one table
in `worth` now), each list was short, and all five stopped at `Sequence` and
`MayDo`: an effect printed
behind "unless you pay" sits in a variant carrying a single effect rather
than a list, so it was read by none of them. Thirty-five effects in the pool
are there — twenty-nine of them a Karoo land sacrificing itself — and two are
counterspells, Flusterstorm and Malevolent Hermit, which the agent held for
ever because `policy` only casts one it knows is a counterspell. `worth`
averages the two halves of an effect only one of which runs, and prices a
draw that would empty the library as the loss it is (CR 704.5b), large enough
that half of it still refuses: over-counting refuses a safe draw,
under-counting decks the seat out, and only the first is allowed.

**What an object can do is printed on `rules`, and which card it is on
`card`.** The two part for a copy (CR 707.2), and an offered ability is a
`(source, index)` whose index counts into the copied card's list. Reading
`card` made a Glasspool Mimic that entered as a Werefox Bodyguard look the
Fox's second ability up on a Mimic that prints one, so the copy was never
used, and where the card underneath did print something at that index the
agent weighed one ability and pressed another (#214). So every reader asking
what an object does — `activate::printed_list`, and through it `worth`,
the shallow loyalty rule and the mana estimate; `filter`'s modal modes;
`tactics`' ward and counter clock — reads `PublicObject::rules`, and `card`
stays where the question is the card itself: a spell cast from hand, a
commander's colours, `IsToken`. A token copy has `rules` and no `card`, so
the agent now reads its abilities too. That is not a leak: the view gates
`rules` on the same entitlement as `card`, and a face-down permanent the seat
may not look at names neither. A registry token has neither, and its
abilities are its definition's (CR 111.3), which `token` names, so a Treasure
is a mana source (#223). A face-down one has no text (CR 708.2).

**What using an ability is worth is one measure, net of what it costs.**
`worth.rs` answers three questions that used to be answered three ways:
whether an activation is taken (`activate::choose`), which loyalty ability a
deep profile uses, and which target an activation names once taken
(`HeuristicAgent::ability_targets`, asked from `tactics::targets`). The first
was a whitelist of effects that are gains wherever they land (a draw, a token,
a search) and a refusal of every cost that sacrificed, discarded or exiled
another card (`activate::useful`, `gives_up_a_card`); the third ranked
candidates by the sign of the effect list. So an ability that is good or bad
by its target or its cost was never taken: no Wasteland, Maze of Ith,
Recurring Nightmare or equip, and no Sea Gate Loremaster, whose "a card for
each Ally" was an amount the whitelist could not count. Now
`activation_worth` is the effects at their best target (`aimed_worth`) less
`cost_worth`, taken when it clears `worth::THRESHOLD` (25), and the target
question is answered from the engine's offer by the same numbers, so the
target named is the one the activation was valued for.

The currency is the crate's usual one: a card is 400, a permanent is
`tactics::material`, a point of life is `worth::life_price` (30 above ten
life, 60 down to six, 150 at five and below), and an effect that ends the game
is `LETHAL` or, for a draw from a library too short, `DECKED`. A draw trigger
on the table (Orcish Bowmasters, Sheoldred) works for whoever controls it. A
land is not a body: `land_worth` is a mana a turn, worth more the fewer lands
its controller has, and more again when it does anything besides make mana,
which is Wasteland's reason to exist. Costs are priced by what they give up
now: a mana by `mana_price` (little on an opponent's turn, almost nothing in
an end step, more while an instant is in hand); tapping a creature by the
blocker it stops being, and not at all before its own attack, where the answer
is still "after combat"; a sacrifice by `given_up`, which is nothing for a
permanent already `doomed` (an opponent's spell or ability points at it, or
this combat kills it). What the table cannot read is `None` and the ability is
left alone, as the whitelist left it, so a mechanic added tomorrow is inert
rather than misplayed; `the_table_reads_most_of_what_the_pool_activates`
counts the share read over the pool (878 of 1,010 activated and loyalty
abilities on 29.09.2026) between a floor and a ceiling, and pins one it must
not read (Aminatou's −6, a control rotation).

The zero point is doing nothing, which is why several rows stay unused on
purpose, each with a test for both sides in `src/tests/worth_tests.rs`:
Homeward Path is used when it brings home more than it hands back; Loran of
the Third Path's draw for both players only when a draw trigger of this
seat's makes the opponent's card cost them; and Liquimetal Coating and
Liquimetal Torque not at all, because a type change is worth nothing until a
card asks about the type, and no card in these decks does. Before this rule,
with nothing to cast, Zuran Orb was fed four Forests on turn 1 for 8 life,
and Viscera Seer sacrificed itself to scry 1; both are still refused, now
because a land on turn 1 and a creature are each worth more than what they
buy (`a_card_is_not_given_up_for_a_small_gain`).

**Loyalty is a price, and the walker's life.** SHARP and EXPERT choose among
loyalty abilities by the same measure, at a floor of nought, since the cost is
already charged: 45 a counter, 300 more for a minus that empties the walker.
A text the table cannot read is weighed by `effect_value`, as the deep
profiles weighed every loyalty ability before, because left alone Teferi,
Time Raveler's +1 and Karn, the Great Creator's +1 were never used and both
walkers spent themselves on their minus. Loyalty is also what an attack
removes (CR 306.8): `reach` sums the power of every hostile creature that can
attack, and a change that takes the walker into that reach costs half its
material, one that lifts it out gains as much, and one that leaves it where
it stands costs nothing. So Venser, the Sojourner's −1 is used when the
unblockable attack kills and its +2 otherwise, Elspeth, Storm Slayer's 0 when
its flying makes the attack lethal, and Aminatou's −1 to bring home a
creature an opponent holds. Jace, the Mind Sculptor's Brainstorm is now used
more and its bounce less than before (221 and 12 uses against 132 and 88 over
the scoreboard below): a card is worth more than a creature's tempo unless the
creature is what kills. NOVICE, CASUAL and STEADY keep the old rule (the
ultimate when affordable, else the largest plus), and a scry, surveil or
reorder is valued only at `mulligan_skill >= 2` (STEADY and up); both are designed
differences, not gaps.

**Mana for an ability.** The engine offers an ability with a mana cost only
once the pool covers it (`Engine::can_afford`), and until
`AIProfile::ABILITY_MANA` nothing in the agent made mana for one the way
`policy::aim` floats it for a spell. So these rows stayed at or near nought
whatever `worth` made of them: Riptide Laboratory (worth using only to save
a doomed Wizard, which is at instant speed in answer to removal), Sensei's
Divining Top (a reorder, worth something from STEADY up), General Tazri's
five-colour pump, and the equip costs of Sword of Hearth and Home and Helm
of the Host (taken the few times the mana floated). `sharp` and `expert`
(`HoldUp::ThreatAware`) now tap toward the ability worth most
by the same measure (`activate::floating`), one source at a time, within
the mana a main phase keeps up: one printed on a permanent of theirs, used
from the battlefield, with no limit or condition, a fixed mana cost and at
most a tap of itself besides, at a moment its timing allows. Once the mana
floats the engine offers it and `activate::choose` takes it. The levels
below and the frozen reference leave such an ability as before. It changed
22 of 1,024 seeds against `sharp` without it (train split, 15 for, 7 against):
few decks print such an ability that `worth` prices above its cost. Two
rows have no reader either: nothing in the agent answers the engine's
suspend offer (`LegalActions::suspendable`: Ancestral Vision, Profane
Tutor), and Raffine's Tower's cycling is a card in hand, which
`activate::printed` does not reach.

**Inspirit, Flagship Vessel was a card finding, not an agent one.** Station
is "tap another untapped creature you control" as a cost (CR 702.184a). The
card wrote the tap as an effect, `TapTarget` under `Cost::FREE`, and its
target filter (`ANOTHER_CREATURE_YOU_CONTROL`) did not say untapped. A free
cost with no parts is the one shape `activate` refuses outright, since the
same offer returns unchanged, so the agent never stationed; and the engine
let an already tapped creature be named again, which a real station cost
would not. It and U.S.S. Enterprise-D now write the cost the rule spells
(`CostPart::TapOther`, `Amount::TappedPower`), which `consumes` counts as
consuming, and `lints::every_station_is_the_ability_its_keyword_spells` holds
every printed station to that shape. Whether the agent now stations well is
unmeasured.

**An attacker the view cannot describe is unknown, not absent.** `Fighter::of`
is three `?` in a row — the object, its power, its toughness — and every
`None` used to reach the decision as "no such attacker". It reached it twice:
the creature left the damage sum, so `lethal` was false, and it left every
blocker's candidate list, so there was nothing to pair with. The seat then
answered a lethal attack with no blocks at all, which is the one outcome the
"do not die" rule exists to prevent. The attack is now read out of **both**
sources at once: `view.combat.attackers`, which carries the whole attack
including what this seat may not block, and the engine's pairings, which are
the authority on what is attacking *this* seat and the one source that
survives a view the attack cannot be read out of. In a healthy game the
second is a subset of the first and the union is the first, so nothing moves;
where they disagree, each covers the other's silence. That matters in both
directions, and the second is the easier one to miss — a creature that reads
perfectly, on the battlefield with a power and a toughness, that
`view.combat.attackers` does not name, used to contribute nought to the
damage sum and leave the position reading as safe. An attacker named in the
pairings that cannot be described is counted as unknown damage, which makes
the position lethal by default and is chumped: the seat cannot prove it
survives, and a creature costs a card while the alternative costs the game. `search::blockers` had a
guard for exactly this condition — it falls back on
`attackers.len() != ids.len()`, which *is* "an attacker did not read" — and it
fell back onto `choose_blocks`, which shared the blind spot. A fallback onto
the same reader is not a second path. The guard now also requires the view to
say what each attacker is aiming at, because the search reads that and
`choose_blocks` does not need it.

**No profile attacks into a swing back that kills it.** Neither blind spot
above was ever shown reachable in a real game; the owner's #123 game is
reproduced from the other side of the table. A 75/75 first striker attacks,
and on the house AI's turn it is still tapped, so nothing can block and every
attack rule says swing: NOVICE, CASUAL, STEADY and SHARP sent all eight 2/2s
for sixteen into twenty, and the creatures stayed tapped through the next turn
(CR 502.3), when the 75/75 came back into a table the engine could offer no
block on. SHARP did the same with the 75/75 untapped. Only EXPERT prices that
retaliation, and injected out of its search it sends all eight too.
`combat::hold_back_for_the_crack_back` is therefore a pass over the finished
attack for **every** profile, the way the menace pass below is a pass over the
finished block: block rule 1 seen from the other side, not a skill level. If
the attack does not end the game and some hostile seat's whole board, which
untaps first and is not summoning sick by then (CR 302.6), would get through
what stays home, creatures are kept home one at a time, the one that stops the
most first, until it would not. Vigilant attackers count as home (CR
702.20b); flying, reach, menace and trample are read through the same
`Fighter` model, and leftover blockers soak a trampler's excess. A table that
dies whatever it keeps home attacks as it meant to. Each hostile seat is asked
on its own, so damage that several opponents add up to over one round, and
commander damage and poison, are not modelled here. SHARP and EXPERT still
part on a swing back that does not kill, which EXPERT prices and SHARP does
not look at.

**A walker is blocked for when the attack would kill it.** Only attackers
aimed at the seat count toward its life (CR 508.1b); the shallow path used to
sum every attacker in combat, so a 5/5 at its walker, or at another seat, made
a 3/3 read as lethal and the only 2/2 was thrown away. Damage aimed at one of
its walkers is kept per walker against loyalty (CR 120.3c, CR 704.5i). A
creature worth less than the walker chumps when that block brings the damage
below the loyalty; a walker that survives the hit or dies anyway is not worth
one, and the seat's own life comes first. The search prices the same thing:
each counter on the creature scale, the whole walker once the damage reaches
its loyalty, including damage from attackers nothing can block. The shallow
path saves a walker with one block at a time and sends the whole attack at
one defender; the search sees multi-blocker rescues.

**An attack that wins goes at the player, and a walker gets what kills it.**
Self-play game r001 #431 never ended: Allytifact's Ally tokens doubled every
turn to tens of thousands, and every turn they all went at the walker Victory
recast from the command zone, so Victory's life never moved. Two readings
made that. Only the search's proof (`AttackSearch::lethal`) sent an attack at
the player past a walker, and the search never runs for the three shallow
profiles and gives up above sixteen creatures a side; and whenever the squad's
power reached a walker's loyalty, the whole squad went at that walker, however
much of it the walker needed. Now every profile also asks
`combat::breaks_through`, the estimate of what gets past the victim's untapped
creatures that the crack-back pass has always used to tell a winning attack
(`through`: greedy blocks, evasion, menace, trample), asked once per decision
and handed to that pass; where it reaches the victim's life the whole attack
goes at the player (CR 704.5a). The search's proof still counts, but only for
the squad it searched, not one the crack-back pass has since thinned. An
attack that does not win keeps the walker rule — the cheapest walker whose
loyalty the squad's power reaches, never one it could only chip — and
`combat::aim` sends it the weakest attackers, fewest first, until what gets
past every untapped blocker reaches its loyalty (CR 120.3c, CR 704.5i); the rest
go at the player. The walker dies as surely as before, and what it did not
need is pressure on the player at no cost to the walker plan; a walker the
blockers could save whatever is sent still gets the whole squad, as before.
That is all the weighing there is: no walker is valued against life points,
and an attack is never split between two walkers or two players.

This is not a skill level, for the reason the crack-back pass is not: a game
that never ends costs every table, and the profiles differ in *which*
creatures attack (the search or the greedy pick), not in noticing that the
ones attacking win. NOVICE's misses are its ±800 spell-score noise and its
individual trades, and it keeps both.

What it did to the teacher, measured on 29.09.2026 with `ai_match` over seeds
1–50 (200 games per pairing, both deck orders and both seats): the profiles'
shares of decided games did not move (expert over sharp 49.5 %, novice over
casual 39.5 %, sharp over steady 58.5 %, steady over steady 50 %), 8–29 % of
the games played differently at all, 2–12 per pairing changed winner, 20 of
the 24 to Allytifact, whose share rose by up to four points, and finished
games got 0.1–1.0 turns shorter on average. No game in that set was capped
before or after. The games that never ended are elsewhere: at steady against
steady, seed 1042 took 19 minutes and seed 222 hit the 20,000-action cap in two
of its four games, and now all eight finish in 0.2 s and 0.1 s; self-play
r001 games 431, 1055 and 2091 hit the trainer's 60-second cap and 685 was
still running when the run gave up on it after ten minutes, and all four end
in turns 29 to 58 now.

**The house does not loop on its own answers.** Self-play mac-d001 (01.10.2026)
ran 1,881 of its 50,000 house-against-house games into a cap, and nearly all
of them were the house repeating itself in four shapes. Each is now ruled
out without memory, from the view alone, so it holds for every host,
including the two that build a fresh agent for each question
(`Session::house_action`, `baylee-seat`):

- *An ability is not activated again while an activation of it waits on the
  stack* (`activate::on_the_stack`). The view shows no effect before it
  resolves, so the second activation was judged on the first one's board
  and taken again, thousands of times in one round of priority (Hopping
  Automaton, Flowstone Hellion, every `{0}` pump). After it resolves the
  active player gets priority again (CR 117.3b) and anything it answered is
  still on the stack, so nothing is lost but stacking several before anyone
  can respond. Mana abilities never use the stack (CR 605.3b).
- *A free ability is refused unless its worth runs out by itself*
  (`activate::settles`). A printed `{0}` is one generic symbol of nothing,
  not `ManaCost::ZERO`, so it had passed for a cost that spends something.
  Two shapes are taken anyway: an equip, whose worth is a difference (what
  it gives where it goes less what it gave where it hung, the attack it
  makes possible counted on both sides, so a move back is never worth it
  too), and a pump of the source until end of turn, which is worth the
  damage it adds and must add more each time, up to the defender's life. A
  pump that takes the creature's toughness to nothing is worth losing the
  creature (CR 704.5f). Lightning Greaves's equip, `Cost::FREE`, is taken
  now for the same reason.
- *Shuko does not travel between two creatures that could both attack with
  it*: the swing was counted only where it went, so each move was worth
  it.
- *An `{X}` creature is not cast for an X that leaves it no toughness*
  (`policy::stillborn`). With no mana the only X is 0; Shifting Wall and
  Walking Ballista died as they arrived, came back and were cast again in
  the same main phase. The X itself was never above what the pool pays:
  `number` plans it against the pool.
- *A land that costs life is not played out of the graveyard, nor below the
  payment margin* (`land::pick`, feedback of 28.10.2026). Fastbond deals
  its controller 1 for each land after the first; with a permission to play
  lands from the graveyard and Petrified Field returning one to the hand,
  the house standing in for an absent seat played lands from 27 life to 0
  in one turn. The view does not say how many lands the seat has played
  this turn, so every land play is priced at what the triggers would deal
  if all fired (the seat's own "whenever you play a land, … damage to you",
  anybody's "whenever a land enters, … to that land's controller": Ankh of
  Mishra, Zo-Zu the Punisher). A priced land is played from the hand only,
  and only while life stays above price + 5, the margin `activate::life_ok`
  keeps; at six life under Fastbond even the free first land waits.

Replaying a seeded sample of 300 of those games (`selfplay --only`, same
arguments), 300 hit a cap before and 1 after; the others finish, at a median
of 673 answers. The one left is Earthcraft untapping two animated Forests
that tap for each other. A count of repeated positions would catch it, but
the view carries no position hash and an agent built per question could
not keep one; the trainer counts at the host (`Session::snapshot_hash`).

**One illegal pair costs the whole declaration, so legality is checked
against the finished answer.** Menace is two blockers or none (CR 702.111b),
and the shallow path pairs one blocker with one attacker by construction — so
`NOVICE`, `CASUAL` and `STEADY` answered a menace attacker with exactly the
declaration the rules forbid, in every shape tried, whatever the search's own
guard was doing. `Engine::declare_blockers` refuses the *whole*
`DeclareBlockers`, not the offending pair, which is why `combat::choose_blocks`
now takes a pass over the list it is about to send rather than judging each
attacker as it goes: a block that is illegal in isolation takes every other
block down with it, and the seat stops at the question. Where a second blocker
may legally be paired it is added — the cheapest one, since it is being spent
to satisfy a rule and not to win an exchange — and where none can be, the
block is dropped. The deeper profiles never needed the pass, because `search`
evaluates the two-blocker leaf on its own merits and refuses the one-blocker
one; what they needed was a scenario in which that refusal decides anything,
which is a 2/6 against a 6/6 and a 2/2 and not the 4/4 that first suggests
itself. That precondition is gone: #156 landed, `combat::can_block` no longer
asks a question it answers before anything is recorded, and the offer now names
a menace attacker wherever two creatures could legally block it. So all of this
is reachable, and the pass above is load-bearing rather than defensive.

What it is worth is **the table, not a point of evaluation**. A refused
`DeclareBlockers` is not a worse block; it is no answer at all, and for an AI
chair there is nothing behind it: `Session::pump` passes priority when the
engine refuses an action at a `Pending::Priority`, and a `ChooseBlockers` is
not one, so it takes the other branch and returns without advancing that
question. No clock expires either — the decision clock is for seats that
answer over a socket (`answers_over_socket`), and an AI seat has none, so
nothing answers on the chair's behalf the way a timeout would. Whether a
later `pump` recovers or meets the same refusal is **unmeasured** and is
#180's to settle; either way the pass above is what keeps the table off
that path. Three
of the five profiles reached that in every shape tried, and the seat it costs
is the one that was trying to block. The
deeper two were never at risk, so the pass is what makes the shallow
profiles' answers *arrive*, and the search's own rule is what makes them
good.

One position in `no_profile_answers_a_menace_attacker_with_one_blocker` is no
longer a board a game reaches: `combat::menace_satisfiable` drops a menace
attacker from the offer entirely where only one creature could legally block
it, so the single-blocker scene arrives from no real table. It is kept, and
its comment says why — `choose_blocks` computes an answer to that shape
whether or not anything presents it, and a pass tested only on offers the
engine has already filtered is a pass nothing tests.

**What the rules make a creature do comes first, and the seat's choice
fills the rest.** Two sentences take a declaration out of the seat's hands:
a creature that attacks each combat if able (CR 508.1d) and a creature
that must block if able (CR 509.1c: a lure, "blocks each attacking
creature"). The question names both — `required` on an attack, `obeying` on
a block — and `combat::obey_attack_rules` and `combat::obey_block_rules`
fit the seat's answer to them after the choice is made. A block keeps the
seat's pairs for every blocker `obeying` does not use and takes `obeying`'s
for the ones it does; a menace attacker the seat meant to block with two,
one of which the requirement took, is then not blocked at all, since the
engine refuses the whole declaration for one lone menace block. The choice
itself does not yet weigh the requirement: a lured attacker is blocked by
everything able because the rules say so, not because the seat judged the
exchange.

**Every answer is held to its question before it is given** (`held`). A
question states every reason the engine refuses an answer to it
(`Pending::answer_fault`, `docs/pending-constraints.md`), so the agent checks
its own answer against it, and an answer that breaks one is a defect in the
picker that built it: "up to four" targets answered with eleven (Meloku's
Illusions, found by the fuzzer, seeds 486 and 1931 on 50050ff3) was one. Such
an answer is refitted to the nearest one the question takes — the picker's own
choices in its order, less repeats and what was not offered, cut to the
maximum and made up to the minimum, an opponent's first; else the answer that
does nothing; else the least offered — so no answer outside what its
question states reaches the engine (a refusal for a reason the question does
not state still can, and is counted as before). It is never silent: the agent logs a warning with
the answer, the fault and the question, and counts it
(`HeuristicAgent::fallbacks`, `Tally::fallbacks`). The self-play sweeps
(acceptance decks, multi-seat tables, AI deck matches, every implemented
card's game) assert that count and the engine's refusals both stay at zero,
so a picker that breaks a bound fails a test instead of being papered over.

**A modal card in hand is what either of its faces can be.** A
`CardIdentity` in hand names the face that is *up*, which for a modal
double-faced card is the spell: Shatterskull Smashing is a sorcery with a
land on its back, and every reading that asked `HandObject::types` counted
nought lands. The engine offers that land drop (CR 712.9), so the agent was
throwing away a hand whose land drops the engine was about to hand it, and
keeping a card it had already decided was a spare spell. A *transforming*
card is not that: in hand it has only its front face's characteristics
(CR 712.6a), and Arguel's Blood Fast reaches its Temple only by turning over
(#152). `castable_from_hand` tells the two kinds apart, held against
Scryfall's `layout` by `xtask validate`. `policy::plays_as_land` asks the
engine's own function, `CardDef::land_faces_from_hand`, so the agent and the
offer it answers cannot disagree: `a_land_on_a_modal_back_is_playable_and_one_on_a_transforming_back_is_not`
floors both populations and pins one card of each, and
`the_pool_prints_lands_on_a_back_face` floors the modal one.

The *cast* faces are a separate reading and they are **not** made yet.
`filter::cast_mode` returns the `Normal` option the moment one is offered, so
an adventure (CR 716) or an MDFC back is taken only when the engine offers
nothing else — which, because it offers only modes the pool can pay for, is
how Petty Theft gets cast on two mana: the right face, and not a decision.
Which half is better needs a value model for "a creature now against a bounce
now" that this crate does not have, so the behaviour is pinned by
`an_adventure_is_offered_and_the_agent_takes_the_printed_front` rather than
claimed, and the row stays open. The same cause will show on two more rows
once `c42/cards-library`'s cast modes reach this branch: the agent always
casts Ragavan for its mana cost over Dash (`CastModeKind::Dash`), and under
Muldrotha it casts Uro for its mana cost rather than escaping it, so Uro is
sacrificed as it enters. The rule wanted is the same for all three: when
several cast modes are offered, value what each leads to.

`act(&PlayerView, &Pending)` remains available and needs no hidden information.
Hosted AI seats additionally receive the selected spell/ability effects from
`Engine::decision_context`, covering cast modes and triggered or copied abilities.
This lets targeting distinguish beneficial counters and buffs from removal,
rank threats, and take lethal burn over a smaller permanent. SHARP and EXPERT
planeswalkers price the actual effect, the loyalty spent and the walker's
exposure (see "Loyalty is a price, and the walker's life"). Creature-type choices follow the hand, battlefield and all
commanders. Counterspells distinguish spells from activated or triggered
abilities, and beneficial player-targeted draw goes to the caster. X uses
affordable coloured mana and the number of distinct legal targets; life-X
weighs friendly casualties and preserves the player's last life. Miracle
checks the actual coloured cost against floating mana and available sources.
Accepting a miracle can open a mana-only payment window after X and targets;
the projected debt drives the same planner that pays ward. The casting route
in `DecisionContext::cast_mode` lets Miracle's X include those sources too.
`tests/ai_miracle.rs` earns the offer in a real game: with no floating mana,
Temporal Mastery taps exactly two Islands, pays {1}{U}, and grants an extra turn.

**Keeping one permanent of each kind is a set decision.** For
`KeepOneOfEachAndSacrificeRest`, the house values the whole retained set rather
than picking each category's best permanent independently: an artifact creature
may cover the artifact choice while a second creature survives as the creature
choice. `best_keep_one_of_each` uses deterministic dynamic programming over the
kept identities, collapses candidates with identical category signatures, counts
a reused multi-type permanent once and skips empty categories. It stops rather
than guessing above 50,000 states or 64 filters. A concrete menu is optimized
only when its legal objects identify exactly one category; ambiguous menus keep
the ordinary deterministic fallback. The same retained-set model prices the
sweeper before casting, so an equal sacrifice exchange is not mistaken for a
profitable one.

**A clone copies what is worth having twice, whoever controls it.** Its choice
arrives as a target question with no effect behind it, because copying is an
ability and not an effect, so `decision_context` used to explain it as nothing
at all and the fallback took an opponent's permanent first: Phyrexian
Metamorph copied the opponent's Llanowar Elves over its own controller's Serra
Angel (#227). `DecisionContext::copying` now names the question and the copy's
modifications, and `copying::copy_target` ranks every candidate by its
copiable values (CR 707.2: the printed size, not a pump or a counter) and
always names one, since most clones that copy nothing are a 0/0. A legendary
permanent this seat already controls is ranked last unless the copy stops
being legendary (Spark Double), because the legend rule keeps only one
(CR 704.5j). Open: lands tie on this value, so Vesuva and Echoing Deeps still
take the lowest id; and Sakashima, whose own static switches the legend rule
off for its controller, is ranked as if it did not.

**A redirect or a copy is answered by the spell it turns or copies.**
Misdirection, Deflecting Swat, Hydroelectric Specimen and Dualcaster Mage each
ask two target questions, and their own effect means nothing to
`tactics::meaning`, so the fallback's opponent-first rule answered both
(#226): Misdirection turned the agent's own Path to Exile from Serra Angel
onto an Ondu Cleric, a turned Path went to the first creature on offer rather
than the best, and Dualcaster Mage's copy of the agent's Path went where the
Path already was. `redirect::stack_targets` answers both now. *Which spell*: a
redirect is worth what of this seat's the spell it turns is aimed at, when
that spell is an opponent's and hurts (`HeuristicAgent::redirect_worth`), and
`policy::spell_score` asks the same number before casting one, so a redirect
with nothing of this seat's under attack stays in hand. A copy is worth what
the copied effect does on this board, whoever controls the original, because
the copy is this seat's. *What it becomes* is asked as the redirect or copy
resolves, and is answered by what the turned or copied spell means: a Path
goes to the best creature across the table, and a Lightning Bolt goes where it
kills, the caster's face at 3 life. What a harmful spell on the stack is
already aimed at scores nothing, whoever cast it, unless the turned or copied
spell is a gift; a copy is one such spell, since it starts with the original's
targets (CR 707.10) and may change them (CR 707.10c). The two questions are
told apart by the stack: the resolving redirect or copy is on it, and a card
being cast is not yet. Open: a spell's X is not in the view and reads as 0; an
opponent's gift to their own creature is not turned onto this seat's; the
Specimen does not weigh that the new target is itself; nothing holds a
redirect or a Dualcaster Mage back for a later spell; and an opponent's
redirect aimed at this seat's own spell is not turned back, because a redirect
hurts nothing by itself.

**A controlled turn is played against its player.** Under Mindslaver the
house makes every decision of the player whose turn it controls (CR 724.1,
723.5): their casts and targets, their attack and its damage, their
resolution choices, their discards. The engine asks that player and awaits
the house, so `controlled::subject` reads the question as theirs when the
view awaits this seat and the question names somebody else; a question of
the house's own in that turn — its blocks, its own priority — is never
theirs and is answered exactly as before (CR 724.8). The measure is the
crate's own read from their side of the table: `controlled::seen_by` makes
the view theirs (their hand where the house's was, their seat as the view's)
out of nothing the house does not already see (CR 724.4), and the answer
taken is the other end of what they would take. Only their resources pay
(CR 724.5a); the payment readers already go through `resource_player`.

- *Priority* acts only once their combat is over, so nothing is spent on a
  creature the house's block was about to kill. A spell is cast where
  `casting_harm` says the worst it can be aimed at costs them more than
  their own best use of the card would have gained them, and never where
  even its worst aim gains them anything: their Lightning Bolt at their own
  creature or face, their Giant Growth or Holy Strength on the house's
  creature, Pacifism on their own, their Ancestral Recall for the house, a
  draw that empties their library. Damage that does not kill a creature
  after combat is worth nothing (CR 514.2), so a Bolt is not left on a 4/4.
  Every permanent but an Aura stays in hand, no land is played, and a text
  `worth` cannot read is not cast. An activation is taken by
  `activate::against`, the worth at its worst target net of its cost, so a
  sacrifice outlet is fed their creature; only a cost that consumes
  something. In their end step every source of theirs that only makes mana
  is tapped, so it stays tapped through the house's turn (CR 502.3). A
  payment the house took on for them (Word of Command) is finished as
  before.
- *Their attack* sends what an untapped creature of the house's can block,
  kill and survive, their best first, and what tapping is worth: a creature
  whose absence lets more of the house's next attack through than it deals
  now, while all of it leaves the house above ten life and costs at most a
  quarter of it. Nothing whose attack sets off a trigger, nothing with
  evasion the exchange does not read, nothing at the house's planeswalkers;
  what must attack goes at another opponent of the house's, else at the
  house. Their divided combat damage gives each of the house's creatures
  what it survives.
- *Their choices*: discards, sacrifices, costs and cards put back are their
  best; what they keep, find, take or put down is their worst and as little
  as allowed (a search finds nothing); a target, a mode and a pile are the
  worst for them by the same measure; an X is all their mana pays, with no
  ceiling under their library and a life X up to all their life (CR 119.4);
  a "may" is done only when it costs them and an unread one is declined; a
  pact's debt goes unpaid; a tax is paid only where refusing it would counter
  the spell cast for them. Scry and surveil keep their worst on top.

What has no reading here falls through to the ordinary answer, which reads
their permanents as the hostile ones. Open: a creature with an ETB that
would hurt them, a mana ability that sacrifices (Ashnod's Altar) and an
ability whose mana does not float yet are never used for them; a subtype,
colour or card name they choose is the house's ordinary choice; and the
harness (`baylee-gamehost::harness`) still answers a question by the player
it names rather than by `Engine::decision_actor`, so a harness game cannot
run a controlled turn yet. `crates/baylee-ai/src/tests/controlled_tests.rs`
pins each behaviour, and `crates/baylee-gamehost/tests/ai_mindslaver.rs`
plays a Mindslaver turn through the real engine with no house fallbacks.

### Fair by default, and the one level that peeks

Every level a player is offered plays from what its seat may see: its own
view, and its own submitted decklist, which a player knows too (owner,
10.11.2026). The one exception is `seer`, `expert` with its eyes open
(`AIProfile::SEER`, `Sight::Peeking`): the host tells it every opponent's
decklist and hand and the next three cards of every library. It is unfair
on purpose, it is listed last, every list of levels labels it so ("Seer
(unfair)", "Seher (unfair)": `i18n::ai_name`, which the lobby, the room
sheet, the offline picker and the seat roster all draw through), and no
table defaults to it. Before this, a hosted `sharp` read the opponents'
decklists and hands and a hosted `expert` the next three library cards as
well; a game the gateway or the engine server hosts no longer does, and
neither does an offline one.

Two places decide, one behind the other. The agent asks
(`HeuristicAgent::scouting_request`): every level its own deck, and its own
sideboard while it makes a wish; only a level that peeks asks for the rest.
The host answers (`scouting::request`): it checks the **current**
`SeatKind::Ai` and that chair's level, and a fair chair is told its own deck
alone whatever it asked (`own_deck_only`). `Session::set_fair_scouting`
holds even `seer` to its own deck, which is what evaluation and self-play
set, so a measurement never depends on hidden cards whatever it seats; a
hosted table never calls it, because its fair levels are fair without it.
To measure `seer` itself, eval4 takes `--seer-peeks`, which lets the level
that peeks be told what it peeks at and holds every other chair fair as
before; its summary says so (`seer_peeks`). A
profile built from the protobuf `AiProfile` message is fair, since the
message has no field for sight; a peeking profile is chosen by its name.
`Sight` reads as `Fair` from a preset or record written before it.

Human seats, human-driven AI chairs, disconnected humans' stand-ins and
invalid seats are denied outright. There is no public Session getter or
network message, the reports have no serialization, and neither human views
nor print disclosure change. Scouting is consumed for one decision and never
retained in the agent kept during a human takeover.

What proves it: `scouting::tests::a_fair_chair_asking_for_everything_is_told_its_own_deck`
(the host's check alone), `a_fair_level_is_never_handed_hidden_zones` (every
level's own request through the host), and
`tests/fair_house.rs`, where a hosted table deals the house two games that
differ only in what it may not see: the opponent's decklist, hand and
library (all Plains against half Llanowar Elves), with the opponent keeping
an empty hand and passing, so the public table is the same for ten turns.
Every fair level, and the frozen reference, answers both deals answer for
answer the same. `seer` does not (the control), and with the old rule
injected back (hands and decklists from `sharp` up, the host's check off)
the test fails at `sharp`'s 43rd answer, which is what the old hosted
`sharp` did.

A seat bridge plays from its socket's view, which carries nothing hidden, so
it cannot play `seer`: `baylee-seat --level seer`, a collector's
`house:seer` and the LLM seat's house fallback at that level are refused in
one sentence that names `expert` (`baylee_seat::house::level`). The LLM
seat's house fallback never peeked and plays as before.

Deck lists are analysed once at setup, by printed properties and DSL effects.
A level's own list sets its creature, draw and artifact priorities and its
tribe. `seer` alone also reads its opponents' lists, hands and library tops:
cheap creature density and interaction set its interaction priority, a known
opposing sweeper discourages committing a third creature, and known empty
threats release reserved mana. From `AIProfile::SEER_DRAWS` it keeps or
throws back an opening hand by the next three cards of its own library
(`keep_with_known_draws`): kept when those draws make two lands by its
second turn and three by its third, with a land and two spells in hand and
no more than six lands among the hand and the draws. Measured against itself
without it on the train split (2048 duel rows, 10.11.2026): 50.9 %, 67
seeds for and 50 against. A combat searched with the opponent's known trick
on the worst creature for it was tried and dropped: it changed 22 of 1,024
seeds, 11 each way. These are heuristic adjustments, not a
learned classifier or a complete combo planner. Reports carry card
identities without actionable hidden object handles.

Randomness is keyed by a **policy seed**, seat, choice sequence and object.
The same complete input replays identically. Sharp and expert use narrow
spell-score bands so equal options vary between games; proven combat lethal is
not randomized.

**The policy seed is not the game seed** (#87). It used to be: `Session::new`
and the harness both handed every chair `preset.seed` — the stream that dealt
the hands and shuffled the libraries — and this file wrote it down as the
rule. For a heuristic that only breaks ties that is untidy; for anything that
samples a belief it is a leak that no seat boundary catches, because nothing
crosses one. A sampler drawing from the stream that produced the hidden state
is correlated with the answer it is supposed to be guessing at. The invariant
is the clean form of it: *with the same authorized observations, the same
policy seed and the same budget, changing the real hidden state or the real
RNG cannot change the agent's answer.*

So `baylee_ai::policy_seed(game, seat)` derives it from what the whole table
can already see — the **public** game identifier a host tells every seat, plus
the seat number, under an explicit `POLICY_SEED_VERSION` that is hashed with
them rather than written beside them. It is FNV-1a and not `DefaultHasher`,
whose algorithm is stable only within one process: a recorded seed has to
survive a toolchain bump, because a replay reproduces the chair as well as the
shuffle. `Session::describe` is where the identifier arrives, once, before the
first socket, so that is where a chair stops playing with the no-identifier
derivation and starts playing with its own. The offline harness seats the
agents it was handed exactly as they were built; a fixture that wants its
chairs to differ says so with `with_seed`.
See [the coverage TODO ledger](ai-coverage-todo.md) for concrete tests to add as
new rules and complete deck families become available. Unsupported cards are
not counted as successful end-to-end coverage.

## When an instant is cast

The house used to cast every instant at the first priority it held one it
could pay for: a Titanic Growth in the opponent's upkeep on whatever ranked
first, a Plummet in its own draw step. And `steady` kept the mana up for
any cheap instant once it had a creature, so a green deck holding a Titanic
Growth left two lands open instead of casting a creature, then threw the
Growth away in the next upkeep. Measured against `casual` on 10.11.2026,
`steady` with that hold-up switched off won more than with it (50.4 % to
48.4 %, test split, 512 duel rows).

From `AIProfile::INSTANT_TIMING`, a level that manages its open mana
(`HoldUp` other than `None`: `steady`, `sharp`, `expert`; `timing.rs`) reads
what an instant-speed card is for (`timing::purpose`) and casts it when that
can happen:

- a **counter** when something worth countering is on the stack, as before;
- a **combat trick**, a boon to one creature until the turn or the combat
  ends (a pump, a keyword) with nothing lasting beside it but a card drawn,
  once blocks are declared and only where it turns an exchange by at least
  a one-drop's worth: a creature of its side that would die survives, one
  of the other side's that would live dies, or an unblocked attack becomes
  lethal. Its target is the creature it turns (`trick_target`), not the one
  `tactics::rank` would name first;
- anything **lasting** (removal, damage, a draw, a token, a flash creature)
  in its own main phase, or in an opponent's turn once they attack or in
  their end step; never in an upkeep or a draw step.

A sorcery-speed card is not timed: the engine offers it in a main phase
alone. And what such a level keeps up in its own main phase
(`timed_reserve`) is a counterspell's mana while it has a creature and an
opponent holds a card, and, before combat while a creature of its own can
attack, a trick's: what it would develop goes down after combat, where the
trick has had its chance. Never mana for an instant that could as well be
cast now. The frozen reference keeps casting at the first chance
(`timing_tests.rs` pins both sides), and so did `novice` and `casual` until
generation 14 (`AIProfile::TIMED_INSTANTS`): from it they time their instants
too, and keep nothing up, so they tap out as before for what they cast at
sorcery speed and cast an instant where the mana is there at its moment.

Measured with eval4 (`--random-policy`, house against house, each game
counted once by `house_duplicate`, train split, 2048 duel rows, 10.11.2026):
`steady` against `casual` went from 48.2 % (generation 2) to 52.1 %, and
50.5 % with the old hold-up switched off.

Control piloting for `expert` (counter mana kept up with no creature of its
own, a small creature spell let through while the opponent holds cards) was
built and measured, and dropped: 49.8 % against `expert` without it in
constructed duels, and 48.7 % in Victory's commander duels (228 games on
round-robin pairings, the deck turned 5 seeds for it and 8 against). So a
counter is still cast on the first spell worth one, and a level keeps
counter mana up only once it has a creature.

## The order of its triggers

At a table the house is asked what order its triggers go on the stack in, like
any seat (CR 603.3b, `docs/protocol.md` §"The order of triggers"). It has no
opinion and answers with the order offered, which is the timestamps' order, so it
plays as it did before there was a question. So does a stand-in in a player's
chair, and a clock that runs out on a player.

## What the search means

This is **public tactical search**, not whole-game determinization. It builds
combat positions from projected power/toughness, damage, keywords, and the
legal attacker/blocker offers. A root move is a set of attackers. The reply
tree assigns each blocker to no attacker or one eligible attacker. It resolves
first and double strike, deathtouch, indestructibility, trample, lifelink, and
gang blocks;
menace leaves with exactly one blocker are rejected. Changing one block
looks up that attacker's exchange in a bounded cache (through eight blockers)
and adjusts accumulated material, damage, and life gain. Death is checked at
each damage step: later lifelink cannot undo lethal first strike. Damage aimed
at a planeswalker is separate from damage to its controller. Commander damage
is evaluated per source, independently for every commander, and can require a
block even at forty life. Expert also preserves a blocker against lethal
commander retaliation. A proven lethal player attack takes precedence over
attacking a planeswalker, and so, for every profile, does one the
`breaks_through` estimate calls lethal. Node visits allocate no vectors; position vectors
are built once per decision and groups/results are fixed arrays on the stack. Counterattack
exchanges are also cached and stably ranked once; a leaf filters out dead or
unavailable blockers. Priority offers are borrowed instead of cloned.

Expert's retaliation is a greedy continuation from the surviving public
creatures, not a second full minimax tree. It includes untapped reserves that
cannot attack, such as Walls and summoning-sick creatures, and checks evasion
and two-creature menace blocks. Tapped opponents untap for this continuation,
and marked damage clears. Attack-side evasion remains an estimate; defender-side pairings come from the engine. Spell responses,
combat triggers, protection, replacement effects, and hidden draws
are outside this model. More than sixteen attackers or opposing creatures, or thirty-two available
retaliation defenders, uses the existing greedy fallback. Nonlethal planeswalker
damage and the sequencing of damage steps in the greedy retaliation remain
estimates. It does not sample guessed cards and pretend they
are observed. A full-game searching successor would need a view-derived belief
state and a determinization adapter; no live host `GameState` belongs in that
API.

A leaf prices combat damage to the defending player at 45 a point. From
`AIProfile::RACE`, expert's attack search prices it by the life it leaves
(`search::race_damage`: 90 a point while more than fourteen are left, 160
down to eight, 300 below), and the swing-back at three quarters of the same
price by this seat's own life, so the last points of a race are worth more
than the first and a low life keeps more at home. Its block search keeps
the flat price: priced by the life it leaves, a chump block outweighed the
creature at ten life while nothing threatened the seat. On the train split
(2048 duel rows, 10.11.2026) expert won 51.6 % against itself without these prices
and without `ABILITY_MANA` (59 seeds for, 26 against; with the prices in
both searches and no ability mana, 52.1 %, 65 for and 22 against); the
same prices on sharp, which has no
swing-back to set against them, 49.7 % against sharp without them.

Sharp gets 16,384 reply-tree nodes per decision, expert 262,144. A sharp
candidate gets at most 4,096; an expert candidate gets at most 196,608, enough
to finish a six-versus-six reply tree before considering its score. The budget
is larger at expert because evaluating a hard attack thoroughly is preferable
to discarding it after a shallow search.
A reply that already refutes an improvement prunes that candidate.
Only completely evaluated reply trees can promote an attack. If the budget
cannot establish that a candidate beats the greedy fallback, the fallback is
retained. A block choice keeps the best legal leaf visited. Every cutoff
returns an incumbent.
These are **node budgets**, deliberately not wall-clock cutoffs: timing an
answer out would violate the stronger requirement that the same complete input
always produce the same action. Latency is measured externally with
`cargo bench -p baylee-ai --bench decisions -- --quick`; time is never a
policy input. Custom profile horizons above two are capped.

## Foresight

From `AIProfile::FORESIGHT` (generation 7) a level has a foresight tier
(`AIProfile::foresight`), and where its host can fork the game it tries its
options in copies before it answers. The search is `baylee-gamehost`'s,
under its `foresight` feature (`session/foresight/`), and a table has to
switch it on (`Session::set_foresight`). The hosting builds have it (the
engine server and the client's own table, on the engine's `fork`), and since
12.11.2026 they switch it on: every hosted table, through the gateway and
the engine server, and every offline table a player starts, plays the house
levels with their foresight, within each level's generation-9 bound and the
host's (§"Foresight at a host"; the owner's conditions of
`docs/proposals/house-ai-strength.md` §7 hold: BayleeAI's fair-search probe
showed +7.4 points, and generation 9 bounds every search). A harness turns
it on itself (`eval4 --house-foresight`); the measurement and study hooks
around it (probes, kept questions, a clock of its own, `Session::fork_for`)
are gamehost's `simulation`, which only the training crates enable.

**What is searched.** Three questions, the ones the engine can fork
(`docs/engine-internals.md` §"Forking a simulation"): priority with nothing
begun (in the seat's own main phase with an empty stack; elsewhere only
where the level means to act, or an opponent's spell or ability is on top
of the stack), an attack, and a block. The candidates come from the
question's own offer (`HeuristicAgent::foresight_candidates`): the level's
own answer first, then passing, each spell (the dearest first), each land
and each ability, one per card; for an attack the level's declaration
beside all, none, and itself with one creature more or fewer, and for a
block the same around the level's blocks. With nothing floating at a
root, a spell with a cost is not castable but offered to be paid while it
is cast (`legal.payable`, CR 601.2g-h); from generation 8
(`AIProfile::SPELL_ROOTS`) each such spell the planner pays for from the
seat's sources (`HeuristicAgent::pays_for`: no X, and the taps found as the
level would find them, nothing held back) is a spell of the list too, cast
in the fork and paid there by the house that plays the chair on. Before
it, a priority search set the level's own plan, which begins with a tap,
only beside passing, lands and abilities, never one spell or one target
against another. A spell or ability that asks for exactly one target is
also tried at each of its first four targets, and the target chosen with
it is given when the engine asks. A declaration that beats the level's
own is tried against its own neighbours for two more rounds. Every
candidate is an answer the question takes (`Pending::answer_fault`).

**A world.** Each candidate is given in a fork of the game and the house
plays every chair on until the turn counter has advanced four turns, the
game ends, or 400 answers are spent. All candidates are tried in the same
worlds: the same deal of the hidden cards and the same future randomness,
from seeds derived from the chair's policy seed, a hash of what its view
shows, and the world's number. The level's own answer stands unless
another's mean value over the same worlds is more than 0.02 higher; then
the best is given.

**Who plays in a fork.** One model in every chair
(`HeuristicAgent::simulated`): `expert`'s heuristics at the searching
level's generation, with the searching chair's sides and sight, seeded
from the world. Not the real opponent: its level, its agent, its seed and
the analysis of its deck are not in a fork, so a search asks what a good house player would do in each seat and cannot
be fitted to the player who sits there. Not the searching level either: a
level's own heuristics give its own answer and every answer no search
takes (mulligans, payments, most choices); its tier says how hard the
search looks.

**What is hidden in a fork.** A fair level forks with
`DeckKnowledge::Prior`: the engine deals every other player's hand, library
and sideboard from a weighted list built from the seat's own view
(`baylee_ai::foresight::prior::public_prior`) and never reads the real
ones. The list supposes that a deck goes on as it began: the cards the
player has shown weigh most (twelve to the six that all the pool's other
castable cards share), two cards in five are lands (the lands shown, and
basic lands for the colours of the spells shown), and a player who has
shown nothing is supposed to play a deck like the seat's own. The seat's
own library is dealt from its own cards, which it knows. No chair but the
seat's is told a deck. `seer`, at a table that lets it peek, forks with
`Known` and keeps the table's deck analyses; from generation 15
(`AIProfile::SEER_HANDS`) its forks also keep every other player's hand as
it is, since it is told those hands, and deal only the libraries (with
`Known` alone each hand was dealt again from what its player hides, and
`seer` searched as `expert` does).

**The leaf.** The game's result where the fork ended; otherwise the
logistic of fourteen weighted counts read from the seat's view of the fork
(`foresight_features`, `LEAF`): each side's life in three bands, cards in
hand, creatures with their power, toughness and keywords, the mana value of
what is on the table, whose turn it is, and the race (how much of the other
side's life each side's creatures take in one attack). The weights are
fitted by `fit_leaf` (`baylee-train`) on positions the house played against
itself on the train split (`eval4 --leaf-log`), a logistic regression
without an intercept and with no weight below zero but the turn's, so that
a correlation (a flooded hand has many lands) is never a reason to hold
something back. The fit reports its log loss on whole games held out. The
value says who is ahead between two house players; nothing in it knows an
opponent.

A richer value was measured and does not stand (11.11.2026,
`docs/proposals/house-ai-strength.md` §6a). Forty-nine features in place of
the fourteen, the same weights fitted on the searching house's own games or
on pairs of worlds played to their end, boosted trees over the features, and
a leaf read at another point of the turn each call a game's result better or
no worse, and each leaves the level's share against its predecessor within
the noise of where the fourteen counts have it. What limits a search is not
what its leaf can tell apart.

**Judging a search.** A table can play the worlds of its searches on
(`Session::set_foresight_probe`, `eval4 --foresight-probe <file>`): of the
questions it searches, one in so many has every world of every answer tried
played on past the leaf until the game ends, and each world is written with
what the leaf showed, what the leaf value made of it and how the game came
out. No answer depends on it. `foresight_probe` (`baylee-train`) reads the
file and says what the search's choices won in those games over the level's
own answers, by question kind and by how far ahead the leaf put the answer
chosen, and what another set of weights would have chosen from the same
leaves; `fit_leaf --pairs` fits weights on the pairs of answers whose games
came out differently.

**Budgets** are counts, so the same view, policy seed and budget give the
same answer on any machine:

| Tier | Level | Candidates | Worlds | Work, from generation 9 |
| ---: | --- | ---: | ---: | ---: |
| 1 | novice | 4 | 2 | 1,333,333 |
| 2 | casual | 5 | 3 | 2,500,000 |
| 3 | steady | 6 | 4 | 4,000,000 |
| 4 | sharp | 8 | 5 | 6,666,666 |
| 5 | expert, seer | 8 | 6 | 8,000,000 |

**The bound.** From generation 9 (`AIProfile::BOUNDED_SEARCH`) a level
bounds its own search: `Foresight::work`, eight million units at
expert's tier and as much less below it as the tier tries fewer
candidates in fewer worlds, and `Foresight::nodes`, the 16,384 nodes one
combat search in a fork may visit whatever the bound. A host may lower
the bound of one searched answer (`Session::set_foresight_work`, `eval4
--foresight-work`) and never raise it: the meter takes the tighter of the
two, so a level plays alike in a measurement, as a teacher and at a host
that leaves the bound as it is. A search counts what it does on a meter
(`baylee_ai::WorkMeter`): each node the forks' combat searches visit is
one unit, and each fork and each house answer in a fork has a price in
the same unit (`session/simulation.rs`: an answer 375 units and 40 a
permanent or stack object, a fork 2,500, fitted to thread CPU time at
about 0.07 µs a unit). Once the bound is spent the search
stops where it stands, drops the world it was in and the round of worlds
that belongs to, and chooses among the answers it tried in the same
worlds; the level's own where fewer than half its worlds were finished.
A level from before generation 9 is bounded only by a host, and under a
host's bound one combat search in a fork may visit a 256th of it
(`SEARCH_SHARE`; `set_foresight_nodes` sets another count). What it cost
and what it bounds is in `docs/proposals/house-ai-strength.md` §7.7; one
engine step is not bounded by the unit.

**Fairness.** `session/tests/foresight.rs` deals the house the two games of
`tests/fair_house.rs`, which differ only in the opponent's decklist, hand
and library, with foresight on: every fair level answers both the same,
answer for answer, and searches the same positions. The same twins run at a
table hosted as an engine server hosts one (`…_at_a_host`), and through the
engine server's own runner on the hosting build, gamehost's `foresight`
without the harness (`baylee-engine-server`'s
`a_fair_level_searches_alike_whatever_is_hidden_at_a_hosted_table`). The
engine's own twin games hold the fork (`engine/simulation_tests/prior.rs`).

**Guards.** A position a search replaced the level's answer at and that
comes back unchanged means the engine took the answer back; the level's own
answer is given there, under its own stall guard. A fork the engine refuses
(a suspended operation, something public naming a hidden card) leaves the
level's own answer.

## Foresight at a host

A level's bound is a count, so it holds a search to the same work on any
machine; what it does not hold is said in
`docs/proposals/house-ai-strength.md` §7.8: one engine step, the level's own
answer before the search, and wall time on a loaded machine. A host adds its
own bounds (`HostForesight`, `Session::set_foresight_host`,
`session/foresight/host.rs`), and the engine server sets them from its
operator's settings (`baylee-engine-server`'s `foresight.rs`,
`docs/dev/environment.md` §"Engine server"):

- **Who decides what a search may spend: the server alone.** A player
  chooses a level by its name in the lobby (`AIProfile::named`, the six
  levels and nothing else: no reference, no generation, no number); the
  gateway builds the chair's profile from the name, with its tier and its
  bound, and the engine grants the host's share of it. A profile read from
  the protocol's numbers (`preset::from_proto`, the dev harness) has no tier
  and never searches. No message from a client carries or changes a tier, a
  bound, a share or a switch.
- **A ceiling and a share,** counts again: no search at a host spends more
  than `expert`'s bound (`AIProfile::EXPERT_WORK`, eight million units),
  whatever generation a preset names, and an operator may lower every level's
  bound to a share of it (`BAYLEE_FORESIGHT_WORK`, percent). Below 100 a
  level plays weaker than it was measured, and its games are no longer that
  level's numbers.
- **Searches at once on the machine** (`BAYLEE_FORESIGHT_SEARCHES`, default
  half its cores, at least one). One engine plays one game and searches one
  answer at a time, so the count that matters is the machine's: the agent
  makes a directory only its user may enter and hands it to every engine it
  starts (`BAYLEE_FORESIGHT_SLOTS`), and a search holds a lock on one of so
  many files there (`flock`) while it runs. A search that finds every slot
  taken is not made: the level answers as its heuristics do, at once, and
  the table counts it (`ForesightStats::busy`). Nothing waits for a slot, so
  nothing queues. An engine that was given no slots, or whose slots' directory
  is gone, searches nothing (and says so); one whose directory is not its
  user's own and 0700 refuses to start. Slot files are made 0600 and never
  opened through a symbolic link, and the agent makes its directory again
  before it starts an engine if something swept it away.
- **A wall guard** (`BAYLEE_FORESIGHT_WALL_MS`, default 3,000). The meter
  is read before each fork and after each answer in one, and so is the
  host's clock: a search past the guard is given up where it stands and the
  level's own answer is given, whatever the search had found
  (`ForesightStats::walled`), and the engine says so in its log
  (`a house search ran past the host's wall guard`). One engine step is not
  interrupted, so a search may run past the guard by the step it was in.
- **One searched answer a slice.** The engine's event loop pumps the house
  in slices; a slice ends after an answer a search was begun for (a slot
  taken, whether the search then ran, was given up or was refused by the
  engine), so the engine reads its socket (clock probes, a flush of the
  record, a seat coming back) between two searches, and a player sees the
  house's land before it thinks about its spell.
- **Only while a person plays** (`HostForesight::needs_a_person`). The house
  searches only while a chair an account sits in still plays: it answers
  over a socket and has not conceded, lost or left
  (`Session::a_person_plays`). The gateway tells the engine which chairs
  those are (`GameSetup.seat_accounts`): not a seat bridge its host handed a
  chair to, which has no account and plays on when its host has gone. When
  the last person concedes, the gateway frees their account at once, and
  the rest of the game is played by the levels' heuristics.
- **A house-only remainder is bounded.** Once no seat answers over a socket
  any more (no person and no seat bridge), the house plays the game out
  alone, so that its result stands (a player who conceded lost; one whose
  house teammate goes on to win won) and its record is whole. It does so by
  its heuristics and within 5,000 of its answers and 20 seconds
  (`REMAINDER_ANSWERS`, `REMAINDER_MS`; measured: two houses finish in
  about 1,000 answers, three in about 4,000, seven take 15,000 to 25,000);
  past either the engine closes the game as an admin closes one, no winners
  (`GameEnded` reason `nobody is left`), the record ending where it stopped.
- **A switch** (`BAYLEE_FORESIGHT=off`): every level answers as its
  heuristics do.

**Determinism and the record.** A search that runs answers as it would
anywhere: its bound, the ceiling and the share are counts. Slots and the
wall guard read the machine, so where they act the same position may be
answered without the search. That is safe for the record, whose inputs are
the answers: the house's answer is written as it was given (`Source::House`),
and a replay, a projection and a table resumed after a restart apply the
recorded answers and ask no house again (`session/resume.rs`,
`record::replay`). Two tables of the same seeds can differ only where a slot
was busy or the guard passed, and each table's log line at its end says how
often (`house searches`: asked, searched, replaced, refused, busy, walled,
the slowest in milliseconds and the most work one search spent).

**The client's own table** (`LocalHost::into_installed`, native builds)
searches as an engine server does by default, on its worker thread: each
level's bound under `expert`'s, one search at a time in the program, a guard
of three seconds and one searched answer a slice. A browser build's table
does not search, because its host runs on the page's own thread. A
`LocalHost` built in a test searches only where it asks
(`LocalHost::searching`).

**What a remote player can make a host spend.** A player picks levels by name
for the chairs of their rooms (eight rooms an account; at least the host's
chair is a person) and plays at whatever pace they like. Per searched
answer: at most the level's bound in work (`expert` eight million units,
about a second of a core) plus one engine step, the level's own answer
before it (up to 262,144 nodes of its combat search, about 20 ms), and at
most the wall guard in time. At once, on one machine: at most
`BAYLEE_FORESIGHT_SEARCHES` searches, so at most that many cores, whatever
the number of tables; past it the house plays by its heuristics, as every
hosted game did before. One search holds one fork of its game at a time.
What a player can take from other players is the slots: tables that search
all the time keep them busy, and the other tables' house then answers
without searching. A table searches only while its person plays it, and an
account plays one game at a time on a gateway (conceding frees it at once,
and the game it leaves searches no more; a seat bridge left behind counts
for nobody), so holding every slot takes as many accounts as there are
slots, each playing its table at the machine's speed. A game in which no
seat answers over a socket any more (no person and no seat bridge) costs at
most 5,000 heuristic answers or 20 seconds more. A seat bridge left alone
with the house is not bounded so: its game runs on without search and
without that cap, as every hosted game did before foresight, paced by the
bridge's own answers.

### What the house's search costs

`baylee-engine-server`'s `tests/foresight_load.rs` (ignored; run by hand on
the machine it measures) starts engines as an agent does, one slots
directory and the operator's settings, and plays each table of `expert`
against a seat it plays over the engine link at `steady`, from its own view;
`BAYLEE_LOAD_THINK_MS` lets the seat think before each answer, 0 plays at
the machine's speed. The times below are each searched answer's wall time
from the engines' own log. Measured 12.11.2026 on the 24-thread workstation
(Ryzen 9 3900X) beside other people's jobs (a load of 10 to 160 that came
and went), with a release build; a small VPS is two of its cores
(`taskset`), on which an engine counts two cores and so takes one slot by
default. The decks are the house's (`allytifact`, `victory`, `maik`,
`schwarzrand`, `weltenbaum`).

| Host | Tables | Seat thinks | Games | Searched / busy / given up | ms p50 / p95 / p99 / max | Engine CPU-s a game | Seat's longest wait |
| --- | ---: | ---: | ---: | --- | --- | ---: | ---: |
| 24 threads, 12 slots | 16 | 300 ms | 16 | 615 / 0 / 0 | 140 / 460 / 515 / 746 | 7.5 | 1.3 s |
| 24 threads, 12 slots | 8 | 0 | 16 | 615 / 0 / 0 | 167 / 525 / 637 / 701 | 8.5 | 1.4 s |
| 24 threads, 12 slots, load ≈ 160 | 24 | 0 | 48 | 1,001 / 1,946 / 0 | 482 / 1,761 / 2,111 / 2,476 | 9.4 | 7.0 s |
| 2 cores, 1 slot | 4 | 300 ms | 4 | 189 / 60 / 0 | 267 / 612 / 666 / 738 | 14.5 (load ≈ 110) | 3.3 s |
| 2 cores, 1 slot | 8 | 300 ms | 8 | 238 / 124 / 0 | 140 / 405 / 478 / 534 | 5.4 | 0.9 s |
| 2 cores, 1 slot | 4 | 0 | 8 | 55 / 317 / 0 | 223 / 482 / 540 / 640 | 2.0 | 1.4 s |
| 2 cores, 2 slots | 4 | 0 | 8 | 118 / 206 / 0 | 145 / 390 / 465 / 599 | 2.7 | 1.0 s |
| 2 cores, search off | 4 | 0 | 8 | 0 / 0 / 0 | | 0.12 | 6 ms |

- **A game against `expert` costs its engine 7.5 to 8.5 CPU-seconds** where
  the levels answered by heuristics alone for an eighth of a second; the
  two rows at 24 threads are the same sixteen games, answer for answer and
  search for search, played at two paces. No search spent more than its
  bound (8,005,708 units at most, the bound and one step), and an engine
  peaked at 57 to 59 MiB with the search as without it.
- **The default is safe on a small VPS.** At two cores the engines take one
  slot, so the searches take at most one core whatever the tables do; with
  tables played at the machine's speed six searchable questions in seven
  found the slot taken and were answered by the heuristics at once, and the
  other core was left to the tables, the seat and the rest of the machine.
  Played at a person's pace, one in five to three in ten did. No search on a
  host in these runs reached the wall guard of three seconds, not even at a
  load of 160 on 24 threads (slowest 2.5 s); a VPS with slower cores than
  this one would reach it first in its slowest searches, which then give
  the level's own answer.
- From 6 % to 29 % of the questions a search would take, by the games
  dealt, were not searched because the engine refused the fork on these
  decks (`refused`), against one in twenty on the train split (§5a of the
  proposal); those get the level's own answer. Which refusals these are is
  not logged at a host; `eval4` names them.

## The seated house

`baylee-seat --mind house` (the ladder's and the collector's `house:<level>`)
and the LLM seat's house fallback answer as the table's own house chair
does (owner, 10.11.2026). An in-process chair answers from the engine's
`DecisionContext` and the report of its own deck; a seat used to answer
from its socket's view alone, which measured about 8 points weaker than the
benchmark's in-process `sharp` (v6t campaign, L1s).

Now the seat is given both, from what a player at the table is given:

- **The decision context.** The table sends the asked seat a
  `DecisionNote` beside its question (`ChoiceRequest.context`,
  `docs/protocol.md` §"What a question is about"): which printed ability or
  mode asks, how far a resolving ability has run, the cast route, the X,
  the second instance of "target" and what the first chose, a clone's
  copy question. `baylee_ai::noted::Noted` rebuilds the context from the
  card registry. What the registry cannot give back (an ability a
  continuous effect grants, a reflexive trigger's body, a resolution's
  spliced operations) stays empty, as the in-process house leaves an
  ability it cannot read.
- **Its own deck.** The seat knows the deck it brought
  (`GameContext::deck`); `HouseMind` analyses it as the host analyses a
  submitted list and answers through `HeuristicAgent::act_with_own_deck`,
  the report a fair chair is handed in-process, built from the seat's own
  list. A seat that knows no deck answers without it.

Both `HouseMind` and eval4 `--house-as-seat` answer through one function,
`baylee_ai::noted::answer_as_seated`, which also strips the socket view as
`Session::agent_view` builds a house chair's (`seated_view`).

`baylee-seat/tests/seated_house.rs` holds the two to each other: at every
question of four hosted duels on the house decks, the seated house's answer
(from `view_for_with_context`, the question's note and the deck list) is set
beside the answer the table's own chair gave.
On 10.11.2026, over four duels (allytifact–victory, victory–maik,
schwarzrand–weltenbaum, maik–schwarzrand), the seated house gave the
table's answer at all 3,070 of its questions; with the view alone it had
given it at 3,028. The difference was all in questions a context explains:
target choices (68 of 68, from 30), numbers (2 of 2, from 0) and players
(2 of 2, from 0). Its own deck changed no answer the view alone missed in
these games. The test asserts at least 99 %, and more than before. At the
same questions it also answers a hostile note in place of the table's (a
card or token the registry does not hold, every index past its list, the
largest X, first targets that name nothing) without panicking: a note is
read within bounds (`Noted`, `docs/protocol.md` §"What a question is
about").
`tests/question_note.rs` holds the note to its rule over two duels and a
game of two teams: told to the asked seat alone, read back off the wire as
built, and naming no card outside the zones the seat sees. The view's own
tests (`view/tests/notes.rs`) cover a face-down source, a grantor and a
copy's card the seat is not shown, a token, a cast and a granted ability;
`tests/ai_mindslaver.rs` covers a controlled turn, whose notes go to the
controller; and the card sweep (`harness::play_report`) builds every
seat's note at every question.

## The frozen reference: `sharp-2026-11`

Training measures BayleeAI against `sharp`, and the house is meant to get
better, so the benchmark is frozen apart from the level (owner, 10.11.2026).
`AIProfile::SHARP_2026_11` is `sharp` as it played on 10.11.2026, its
constants written out rather than borrowed from `SHARP`, and
`AIProfile::REFERENCES` names it `sharp-2026-11`. A player never sees it:
`AIProfile::named`, which the lobby, the gateway and the client ask, knows
only the levels of `NAMED`. The tools that measure take both lists through
`AIProfile::lookup` and `resolve`: `eval4 --against`/`--baseline`, `arena`,
`league` (`house:sharp-2026-11`), `selfplay --profiles`, the collector's
`house:<level>`, `ai_match` and `baylee-seat --level`. Today it plays exactly
as `sharp`; a run that is to stay comparable after the house moves names it.

The constants are half of a profile; the other half is the code they drive,
and `crates/baylee-gamehost/tests/reference_profile.rs` pins that. Six
tables (the scoreboard's duels on the house decks, and one two-against-two
table, whose answers read teams) are played from the start with the
reference in every chair, seated as the benchmark seats it: `Session::new`,
`describe`, fair scouting, the house answering through `pump`. Every answer
is in `tests/reference/sharp-2026-11.json` with the public table after it
(the record's `seen`, the same on every build), and the fixture's FNV-1a
answer hash is over all of them. The test reports the first difference as
one of two things: the reference answered differently with the same table
before it, or the same answer led to a different table (the engine moved).

So a change that means to make the house play better goes behind a profile
field the reference does not set, and the test stays green. That field is
`AIProfile::generation`: every level plays `AIProfile::GENERATION`, the
reference `FROZEN_2026_11` (0, which is also what a profile written before
the field reads as), and a change says the generation it arrived in and
asks `profile.plays(...)`. The generations so far:

1. `TEAM_PLAY` ("The house plays as a team, and teammates show each other
   their hands", above).
2. `EXACT_SPELL_X`: an X that must equal a target's mana value is planned
   for a spell on the stack as for a permanent (Spell Blast announces an X
   some spell has, or is not cast).
3. `INSTANT_TIMING`: a level that manages its open mana casts an instant
   at its moment and keeps mana up for a counterspell (§"When an instant
   is cast").
4. `RACE`: `expert`'s attack search prices damage by the life it leaves
   (§"What the search means").
5. `ABILITY_MANA`: `sharp` and `expert` make mana for an ability worth its
   price (§"Mana for an ability", above).
6. `SEER_DRAWS`: `seer` mulligans by the draws it is told (§"Fair by
   default, and the one level that peeks").
7. `FORESIGHT`: a level tries its options in forks of the game before it
   answers, where its host can fork and the table switches it on
   (§"Foresight"). A level at an earlier generation has no tier
   (`AIProfile::at_generation`), and every level's answers at generation 6
   are pinned beside the reference (`tests/generation_pins.rs`,
   `tests/reference/generation-6.json`), so a measurement against
   `<level>:generation=6` plays the level as it was.
8. `SPELL_ROOTS`: a priority search also tries each spell the seat's mana
   pays for while it is cast (§"Foresight").
9. `BOUNDED_SEARCH`: a level bounds its own search (§"Foresight", "The
   bound"). The searched answers of `expert` at generation 7, the
   trained AI's house teacher, are pinned
   (`tests/reference/foresight-generation-7.json`).
10. `FULL_CASTS`: the house pays through the planner the client, the seat
    and the trained AI's menu share (`baylee_ai::spend`; `policy/full_casts.rs`):
    restricted mana where the engine names it for the payment (CR 106.6), a
    Phyrexian symbol with 2 life where no mana pays it and the seat keeps
    more than six life (CR 107.4f; the question whether to pay one with life
    is answered no where the mana floating and the untapped sources pay the
    whole cost), every counter of a counter-paid mana ability, a permanent
    whose own tapping ability waits for mana kept untapped (the second
    Rishadan Port), a conversion beside restricted mana. And it taps for a
    card it may cast from exile or a graveyard at the price the engine names
    for each way (`LegalActions::cast_ways`): an Adventure's creature, a
    foretold card, a permission's card, a disturb face, each scored as the
    face the way puts on the stack. Before it such a card was cast only
    where its mana already floated (`docs/observed-faults.md` 70, 72, 73).
11. `TARGET_SIDES`: a spell is aimed at the side it helps or hurts
    (`sides.rs`). This seat's own Aura goes on the creature its statics do
    the most for (a Holy Strength on its own creature, a Pacifism on an
    opponent's); before it an Aura's attach read no side and went on an
    opponent's creature first. And a spell is not cast where every target it
    could name is on the wrong side: an Aura no creature gains from, a spell
    that helps what it targets with no target of this side, one that hurts
    with none of another side (a Broadside Barrage with only this seat's
    creatures to hit).
12. `EQUIPMENT_AND_X`: an Equipment's statics are read inside the
    `And(creature, attached to the source)` most of them are written with,
    so an equip is worth what it gives (it read as nothing, and was never
    paid for); and an instant or sorcery with X in its cost is not cast at
    X = 0.
13. `QUIET_DRAW_STEP`: nothing is begun in the seat's own draw step with
    nothing on the stack. A level that keeps no mana up cast its instants
    there on sight, and no search could weigh them: the engine forks nothing
    while the turn's first draw waits for its miracle check, which is the
    whole of that round (8 % of novice's searched questions were refused
    so).
14. `TIMED_INSTANTS`: every level casts an instant at its moment
    (§"When an instant is cast"), not only one that keeps mana up. `novice`
    and `casual` still keep nothing up and tap out for what they cast at
    sorcery speed; a combat trick is no longer cast on sight.
15. `SEER_HANDS`: `seer`'s forks keep the hands it is told (§"Foresight",
    "What is hidden in a fork").
16. `NOISE_BAND`: a level's noise is at most ±400 milli-units
    (`AIProfile::noise_milli`). Noise is there so that players cannot learn
    to exploit a level, never to weaken one, and `novice`'s ±800 was twice
    any other level's.
17. `CARD_SKILL`: `casual` reads its cards as `steady` does
    (`AIProfile::card_skill`): it plans its colours, checks its opening hand
    for an early play, and reads what it scries, surveils and keeps.
    `novice` keeps every opening hand, the ladder's lowest card skill; a
    step for it too lifted it to 67 % against its generation-6 self and left
    it level with `casual` (51 %, `docs/proposals/house-ai-strength.md` §8).

Only a change
that has to move the reference, such as an engine or card fix that changes
these games, re-records it, in a commit of its own that says why:
`BAYLEE_REFERENCE_RERECORD=1 cargo nextest run -p baylee-gamehost --test
integration reference_profile`. A frozen runtime (`--builds`) stays the way
to compare numbers across builds; the fixture is what keeps the reference
from drifting by accident between them.

One change reaches the reference on purpose: **the stall guard**
(`baylee_ai`'s `stall`, owner 10.11.2026). A cast or activation the engine
cannot finish (an X that leaves no legal target, a cost that cannot be
paid) is taken back, and the game is where it was. A deterministic house
asked again from there gave the same answer, and the game went round until
eval4's 5,000-answer cap ended it: the campaign's capped pairings were
these loops (Spell Blast at an X no spell had, a seat's Kaervek's Purge at
X 0). Each agent now remembers its last priority action and the position
it gave it from (`StallMemory`, shared by its clones and kept by
`HouseMind` across the agents it builds). Asked again from the same
position, it would give the same action only because the engine took it
back, and it passes instead. A position the house acted from does not
come back once the action stands, so the guard changes no game that ends,
and it is a loop guard at every level rather than a strength change. The
pinned answers are unchanged. `tests/stall_guard.rs` replays the start of
one capped game (`stall/purge-x0.jsonl`) and asks the house as the seat
was asked then; without the guard it casts and is taken back until the
test's cap.

## Evidence

`docs/ai-results/` holds the paired acceptance-deck scoreboard, recorded seeds,
and outcomes. `cargo run -p baylee-gamehost --example ai_match -- sharp novice
1,7,42,1337,2,3,5,11` runs both deck orders and both seat assignments per seed.
Use the development profile: the workspace's release profile aborts on panic,
whereas the scorer needs unwinding to record a broken game and continue.
Report unfinished games beside every win rate. The two decks and paired seeds
do not establish a universal ordering of playing strength.

Those seeds are the **shuffle's**, and since #87 they are no longer the
chair's: the harness seats the agents it is handed rather than overwriting
them, so a scoreboard run varies the deal and not the tie-breaking. That is
the correct direction — a chair keyed to the deal is the leak — but it does
narrow what the scoreboard samples, and a run that wants both dimensions
supplies an independent policy seed per game at the call site.

Regression tests prove different decisions between every adjacent profile,
repeatability, bounded search work, coloured payments, mana reservation,
opening-hand selection, commander tax, convoke payment, menace blocks, and a blocked attack that used to be
mistaken for lethal. The original landless-hand, mana-colour, and false-lethal
tests were run against the old policy and failed. The scoreboard's loss
mapping was also deliberately broken and its injected-loss test failed.

**`worth` against the agent before it** (29.09.2026). Each profile played the
agent at 23fc46ff with the same profile, same seeds, both seats and both deck
orders: allytifact–victory seeds 1–90, and maik–schwarzrand,
schwarzrand–victory and weltenbaum–allytifact seeds 1–30, 720 games a profile,
with a paired runner kept outside the repository. Wins for the new agent, with
Wilson 95 % intervals over decided games:

| Profile | New wins | Interval | Mean turns | Unfinished |
| --- | ---: | --- | ---: | ---: |
| NOVICE | 57.2 % | 53.5–60.7 | 27.6 | 1 |
| CASUAL | 56.3 % | 52.7–59.9 | 26.6 | 1 |
| STEADY | 53.9 % | 50.2–57.5 | 25.4 | 2 |
| SHARP | 51.7 % | 48.0–55.3 | 25.9 | 0 |
| EXPERT | 54.7 % | 51.1–58.3 | 26.5 | 0 |

No game reached a turn cap or repeated a position. The four unfinished games
are one engine panic, `expect("target plan set")` after a Spark Double that
copied a planeswalker activated a loyalty ability, fixed by 31263242, which
the measured build did not have yet; the new seat activated in two of them and the
old seat in two.

## Decision benchmarks

Measured on the developer's M1 Max with Criterion `--quick` in the optimized
workspace profile; the interval is Criterion's reported estimate, not a latency
SLA. Other worktrees shared the machine; background load was not controlled.

| Decision | Estimate | Nodes | Completed or refuted attack sets |
| --- | ---: | ---: | ---: |
| 6 attackers / 6 blockers, novice | 471 ns | 0 | 0 |
| same, casual | 471 ns | 0 | 0 |
| same, steady | 478 ns | 0 | 0 |
| same, sharp | 178 µs (177.31–178.36 µs) | 16,384 | 15 |
| same, expert | 1.084 ms (1.082–1.094 ms) | 31,180 | 64 |
| 8 attackers / 8 blockers, sharp | 257 µs | 16,384 | 15 |
| same, expert | 11.40 ms (11.372–11.408 ms) | 262,144 | 128 |
| empty priority, expert | 16 ns | — | — |
| eight lands / four spells, expert | 812 ns | — | — |
| same with scouting report, expert | 962 ns | — | — |
| analyse 100-card deck once | 712 ns | — | — |
| mana colour / eight lands / four spells, expert | 2.275 µs | — | — |

These include position construction and result allocation. The combat fixtures have
3/3 attackers facing 2/2 blockers; a different board can exhaust either
profile's full budget. The benchmark prints the node counts alongside timing
so a fast fallback cannot masquerade as a search improvement.

These are the third iteration's measurements. Scouting includes report
allocation and identity copies but excludes host zone walks, view construction
and context construction. Deck analysis is cached once per game. Commander and
defender checks are cached with each exchange. Sharp pays some additional
latency for commander correctness compared with iteration two's 156/235 µs;
expert remains near its previous 1.05/11.13 ms. Quick Criterion runs are
estimates under shared-machine load, not controlled latency guarantees.
Raw runs, exact test scope and mixed match results are recorded in
[iteration-3](ai-results/iteration-3/README.md).

**Token boards** (`combat/army-1000/*`): an army of 1/1 tokens attacks four
3/3s and a walker, four 3/3s attack into it, it blocks four 3/3s, and four
3/3s block it. Every combat decision used to find each creature with
`PlayerView::object`, a walk over every zone, once per creature and in one
`sort_by_key` once per comparison, which made the decision quadratic in the
board; on self-play r001 #431, grown to tens of thousands of tokens, about
90 % of the main thread was in that walk. `board::Board` indexes the view's
objects and declared attackers by handle once per decision, answers every
handle as the walk does, and leaves the view's wire shape alone. Measured on
29.09.2026 under heavy background load, so the orders of magnitude are the
claim, with the army at 10,000 (`ARMY` in the bench):

| 10,000 tokens, all three profiles benched | before | after |
| --- | ---: | ---: |
| the army attacks | 183–188 ms | 1.5–2.9 ms |
| the army blocks | 373–382 ms | 3–10 ms |
| four block the army | 404–528 ms | 7–16 ms |
| four attack into the army, novice | 1.8 ms | 3.8 ms |
| same, sharp and expert | 34 ms | 1.6–2.0 ms |

The one row that got slower pays for the index: NOVICE never searches, so it
looked up only its own four creatures and now indexes all ten thousand. On
the 6v6 board the index costs about half a microsecond.
