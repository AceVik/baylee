# House AI: a strict strength order, then rounds against BayleeAI

Plan, written 10.11.2026, and the handover for whoever continues it (§0);
apart from the loop fixes of §1a nothing here is built. The owner's
decisions it follows:

- The levels are strictly ordered in strength: novice < casual < steady <
  sharp < expert. `seer` sits above them, unfair by design and never a
  default. Today the order does not hold: `sharp` was seen beating
  `expert`, and the first ladder below finds `steady` losing to `casual` and
  `expert` playing `sharp`'s game in all but ten seeds.
- No level imitates human mistakes. A level's randomness exists only so that
  players cannot learn to exploit it; it stays for that reason alone.
- The frozen `sharp-2026-11` does not move. Every change below is gated by
  the profile generation it arrives in (`AIProfile::plays`), and the pinned
  answers (`tests/reference_profile.rs`) and the fair-play test
  (`tests/fair_house.rs`) stay green after every step.
- After the order holds, the house improves in rounds alongside BayleeAI.
  Each round studies the games BayleeAI or a language-model seat won against
  live `expert` and fixes general weaknesses, never exploits aimed at
  BayleeAI. The house stays an opponent, never a teacher.

## 0. Handover (10.11.2026)

For the agent that takes this up. Everything above §1 is binding; the rest
is the plan and what has been learned.

**Where it stands.** The ladder after the loop fixes (5056ce4f5), duels on
the constructed test split (`decks-20261107c`), 512 eval4 rows per pair,
each game counted once (§1a has the table with the open games):

| Pair | Share | Wilson 95 % | Seeds for / against |
| --- | ---: | --- | ---: |
| casual – novice | 55.9 % | 51.5–60.1 | 43 / 13 |
| steady – casual | 48.4 % | 44.1–52.8 | 20 / 28 |
| sharp – steady | 53.1 % | 48.8–57.4 | 23 / 7 |
| expert – sharp | 50.8 % | 46.5–55.1 | 7 / 3 |
| sharp – `sharp-2026-11` | 50.0 % | 45.7–54.3 | 0 / 0 |
| seer – expert (`--seer-peeks`) | 49.2 % | 44.9–53.5 | 2 / 6 |

Only casual over novice holds. No game is capped any more.

**Generations** (`AIProfile::generation`, asked as `profile.plays(...)`,
`crates/baylee-core/src/preset.rs`). Every level a player is offered plays
`GENERATION`, the newest:

- `FROZEN_2026_11` = 0: `sharp-2026-11`, the benchmark BayleeAI is
  measured against. It never moves. Its answers are pinned by
  `crates/baylee-gamehost/tests/reference_profile.rs` (six tables, 8,111
  answers, fixture `tests/reference/sharp-2026-11.json`, answer hash
  `17cf06640914fac8`); only an engine or card fix that changes those games
  re-records it (`BAYLEE_REFERENCE_RERECORD=1`, in a commit of its own).
- `TEAM_PLAY` = 1: house teammates show each other their hands and read
  them; an ally's board and spells count as its side's.
- `EXACT_SPELL_X` = 2: an X that must equal a target's mana value is
  planned for a spell on the stack (Spell Blast).
- `INSTANT_TIMING` = 3, `RACE` = 4, `ABILITY_MANA` = 5, `SEER_DRAWS` = 6:
  the second round (§0a).
- A strength change adds the next generation, sets `GENERATION` to it and
  asks `plays(...)`; `docs/house-ai.md` §"The frozen reference" lists them.
  The one change that reaches generation 0 on purpose is the stall guard
  (a loop guard that changes no game that ends).

**Order of work.** steady over casual first (§3: ablate steady's hold-up
and mulligan against casual, then a hold-up that knows when to tap out),
then the expert ablations and repairs (§2, §3), then a seer that uses what
it sees (§3a). Each step measured on the ladder before the next starts.

**Gates** (coordinator, 11.11.2026; this replaces a Wilson lower bound
over games, which treats the games of a duplicate pair as independent).
An order between two levels holds where the two-sided sign test over the
seeds the higher level turned (won from both seats against lost from both)
gives p < 0.05 in its favour, on the test split with round-robin pairings
(`eval4 --deck-schedule round-robin`, on main since b65ce6c80), 512 duel
rows; `house_duplicate` prints it. Per deck: no deck with ten or more seeds
turned against more often than for (`house_duplicate`'s per-deck rule
line). seer must clearly beat expert by the same test, and not do worse at
a 2v2 table. Train-split numbers are development numbers only. After every
step the pin test (hash unchanged), `tests/fair_house.rs` and
`tests/team_play.rs` stay green.

**Tools.**

