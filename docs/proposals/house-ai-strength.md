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
  BayleeAI. The house was to stay an opponent, never a teacher; since
  11.11.2026 the fair generation-7 `expert` with foresight is also a teacher
  for BayleeAI (owner; the campaign builds that path), so a generation that
  changes `expert`'s answers is told to the campaign.

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

**The held-out gate was run once** (11.11.2026, build bf1e9a720; the owner
set its bar at 58 % and general, in place of §5's 62 %, and banked the
round on it). Decks no development number had seen: the test split and
`holdout2` of `manifest.v4.holdout2.json`, round-robin pairings, each game
counted once by `house_duplicate`, every level with foresight on against
its own generation-6 self without it. "For / against" are the seeds the
level turned (won from both seats against lost from both); the sign test
is two-sided over them.

| Level | Test | `holdout2` | Together | Wilson 95 % | For / against | Sign test |
| --- | ---: | ---: | ---: | --- | ---: | --- |
| expert | 306 / 512 = 59.8 % | 301 / 512 = 58.8 % | 607 / 1,024 = 59.3 % | 56.2–62.3 | 101 / 6 | p < 0.0001 |
| sharp | 306 / 512 = 59.8 % | 311 / 512 = 60.7 % | 617 / 1,024 = 60.3 % | 57.2–63.2 | 111 / 6 | p < 0.0001 |
| steady | 153 / 256 = 59.8 % | 146 / 256 = 57.0 % | 299 / 512 = 58.4 % | 54.1–62.6 | 49 / 6 | p < 0.0001 |
| casual | 138 / 256 = 53.9 % | 153 / 255 = 60.0 % | 291 / 511 = 56.9 % | 52.6–61.2 | 45 / 9 | p < 0.0001 |
| novice | 131 / 256 = 51.2 % | 136 / 256 = 53.1 % | 267 / 512 = 52.1 % | 47.8–56.4 | 27 / 16 | p = 0.13 |

expert, sharp and steady meet the bar; casual is a point short of it and
clearly better than its predecessor; novice's two worlds of four
candidates are not shown to gain anything. No deck reached ten turned
seeds in any run, so the per-deck rule had nothing to read. One game of
1,024 stayed open (casual, `holdout2`).

*Generalisation,* the same deals against the frozen `sharp-2026-11`, which
no fork seats:

| Level | Generation 6 | With foresight | Gain |
| --- | ---: | ---: | ---: |
| expert, test | 283 / 512 = 55.3 % (35 / 8) | 320 / 512 = 62.5 % (68 / 4) | |
| expert, `holdout2` | 287 / 512 = 56.1 % (37 / 6) | 331 / 511 = 64.8 % (80 / 5) | |
| expert, together | 570 / 1,024 = 55.7 % | 651 / 1,023 = 63.6 % | +7.9 |
| sharp, test | 270 / 512 = 52.7 % (25 / 11) | 319 / 512 = 62.3 % (68 / 5) | |
| sharp, `holdout2` | 267 / 512 = 52.1 % (22 / 11) | 326 / 512 = 63.7 % (73 / 3) | |
| sharp, together | 537 / 1,024 = 52.4 % | 645 / 1,024 = 63.0 % | +10.6 |

The gain against an opponent the search never models is as large as the
gain against the predecessor, on held-out decks as on train (below).
Against opponents that are no house: BayleeAI r1 with its tactical bundle
on `holdout2` wins 46.1 %, 44.1 % and 44.3 % against the new expert, sharp
and steady (the run described further down), and DeepSeek at its low
setting, in 96 games each of the coordinator's collector, loses to the
house as often as not: the new expert won 47.9 % of them and the new sharp
40.6 %. So the order sharp < expert shows against no outside opponent yet.

*The 2v2 run per level* (must not be worse than 50 %; the test split of
`manifest.v4.holdout2.json`, 256 rows, teams 1,2,1,2, fixed pairings, each
game once; build bf1e9a720), each
level with foresight against its own generation-6 self:

| Level | Won | Share | Wilson 95 % | For / against | Sign test |
| --- | ---: | ---: | --- | ---: | --- |
| expert | 75 / 128 | 58.6 % | 49.9–66.8 | 13 / 2 | p = 0.007 |
| sharp | 79 / 128 | 61.7 % | 53.1–69.7 | 17 / 2 | p = 0.0007 |
| steady | 79 / 128 | 61.7 % | 53.1–69.7 | 15 / 0 | p = 0.0001 |
| casual | 72 / 128 | 56.2 % | 47.6–64.5 | 11 / 3 | p = 0.057 |
| novice | 66 / 128 | 51.6 % | 43.0–60.0 | 6 / 4 | p = 0.75 |

