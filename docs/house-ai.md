# House AI

The five names in `AIProfile::NAMED` drive both the offline picker and the
network lobby. Existing `novice`, `steady`, and `sharp` keys still work.

| Profile | Opening hand | Spell selection and mana | Combat |
| --- | --- | --- | --- |
| novice | Keeps every hand | ±800 deterministic score noise; taps out | Individual trades |
| casual | Rejects fewer than two or more than five lands | ±400 noise; taps out | Individual trades |
| steady | Also checks early plays and colour access; bottoms excess lands | ±100 noise; reserves cheap interaction after establishing a creature | Individual trades |
| sharp | Same opening policy as steady | ±30 seeded score noise; releases the reserve when opponents have no visible threat or cards | Searches whole attack sets and blocking assignments, including gang blocks |
| expert | Same opening policy as sharp | ±12 seeded noise; sees the next three library cards | Also simulates retaliation by surviving opponents |

All levels plan coloured payments through the same renderer-free mana matcher
as the human client. Simple printed and granted mana abilities count, with one
source per permanent. The agent does not tap toward an unaffordable spell.

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
a priced tap only where no clean one fits the pip (#210).
Command-zone commanders participate in these plans, including their public
cast-count tax, and so does a graveyard card the view says the seat may
flash back, at the price it names (`PublicObject::flashback`, #242): the
engine lists a graveyard spell as castable only once that price floats, so a
card the plan does not walk is one it never taps for. Steady and harder levels choose a mana colour by the casts it
can complete with the
remaining visible sources; an expensive, uncastable card cannot drown out the
colour needed for an affordable play.
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
countered on sight: the worth ranks what is up at once, and nothing holds a
counter back for a threat that has not been cast. So a Counterspell is
traded for a lone Dark Ritual whose caster has nothing left to spend the
mana on; `a_counterspell_is_not_spent_on_a_spell_that_only_replaces_itself`
pins that as a limitation.
Removal needs something opposing on the battlefield, and a deferred pay-or-lose
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
while a card is the cheaper thing to give up. The same ward is read a second
time at target choice, since CR 601.2 picks targets before mana is paid: a
warded creature is priced against the material scale when the seat can cover
the spell and the tax together, and sinks below every other candidate when it
cannot — going around a ward costs nothing, walking into one it cannot pay
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

**A pact is a counter paid for next turn.** Pact of Negation's
`Effect::PayCostOrLoseLater` used to refuse the cast outright. `policy::spell_score`
now casts it as a counterspell on two further conditions: every mana source
this seat has, untapped again, makes the price (`pays_next_turn`, CR 502.3),
and the best opponent's spell it could counter (its material plus what of this
seat's it is aimed at) is worth more than the price at 150 a mana, which is
then taken off the counter's value. The engine asks for the price once a round
of passes on an empty stack closes the upkeep, from what is still untapped,
and nothing in the view says a price is coming, so this seat spends nothing in
its own upkeep on an empty stack (`a_seat_spends_nothing_in_its_own_upkeep`: a
fetchland is cracked in the draw step instead). Inside the window a dual
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
filter (a chosen type, the commander's types) is a no. It taps at most one
restricted source per spell, and that tap comes last: once the mana floats, no
plan counts it, and only the engine's merge makes the spell castable. Its mana
is spent first, so it never counts as held up. Its colour is named for a spell
it may pay for. A price floated before the cast (an X, a kicker) is paid from
unrestricted sources only.

Searches, bottoming,
and surveils at the skilled levels evaluate only identities actually visible
in `PlayerView`, including `looking_at` while a search is open.

An agent is never handed the game log (#262). The host sends log lines only
to a seat answered over a socket, never to an AI chair or to a chair the house
holds for an absent player
(`session::tests::a_seat_the_house_answers_is_never_told_the_log`). What an
agent knows of the past is what its view shows now.

An agent is never handed a teammate's hand either (#265). Every view an
agent answers from is built by `Session::agent_view`, which leaves
`shared_hands` and the three sets beside it empty; only a view on its way to
a socket carries them. A chair the house AI plays is never shown a hand, and
shows its own at once to a teammate who asks: showing it changes nothing its
agent sees. A chair the house holds for an absent player keeps the shares
its player was given, is played without them, and leaves requests to the
player (`view::tests::no_agent_is_handed_a_teammates_hand`).

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

**Open: an ability with a mana cost is taken only when the mana already
floats.** The engine offers such an ability only once the pool covers it
(`Engine::can_afford`), and nothing in the agent floats mana for an ability
the way `policy::aim` floats it for a spell. So these rows stay at or near
nought whatever `worth` makes of them: Riptide Laboratory (worth using only
to save a doomed Wizard, which is at instant speed in answer to removal),
Sensei's Divining Top (a reorder, worth something from STEADY up),
General Tazri's five-colour pump, and the equip costs of Sword of Hearth and
Home and Helm of the Host (taken the few times the mana floated). Two rows
have no reader either: nothing in the agent answers the engine's suspend
offer (`LegalActions::suspendable`: Ancestral Vision, Profane Tutor), and
Raffine's Tower's cycling is a card in hand, which `activate::printed` does
not reach. The planner is the next slice.

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
nought lands. The engine offers that land drop (CR 712.12), so the agent was
throwing away a hand whose land drops the engine was about to hand it, and
keeping a card it had already decided was a spare spell. A *transforming*
card is not that: in hand it has only its front face's characteristics
(CR 712.8a), and Arguel's Blood Fast reaches its Temple only by turning over
(#152). `castable_from_hand` tells the two kinds apart, held against
Scryfall's `layout` by `xtask validate`. `policy::plays_as_land` asks the
engine's own function, `CardDef::land_faces_from_hand`, so the agent and the
offer it answers cannot disagree: `a_land_on_a_modal_back_is_playable_and_one_on_a_transforming_back_is_not`
floors both populations and pins one card of each, and
`the_pool_prints_lands_on_a_back_face` floors the modal one.

The *cast* faces are a separate reading and they are **not** made yet.
`filter::cast_mode` returns the `Normal` option the moment one is offered, so
an adventure (CR 715) or an MDFC back is taken only when the engine offers
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
checks the actual coloured cost.

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

### Authorized AI scouting

House AIs now have an intentional advantage. The host's private
`scouting::request` can supply main-deck lists, every commander, current hands,
current sideboards and either bounded or complete library order, top first.
The current built-in policies request their own deck at every level, opposing
decks and hands at sharp, and three upcoming library cards at expert. Sideboards
are requested during wishes. Full-library access is supported and tested but is
not the default: the tactical policy does not benefit from copying it every move.

Every request checks the **current** `SeatKind::Ai`. Human seats, human-driven
AI chairs, disconnected humans' stand-ins and invalid seats are denied. There
is no public Session getter or network message, the reports have no serialization,
and neither human views nor print disclosure changes. Scouting is consumed for
one decision and never retained in the agent kept during a human takeover.
The ordinary view-only tests still prove independence from hidden cards; the
privileged path deliberately behaves differently. A weakened access guard was
injected and the security test failed before the guard was restored.

Deck lists are analysed once at setup, by printed properties and DSL effects.
Cheap creature density, interaction, draw and artifacts influence deployment
and resource priorities; a known opposing sweeper discourages committing a third
creature, and known empty threats release reserved mana. These are heuristic
adjustments, not a learned classifier or a complete combo planner. Reports carry
card identities without actionable hidden object handles.

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