- eval4 (`crates/baylee-train`, `--features onnx`, built with
  `--profile selfplay`): `--random-policy` puts the net seat on noise so
  only the house-against-house baseline game counts; `--baseline <level>
  --against <level>`, levels by name including `sharp-2026-11` and `seer`;
  `--seats 2` for duels, `--seats 4 --teams 1,2,1,2` for 2v2; `--seer-peeks`
  lets seer be told what it peeks at (every other chair stays fair, and
  the summary records it). The ladder ran as
  `eval4 --model <r1-4eb fp16> --random-policy --decks-manifest
  ~/baylee-data/v6/decks-20261107c/manifest.v4.json --deck-split test
  --deals 512 --threads 4 --seats 2 --baseline A --against B --out A-vs-B.json`
  (eval4 wants a model; with `--random-policy` the net's answers are
  random and its game is not read).
- `house_duplicate` (`cargo run -p baylee-train --features play --bin
  house_duplicate -- --run A-vs-B.json --out A-vs-B.dup.json`): each game
  once, the share with its Wilson interval, seeds by sides won, per-deck
  turns. A stopgap: once `baylee_train_ops::evaluation::games`
  (`c42/v6t-games`) is on main it is removed or becomes a thin reader over
  `games`, so that one counter stays.
- `team_exercise` (`cargo nextest run -p baylee-gamehost --run-ignored
  only --no-capture team_play_is_exercised`): how often team play changes
  an answer at a real 2v2 table.
- `baylee-seat` `tests/seated_house.rs`: the seated house must keep giving
  the table house's answer (3,070 of 3,070 on 10.11.2026).

**CPU.** BayleeAI trains on the same machine. Ask the coordinator for a
slot before any measurement run, and run it with 4 threads under `nice -n
10`. Never use the GPU. A 512-row duel pair takes about 7 s, the whole
ladder about 2 minutes, a 2v2 run about 70 s.

**What the code does not tell you.**

- expert and sharp play the same duel in all but ten of 256 seeds: the
  retaliation search (lookahead 2) almost never changes a decision. expert
  is not yet a different level; §2's ablations are where to start.