No level is worse than 50 %; novice, as in the duels, is not shown to
gain. No game stayed open.

*What a searched answer cost in the gate.* The gate ran eight games at a
time beside other jobs (a load of about forty on twenty-four threads), so
these are times under load, not the cost of §"Cost" below: at expert's
tier a searched answer took 100 to 119 ms at the median and 0.37 to 1.07 s
at the 95th percentile, and the slowest single answers took 8, 10, 29 and
118 s; sharp's slowest took 88 s. A count of candidates, worlds and
answers does not bound a searched answer; §7 is that work.

**On the train split,** as development went (every number from here to §6
is a development number: round-robin pairings, each game counted once, the
level against its own generation-6 self):

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

**The gain is general.** Against the frozen `sharp-2026-11`, which no
fork ever seats, on the same 1,024 train deals: `expert:generation=6` wins
55.7 % (Wilson 52.6–58.7; 91 seeds for, 33 against) and the new expert
63.3 % (60.3–66.2; 152 / 16). That is 7.6 points against an independent
opponent beside 9.9 against its own predecessor.

**Better rollouts alone do not lift it.** A probe that let the searching
seat's model try its own attacks and blocks one turn deep inside every fork
(a search within the search) won 62.1 % against 60.9 % for the same 256
deals without it (34 / 3 against 32 / 4): inside the noise, at 1.6 times
the cost. It was not kept.

**The ladder with foresight** (train, 512 games a pair, both levels
searching at their tiers): casual over novice 57.6 % (56 seeds for, 17
against, p < 0.0001), steady over casual 56.1 % (52 / 21, p = 0.0004),
sharp over steady 51.4 % (26 / 19, p = 0.37), expert over sharp 51.6 %
(22 / 14, p = 0.24). The two lower rungs are wider than they ever were
(§0a: 53.8 % and 51.9 % on train); the two upper rungs lean the right way
and are not told apart at 512 games, their tiers being close (6 × 4,
8 × 5, 8 × 6). If the order has to be provable there, the tiers below
expert have to be spaced further apart, which costs those levels part of
their gain over generation 6.

**Against BayleeAI** (the campaign's run on its GPU runtime built from
e1db617b1, holdout2 as BayleeAI's opponent pool, round-robin, 512 deals a
pair; the house is not selected on these numbers): r1 with its tactical
bundle wins 46.1 % against the new expert, 44.1 % against the new sharp and
44.3 % against the new steady; raw r1 wins 19.7 %, 20.5 % and 21.5 %.
Against an outside opponent the new steady plays as strongly as the new
expert: foresight dominates the tier, and steady < sharp < expert does not
show there. Together with the train ladder above, the order of the three
upper levels is not established under foresight; how far apart the tiers
are set is the owner's decision.

**A cap on answers does not bound the slow tail.** With one searched
answer capped at 10,000 house answers in its forks nothing changes (no
search reaches it); capped at 3,000, expert keeps 60.0 % (121 / 18) and the
worst searched answer is as slow as before. The slow answers are slow per
answer (large boards, where the model's own combat search is dear), not
long in answers. A bound a host can rely on has to count finer work than
answers: the engine's fuel of `lookahead-design.md` Q5, or the combat
search's nodes summed over a search.

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
2. **Whether better rollouts pay at all** (done, §5a: barely). Before distilling anything, a
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

## 6a. What the leaf round measured (11.11.2026)

§6 (1) is done, and its answer is no: a better leaf does not move the
share. Nothing of the leaf was changed; the round leaves tools
(`eval4 --foresight-probe`, `foresight_probe`, `fit_leaf` with games held
out, a choice of features, a ridge and `--pairs`) and this record.

**The measurement** is §5a's: expert against `expert:generation=6`, 1,024
train games, each counted once (`house_duplicate`). The build reproduces
§5a's row exactly (613 of 1,023 decided, 121 seeds turned for and 19
against). Log loss is on whole games held out, every fifth, where the
earlier 0.571 held out single positions of games the fit had seen. The
figures of one column are of one split; which games fall in the fifth
moves them by about 0.02 (a log from another run numbers its games in
another order: 0.576 for the standing weights), so they compare with each
other and not with a figure from another log.

