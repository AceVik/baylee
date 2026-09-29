# The trained AI

Baylee's own Magic AI: a net that knows every card by its `CardIndex` alone,
reads only what its seat may see, scores only the answers the engine offers,
and runs on a player's CPU, GPU or NPU. This is how it is built, what its
interfaces promise, and where it stands. Code: `crates/baylee-train` (Rust) and
`tools/trainer/` (Python). Data and checkpoints live outside git
(`~/baylee-data/` on the training PC).

## The pipeline

```
selfplay ──records──▶ convert ──columns──▶ train_value / train_policy ──▶ export_onnx ──▶ arena / client
(Session, house AI)   (replay + encode)     (PyTorch)                      (score tables)   (ort, Rust)
```

1. **Which cards.** Only cards that work 100 % are dealt: `Coverage::Implemented`
   *and* named in the engine's test code (`card_index("…")`, an oracle-id literal
   or an `index::` constant; `baylee_train::working`). `xtask deck-check --tested`
   holds a deck to the same rule. The set's SHA-256 stamps every run; a deck with
   a card outside it plays only with `--working-only`, which puts the deck's own
   basics in those cards' places.
2. **Games** (`selfplay`): every chair the house's, hosted by a `Session`, so a
   record is what a gateway keeps. Stopped by an answer cap (100k) and a wall
   cap (15 s, checked after every answer past half of it); stopped and panicked
   games are findings with a replayable record (`--name --only` replays one game
   byte for byte; `inspect` names where it stopped), never data.
3. **Decisions** (`convert`): each record is replayed with the hash checked after
   every input. A record that no longer replays was played by other rules (a card
   changed) and is refused whole — that is how a dataset drops the games a change
   touched. Each answer becomes one or more steps (see Encoding), written as flat
   column files a trainer memory-maps.
4. **Training** in PyTorch on the 4070 Ti (WSL2; `torch.compile`; CUDA graphs for
   inference).
5. **Export** of the fixed-shape part of the net to ONNX, checked against torch on
   held-out decisions before it is kept.
6. **Play** through ONNX Runtime from Rust (`netplay`, feature `onnx`): the net
   takes a chair (`Session::take_over` → `view_for` → `act`), exactly as a socket
   player would.

## Encoding (versioned: `features::ENCODER_VERSION`)

A decision is what the house AI answers from (`Session::agent_view`): no clock,
no policy acts, no teammate's hand. Hidden information is never an input to the
actor; the omniscient half (other hands, library tops) is written apart, for the
training-only critic and auxiliary heads.

- **Entities**: one row per object the seat sees (hand, stack, battlefield,
  command, looked-at, revealed tops, graveyards, exile), at most 256, the objects
  the question offers kept first. 38 raw columns: zone, controller and owner
  relative to the seat, status bits, types, colours, mana value, P/T, loyalty,
  damage, counters, relations as row numbers (attached to, attacks, blocks,
  targets), what the question offers to do with it, 128 keyword bits.
- **Card ids**: `CardIndex + 1` (0 is padding), one id for an object the seat may
  not identify, then the registry tokens. The embedding is sized for all 33,694
  ledger rows from the start; a new set adds rows, never layers.
- **Globals**: turn, phase, step, who is active and asked, question kind and
  answer count, per seat life, poison, energy, zone sizes, mana pool.
- **Options** (v2): every answer a `Pending` offers is a triple `(head, a, b)` —
  a fixed answer, something done with one object, one of its abilities, a pair of
  objects, an attacker and a player, a player, a colour, a creature type, a
  number, a way to cast (`policy.rs`). No option cap. A multi-pick answer is a
  sequence of single picks plus "done", the picks so far marked on their rows;
  `policy::assemble` puts picks back into one `PlayerAction` (tested as the exact
  inverse of `policy::steps` over whole games). `Arrange` and numbers above 63
  are not answered by the net yet and are counted per kind.
- **v3 (required, next)**: piles and runs. Battlefield objects that differ in
  nothing a player reads are one row with a count, by the client's own predicate
  (`baylee_view::ObjectSummaryKey`, shared rather than copied), kept apart when
  something individual applies (an aura, a spell pointed at one, face down) and
  split by whether each member is a legal option of the question. Consecutive
  identical stack items are one run, the top few kept individual. A pick on a pile
  unfolds to a concrete legal member. Grouping comes before the entity cap, so a
  question never loses an offered object: that becomes a test over the swarm
  games of r001/r002, not a statistic.

