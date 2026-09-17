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