| Leaf | Log loss held out | Share | Wilson 95 % | For / against | Games won / lost that the standing leaf lost / won |
| --- | ---: | ---: | --- | ---: | ---: |
| the fourteen counts, fitted on heuristic self-play (stands) | 0.558 | 59.9 % | 56.9–62.9 | 121 / 19 | |
| the same counts, fitted on the searching house's games | 0.592 on those games (the standing weights: 0.582) | 60.3 % | 57.2–63.2 | 115 / 9 | 26 / 22 |
| 49 features, linear | 0.540 | 59.9 % | 56.9–62.9 | 115 / 13 | 32 / 32 |
| 49 features, each also by the turn, and 200 boosted depth-2 trees | 0.526 | 59.4 % | 56.4–62.4 | 113 / 16 | 36 / 41 |
| the fourteen counts, read at the start of the seat's own turn | 0.558 | 59.9 % | 56.9–62.9 | 119 / 17 | 11 / 11 |
| the fourteen counts, read at the start of another seat's turn | 0.558 | 61.2 % | 58.1–64.1 | 123 / 9 | 53 / 41 |

The last column is `house_duplicate --before`: the games are the same deals
against the same opponent, so a variant is read by the games that went
another way, and none of these splits is far from even (the widest, 53 to
41, has a two-sided p of 0.26).

The 49 features are the fourteen and, each one side less the other: power
with evasion, the largest power, flying, first strike, deathtouch, reach,
vigilance, haste, defender, menace, hexproof and indestructible as counts,
double strike, lifelink, trample and infect by power, the other keywords,
untapped lands and untapped creatures with their power and toughness, the
library's last ten cards, the graveyard, planeswalkers and their loyalty,
artifacts, enchantments, tokens, the creatures' mana value, the sums of
`tactics::material` and of the lands' `permanent_worth`, printed abilities
and mana makers that are no land, the damage that gets past the other
side's blockers in one attack, and whether the side to move has the other
dead on the table. Without the bound at zero the linear fit reaches 0.537;
a weight for each feature that grows with the turn, 0.538. The searching
house's games (35,295 positions of some 650 games) are
harder to call than heuristic self-play, and a fit on so few of them calls
them worse than the standing weights do.

So a value that calls games better (0.558 to 0.526) changes the answers of
about two searched questions in a hundred and wins nothing with them. The
experiment code is on the local branch `c42/house-leaf-experiments`; it is
not for merging.

**Where the leaf is read.** A world stops where the fourth turn boundary
falls: in a duel that is the start of the searching seat's own turn for a
question asked in its turn, and of the opponent's for a block. Letting a
world run on (two turns at most) until the seat's own turn starts changes
22 games and wins none. Letting it run on until another seat's turn
starts, so that the searching seat has always had the last turn, costs a
fifth turn of rollout at every question of its own turn and is the one
variant that leans up: 53 to 41 on the measured deals and, played again on
the next 1,024 train deals (`--deal-offset 1024`: 58.7 % against 58.2 %
for the standing search), 39 to 33. Together 92 games won and 74 lost of
2,043, nine tenths of a point, two-sided p 0.19: not shown, and too small
to carry the share to 62 % if it were.

**Why, measured.** `eval4 --foresight-probe` plays the worlds of one
searched question in three on to the end of the game, in games the
searching house plays against itself (deals from 8448 on, stopped at the
thirty-minute bound: 4,114 questions, 174,204 worlds). `foresight_probe`
then judges a choice by results: the *lead* is how much more often the
chosen answer's worlds were won than the level's own answer's. The worlds
are the search's own, with the hidden cards it supposes and a house player
in every chair, so a lead is what the search's model of the game says a
choice is worth, not a share of real games.

| Choice by | Replaced | Lead per 100 questions | On half the worlds, judged on the other half |
| --- | ---: | ---: | ---: |
| the standing leaf | 6.5 % | 0.25 ± 0.07 | 0.15 ± 0.06 |
| 49 features, linear | 6.6 % | 0.32 ± 0.07 | 0.16 ± 0.06 |
| 49 features by the turn | 6.7 % | 0.33 ± 0.07 | 0.14 ± 0.06 |
| the same and 200 trees | 7.2 % | 0.33 ± 0.06 | — |
| fourteen counts fitted on pairs (`fit_leaf --pairs`) | 7.0 % | 0.32 ± 0.06 | 0.18 ± 0.06 |
| the played-out results themselves, of three worlds | 3.3 % | — | 0.08 ± 0.05 |

- A richer leaf looks a quarter better where a choice is judged in the
  worlds it was made in (0.07 to 0.09 over the standing leaf question by
  question, each ± 0.03), and no better where it is judged in other worlds,
  which is how the real game judges it. It learns the six deals, not the
  position.
- Knowing how each world ends is worth less than the leaf: three results
  are too few to choose by. The leaf is already the less noisy judge of a
  world, which is why playing forks to the end lost in §5a.
- The margin hardly matters: 0.0 to 0.16 gives 0.14, 0.15, 0.15, 0.14,
  0.12 and 0.08 on half judged on half. Nearly half the lead comes from the
  replaced answers the leaf puts 0.16 or more ahead, 0.7 % of the questions
  asked; those the leaf puts 0.02 to 0.04 ahead, four in ten of the
  replaced, win nothing.