- steady loses to casual mostly where a green creature deck taps out
  against it (M19 *Jungle Secrets*, *Forest's Might*): its hold-up keeps
  cheap interaction up where developing the board wins.
- seer is told every hidden card (held fair it plays exactly as expert,
  0 / 0) and does not use it: with the peek it turned two seeds for and six
  against, so what little of expert reads a peek (the library top) does not
  pay as it stands.
- The house does not firebreathe a Shivan Dragon for lethal (three
  Mountains, the opponent at 8): it wins over two turns instead. A free
  pump (Flowstone Hellion) it does use, one activation at a time; the
  stall-guard test pins that.
- `policy::reserve` keeps no mana up for a seat with no creature in play,
  which is exactly a control deck's board (§3, control piloting).
- One house agent asked a new question (a new view `seq`) from an
  unchanged position reads it as its action taken back and passes (the
  stall guard). A test that asks one agent many questions from one
  position needs an agent per question.
- `sharp` and `sharp-2026-11` are identical in duels; team play shows only
  at team tables.

## 0a. The second round (10.–11.11.2026)

What the second agent built, measured and dropped, and where the ladder
stands after it. §0 above is the state it started from.

**The ladder now** (925feb88c, generation 6, duels, each game counted
once by `house_duplicate`, round-robin deck pairings, 11.11.2026). Test
split as the gate reads it, 512 rows; train split, 2048 rows, beside it as
development numbers. The sign test is two-sided over the seeds the level
turned (won from both seats against lost from both), which is the
information a duplicate pairing carries; the Wilson interval treats the
games as independent and is far wider.

| Pair | Test 512: share, for / against, p | Train 2048: share, for / against, p |
| --- | --- | --- |
| casual – novice | 51.8 %, 29 / 20, 0.25 | 53.8 %, 161 / 83, < 0.0001 |
| steady – casual | 52.9 %, 30 / 15, **0.036** | 51.9 %, 119 / 80, 0.007 |
| sharp – steady | 50.8 %, 17 / 13, 0.58 | 52.9 %, 103 / 44, < 0.0001 |
| expert – sharp | 51.4 %, 11 / 4, 0.12 | 51.3 %, 63 / 35, 0.006 |
| seer – expert (`--seer-peeks`) | 51.2 %, 18 / 12, 0.36 | 51.5 %, 79 / 48, 0.0075 |
| steady – `sharp-2026-11` | 52.9 %, 28 / 13, 0.028 | |
| sharp – `sharp-2026-11` | 52.7 %, 25 / 11, 0.029 | 54.3 %, 136 / 48, < 0.0001 |
| expert – `sharp-2026-11` | 55.3 %, 35 / 8, < 0.0001 | |

seer against expert at a 2v2 table (test split, 512 rows, fixed pairings,
since round-robin seats duels only): 48.8 %, 11 / 14, p = 0.69, not
measurably worse. No deck reaches ten seeds at 512 test rows, so the
per-deck rule has nothing to read there.

By the gate (test split), steady over casual holds; the other rungs lean
the right way and stay short of p < 0.05, casual over novice included, which
held under the old fixed pairings. On the train split every rung holds. A
rung that moves by one to three points turns 15 to 50 seeds at 512 test
rows, too few for this test to tell; the test split holds 108 constructed
decks, so a gate read on 2048 test rows (or the test and val splits
together) would decide where 512 cannot. Every level now beats the frozen
reference: the benchmark BayleeAI is measured against plays below today's
`steady`.

**Generations added** (`AIProfile`, `docs/house-ai.md` §"The frozen
reference" lists them; every one stays off `sharp-2026-11`, whose 8,111
pinned answers are unchanged):

- 3 `INSTANT_TIMING` (steady and up, `HoldUp` other than `None`,
  `crates/baylee-ai/src/timing.rs`): an instant is cast at its moment (a
  trick once blocks are declared where it turns an exchange, on the
  creature it turns; anything lasting in its own main phase or the
  opponents' combat or end step), and mana is kept up for a counterspell
  only, and for a trick before combat. steady against casual on train
  (2048 rows): 48.2 % before, 52.1 % after; the old hold-up switched off,
  50.5 %.
- 4 `RACE` (expert, lookahead 2, `search::race_damage`): the attack search
  prices damage by the life it leaves (90/160/300 a point above 14, above
  7, below), and the swing-back at three quarters of the same price. expert
  against itself without these prices and without generation 5: 51.6 % (59
  for, 26 against; sign test p ≈ 0.0003). The same prices on sharp, whose search has no swing-back: 49.7 %.
  The block search keeps the flat price (tests showed expert chumping at
  ten life otherwise). Tuned on train over eight price sets; all the
  race-aware ones landed between 51.2 % and 52.2 %.
- 5 `ABILITY_MANA` (sharp and expert, `activate/floating.rs`): mana is
  made for an ability worth its price that the engine offers only once the
  mana floats (equip, paid pumps, paid draws). 22 of 1,024 seeds changed
  (15 for, 7 against): the house decks print few such abilities that
  `worth` prices above their cost.
- 6 `SEER_DRAWS` (seer): the mulligan reads the next three cards of its
  own library. seer against itself without it: 50.9 % (67 for, 50
  against).

**Tried and dropped** (measured, not carried):

- Control piloting for expert (a `HoldUp::Control`: counter mana kept up
  with no creature of its own, a small creature spell let through while
  the opponent holds cards), the owner's tip after Victory. In
  constructed duels 49.8 % against expert without it; in Victory's
  commander duels (`--shape commander --held-out-cards`, Victory's deals
  written by `--write-deals --pilot-decks house/victory` and played with
  their paired deals, 228 games on round-robin pairings) 48.7 %, the deck
  turned 5 seeds for and 8 against. Few decks hold a counterspell, and
  holding one for a later threat did not pay where one does.
- A turn plan (cast the first spell of the best set the mana pays for,
  not the best spell alone): 49.5 % for steady and for sharp against
  themselves without it.
- seer's combat with the opponent's known trick expected on the worst
  creature for it: 22 seeds changed, 11 each way. It also needed the
  agent to ask for a scouting report at attacks and blocks, which it does
  not today.
- Race pricing for sharp (above), and race pricing in expert's block
  search.

**Ablations** (test split, 512 rows, before generation 3): steady without
its hold-up 50.4 % against casual (48.4 % with it); with casual's mulligan
47.1 %; with casual's noise 48.2 %; with sharp's search 51.6 %, and without
its hold-up as well 54.1 %. sharp without its hold-up 55.9 % against
steady (53.1 % with it). expert with lookahead 1, or with sharp's noise,
50.2 % against sharp. The noise between ±12 and ±400 makes no measurable
difference anywhere.

**Tools added.**

- eval4 levels take knobs after a colon for an ablation:
  `--baseline steady:hold_up=none,mulligan=1`, `--against
  sharp:generation=4` (`eval4/profile_spec.rs`; `lookahead`,
  `temperature`, `mulligan`, `hold_up`, `politics`, `generation` up to
  this build's; never sight). `X:generation=N` is the level without the
  generations after N, which is how every row above measures one change.
- `eval4 --record-baseline <dir>` records the house-against-house game of
  every deal, laid out as a `selfplay` run; with `--only-deals` (a JSON
  list) it replays chosen deals. `gamelog4 show --run <dir> --game <deal>
  --seat <n>` prints one from a chair. The useful reading: a deal a level
  lost from both seats, and the same seat with the same deck and seed under
  the other level (deal `2t` and `2t+1`): the games are the same up to the
  first answer where the levels differ.
- `house_duplicate` prints the gate's numbers: the two-sided sign test
  over the seeds the level turned, and the per-deck rule line.
- `eval4 --write-deals <file> --pilot-decks <keys>` (upstream, deals4)
  writes the deals a deck is piloted in; add each deal's pair (`d xor 1`
  in a duel) before `--only-deals`, so each seed is played from both
  seats and `house_duplicate` can count it.

**Method notes.**

- Develop on the train split (2048 rows, about 30 s a pair on 4 threads)
  and read the gate on test; 512 test rows move by two or three points
  between builds that differ in a handful of seeds. A pair's measurement
  noise is about the square root of the seeds that differ.
- Never edit sources while a test build runs (the build reads half a
  change), and use `X:generation=N` rather than a second build to compare.

**What the code does not tell you (second round).**

- Every skill found here moves a rung by one to four points. The levels
  sit close because deck matchups decide most deals: in a duel pair of
  house levels, 78–90 % of seeds are won by the same deck from both seats.
- expert is still about sharp: what sets it apart now (retaliation with
  race prices) gains it a point or two against its own older self and
  about nothing against sharp. Its next real lever is lookahead, which
  waits on the owner's two decisions in §3 (4).
- seer gains little from what it sees through the heuristic paths tried
  (its own draws at the mulligan, known tricks); the uses of §3a that
  remain are counters around a known hand and removal timing. A clear
  margin looks out of reach without the engine fork with
  `DeckKnowledge::Known`, which seer may use but which is not shipped to
  the hosts (§3 (4), the owner's decision).

## 1. Measure the ladder first

House against house on eval4, CPU only, with the net seat on
`--random-policy` so that only the baseline game counts:

- Pairs, each its own run of 512 rows on the campaign's constructed test
  decks (`decks-20261107c`): novice–casual, casual–steady, steady–sharp,
  sharp–expert. Add sharp–`sharp-2026-11` as a control (it should sit at
  50 %), expert–`sharp-2026-11` (what the benchmark sees) and expert–seer.
- A duel's rows rotate the focal seat with one seed, so every seed is played
  from both seats. `house_duplicate` counts each game once and reports the
  higher level's share with a Wilson interval, the seeds by sides won, and
  per deck the seeds that the level alone turned.
  It is a stopgap: once `baylee_train_ops::evaluation::games`
  (`c42/v6t-games`, eval4 rows carrying `"game"`) is on main, the ladder
  reads eval4's own game counts and `house_duplicate` is removed or becomes a
  thin reader over `games`, so that one counter stays.
- An order holds where the higher level's Wilson lower bound is above 50 %.
  Where it does not, the deck table says where it fails.
- Deck pairings: eval4 seats deck `d mod N` against `(d xor 1) mod N`
  whatever the seed, so 256 seeds cover only N/2 fixed pairings. The first
  ladder is read with that caveat; the final ordering numbers use the
  campaign's `--deck-schedule round-robin` once it lands (`c42/v6t-victory`).
- `seer` against `expert` needs the peek let through: eval4 holds every
  chair to its own deck (`--fair-scouting`, on by default), which would make
  it `expert` against `expert`. `eval4 --seer-peeks` lets the level that
  peeks be told what it peeks at and keeps every other chair fair; the
  summary records it.
- Victory, a Commander deck, is measured by the campaign's commander-shape
  study, not here.
- Cost: a 512-row duel run took about 70 s on 4 threads for 2v2, and duels
  are cheaper still. The whole ladder is roughly 10 minutes on 4 threads.
  It is asked for as a CPU slot before it runs.

### First ladder (10.11.2026, the team step's build, before any strength change)

Duels on the constructed test split (`decks-20261107c`), 256 seeds each
played from both seats, each game counted once (`house_duplicate`); the
deck pairings are eval4's fixed ones (see above). Share of decided games for
the higher level, Wilson 95 %, and seeds whose result the level alone
decided (won from both seats / lost from both):

| Pair | Share | Interval | Seeds for / against |
| --- | ---: | --- | ---: |
| casual – novice | 55.6 % | 51.3–59.9 | 42 / 13 |
| steady – casual | 48.2 % | 43.9–52.6 | 19 / 28 |
| sharp – steady | 53.1 % | 48.7–57.4 | 23 / 7 |
| expert – sharp | 50.9 % | 46.5–55.2 | 7 / 3 |
| sharp – `sharp-2026-11` (control) | 50.0 % | 45.7–54.3 | 0 / 0 |
| expert – `sharp-2026-11` | 50.9 % | 46.5–55.2 | 7 / 3 |
| seer – expert (`--seer-peeks`) | 49.0 % | 44.7–53.4 | 1 / 6 |
| seer – expert (held fair, control) | 50.0 % | 45.7–54.3 | 0 / 0 |

The seer rows are from the seated-house build (75584ba33); the house's own
play is the same as the team step's. Each pair has three to six games
eval4 capped at 5,000 answers, counted open; those are the loops of §1a.

Only casual over novice holds. steady loses to casual, and most where a
green creature deck taps out against it (M19 *Jungle Secrets* and *Forest's
Might*, 0 for / 3 against each): steady's hold-up keeps cheap interaction up
where developing the board wins. sharp over steady leans right but its
interval reaches below 50 %. expert and sharp play the same game in all but
ten seeds, so expert is not yet a different level in a duel: its retaliation
search almost never changes a decision. The control shows the frozen
reference and today's `sharp` identical in duels (team play only shows at
a team table).

`seer` is told the hidden cards (the peek reaches it: held fair it plays
exactly as expert), but nothing in expert's play uses them yet beyond the
old library peek, and it is no stronger: one seed for, six against.

So the first work is steady over casual (hold-up that knows when to tap
out), then a real difference at the top (§3), each measured on this ladder
and, for the final numbers, on round-robin pairings, and a seer that uses
what it sees (§3a).

### 1a. Games that never ended (fixed 10.11.2026)

The capped games were house loops: the engine took a cast back and the
house cast it again from the same position. Spell Blast was cast at an X no
spell on the stack had (fixed behind `AIProfile::EXACT_SPELL_X`), and a
seat answered without the question's note announced Kaervek's Purge at X 0
(fixed by the note). A stall guard at every level now passes instead of
giving an action again from the position it was taken back to
(`docs/house-ai.md` §"The frozen reference").

The ladder again on that build (5056ce4f5), the same 512 rows per pair;
open games are the ones eval4 capped at 5,000 answers:

| Pair | Share | Interval | Seeds for / against | Open before → after |
| --- | ---: | --- | ---: | ---: |
| casual – novice | 55.9 % | 51.5–60.1 | 43 / 13 | 5 → 0 |
| steady – casual | 48.4 % | 44.1–52.8 | 20 / 28 | 6 → 0 |
| sharp – steady | 53.1 % | 48.8–57.4 | 23 / 7 | 5 → 0 |
| expert – sharp | 50.8 % | 46.5–55.1 | 7 / 3 | 5 → 0 |
| sharp – `sharp-2026-11` (control) | 50.0 % | 45.7–54.3 | 0 / 0 | 4 → 0 |
| expert – `sharp-2026-11` | 50.8 % | 46.5–55.1 | 7 / 3 | 5 → 0 |
| seer – expert (`--seer-peeks`) | 49.2 % | 44.9–53.5 | 2 / 6 | 6 → 0 |

No game is capped any more, and the formerly open games split as before:
the order still holds only for casual over novice.

## 2. Why `expert` does not beat `sharp` (to check before building)

`expert` differs from `sharp` in three ways: a combat search that also
prices the opponents' retaliation (lookahead 2, 262,144 nodes against
16,384), narrower noise (12 against 30), and, before the fair-play change,
the next three library cards. Ablations on paired deals, each against
`sharp`:

1. `expert` with lookahead 1. This separates the retaliation search from
   the noise.
2. `expert` with sharp's noise. This separates the noise from the search.
3. Per deck, especially aggressive and token decks. The retaliation pass
   keeps blockers home and may cost tempo that the attack search would have
   won.

Whichever of these accounts for the gap is the first thing to fix, behind a
new generation.

## 3. Making `expert` clearly the strongest fair level

The order also fails one rung lower, and that is fixed first: **steady
over casual.** Ablate steady with `HoldUp::None` (casual's) and with
casual's mulligan skill against casual on the same deals; the first ladder
points at the hold-up, which keeps a cheap instant's mana up where a
creature deck should develop. The fix is a hold-up that weighs the threat
it holds up for against the board it gives up, for steady and up.

In order of expected value per effort, each measured on paired deals against
both `sharp` and `sharp-2026-11`, per deck, before the next starts:

1. **Repair what the ablations find.** For example, the retaliation search
   could keep a creature home only when the crack-back it fears is not
   already answered by the creatures `hold_back_for_the_crack_back` keeps.
2. **Control piloting**, the owner's Victory note.
   - *Hold counters for the opponent's turn.* `policy::reserve` keeps no
     mana up for a seat with no creature in play, which is exactly a
     control deck's board.
   - *Counter the threats that matter.* Today every spell but a cantrip is
     countered on sight; a counter should instead be weighed against the
     threats the opponent can still cast.
   - *Do not tap out needlessly.* A sorcery-speed play in the main phase
     should leave counter mana up where the hand holds a counter.
   - These are rules for `expert`; `sharp` and below keep today's
     behaviour, so the order widens.
3. **Tune the worth constants against the frozen reference.** Candidates:
   - the activation threshold (`worth::THRESHOLD`, 25);
   - the life prices (`life_price`: 30, 60 and 150 per point by bracket);
   - the mana price at instant speed (`mana_price`);
   - the punish value of draw triggers (`PUNISH`, 150);
   - the hold-up reserve.

   The method is coordinate descent with a paired duplicate measurement on
   the train split. The final number is taken once on held-out test decks.
   A per-deck guard rejects any change that turns more seeds against than
   for on any deck with ten or more seeds. The tuned constants live in the
   profile (a new generation), so `sharp-2026-11` keeps today's.
4. **End-of-turn lookahead through the engine fork.** Fork at the
   seat's own end step or at a decision with several close options, play a
   short horizon with the house on every seat, and score the leaves. Two
   owner decisions come first:
   - *Fairness.* `Engine::fork_for(seat, DeckKnowledge::Known, ...)` deals
     the hidden cards from every seat's real deck list, which a fair level
     may not read. A fair fork needs a public-prior belief: deal the
     opponents' hidden cards from a prior (the format's card pool weighted
     by what they have shown), not from their list. That is an engine
     capability to add and test (`DeckKnowledge::Prior`). `seer` may use
     `Known`.
   - *Hosting.* The fork lives under the engine's `simulation` feature,
     which `cargo dev gate shipped-features` keeps out of every hosting
     build. A hosted `expert` with lookahead means shipping that feature to
     the engine server, a security and performance decision
     (`docs/proposals/lookahead-design.md`, owner rules on internet-facing code).
     Without it, lookahead stays an offline or eval-only level.

The noise stays: each level keeps a seeded spell-score band
(`temperature_milli`) so that equal options vary between games. It is not
widened to weaken a level; the levels differ in search, discipline and
mulligan skill.

## 3a. A seer that uses what it sees

The owner wants the unfair level to be the strongest: `seer` must clearly
beat `expert`. Today it is told every opponent's hand and library top
(`Sight::Peeking`, `scouting_request`) and plays `expert`'s game with it.
Each use below goes behind `profile.peeks()` (never a fair level) and a new
generation, and reads only what the host's scouting report hands a peeking
chair. In order of what obviously pays:

1. **Combat.** Attack into an open hand that holds no instant that can
   answer the attack (no trick, no removal, no flash blocker), and hold
   back from the attack or the block a known trick punishes. The combat
   search takes the opponents' castable instants as part of the position.
2. **Counters.** Keep a counter up for the known threat in the opponent's
   hand rather than spend it on the first spell, and do not cast into a
   known counter the opponent can pay for: bait with the lesser spell, or
   wait until they tap out.
3. **Removal timing.** Against a known hand, spend removal on the threat
   the hand cannot replace, hold it when a bigger threat is coming, and
   play around a known sweeper by not over-committing.
4. **Library knowledge.** Draws and scry, surveil and "look at the top"
   choices read the known top cards; a cantrip is cast when the top card is
   wanted, and the mulligan and the land drop plan for the known draws.

Measured against `expert` in duels on round-robin pairings with
`--seer-peeks`, and once at a 2v2 table. The gate is a clear margin: the
Wilson lower bound of seer's share above 55 % over 512 duel rows (and
above 50 % in the 2v2), with no deck turned more often against than for
among decks with ten or more seeds. Each of the four is measured alone
first, so a use that does not pay is dropped rather than carried.

