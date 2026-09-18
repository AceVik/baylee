# Third AI iteration

The AI now evaluates the selected effect, adapts to deck properties and uses
seeded variation. Hosted house AIs can request privileged scouting, as
explicitly authorized for this iteration. Humans, driven AI chairs and
stand-ins cannot request it. The report never enters a view, print table or
wire message. The agent receives neither `Engine` nor `GameState`.

Implementation milestones: `ee2b23f4` (effect context and targets), `4637c7d7`
(scouting, deck analysis and variation), `1763eee6` (stack targets, variable
draws and commander retaliation), `a2f6a8dd` (rebound crash), and `6f64c484`
(cached commander/defender checks).

## Paired games

Same sixteen seeds, both deck orders and both profile assignments as iteration
two: `1,7,42,1337,2,3,5,11,17,19,23,29,31,37,41,43`. Development build,
acceptance decks Allytifact and Victory, 20,000-action cap. The six JSON files
beside this document contain all 384 games, actions and trailing decisions.

| A | B | A wins | B wins | Unfinished | A decisive win rate |
| --- | --- | ---: | ---: | ---: | ---: |
| casual | novice | 37 | 27 | 0 | 57.8% |
| steady | casual | 31 | 33 | 0 | 48.4% |
| sharp | steady | 42 | 22 | 0 | 65.6% |
| expert | sharp | 30 | 34 | 0 | 46.9% |
| expert | novice | 50 | 14 | 0 | 78.1% |
| sharp | novice | 43 | 21 | 0 | 67.2% |

All games finished with zero refused actions, draws, panics, repeated-state
halts or action caps. Expert/novice improved from 38–26 to 50–14 and
sharp/steady from 38–26 to 42–22. Steady/casual and expert/sharp regressed.
These policies include privileged information, so this is not a comparison
of equal-information search alone. Two decks and correlated paired seeds do
not establish a universal ordering of strength.

The first run did expose two engine panics: expert/sharp, seed 5, deck order 1,
A in seat 1; expert/novice, seed 29, deck order 0, A in seat 0. A copied
Ephemerate inherited `cast_from_hand`, scheduled rebound and ceased to exist.
The delayed cast then panicked in the casting wizard. The fix clears the
copy's flag, validates free casts and binds delayed permission to the exiled
object's version. Both affected games now finish. The other 382 game records
are identical to that first run, including action counts and trails.
Pre-fix outputs remain in `/tmp/baylee-ai-third-before-rebound-fix/`.

## Regression evidence

The following cases failed before their fixes:

- Target beneficial counters at a friendly permanent and harmful counters
  at an opponent; select lethal player damage over a smaller permanent.
- Choose a creature type from the deck, hand, battlefield and commanders,
  rather than always choosing Ally; use Jace's bounce when it removes a threat.
- Spend mana on meaningful X, respect the available target count, and direct
  Commander's Insight at its caster. Entreat the Dead with no legal targets
  must choose X=0 without a refused action.
- Reject a miracle when floating mana lacks the printed colours; an
  untapped land is not a payment opportunity inside that question.
- Do not counter the AI's own spell when the opposing stack entry is an
  ability that Counterspell cannot counter.
- Recognize commander lethal independently of life totals, including keeping
  a blocker against the opponent's commander on the next turn.
- Do not rebound a spell copy, or follow an exiled card after it leaves and
  returns as a new object.

Additional tests cover multiple commanders and their independent cast counts,
seed replay and variation, deck adaptation, and scouting order and limits.
The two-commander casting fixture uses Freeform; it is not evidence for every
legal Commander pairing. A deliberately weakened scouting guard made the
human-access test fail. The restored guard rejects Human, Driven, StandIn and
invalid seats, revokes access on takeover, and leaves human views and print
disclosure unchanged. The scoreboard's injected-loss test remains in the gate.

Four small, fully implemented Freeform deck families—tribal, artifacts,
control and graveyard—also complete eight games with no refused actions.
Each fixture card must have `Coverage::Implemented`; a stub fails setup.
These are mechanic probes, not tournament lists or proof of full-MTG coverage.

## Performance

