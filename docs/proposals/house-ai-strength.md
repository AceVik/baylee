# House AI: a strict strength order, then rounds against BayleeAI

Plan, written 10.11.2026; apart from the loop fixes of §1a nothing here is
built. The owner's decisions it follows:

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