## 4. Rounds after the order holds

Per round:

1. Read the games BayleeAI champions and language-model seats won against
   live `expert` on the campaign's shared rating ladder. Classify the
   losses by window (priority, attack, block, target, mulligan, payment)
   and by deck.
2. Fix general weaknesses only. A rule that keys on how BayleeAI plays is
   refused.
3. Measure: the adjacent-pair ladder (the order must still hold), live
   `expert` against `sharp-2026-11` (no regression), the pinned answers
   unchanged, and fair play unchanged.
4. Report per round: what changed, the ladder, expert against the
   reference, per deck, and the rating ladder's numbers from the campaign.

CPU runs go through the coordinator; nothing here uses the GPU.

## 5. The third round: foresight through the engine fork (11.11.2026)

The owner's goal for this round: every level wins at least 62 % against
its own generation-6 self, by playing better and not by playing against
that predecessor. §0a found that a hand-written rule moves a rung by one to
four points, so this round builds §3 (4): the house tries its options in
copies of the game and plays the one that comes out best. This section is
the plan, written before the build; §5a and later record what was built
and measured.

### 5.1 How the result is measured (fixed before the work)

1. **The predecessor is pinned.** `tests/generation_pins.rs`
   (`baylee-gamehost`) plays every level at generation 6 over the
   reference's six tables and holds the answer hashes
   (`tests/reference/generation-6.json`, recorded on dd9f9f329, whose
   house code is main's e2f693dab). `eval4 --against expert:generation=6`
   builds the same profile (`AIProfile::at_generation`). A change to
   shared code that reaches generation 6 fails the pin and is gated.
2. **Development on the train split, the gate once on held-out decks.**
   The gate: test split and `holdout2`
   (`manifest.v4.holdout2.json`), `--deck-schedule round-robin`, each
   game counted once (`house_duplicate`), at least 1,024 independent duel
   games per level, `<level>` against `<level>:generation=6`. The number
   is the share of decided games with its Wilson interval and the sign
   test over turned seeds; the goal is a point estimate of 62 %. One 2v2
   run per level must not be worse than 50 %.
3. **Generalisation.** The new level and its generation-6 self each play
   the frozen `sharp-2026-11` and BayleeAI r1 with its tactical bundle on
   the same deals; the gain must show there too. A large old-against-new
   margin with a small gain against these two would be exploitation and
   does not count.
4. **Per deck:** no deck with ten or more seeds turned against more often
   than for (`house_duplicate`'s rule line).
5. **The ladder stays ordered,** by budget: a higher level tries more
   options in more worlds.

### 5.2 What is built

**Foresight** (generation 7, `AIProfile::FORESIGHT`, a level's
`foresight` tier, zero for a profile from before it). At a decision with
more than one sensible answer the host forks the game for the seat
(`Session::fork_for`), gives one candidate answer in the fork, lets the
house play every chair on to a horizon, and scores where the game stands
from the seat's view in that fork. Each candidate is tried in the same
worlds (the same deals of the hidden cards and the same future
randomness), and the level's own answer stands unless another candidate's
mean score beats it by a margin.

- *Roots.* What the engine can fork: ordinary priority with nothing
  suspended, declare attackers, declare blockers. Candidates come from the
  question's own offer: the level's answer first, then pass, each land,
  each spell and ability offered, and for combat the level's declaration
  beside none, all, and its neighbours by one creature.
- *The opponent in a fork is never the real opponent.* Every chair of a
  fork is played by the searching level's own generation-6 play, seeded
  from the policy seed, with no deck knowledge for the other chairs. The
  real opponent's level, agent, seed and deck analysis are not in the
  fork. That is the fairness rule and also the rule against playing the
  predecessor: the search models "a house player", whoever sits there.
- *The hidden cards.* A fair level forks with `DeckKnowledge::Prior`
  (new, §5.3); `seer` with `Known`.
- *The leaf.* The game's result if it ended; otherwise a value in [0, 1]
  from the seat's view: life, the board's material as `worth` already
  prices it, cards in hand, lands. One function for every level, fitted on
  self-play outcomes of the train split, never on games against a
  particular opponent.
- *Budgets are counts,* never time: candidates, worlds, horizon in turns
  and a cap on answers per rollout, by tier. The same view, policy seed
  and budget give the same answer.
- *Where it runs.* In `baylee-gamehost` under the `simulation` feature,
  which only the training crates enable. A hosting build has no fork, and
  there a generation-7 level answers as its heuristics do. Shipping the
  feature is the owner's decision (§5.4).

### 5.3 A fair fork: `DeckKnowledge::Prior`

A fair level may not deal the opponents' hidden cards from their lists.
`DeckKnowledge::Prior(&PublicPrior)` deals each other owner's hidden slots
(hand, library, outside the game) by weight from a list the caller built
from the seat's view alone, on the belief stream; the seat's own library
is dealt from its own cards, which it knows. No refusal on that path may
depend on which card an opponent hides. The prior the house builds
(`baylee-ai`): the cards the opponent has shown (a deck repeats its
cards), basic lands of the colours shown at a deck's usual land share,
and the card pool's cards of those colours at a low weight. Tests: the
engine's twin games (two games that differ only in the opponent's list,
hand and library fork to the same game), and `tests/fair_house.rs`
extended to foresight: every fair level answers both deals the same.

### 5.4 What the owner decides, and what is measured for it

Shipping `simulation` to the hosts is not part of this round. Measured
for the decision: the time per searched decision (p50, p95, max) and the
CPU per game at each tier, and proposed bounds: a hard budget of rollouts
and answers per decision, a cap on searching chairs per host, and one
searched decision per pump slice.

## 5a. What the third round built and measured (11.11.2026)

Built as planned in §5.2 and §5.3, with these differences: the model in
every chair of a fork is `expert`'s heuristics for every searching level
(a noisy level's own heuristics made poor rollouts), foresight is off
unless a table switches it on (`Session::set_foresight`, `eval4
--house-foresight`), and a host can cap the work of one answer
(`set_foresight_work`). `docs/house-ai.md` §"Foresight" describes what
stands; `docs/engine-internals.md` §"Forking a simulation" the prior.

**The held-out gate has not been run** (coordinator, 11.11.2026: it is
spent once, on a version that train says meets 62 %). Every number below is
a development number on the train split, round-robin pairings, each game
counted once by `house_duplicate`, the level against its own generation-6
self; "for / against" are the seeds the level turned.

| Level (tier: candidates × worlds) | Games | Share | Wilson 95 % | For / against |
| --- | ---: | ---: | --- | ---: |
| novice (4 × 2) | 512 | 53.6 % | 49.3–57.9 | 25 / 7 |
| casual (5 × 3) | 512 | 53.9 % | 49.6–58.2 | 29 / 9 |
| steady (6 × 4) | 512 | 55.8 % | 51.4–60.0 | 46 / 16 |
| sharp (8 × 5) | 512 | 59.7 % | 55.4–63.9 | 57 / 8 |
| expert (8 × 6) | 1,024 | 59.9 % | 56.9–62.9 | 121 / 19 |

So the fork is worth about ten points at the top tier, where a heuristic
was worth one to four, and it is short of 62 %.

**How expert got there** (1,024 train games a row unless said; a row's
noise is about ±1.2 points):

| Setting | Share | For / against |
| --- | ---: | ---: |
| Known decks, hand-written leaf, 2 turns (256 games) | 54.7 % | 17 / 5 |
| the same, 4 turns / 8 turns (256 games) | 57.9 % / 59.4 % | 23 / 3, 27 / 3 |
| the same, played to the end, 4 × 4 (256 games) | 52.6 % | 15 / 9 |
| fair prior instead of Known, 4 turns (256 games) | 57.9 % | 22 / 2 |
| fair prior, fitted leaf, 4 turns | 57.5 % | 96 / 19 |
| + roots where the level's plan starts with a mana tap | 59.0 % | 107 / 15 |
| + single targets, second combat round, race feature | 59.6 % | 118 / 19 |
| + instant-speed roots where the level acts or must answer | 59.9 % | 121 / 19 |
| 12 worlds instead of 6 | 60.0 % | 118 / 16 |
| a second look at every challenger in six more worlds | 58.7 % | 104 / 16 |

The fair prior costs nothing against dealing the real lists. More worlds,
a longer horizon, more roots and a stricter choice do not move the share
past 60 %: it is not noise that limits it. About three seeds in four are
won by the same deck from both seats whoever pilots it; the new expert
turns 23.6 % of seeds for and 3.7 % against, and 62 % needs a net 24 %.
An instant-speed root at every priority outside the main phase made a run
many times slower and was cut to the two cases above.

**Cost** at expert's tier on this machine (one thread a game): about 41
searched answers a game; a searched answer takes 78 ms at the median,
0.39 s at the 95th percentile and 3.7 s at worst; about 6.5 CPU-seconds a
game for the searching chair. About 5 % of the questions a search would
take are refused by the fork (something public naming a hidden card, a
miracle check queued, triggers not yet scanned) and get the level's own
answer.

**Tools.** `eval4 --house-foresight [--foresight-work N]` (the summary's
`foresight` holds counts, refusals by reason and the time per searched
answer), `eval4 --leaf-log <file>` and `fit_leaf --log <file>`
(`baylee-train`).

## 6. Round four: past the plateau (plan, 11.11.2026)

One step of rollout improvement over the heuristics saturates near 60 %.
The next step has to improve what the rollouts are made of. The house may
learn from its own self-play (coordinator, 11.11.2026); it stays no teacher
for BayleeAI, deterministic, fair, and Rust at play time, with no neural
model in the house without a new decision of the owner. In order:

1. **A better leaf.** The leaf decides every trade a search weighs, and
   today's is fourteen counts. Fit it on positions of games the *searching*
   house plays (`--house-foresight --leaf-log`), with what the counts miss:
   power that evades blockers, each common keyword by itself, tapped and
   untapped creatures at the leaf, what a side can still cast with its
   lands, cards left in the library, and the turn. Linear first; a small
   tree scorer if the linear fit leaves log loss on the table. Measured as
   §5a: expert against expert:generation=6 on train.
2. **Whether better rollouts pay at all.** Before distilling anything, a
   nested probe: in a fork, let the model chair of the searching seat try
   its combat declarations in one world for one turn. If expert does not
   gain from that at 256 games, distillation of combat cannot gain either.
3. **Distil the search into the base.** Record, at searched roots of
   self-play on train, the level's answer, the search's answer and the
   margin. Where they disagree systematically, refit the constants the
   heuristic already has (`worth`, the attack search's prices, the
   hold-up reserve) or a small linear scorer over its own features, behind
   generation 8, and search on top of the improved base. Iterate while
   train improves.
4. **The gate** once train shows 63 % or more for expert, then the
   BayleeAI slot.