## The net

An entity transformer: every object a token (card embedding + its state), the
globals one more token (plus, for the policy, the house profile asked for), a
transformer encoder over all of them (pre-norm, no positions: order means only
what the cap keeps). Heads:

- **Value**: the win chance of the deciding seat, from the globals token. It is
  the win-chance bar and the critic's baseline.
- **Policy**: scores each option triple — fixed answers and parameters off the
  globals token, verbs and abilities off the object's token, pairs by a bilinear
  form of two tokens (`PolicyNet.tables` + `PolicyNet.score`).
- Later: the privileged critic (training only), auxiliary heads (opponent's hand,
  turns to the end, their next action), a GRU memory once the log rule is decided.

The profile input is the level knob at play time (novice … expert). Imitation
tops out at the house's own level; any level above expert comes from RL and is
labelled only once measured.

## Interfaces that stay fixed while the net grows

The owner asked that size never become a problem. Size is configuration; the
interfaces are what must not move silently.

1. **Size is configuration, not code.** Width, depth, heads and feed-forward
   width are a `Config` saved with every checkpoint and read back on load; the
   ONNX export records them in `policy.onnx.json`. The Rust player reads every
   shape from the model file (the pair width from the `pair_a` output, the
   entity count from the export's metadata) and assumes no size.
2. **The export interface is versioned**: the encoder version, the table
   layout (`PolicyNet.TABLES`, in order) and the entity count travel in
   `policy.onnx.json`; a loader refuses a mismatch. A bigger net drops in with
   no Rust change; a changed encoding is a version bump.
3. **Retraining stays cheap.** Records are kept and the converter is
   deterministic, so any net can be trained again from scratch. To grow in place:
   function-preserving widening and deepening (Net2Net: new layers start as the
   identity, widened units split), or distilling the old net into the bigger one
   before RL.
4. **Size is chosen by measurement**: trunks of about 5M, 20M and 60M on the same
   data, compared by held-out loss, calibration, agreement and win rate against
   compute; the report says where the curve flattens. The teacher trained here can
   be as large as that says; what ships is a distilled student.
5. **Headroom (the owner's 33 %)**: the deployed student is chosen so that a net
   one third larger would still meet every level's decision-latency budget on the
   reference laptop's CPU path (`acenb`: Ryzen 7 PRO 7840U, 8 Zen 4 cores). The
   margin sits in the budget, so the net can grow by a third later without
   missing the target.

## Which cards it trains on: the verification ladder

The net can only learn a card the engine plays right, so the training pool is a
level of `cargo run -p xtask -- verify` (`xtask/src/verify.rs`):

| Level | Evidence |
|---|---|
| L1 | `Coverage::Implemented`: every clause of the card was read |
| L2 | and `validate` reports nothing for its file |
| L3 | and the engine's test code names it |
| L4 | and every ability of it fired in a test, no mechanic it uses is untested, and it leaves the battlefield clean |
| L5 | and removing any one of its abilities makes one of its tests fail |

L4's mechanics part is read now (`--coverage`, `xtask/src/mechanics.rs`). The
engine's rule tests run under `cargo llvm-cov` without the per-card tests.
`syn` reads the engine's `match` arms, `if let`s and checks on each DSL variant a
card can hold. A variant is tested where all of its sites ran, partly where some
did, untested where none did. Code that faces the AI rather than the rules
(`engine/decision.rs`), hashing and formatting are no evidence. The report
ranks what is missing by the house-deck and L3 cards that use it, and names the
functions whose arms never ran. A static's layer counts as structure: its
modifier decides it (`Modifier::layer`), and the modifier is the mechanic.

L4's firing part and L5 read the engine's test hooks (`docs/verification-hooks.md`,
`xtask/src/hooks.rs`). `--ability-log <dir>` is a run of the engine's tests with
`BAYLEE_ABILITY_LOG=<dir>`. A card's abilities have all fired when every entry of
the pool inventory that a door logs appears in some test's file. An entry no door
logs (ward, toxic, …) passes when the rule tests run its variant. `--mutate`
replaces each ability of every L4 card by nothing (`BAYLEE_MUTATE`), one process
per ability, and runs the tests that fired the card. The card's own tests run
first; the five pool-wide sweeps run only if those let the mutant survive. The
card is L5 when every mutant is killed. L4's leave part has no hook yet. The
report says so, and grants L4 without it.

```text
cargo run -p xtask -- verify --coverage <llvm-cov export> --ability-log <dir> --mutate
```

Numbers on main (2026-09-30), without the hooks:

- The pool: 2716 cards, 2242 at L1, 2242 at L2, 2241 at L3.
- Of 387 variants the pool uses, 304 are tested, 49 partly, 11 untested and 23
  unsited (no engine code names them, for instance `ActivationTiming::InstantSpeed`,
  which is the absence of a restriction).
- Of the L3 cards, 2227 use no untested mechanic, and 762 use only fully run ones.

With the hooks (`c42/engine-verify-hooks`, not yet on main):

- The pool: 2749 cards, 2293 at L1, 2293 at L2, 2292 at L3, 2210 at L4 and 1855 at
  L5. L5 took 16.5 minutes for 2819 mutants.
- Of the 383 mutants that survive, 328 remove a mana ability. The only test that
  fires those abilities takes its expectation from the card itself, so a mutant
  that removes the ability also removes what the test expects.

The fuzzer (`bin/fuzz`) plays decks generated from the L3 pool. Half of each
chair's answers are picked at random among what the question offers, and the
house gives the rest. It reports:

- engine panics;
- offered answers the engine refuses;
- refusals that change the game;
- records that do not replay;
- games that do not end.

`--only N` plays a finding's game again, move for move:

```text
cargo run --profile selfplay -p baylee-train --bin fuzz -- --games 10000 --out ~/baylee-data/fuzz/f002
```

Against the engine fixes of `c42/engine-karn-targets`, 10,000 games left two
kinds: mana abilities of two-basic-type duals, and blocks of a menace attacker by
one creature. The second is a constraint `ChooseBlockers` does not state yet.

## Where it stands (2026-09-30)

- **Value net v2** (8.2M parameters, d002→d003: 200k games, every 32nd decision):
  held-out Brier 0.160, log loss 0.477, accuracy 75.3 %, calibration error 0.006;
  rated under 10 % → won 4.1 %, over 90 % → won 96.2 %; calibrated in every turn
  bucket (ECE ≤ 0.011). A logistic model on life, hand, library and board scores
  Brier 0.209. It measures house AIs playing on, not people, on two decks.
- **Value net v1** overfit (20k games are 20k labels however many positions they
  are cut into); the fix was more distinct games, sparser positions and keeping
  the best checkpoint on held-out games.
- **Policy v1** (imitation of the house, d003): 90.5 % top-1 agreement on
  held-out games (a uniform pick among the offered options agrees 26 % of the
  time). In the arena (`bin/arena`, ONNX through `ort`) it wins 37.0 %
  [32.6, 41.6] of 463 games against the house: an imitation below its teacher, as
  expected before RL.
- **The value net's early edge** is partly the decks (a baseline on deck, seat and
  turn scores Brier 0.245 at turns 1–3, v2 0.238), but it survives mirror
  matches, where the deck says nothing (`tools/trainer/eval_value.py --no-deck`).
- **Engine findings** from self-play, handed to the engine agents with replayable
  records: Karn, the Great Creator's loyalty targets (a panic), a refused `apply`
  that changed the engine (records that do not replay), a question with no legal
  answer (`ChooseCards` min 2 of one), and quadratic legality checks on token
  swarms (fixed on `c42/engine-pay-scaling`).

## Open

- Encoding v3 (piles and runs) — a requirement for the NPU export. It also
  covers up to 8 seats: seats become entities, and the value head a distribution
  over the winner. Self-play already seats 2 to 8 decks (`--seats`, `--teams`).
  A four-seat game of house decks takes about 3000 answers and 5 s, against 40 ms
  for a duel.
- The arena: win rate against each house profile with 95 % intervals, then RL
  against a league of house profiles and older nets.
- The scaling measurement and the student's latency on `acenb`.
- Regenerating data once the house AI's lethal-first fix lands: the policy
  imitates the house, so it learns whatever the house does.
