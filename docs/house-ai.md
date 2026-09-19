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
Command-zone commanders participate in these plans, including their public
cast-count tax. Steady and harder levels choose a mana colour by the casts it
can complete with the
remaining visible sources; an expensive, uncastable card cannot drown out the
colour needed for an affordable play.
Counterspells need an opposing stack entry, removal needs something opposing
on the battlefield, and a deferred pay-or-lose obligation is declined: the
current stateless policy cannot plan its future payment.

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
against the real engine. Searches, bottoming,
and surveils at the skilled levels evaluate only identities actually visible
in `PlayerView`, including `looking_at` while a search is open.

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
prints a mode that costs more than the card does.

**What a card says includes what it says behind a price.** Every reader here
that walks an effect list — `tactics::meaning`, `activate::gains`,
`harmless`, `draws`, `intelligence::sweeper` — asks
`baylee_cards_dsl::Effect::branches` which effects run another effect, rather
than listing them itself. Each of them used to carry its own list, each list
was short, and all five stopped at `Sequence` and `MayDo`: an effect printed
behind "unless you pay" sits in a variant carrying a single effect rather
than a list, so it was read by none of them. Thirty-five effects in the pool
are there — twenty-nine of them a Karoo land sacrificing itself — and two are
counterspells, Flusterstorm and Malevolent Hermit, which the agent held for
ever because `policy` only casts one it knows is a counterspell. `draws` sums
both halves of a two-branch effect although only one of them runs, which is
wrong in the one direction it is allowed to be wrong in: over-counting
refuses a safe draw, under-counting decks the seat out (CR 704.5b).

`act(&PlayerView, &Pending)` remains available and needs no hidden information.
Hosted AI seats additionally receive the selected spell/ability effects from
`Engine::decision_context`, covering cast modes and triggered or copied abilities.
This lets targeting distinguish beneficial counters and buffs from removal,
rank threats, and take lethal burn over a smaller permanent. Skilled planeswalkers
price the actual effect and loyalty spent; Jace can bounce a threat instead of
always ticking up. Creature-type choices follow the hand, battlefield and all
commanders. Counterspells distinguish spells from activated or triggered
abilities, and beneficial player-targeted draw goes to the caster. X uses
affordable coloured mana and the number of distinct legal targets; life-X
weighs friendly casualties and preserves the player's last life. Miracle
checks the actual coloured cost.

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

Randomness is keyed by the game seed, seat, choice sequence and object. The
same complete input replays identically. Sharp and expert use narrow spell-score
bands so equal options vary between games; proven combat lethal is not randomized.
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
attacking a planeswalker. Node visits allocate no vectors; position vectors
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

Regression tests prove different decisions between every adjacent profile,
repeatability, bounded search work, coloured payments, mana reservation,
opening-hand selection, commander tax, convoke payment, menace blocks, and a blocked attack that used to be
mistaken for lethal. The original landless-hand, mana-colour, and false-lethal
tests were run against the old policy and failed. The scoreboard's loss
mapping was also deliberately broken and its injected-loss test failed.

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
