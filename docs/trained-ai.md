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

## Where it stands (2026-09-29)

- **Value net v2** (8.2M parameters, d002→d003: 200k games, every 32nd decision):
  held-out Brier 0.160, log loss 0.477, accuracy 75.3 %, calibration error 0.006;
  rated under 10 % → won 4.1 %, over 90 % → won 96.2 %; calibrated in every turn
  bucket (ECE ≤ 0.011). A logistic model on life, hand, library and board scores
  Brier 0.209. It measures house AIs playing on, not people, on two decks.
- **Value net v1** overfit (20k games are 20k labels however many positions they
  are cut into); the fix was more distinct games, sparser positions and keeping
  the best checkpoint on held-out games.
- **Policy v1** (imitation of the house, d003): training; 87.7 % top-1 agreement
  on held-out games at 40 % of the run (a uniform pick among the offered options
  agrees 26 % of the time).
- **Engine findings** from self-play, handed to the engine agents with replayable
  records: Karn, the Great Creator's loyalty targets (a panic), a refused `apply`
  that changed the engine (records that do not replay), a question with no legal
  answer (`ChooseCards` min 2 of one), and quadratic legality checks on token
  swarms (fixed on `c42/engine-pay-scaling`).

## Open

- Encoding v3 (piles and runs) — a requirement for the NPU export.
- The arena: win rate against each house profile with 95 % intervals, then RL
  against a league of house profiles and older nets.
- The scaling measurement and the student's latency on `acenb`.
- Regenerating data once the house AI's lethal-first fix lands: the policy
  imitates the house, so it learns whatever the house does.
