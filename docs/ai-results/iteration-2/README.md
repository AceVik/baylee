# Second AI iteration

The mana-choice fix is `db81ec67`; combat correctness and expanded search are
`29e6455e`; cached counterattacks and borrowed priority offers are `35011c21`.
The agent still receives only `PlayerView` and `Pending`. No engine state,
hidden-card identities, wall clock, or unseeded randomness enters a decision.

## Paired games

Same sixteen seeds and four deck/seat permutations as the previous run:
`1,7,42,1337,2,3,5,11,17,19,23,29,31,37,41,43`.
Development build, acceptance decks Allytifact and Victory, 20,000-action cap.

| A | B | A wins | B wins | Unfinished | A decisive win rate |
| --- | --- | ---: | ---: | ---: | ---: |
| casual | novice | 35 | 29 | 0 | 54.7% |
| steady | casual | 35 | 29 | 0 | 54.7% |
| sharp | steady | 38 | 26 | 0 | 59.4% |
| expert | sharp | 31 | 33 | 0 | 48.4% |
| expert | novice | 38 | 26 | 0 | 59.4% |
| sharp | novice | 37 | 27 | 0 | 57.8% |

All 384 games finished, with no draws, panics, repeated-state halts or action
caps. This does not fix the engine assertion seen in the previous run: the
new trajectories simply did not reach it. Three actions were refused, all
novice accepting unaffordable miracle casts: seed 3, deck order 0, A in seat 0
in expert/novice and sharp/novice (`Entreat the Dead`); and seed 3, deck order 1,
A in seat 1 in sharp/novice (`Temporal Mastery`). A temporary engine/view
replay diagnosed those exact decisions; its source and output are in
`/tmp/baylee-ai-iteration-refusal-probe.rs` and
`/tmp/baylee-ai-iteration-refusal-trace.log`. These refusals remain counted.

The results are mixed. Sharp/steady improved from 33–31 to 38–26, but
expert/novice fell from 44–20 to 38–26 and sharp/novice from 43–21 to 37–27.
The extra search and corrected tactical cases are not evidence of an overall
strength increase, or of expert beating sharp. Two decks and correlated
paired seeds do not establish a universal difficulty ordering.

The full 64-game expert/novice and 64-game expert/sharp outputs match their
uncached `29e6455e` binaries byte-for-byte, including action counts, failure
counts and trailing decisions. The performance changes therefore preserved
these 128 games exactly. Novice and casual policies did not change; their
35–29 pairing also matches the previous iteration.

## Regression evidence

The following decision tests were run and failed before their fixes:

- Choosing blue to complete Baleful Strix with City of Brass and Swamp,
  despite white demand from unaffordable cards. Also reproduced through the
  real engine's activation, colour choice and casting sequence.
- Blocking with lifelink to survive otherwise lethal flying damage. The
  real-engine fixture now survives at two life.
- Rejecting life gain that would arrive after lethal first-strike damage.
  A separate engine fixture confirms the chosen block survives at two life.
- Counting a reserve Wall in expert's counterattack estimate. Follow-up
  cases cover summoning sickness, tapped/phased-out reserves, and flying.
- Keeping a proven lethal attack on the player instead of redirecting it to
  a planeswalker, and blocking player lethal before protecting a planeswalker.

Additional exchange tests cover excess lifelink damage and a double-striking
blocker whose first strike kills its only damage recipient. Existing tests
cover menace, trample/deathtouch, determinism, work budgets and hidden views.

## Performance

Criterion `--quick`, optimized M1 Max build, including position construction
and allocations. The game sweep and client were stopped during these timings;
other worktrees and desktop applications still shared the machine. Raw output
is in `bench-before-cache.txt` and `bench-after-cache.txt`. These short runs
are estimates, not controlled latency guarantees.

| Expert decision | Before counterattack/priority caching | After | Search work |
| --- | ---: | ---: | --- |
| Six attackers / six blockers | 1.34 ms | 1.05 ms | 31,180 nodes, all 64 attack sets completed/refuted |
| Eight attackers / eight blockers | 18.92 ms | 11.13 ms | 262,144 nodes, 128 sets completed/refuted |
| Empty priority | 36 ns | 16 ns | no search |
| Eight lands / four spells | 830 ns | 809 ns | coloured payment planning |

The original expert budget completed/refuted 63 of 64 sets in the six-versus-six
fixture at 957 µs. The expanded search spends slightly longer to finish that
last difficult candidate. Sharp uses 16,384 nodes; expert uses 262,144. The
cutoff remains deterministic, with no wall-clock-dependent action selection.

## Automated checks

- 49 AI unit tests, 62 host tests (one explicitly ignored soak), and three
  engine/view integration tests passed.
- The offline difficulty regression failed before `62e2bc3a`, then all 18
  offline client tests passed. The existing profile-to-preset test now also
  checks the listing contract that makes the difficulty picker accessible.
- `cargo fmt --all`, `git diff --check`, the full `scripts/gate-rules.sh`, and
  `cargo clippy --workspace --all-targets -- -D warnings` passed.
- macOS's debug client/test linker emitted its existing large `__eh_frame`
  warning. Compilation and tests succeeded; optimized AI benchmarks were
  measured separately.

## Manual client cases

Used the rebuilt native Bevy client with `--features dev-control`, its loopback
control socket on 28790, and the real offline lobby. Inputs followed the
current pending question; assertions read the client's public view. Screenshots
and state dumps are in `/tmp/baylee-ai-iteration-live/`, outside the repository
because screenshots include card art. Only this task's client processes were
stopped afterwards.

1. **Difficulty selection.** Before the fix, the offline room showed an AI
   chair but no difficulty controls. After `62e2bc3a`, all five controls were
   visible. Selecting expert changed the chair label to `KI (Experte)` and the
   game displayed `expert 1`.
2. **Mana choice under misleading colour demand.** Expert played Allytifact
   with City of Brass and Swamp on its initial battlefield, and Baleful Strix
   plus three Elesh Norn, Mother of Machines in its initial hand. The human
   kept Forest and passed. The expensive white cards remained unaffordable
   after the AI's land drop. On its first turn expert cast Strix using City
   and Swamp, leaving Cavern of Souls untapped; it ended at 39 life from City.
   `mana-latest.json` and `mana-pass.png` capture the result, with no client
   action error. An earlier Loran fixture gained a third land and correctly
   cast Loran; increasing the white cards' cost kept this manual case about
   choosing mana for an affordable multicolour spell.
3. **Player lethal despite an exposed planeswalker.** The human started at
   40 life with Jace, the Mind Sculptor; expert started with fourteen
   Restoration Angels. Both initial hands were Forest. The human kept and
   passed. The client's blocking question showed all fourteen attackers
   assigned to `Player(0)`, not Jace. Expert won in turn two's combat damage
   step, with human life −2 and `GameOver` naming player 1. `lethal-initial.json`,
   `lethal-latest.json` and `lethal-pass.png` record the case; no client action
   error was reported.

Lifelink and first-strike timing were tested through the real engine in the
integration tests above; they were not claimed as additional manual cases.