- By question: a priority is worth 0.19 ± 0.07 per 100, an attack
  0.33 ± 0.21, a block 1.25 ± 0.49, at 83 %, 13 % and 3 % of the questions.

**What it says about §6.** A search wins by the few answers where the
level's own is plainly wrong, and it finds those with any sensible leaf.
The share is bounded by which answers are tried and by how the house plays
the worlds on, not by how finely the end of a world is priced: (2) and (3)
are the open steps, and a block is the question to start (2) with. A
neural leaf is not indicated by anything here.

## 7. Handover: a hard budget for a search, and block repairs (11.11.2026)

Written when the round changed hands in the middle of two tasks. The
owner's decisions in force: the foresight round is banked on the gate of
§5a; next come **heuristic repairs mined from the search's disagreements,
blocks first**, behind a new generation; no neural model in the house; the
tiers stay as they are. The fork goes onto the internet-facing engine
server only once BayleeAI's own search probe shows five points or more
**and** a hard budget per decision exists; until then foresight stays opt-in
and off in hosting. The fair generation-7 `expert` with foresight is now
also a teacher for BayleeAI, so a generation that changes `expert`'s
answers is told to the coordinator for the campaign.

**Since the handover** (the evening of 11.11.2026): the analyst's widest
finding was true, and generation 8 (§7.6) lets a priority search try the
spells its lands pay for: expert 64.1 % against its generation-6 self on
train, where generation 7 had 59.9 %. Generation 9 (§7.7) puts a bound in
the level, eight million units at expert's tier, which cost nothing
measurable on generation 8 and holds a searched answer under a second of
CPU time; §7.8 is what that bound does and does not promise a host. The
coordinator spends the held-out gate on generation 9. The teacher stays
pinned at generation 7 (`tests/reference/foresight-generation-7.json`).

### 7.1 Where a search's time goes (measured)

`eval4`'s summary now times every searched answer by the CPU time of its
game's thread (`foresight.ms_*`, `cost`, `cost_slowest_hundredth`,
`slowest`, `work`, `by_board`), so a time is the answer's own work whatever
else the machine runs. expert against `expert:generation=6`, the first 256
train deals, round-robin, six games at a time at nice 10 beside a load of
48 to 88 on 24 threads; 9,776 searched answers, 1,643 CPU-seconds:

| | p50 | p95 | p99 | p99.9 | max |
| --- | ---: | ---: | ---: | ---: | ---: |
| CPU time of a searched answer, ms | 116 | 464 | 852 | 1,523 | 4,249 |
| work on the meter, units (provisional prices) | 1.8 M | 5.8 M | 10.1 M | | 54 M |

- **All searches:** the engine applying answers 49 %, the house answering
  what is no attack or block 43 %, attacks and blocks 3 %, making forks
  2.6 %, leaves 0.15 %. A search makes 21 forks and gives 1,850 answers in
  them at the mean.
- **The slowest hundredth:** attacks and blocks 18 %. **The five slowest
  (2.1 to 4.2 s):** attacks and blocks 65 %: boards of 38 to 43 permanents
  where the forks' model, which is `expert` with its 262,144 nodes for every
  declaration, visits 11 to 36 million nodes in one searched answer (about
  80 ns a node). This is the suspected tail, confirmed.
- **Below those, at 1.3 to 1.4 s:** priority searches on boards of 23 to 28
  permanents with 145 forks and 12,000 to 14,000 answers and no combat
  search to speak of. So the tail has two parts, and a node cap alone
  leaves the second: the count of forks and answers has to be bounded too.
