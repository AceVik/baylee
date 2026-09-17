# House AI

The five names in `AIProfile::NAMED` drive both the offline picker and the
network lobby. Existing `novice`, `steady`, and `sharp` keys still work.

| Profile | Opening hand | Spell selection and mana | Combat |
| --- | --- | --- | --- |
| novice | Keeps every hand | ±800 deterministic score noise; taps out | Individual trades |
| casual | Rejects fewer than two or more than five lands | ±400 noise; taps out | Individual trades |
| steady | Also checks early plays and colour access; bottoms excess lands | ±100 noise; reserves cheap interaction after establishing a creature | Individual trades |
| sharp | Same opening policy as steady | No noise; releases the reserve when opponents have no visible threat or cards | Searches whole attack sets and blocking assignments, including gang blocks |
| expert | Same opening policy as sharp | Same spell policy as sharp | Also simulates retaliation by surviving opponents |

All levels plan coloured payments through the same renderer-free mana matcher
as the human client. Simple printed and granted mana abilities count, with one
source per permanent. The agent does not tap toward an unaffordable spell. Command-zone commanders
participate in these plans, including their public cast-count tax. Steady and
harder levels choose a mana colour by the casts it can complete with the
remaining visible sources; an expensive, uncastable card cannot drown out the
colour needed for an affordable play.
Counterspells need an opposing stack entry, removal needs something opposing
on the battlefield, and a deferred pay-or-lose obligation is declined: the
current stateless policy cannot plan its future payment. Searches, bottoming,
and surveils at the skilled levels evaluate only identities actually visible
in `PlayerView`, including `looking_at` while a search is open.

`act(&PlayerView, &Pending)` is unchanged. There is no engine state constructor,
clone, or reference in the agent. The host test changes an opponent's entire
hidden hand and library and asserts identical views and actions for all five
profiles. The AI depends on `baylee-client-core` for its existing mana matcher;
that crate is renderer-free and adds no capability to inspect a game.

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
at a planeswalker is separate from damage to its controller. A proven lethal
player attack takes precedence over attacking a planeswalker. Node visits allocate no vectors; position vectors are built once per
decision and groups/results are fixed arrays on the stack.

Expert's retaliation is a greedy continuation from the surviving public
creatures, not a second full minimax tree. It includes untapped reserves that
cannot attack, such as Walls and summoning-sick creatures, and checks evasion
and two-creature menace blocks. Tapped opponents untap for this continuation,
and marked damage clears. Attack-side evasion remains an estimate; defender-side pairings come from the engine. Spell responses,
combat triggers, protection, replacement effects, commander damage, and hidden draws
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
answer out would violate the stronger requirement that the same view always
produce the same action. Latency is measured externally with
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
| 6 attackers / 6 blockers, novice | 553 ns | 0 | 0 |
| same, casual | 551 ns | 0 | 0 |
| same, steady | 555 ns | 0 | 0 |
| same, sharp | 125 µs (125.10–125.23 µs) | 4,096 | 7 |
| same, expert | 957 µs (956–959 µs) | 9,871 | 63 |
| empty priority, expert | 44 ns | — | — |
| eight lands / four spells, expert | 973 ns | — | — |

These include position construction and result allocation. The fixture has six
3/3 attackers facing six 2/2 blockers; a different board can exhaust either
profile's full budget. The benchmark prints the node counts alongside timing
so a fast fallback cannot masquerade as a search improvement.