Criterion `--quick` on the M1 Max, optimized profile, with this task's game
sweep and client stopped. Other worktrees and desktop applications still
shared the machine. Raw runs are `bench-before-cache.txt` and `bench.txt`.
The scouting benchmark includes report allocation, copying
eleven hand identities and six library identities, and the resulting AI
decision. It excludes walking engine zones and constructing the ordinary
player view and decision context. Deck analysis is measured separately
because it is cached at setup; its fixture repeats four spell identities.

| Decision | Before caching commander/defender checks | Final estimate | Nodes / completed or refuted attack sets |
| --- | ---: | ---: | --- |
| Sharp, 6v6 | 187 µs | 178 µs | 16,384 / 15 |
| Expert, 6v6 | 1.060 ms | 1.084 ms | 31,180 / 64 |
| Sharp, 8v8 | 264 µs | 257 µs | 16,384 / 15 |
| Expert, 8v8 | 10.98 ms | 11.40 ms | 262,144 / 128 |
| Expert, empty priority | 16.20 ns | 16.17 ns | — |
| Expert, eight lands / four spells | 825 ns | 812 ns | — |
| Same, with scouting report | 957 ns | 962 ns | — |
| Analyse 100-card deck once | 717 ns | 712 ns | — |
| Expert, mana colour choice | 2.286 µs | 2.275 µs | — |

Commander history and the defender do not change within a decision, so their
checks now live in the exchange cache. All 384 game records from the final
cached build match the pre-cache build byte for byte, including action counts
and trails. Comparison outputs are in `/tmp/baylee-ai-third-cached-games/`.
Search work is unchanged. The quick
runs did not establish statistical significance; the timings are estimates,
not guarantees. Sharp remains slower than iteration two's 156/235 µs at
6v6/8v8, while expert is close to its previous 1.05/11.13 ms. The additional
commander evaluation is retained for correctness. Decisions use fixed node
budgets, never wall-clock timeouts.

## Automated checks

`cargo fmt --all`, `scripts/gate-rules.sh` and
`cargo clippy --workspace --all-targets -- -D warnings` passed. The rules gate
ran 1,966 passing tests across 41 test binaries, with two existing explicit
ignores: the exhaustive card soak and the separate process-launch test.
Focused suites contain 60 AI unit tests, 64 host unit tests, seven engine/view
decision integrations, eight deck-family games in one test, and 604 engine
unit tests. Gate logs are `/tmp/baylee-ai-third-gate.log` and
`/tmp/baylee-ai-third-workspace-clippy.log`.

## Manual client cases

Built the native client with `--features dev-control` from `1763eee6`, which
contains the final AI policy, and used the repository's dev-control skill on
loopback port 28796. Each case selected expert in the real offline lobby and
answered the current pending question. The subsequent engine-only rebound
fix was verified by its regressions and the full match rerun above.

1. **Planeswalker choice.** Human battlefield: Restoration Angel; AI
   battlefield: Jace, the Mind Sculptor. Both hands: Forest. On the AI's first
   turn, Jace used −1, returning the Angel to the human hand and leaving two
   loyalty. `jace-initial.json`, `jace-pass.json` and `jace-pass.png` record the
   result with no client action error.
2. **X draw and target.** AI battlefield: five Islands; AI hand: Commander's
   Insight and Forest. Human hand: Forest. During the human's first turn,
   expert tapped all five Islands, cast Insight and drew two cards. The AI
   library fell from 99 to 97 and its hand rose from two to three after the
   spell left it. Human hand and library stayed at one and 99. The resolved
   spell appeared in the graveyard; no client action error occurred.
   `draw-initial.json`, `draw-pass.json` and `draw-pass.png` record the case.

State dumps and screenshots are in `/tmp/baylee-ai-third-live/`; screenshots
stay outside the repository because they include card art. Both client
processes started for these cases were stopped afterwards.

## Remaining coverage

The AI reads card properties and DSL effects rather than card/deck names, but
new rules still need decision tests. It does not yet plan arbitrary combos,
model every counter's context, simulate spell responses in combat search, or
understand every future effect. See the concrete
[coverage TODO ledger](../../ai-coverage-todo.md) for tests to add as those
mechanics and complete decks become available. TODOs are not passing tests.