- **An answer costs more on a fuller table** (the engine's step included,
  by the mean permanents and stack objects over a search's answers): 41 µs
  below 8, 57 µs from 8, 77 µs from 16, 109 µs from 24, 154 µs from 32.
  About 30 µs and 3.2 µs a permanent. A fork is about 200 µs whatever the
  table. The longest single answer in a fork was 48 ms, a combat search.
- The times the gate saw (up to 118 s for one answer) were wall time at
  nice 10 on a machine at load forty. They are scheduling, not work; a
  host's wall bound is a question of its own scheduling.

### 7.2 The budget unit (as handed over; measured and decided in §7.7)

Built on the branch (commit below), off unless a table sets it:

- `baylee_ai::WorkMeter`: a count and a bound. An agent counts the nodes
  its combat searches visit into its meter and asks it how many it may
  still visit (`search::attackers_on` and `blockers_within` take the
  allowance; with none left the greedy declaration stands). An agent nobody
  gave a meter has an unbounded one of its own, so no answer outside a
  bounded search changes: the pins of generation 0 and 6 are untouched by
  construction, and the 256-deal run above reproduces the unmetered one
  game for game (156 of 256, 32 seeds for, 4 against).
- A search has one meter for all its forks (`session/foresight.rs`). The
  unit is one combat-search node; a fork and a house answer in a fork are
  priced in the same unit by what is on the table
  (`session/simulation.rs`: `fork_work`, `answer_work`).
- `Session::set_foresight_work(units)` / `eval4 --foresight-work N` bounds a
  search. It checks the meter before every fork and after every answer;
  out of work it drops the world in progress and the round of worlds that
  belongs to, and chooses among the plans tried in the same worlds (the
  level's own if not one round was finished). It overruns by at most the
  step it was in. Under a bound one combat search gets a 256th of it
  (`SEARCH_SHARE`, `--foresight-nodes` overrides). A probed world's
  play-on past its leaf is taken off the meter (`unmetered`).
- This replaces the old cap on answers (same flag, new unit), which
  bounded nothing (§5a).

**Still to do, in order** (as handed over; all five are done, §7.7 and
§7.8 say how):

1. *Fit the prices.* They are provisional (`750 + 25 × board` for an
   answer, `1500 + 8 × objects` for a fork). The table above says about
   `375 + 40 × board` and a flat 2,500 at 80 ns a unit. With the
   provisional prices a unit took 0.063 µs at the median of searches, 0.108
   at the 99th percentile and 0.136 at worst (`foresight.work`); the aim is
   to bring those together, so that a bound in units is a bound in time
   with a known worst factor. Read `by_board` and `work.micros_per_unit_*`
   again after refitting.
2. *Measure that the share holds.* expert against `expert:generation=6`,
   1,024 train deals (59.9 %, 121 / 19 unbounded), at
   `--foresight-work 8000000`, `4000000` and `2000000`; compare game by
   game with `house_duplicate --before`. Report `foresight.cut` (searches
   stopped by the bound) and `ms_p50/p95/p99/max`. Expected from the work
   percentiles: 8 M cuts about 2 % of searches, 4 M about 10 %.
   Separately try the node share alone (`--foresight-work` huge,
   `--foresight-nodes 16384`): §5a found better rollouts worth nothing, so
   a model that searches 16,384 nodes instead of 262,144 should cost
   nothing either and removes the worst tail by itself.
3. *Decide where the bound lives.* As a host setting it changes no level.
   If hosting wants one expert everywhere (and the campaign one teacher),
   the bound belongs in `AIProfile::foresight()` behind the next
   generation, with the host's setting only ever tightening it.
4. *What the unit does not bound:* one engine step. An answer is priced
   by the table's size, not by what applying it does; a step that makes
   thousands of objects or resolves a deep trigger chain costs more than
   its price. The fork's 400-answer cap and the table-size price cover the
   common cases; a real engine fuel (`lookahead-design.md` Q5) would cover
   the rest and is a change inside the engine's hot paths. Say so in the
   hosting proposal rather than claim a strict p100.
5. Tests to add before it is relied on: a bounded search is deterministic
   (same view, seed and bound, same answer and same `work`), a bound of 0
   gives the level's own answer, and `tests/foresight.rs`'s fairness test
   with a bound set.

### 7.3 Block repairs: the approach and the tools

`Session::set_foresight_questions` / `eval4 --foresight-questions <file>
[--foresight-question-kinds block,attack,priority]
[--foresight-questions-all]` writes every searched question of the kinds
named as a JSON line: the seat's whole view, the question, each answer
tried with its value in every world, and the answer given. Without `-all`
only the questions the search answered otherwise.

`foresight_mine --log <files> [--ask <level>] [--show N] [--class <part>]`
reads block questions and tables what changed from the level's blocks to
the search's (`added`, `dropped`, `moved`, `mixed`; and the block that
changed as `chump`, `trade`, `eat`, `wall` or `gang`), where the seat stood
(life band, whether the level's blocks let lethal through), and each
class's gain (the search's value of its answer less the level's own,
summed). `--ask expert:generation=8` asks a level's heuristics every logged
question again, with no game played: how many of the search's replacements
it now gives itself (`repaired`), how many answers the search let stand it
now changes, and what the search made of those where it had tried them.
That is the development loop for a repair: change the heuristic behind the
new generation, `--ask` it against the log, and only play games (train:
against its generation-7 self and against `sharp-2026-11`) when the table
moves. Asked again, `expert` gives the logged answer at 265 of 265, so the
re-asking is exact.

The commands as run (bigace paths; six threads at nice 10 there):

```
eval4 --model ~/baylee-data/v6/train/v6s-league/r1-4eb/fp16 --random-policy \
  --decks-manifest ~/baylee-data/v6/decks-20261107c/manifest.v4.json \
  --deck-split train --threads 6 --seats 2 --deck-schedule round-robin \
  --deals 256 --baseline expert --against expert:generation=6 \
  --house-foresight --foresight-questions blocks.jsonl \
  --foresight-question-kinds block --foresight-questions-all --out run.json
foresight_mine --log blocks.jsonl --ask expert --show 3
eval4 … --house-foresight --foresight-probe probe.jsonl   # then: foresight_probe --log probe.jsonl
```

**A first, small table** (the 256 deals above: 265 blocks searched, 30
answered otherwise, the search's gain 3.27 in leaf value):

| What changed | Questions | Gain | A question |
| --- | ---: | ---: | ---: |
| dropped trade | 5 | 1.14 | 0.227 |
| added chump | 11 | 1.13 | 0.103 |
| added trade | 9 | 0.73 | 0.082 |
| mixed | 3 | 0.19 | 0.065 |
| dropped chump, dropped gang | 2 | 0.07 | |

Thirty questions decide nothing; log 2,048 train deals or more first (deals
from another range than the probe's 8448 on, `--deal-offset`). What the
thirty suggest, as hypotheses:

- *Added chump* is mostly not a chump: the blocker has a use in dying
  (Penumbra Kavu leaves a 3/3, a Devil token deals a damage) or the level
  left home a token worth nothing against an attacker that then connects.
  `search::worth` prices a creature by mana value, power and toughness
  alone, so a token is 100 plus its stats and a death trigger is nothing.
- *Added trade*: an even trade refused at a comfortable life (a 1/1 for a
  1/1) where the search takes it; the block search prices damage flat at 45
  a point and the trade at zero.
- *Dropped trade* and the `mixed` double blocks: the level trades or
  gang-blocks into an attacker while the opponent keeps creatures home
  and cards in hand; the search keeps the blockers. The block search's
  horizon is 0: it does not see what the block leaves it for its own turn.
- By life, 24 of the 30 are above ten: this is not about racing at low
  life, which is where the second round tried (and dropped) race prices in
  the block search.

The probe says a block is worth 1.25 ± 0.49 won games per 100 questions
against 0.33 for an attack and 0.19 for a priority (§6a), at 3 % of the
questions: a repair of all of it is worth a point or two of share at most.
Read the larger log before choosing which class to repair.

### 7.4 The Carbon analyst's findings (one verified and built, the rest hypotheses)

`~/baylee-data/v6/house/carbon/summary.md` and `findings.md`: an analyst
reading the new house's games against DeepSeek (all 192). Nearly
all of it is casts and targets, not blocks. Checked against the code so
far:

- **Verified, built as generation 8 and measured (§7.6): the priority
  roots tried no spell that costs mana.** `foresight::priority_candidates`
  listed `legal.castable`, and at a root the pool is empty (`settled`),
  where every spell of cost one or more is in `legal.payable`. So a search
  compared the level's own plan
  (which starts with a mana tap) with passing, lands and abilities, and
  never one spell with another or a target with another (`targets_of` is
  reached only for free spells). A `CastSpell` of a payable card is an
  answer the question takes; whether the forks' model then pays it
  (`policy::pay_owed`) has to be tried. If it does, payable spells as
  candidates are the widest thing a search does not yet try, and worth
  measuring before any heuristic repair. It does, and it was.
- To verify, each a small general fix behind the new generation: Auras and
  pumps aimed at an opponent's creature (`target_side::aura_side` is said
  never to be called by `baylee-ai`; `target_side::amount` gives 0 for
  `Amount::CountOf`), "attacking creatures get" pumps cast outside combat
  (`timing::read` has no `PumpFilter`), sweepers at X = 0
  (`bad_removal_exchange` skips costs with X), Equipment whose static is
  `Filter::And(&[CREATURE, AttachedToBySource])` never equipped
  (`worth::equipped`), a creature of power 0 declared as an attacker, and
  `policy::produced_colors` reading basic land types only at the mulligan.
  None of these has been reproduced here.

### 7.5 State of the machines and runs

- **Branch** `c42/house-g8` in `~/Projects/baylee-house-lookahead`, target
  dir `/mnt/xdata/cargo-target/house-g8`. Generations 8 and 9 and the
  generation-7 search pin are on it (§7.6, §7.7).
- **The 2v2 gate chain** of the round before finished; its lines are in
  §5a.
- **bigace** is for builds and tests only while the campaign collects
  there (coordinator, 11.11.2026); an eval that must run there takes at
  most two threads at nice 15.
- **acenb** (`ace@192.168.0.12`, all sixteen threads at normal priority,
  start no server, leave MediaVault, Pulse and the net failover alone)
  runs every eval. bigace's `eval4` runs there from a mirror of the
  checkout at the same path (`/home/ace/Projects/baylee-house-lookahead`,
  rsync of the sources without `.git` and `target`), because it reads the
  checkout it was built in (`baylee_train::working::repo_root`); the decks
  are under `~/baylee-work/house/decks*` with path-adjusted manifests, the
  model in `model/fp16`, binaries in `bin/` (named `g8-c<n>-<tool>` by
  build), outputs in `out/`. The same 24 train deals give the same
  `house_duplicate` line and the same `foresight` counts on both machines.
  A level name in zsh is quoted (`"${l}:generation=6"`): `$l:g` is a zsh
  modifier.
- Scratch files of this round are named `g8-*` in the session scratchpad:
  `g8-prof-b.json` (the profile of §7.1), `g8-q-blocks-a.jsonl` (the 265
  block questions of §7.3), `g8-t2-*.json` and `g8-t3-*.json` (the runs of
  §7.6 and §7.7), `g8bin/` (the binaries by build).

### 7.6 Generation 8: a search tries the spells its lands pay for (measured)

The Carbon finding of §7.4 held: at a root the pool is empty, so
`legal.castable` holds only spells that cost nothing, and a priority search
set the level's own plan (a tap) beside passing, lands and abilities only.
Generation 8 (`AIProfile::SPELL_ROOTS`, commit 93b113f9a) adds each spell of
`legal.payable` the planner pays for (`HeuristicAgent::pays_for`: no X, the
taps `spend_on` finds, nothing held back) as a `CastSpell` candidate; the
fork's model pays it in the payment window (`pay_owed`), which
`session/tests/simulation.rs` checks spell by spell. The forks' model now
plays the searching level's generation, so `expert:generation=7` searches as
it did: `tests/reference/foresight-generation-7.json` pins its searched
answers in two reference duels to turn 6 (the same pin passes on
06421a05f's code), and 24 train deals replay game for game.

Train split, round-robin, each game once, acenb (sixteen threads):

| Run | Games | Share | Wilson 95 % | For / against |
| --- | ---: | ---: | --- | ---: |
| expert gen 8 against `expert:generation=6` | 1,021 | 64.1 % | 61.1–66.9 | 155 / 11 |
| expert gen 7 against `expert:generation=6` (§5a) | 1,023 | 59.9 % | 56.9–62.9 | 121 / 19 |
| expert gen 8 against `expert:generation=7` | 1,020 | 54.8 % | 51.7–57.8 | 81 / 33 |
| expert gen 8 against `sharp-2026-11` | 1,023 | 67.7 % | 64.8–70.5 | 192 / 11 |
| expert gen 7 against `sharp-2026-11`, the same deals | 1,022 | 63.3 % | 60.3–66.2 | 152 / 16 |
| steady gen 8 against `steady:generation=6` (bigace) | 511 | 62.4 % | 58.1–66.5 | 73 / 10 |

Against the frozen reference the same deals go 92 games to 47 for
generation 8 over generation 7. The searching chair spends about 10.1
CPU-seconds a game on its searches where generation 7 spent 7.0, and the
slowest searched answer took 14.7 s of thread CPU time (4.9 s for
generation 7 on the same deals); §7.7 bounds it.

### 7.7 The bound, measured, and where it lives

**The prices** (commit 6e14e9555) are fitted to §7.1's `by_board`: an
answer in a fork 375 units and 40 a permanent or stack object, a fork a
flat 2,500 (it took 196 to 206 µs on every size of table). On generation 8
a unit then took 0.068 µs of thread CPU time at the median of searches,
0.089 at the 95th percentile, 0.100 at the 99th and 0.13 to 0.19 at worst
(searches of 100,000 units or more, acenb under a full load of its own),
so a bound in units is a bound in time within about twice its median
price.

**What a bound costs** (expert at generation 8 against
`expert:generation=6`, the 1,024 train deals of §7.6, the table's bound
`--foresight-work`, each game set beside the unbounded run's by
`house_duplicate --before`; times are thread CPU time per searched answer):

| Bound, units | Share | For / against | Games won / lost against unbounded | Searches cut | p50 | p95 | p99 | p99.9 | max |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| none | 64.1 % | 155 / 11 | | 0 | 163 ms | 799 ms | 1.44 s | 3.72 s | 14.7 s |
| 8,000,000 | 63.7 % | 151 / 11 | 7 / 11 (p = 0.48) | 9.4 % | 166 ms | 637 ms | 761 ms | 867 ms | 973 ms |
| 4,000,000 | 61.5 % | 137 / 19 | 25 / 51 (p = 0.004) | 30 % | 165 ms | 350 ms | 398 ms | 442 ms | 482 ms |
| 2,000,000 | 57.3 % | 93 / 19 | 37 / 106 (p < 0.0001) | 60 % | 124 ms | 177 ms | 201 ms | 224 ms | 254 ms |

Unbounded, the work of a searched answer was 2.4 M units at the median,
10.4 M at the 95th percentile, 18 M at the 99th and 107 M at most; the
slowest searches were attacks and blocks on boards of 34 to 37 permanents
where the forks' combat searches visited 50 to 86 million nodes. Eight
million units cost nothing measurable and take the slowest answer from
14.7 s to under one; four million cost two and a half points.

The forks' combat searches alone (`--foresight-nodes 16384` under a bound
too large to cut, in place of the 262,144 nodes `expert` searches for one
declaration) kept 64.0 % with 1 game won and 2 lost against unbounded, and
took the slowest answer to 6.0 s: the model's depth in a fork is worth
nothing measurable, and the tail needs the bound as well.

**Where the bound lives: in the level** (decided 11.11.2026, with a Fable
advisor's reading; generation 9, `AIProfile::BOUNDED_SEARCH`). A bound set
only by a host would make a hosted `expert` a player nobody measured: the
numbers above, the teacher's campaign and the ladder would each describe
another `expert`. And a host's bound did two things at once: it stopped
searches, and through `SEARCH_SHARE` it also cut the forks' combat depth to
a 256th of it, so it changed answers in searches it never stopped. So:

- `Foresight` carries `work` and `nodes`. From generation 9 `work` is eight
  million units at expert's tier and scales with the tier's candidates ×
  worlds (sharp 6.67 M, steady 4 M, casual 2.5 M, novice 1.33 M), so every
  tier is cut about as rarely and the spacing of the tiers is not moved
  (steady's searches on train, 4.0 M being half of expert's: 5.2 M at the
  95th percentile where expert's are 10.4 M). `nodes` is 16,384 whatever
  the bound, so a bound that does not bind changes no answer.
- The meter takes the tighter of the level's bound and the host's
  (`Session::search_meter`): a host may lower a level's bound, never raise
  it. Before generation 9 both are unbounded, so generations 7 (the
  teacher's) and 8 search as they did.
- A search cut before half its worlds were played keeps the level's own
  answer: its comparison rests on too few deals.
- Tests (`session/tests/foresight.rs`): a bounded search answers and spends
  alike when played again and stays inside its bound but for the step it
  was in; a bound of nothing plays as a table without foresight, answer for
  answer, with no fork made; the fair twin deals are searched alike under a
  bound, unit for unit; a host only ever tightens a level's bound.

### 7.8 For hosting: what the bound holds, and what it does not

Proposed for the owner's decision on putting foresight on the
internet-facing engine server (§7: once BayleeAI's search probe shows five
points or more and a hard budget exists):

- **Per searched answer,** the level's bound in units: at expert's tier
  eight million, measured at most 0.97 s of thread CPU time on acenb
  (p99 0.76 s) over some 43,000 searches. A search runs past its bound by
  at most the price of the step it was in: one fork (2,500 units) or one
  house answer (375 units and 40 a permanent or stack object); a combat
  search in a fork is held to what is left.
- **Outside the meter:** the level's own answer, which a search computes
  before it forks, with the level's own combat search (up to 262,144 nodes
  for expert, about 20 ms).
- **What the unit does not bound: one engine step.** An answer is priced by
  the size of the table, not by what applying it does. A step that makes
  thousands of objects, resolves a long chain of triggers, or runs a loop
  until the engine's loop watch stops it costs more than its price, and the
  meter only notices after it. The fork's cap of 400 answers in a world
  and the table-size price cover the common cases; an engine fuel counted
  inside the engine's own steps (`lookahead-design.md` Q5) would cover the
  rest and is a change in the engine's hot paths. So no strict p100 is
  claimed: the measured maximum is what the train decks produced.
- **Time is CPU time, not wall time.** On a loaded host a searched answer
  waits for a core; the gate's runs saw 118 s of wall time for an answer
  that was a few seconds of work. A host needs its own scheduling: a cap on
  searching chairs per core, and one searched decision per pump slice
  (§5.4).
- **A host bound tighter than the level's makes another player.** Its games
  are not `expert`'s numbers; a measurement names it as `expert:work=N`
  would, and a hosted record should say the bound it played under (not
  built).
- **Determinism** rests on a search playing its forks one after another
  (`work.rs`): the same view, seed and bound spend the same units on any
  machine.
