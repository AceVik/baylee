# House AI measurements

The scoreboard is `cargo run -p baylee-gamehost --example ai_match -- A B 1,7,42,1337,2,3,5,11`.
Every seed plays both acceptance-deck orders and both profile assignments (four
games). The JSON records every seed, assignment, result, action count, refusal
count and failure trail. A win rate excludes unfinished games; always report
those alongside it. These are paired observations, not independent trials.

`baseline.json` was measured before changing the AI: sharp versus novice,
14 wins each, no draws, four unfinished out of 32 games. Two were repeated
casting decisions (seed 3), two panicked at `cast_wizard.rs:770` (seed 11).
The scorer's forced-loss test was mutation-checked: making every winner count
for A failed `the_scoreboard_observes_a_loss_and_a_cap`, then passed when restored.


## Final paired run

Policy revision `2cbea143`, development build, 16 seeds and four permutations
per seed (64 games per pairing):
`1,7,42,1337,2,3,5,11,17,19,23,29,31,37,41,43`.

| A | B | A wins | B wins | Unfinished | A win rate among decisive games |
| --- | --- | ---: | ---: | ---: | ---: |
| casual | novice | 35 | 29 | 0 | 54.7% |
| steady | casual | 31 | 32 | 1 | 49.2% |
| sharp | steady | 33 | 31 | 0 | 51.6% |
| expert | sharp | 32 | 32 | 0 | 50.0% |
| expert | novice | 44 | 20 | 0 | 68.8% |
| sharp | novice | 43 | 21 | 0 | 67.2% |

No draws. Of 384 games, 383 finished and one panicked at the already-observed
`cast_wizard.rs:770` assertion, `wizard card known`: steady/casual seed 23,
deck order 0, A in seat 1. No final game stopped for a repeated action or cap.
The failure is retained in `steady-casual.json`; it is not counted as a win.
The exact original 32-game seed slate now gives sharp 21 wins and novice 11,
with no unfinished games. Both policies changed, so this is not a direct
new-versus-old-agent tournament.

The complete 64-game expert/novice run was repeated: its JSON was byte-for-byte
identical, including every action count and trailing decision. All completed
games across the six pairings recorded zero refused actions.

The results support a sharp/expert advantage over novice in these acceptance
decks. They do **not** establish five strictly ordered strength levels:
steady/casual is essentially tied, as is expert/sharp. Adjacent-profile unit
tests establish distinct decisions, not a statistical strength guarantee.

An intermediate 384-game run found convoke payment loops in casual/novice
seed 41 and expert/novice seeds 5 and 31. That finding produced a regression
which selected one permanent before the fix and six after it. A later live
observation found command-zone cards absent from prospective mana plans;
that regression also failed before its fix. The committed JSON files were
regenerated after both fixes.

## Live observation

Used `xtask dev-table --seats 2 --ai sharp` against an isolated local gateway
on port 28786, and a networked Bevy client with `dev-control` on 28790. The
client was driven by reading each pending question, playing offered human
lands, answering fetch searches, and passing combat/priority. This was an
observation run, not a competitive match counted above.

Sharp developed its mana, cast Restoration Angel, Baleful Strix and Orcish
Bowmasters, created an Army, and attacked. The turn-29 screenshot showed the
human at 23 life and the AI at 37; the AI won by combat on turn 36. The final
client state reported `GameOver { winner: Player(1), reason:
LastPlayerStanding }`, with no client action error. Local screenshots live
at `/tmp/baylee-ai-live/`; they are deliberately not committed because they
contain card art. Only this task's gateway, agent, engine and client processes
were stopped afterwards.

The live run also demonstrated remaining limitations: colour-choice and
spell-target selection are heuristics, and a commander being considered by
the planner does not guarantee it gets cast ahead of other spells. See
[the model and timing limits](../house-ai.md).


## Checks on the final code

- `cargo fmt --all` and `git diff --check`.
- `DATABASE_URL=postgres://baylee:baylee@127.0.0.1:5432/baylee ./scripts/gate-rules.sh`.
- `cargo clippy --workspace --all-targets -- -D warnings`.
- `cargo test -p baylee-engine --lib --release`: all 602 tests passed.
- 42 AI tests, the multiplayer soak, the hidden-zone host regression, and the
  mutation-checked scoreboard and dev-table regressions passed.
- `cargo bench -p baylee-ai --bench decisions -- --quick`; numbers are in
  [house-ai.md](../house-ai.md#decision-benchmarks).

## Further iterations

See [iteration two](iteration-2/README.md) for corrected mana and combat edge
cases, larger search budgets, caching, client checks, and the new 384-game
run. The original results above remain available for comparison.
