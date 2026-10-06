# Performance Baseline

Hardware: MacBook M1 Max, 64 GB. Measured with `cargo bench -p baylee-engine`
(criterion, debug assertions off for benches). CI runs the benches
(`--quick`) so they never rot; comparing numbers against this table is a
**manual** step — shared-runner timing is too noisy for a hard regression
gate. (An earlier version of this paragraph claimed automated 10 %
budgets; that job never existed.)

## M1.S4 (2026-08, Rust 1.98)

| Path | Baseline | Blueprint budget | Status |
|---|---|---|---|
| `setup/from_preset` (2×60-card decks, shuffles, opening hands) | 13.8 µs | — | — |
| `state/clone` (full game state, AI lookahead primitive) | 8.6 µs | < 5 µs | ⚠️ optimize in M2 (arena layout) |
| `state/snapshot_hash` | 5.3 µs | — | — |
| `engine/priority_pass_x4` (4× priority pass incl. legality computation) | 3.7 µs | < 50 µs per `legal_actions` | ✅ far under |

Notes:
- Clone is currently dominated by `Vec` allocations in zones/arena; the M2
  plan is copy-on-write zone storage or arena slabs with shared tails.
- Legality is recomputed per priority grant; M2 adds invalidation-scoped
  caches keyed on the effect/event generation.

## Layer projection rewrite (2026-08-31)

Both columns were measured on the same machine in the same session: "before"
is commit `67b1815` with the *current* bench file copied in, so the two runs
measure the same work.

| Path | Before | After | Change |
|---|---|---|---|
| `setup/from_preset` | 13.65 µs | 13.06 µs | −4 % |
| `state/clone` | 8.88 µs | 7.80 µs | −12 % |
| `state/snapshot_hash` | 5.28 µs | 6.25 µs | **+18 %** |
| `engine/priority_pass_x4` | 3.03 µs | 2.78 µs | −8 % |
| `layers/refresh_x1` | 4.05 µs | 4.68 µs | +15 % |
| `layers/refresh_x8` | 15.66 µs | 6.18 µs | **2.5× faster** |
| `layers/refresh_x32` | 116.6 µs | 13.67 µs | **8.5× faster** |

`layers/refresh_xN` is a new bench: a ~60-permanent battlefield under N
anthems, forced to recompute. It exists because nothing else in the file
grows with the number of continuous effects, so the change this table is
about was invisible before.

What the numbers say:

- **Scaling is the point.** Going from 1 to 32 effects cost 28.8× before and
  costs 2.9× now. The old projection sorted every effect from scratch for
  every object it touched (`objects · layers · effects²`); the plan is now
  built once per refresh and each object walks only the effects that can
  reach it (`layers · effects² + objects · effects`). A board under a handful
  of anthems is where real games live, and that is the 2.5× column.
- **The fixed cost went up slightly.** At a single effect the plan build is
  pure overhead (+15 %). That is the trade, and it is the right way round.
- **Clone got 12 % cheaper** because `GameObject` went 768 → 512 B (the
  projection cache became a lazy 24-byte slot). `tests/footprint.rs` keeps it
  there. Since raised to **528 B**: a token now carries the `TokenDef` it was
  made from, which is what gives it its printed abilities (a Treasure that
  can actually be cracked) and the handle the client keys its art on. One
  pointer, ~3 % of the object, taken deliberately against the 33 % the
  cache rewrite bought.
- **`snapshot_hash` regressed 18 % and the cause is the object layout, not
  the added `deathtouched` field** — removing that field from the hash
  entirely measures the same 6.3 µs. It is not on a budgeted path (resync and
  loop-free replay comparison), so it is recorded rather than chased.

## The stack stops being walked (2026-08-31, later)

An Ally deck can put six figures of triggered abilities on the stack. The
layer refresh runs once per engine step and every counter placed invalidates
it, so walking those abilities made the cost of one step proportional to the
whole stack — and an ability on the stack has no characteristic a layer can
change, so all of it was waste. `Zones` now keeps the projectable subset
(spells only) and the refresh reads that instead.

Same machine, same session, one line apart — the bench builds a 20 000-deep
stack of abilities and times one full refresh:

| Path | Before | After | Change |
|---|---|---|---|
| `layers/refresh_over_20k_stack` | 129.6 µs | 8.65 ns | **~15 000×** |

It is a constant now, not a factor, which is the point: extrapolated to the
million-ability stack the rules actually permit, the old path cost ~6.5 ms
per engine step, and there is one step per resolution.

Measured, and *not* fixed here — recorded so the next person does not have
to rediscover it:

- `state/clone` now measures **11.4 µs**, up from the 7.80 µs above. The 16
  bytes `GameObject` gained for the token pointer are far too few to explain
  it, and the cache-line theory was tested and disproved: forcing
  `#[repr(align(64))]` (stride 640, exactly ten lines per slot instead of
  8.5) made it *slower*, 12.37 µs. Clone cost tracks bytes copied linearly,
  so the likeliest explanation is that the 7.80 µs reference is not
  comparable. Worth re-establishing on a quiet machine.
- `Counters` holds four kinds inline, so thousands of tokens with +1/+1
  counters allocate nothing — but the amount is a `u16` with
  `saturating_add` and caps silently at 65 535.

## The printed face moves out of the object (2026-08-31, later still)

`GameObject` inlined 256 bytes of *printed* characteristics — name, mana
cost, the 512-bit subtype bitmap, types, P/T. Every object carried its own
copy even though a 60-card deck prints a dozen faces, a board of three
thousand Zombie tokens prints one, and a stack of a million triggered
abilities prints a handful. The face is now an `Arc<Characteristics>` handed
out by `GameState`'s base cache, and the three places in the engine that
write one go through `GameObject::base_mut`, which splits the sharing first.

Both halves matter. The `Arc` alone shrinks the object; without the cache it
would trade 256 inlined bytes for a `malloc` per object and a cache miss per
read, which is worse at exactly the scale this is for. `create_card`,
`create_bare` and the token factory all take their face from the cache.

Same machine, minutes apart: HEAD in a throwaway worktree against the change,
both with the benches below.

| Bench | Before | After | Change |
|---|---|---|---|
| `state/clone_3k_tokens` | 258.4 µs | 163.2 µs | **−37 %** |
| `state/clone` | 11.37 µs | 8.66 µs | −24 % |
| `setup/from_preset` | 17.86 µs | 13.54 µs | −24 % |
| `engine/priority_pass_x4` | 2.32 µs | 1.89 µs | −19 % |
| `layers/refresh_x32` | 15.23 µs | 13.67 µs | −10 % |
| `layers/refresh_3k_tokens` | 249.4 µs | 244.1 µs | −2 % |
| `layers/refresh_x1` | 6.44 µs | 6.01 µs | −7 % |
| `state/snapshot_hash` | 6.23 µs | 6.29 µs | +1 % |

`GameObject` is **528 → 272 B**; `tests/footprint.rs` holds it there.

Two things the table does not say:

- **The layer refresh barely moved**, and that is the honest result. It reads
  every base once per object, so it trades an inline read for a pointer
  chase into an allocation the whole board shares — a hot line, not a cold
  one, but still a dependent load. What it buys back is the halved arena it
  walks. Net: a few percent, in the right direction.
- **The clone win grows with the board.** At 120 objects it is 24 %; at 3 000
  it is 37 %, because that is where the per-object 256 bytes stopped being
  noise. The AI clones once per ply, so this is the number that compounds.

`state/clone` at **11.37 µs** here also settles the open question from the
section above: the 7.80 µs reference was not comparable, and 11.4 µs was the
real cost of the old layout. Measured fresh, in a worktree with no criterion
history, minutes before the 8.66 µs.

## `SubtypeSet` widened to 1024 bits (2026-09-19, #43)

507 of the old 512 bits were assigned, so the map had to grow before a set
forced it. Both columns are `cargo bench -p baylee-engine --bench basics --
--quick` on this machine minutes apart, the same tree with only
`SUBTYPE_WORDS` flipped, so they measure the width and nothing else.

| Bench | 8 words | 16 words | Change |
|---|---|---|---|
| `setup/from_preset` | 9.39 µs | 9.41 µs | +0.2 % |
| `state/clone` | 5.47 µs | 5.43 µs | −0.8 % |
| `state/clone_3k_tokens` | 124.1 µs | 124.0 µs | −0.1 % |
| `state/snapshot_hash` | 6.39 µs | 7.10 µs | **+11 %** |
| `engine/priority_pass_x4` | 2.17 µs | 2.06 µs | −5 % |
| `layers/refresh_x1` | 5.59 µs | 5.73 µs | +2.5 % |
| `layers/refresh_x8` | 7.10 µs | 7.75 µs | +9 % |
| `layers/refresh_x32` | 13.17 µs | 13.85 µs | +5 % |
| `layers/refresh_3k_tokens` | 207.1 µs | 249.0 µs | **+20 %** |

Sizes, measured the same way: `Characteristics` 256 → 320 B (the budget in
`tests/footprint.rs` was raised to match, deliberately), `PublicObject`
304 → 368 B, and a 25-object view's serialized payload 10 531 → 10 851
bytes — 16 JSON bytes per object that carries a set. **`GameObject` did not
move** (272 B), because it holds its characteristics behind an `Arc`, which
is why `state/clone` is flat in both columns including at 3 000 tokens.

What the numbers say:

- **The cost is proportional to the width and lands on whatever copies or
  reads a whole set**: the snapshot hash walks every word per object, and the
  layer refresh writes a projected `Characteristics` per object. Clone does
  not, because it copies a pointer.
- **It is paid once.** Nine words would have cost a fraction of this and
  bought 69 ids, and then a second widening — which is a second
  `VIEW_VERSION` break. The break is the expensive part, not the bytes.
- **Where to look if it ever bites**: not a narrower set, but the projection
  copying a whole `Characteristics` per object per refresh. The bitmap is its
  largest field, and the base is already shared behind an `Arc` until
  something writes to it.


## Ordered zone removal (2026-09-21)

Stack resolution and drawing both remove the last entry of an ordered zone.
Previously `Zones::remove` searched from the beginning even in that case,
making a complete stack drain quadratic. It now checks the last entry first;
removals elsewhere still use an order-preserving search and removal. The
spell-only projection subset uses the same operation.

Measured on the M1 Max in the same session, using the new benchmark on
`16d98a95` and after the change (20 samples, 1 s warmup, 2 s measurement):

| Benchmark | Before | After |
|---|---|---|
| `zones/drain_stack_100` | 2.437 µs | 240.6 ns |
| `zones/drain_stack_20000` | 63.76 ms | 45.26 µs |

These numbers cover zone removal only, not full ability resolution or whole
games. Zone cloning is outside the timed body. The mixed-removal regression
test checks all eight zones against an independent ordered-list model,
including top, bottom and middle removals, repeated removals, and the stack's
spell subset.

Reproduce with `cargo bench -p baylee-engine --bench basics --
zones/drain_stack --sample-size 20 --measurement-time 2 --warm-up-time 1`;
use Criterion's `--save-baseline before` / `--baseline before` for comparison.

## A second instance of "target" (23.09.2026)

`GameObject` **272 → 280 B**. A fight names two creatures through two
instances of the word "target" (CR 115.3), and the second instance's list and
requirement were first written inline — 32 bytes on every object in every
library, which `tests/footprint.rs` refused at 304 B. They are one
`Option<Box<SecondInstance>>` now: null everywhere but a spell or ability on
the stack that says "target" twice, so `GameState::clone` pays eight bytes per
object and one allocation per such stack object. The budget was raised to 280
deliberately. `state/clone` was **not** re-benched for it; eight bytes on 272
is under 3 % of the object and the arena is not the whole state.

## Where an ability list is printed (24.09.2026)

`GameObject` **280 → 288 B**. A copy's abilities are printed on the copied
card and an ability on the stack keeps its source's, so the object carries
which card and face its own list is (`GameObject::own_face`) — a client draws
that card's sentence, and the list cannot say where it came from: identical
lists share one address, 31 of them in a debug build and 162 in release. A
plain `Option<{ card, face }>` was 12 bytes and measured 296 B; the face is
packed into one `NonZeroU32` instead, so the `Option` is four bytes and the
object grows by the eight its alignment rounds them to. The budget was raised
to 288 deliberately. `state/clone` was not re-benched, on the argument the
entry above makes for the same eight bytes.

## What was paid for a spell (29.09.2026)

`GameObject` **288 → 296 B**. "The sacrificed creature's mana value"
(Eldritch Evolution, Neoform, Birthing Pod) and "the amount of mana spent to
cast this spell" are read back from the stack object as it resolves, and by
then the sacrificed creature is a card in a graveyard. The payment writes a
`PaidRecord` onto the stack object, behind one `Option<Box<…>>` for the reason
`second` is: null on every object that is not a paid-for spell or ability on
the stack, and dropped at every zone change. The eight bytes of the pointer
are the whole cost in a clone (plus one small allocation per spell cast for
mana); the budget was raised to 296 deliberately.
`state/clone` was not re-benched, on the argument the two entries above make
for the same eight bytes.

## "That much" damage an event dealt (29.09.2026)

`GameObject` **296 → 304 B**. Questing Beast deals "that much damage" as the
combat damage that triggered it, so the amount rides from the event onto the
triggered ability as it is put on the stack (`GameObject::event_amount`). It
is an `Option<NonZeroU16>` (damage of 0 is never dealt, CR 120.8), two
bytes; 296 had no padding left, so the object grew by the eight its alignment
rounds to, and a `Box` would have cost the same pointer. The budget was
raised to 304 deliberately. Folding `event_object` and `event_amount` into one
`Option<Box<…>>`, null on every object that is not a triggered ability,
would take the object back to 296 at the cost of touching every reader of
`event_object`. `state/clone` was not re-benched, on the argument the entries
above make for the same eight bytes.

## A chosen card name (29.09.2026)

`GameObject` **296 → 304 B**. "As this artifact enters, choose a card name"
(Pithing Needle) is kept on the permanent as the card and face it names
(`GameObject::chosen_name`), beside the chosen subtype and colour, because the
lock reads it on every offer and the view shows it. It is a `PrintedFace`, so
the `Option` is four bytes; the object had no four-byte hole left and grows by
the eight its alignment rounds them to. The budget was raised to 304
deliberately, and `state/clone` was not re-benched, on the argument the
entries above make for the same eight bytes.

These two entries were made on two branches, each from 296. Merged, the
two fields share the one eight-byte step, and `GameObject` measured 304 B
with both (`tests/footprint.rs`).

## The snapshot hash names every field (24.09.2026, #122)

`GameState::snapshot_hash` now takes every struct it walks apart by name, so
it reads about forty fields it used to skip: X, kicker, the chosen mode, the
land drop, the delayed triggers, the queued extra turns, what was used this
turn, the cards outside the game and more. Both columns are on this machine,
under the cargo lock, in two interleaved rounds (old, new, old, new).
`state/snapshot_hash_3k_tokens` is new: the token board from
`layers/refresh_3k_tokens`, where every token carries its definition and has
one entry in `ability_fires`.

| Bench | Before | After | Change |
|---|---|---|---|
| `state/snapshot_hash` | 7.37 / 7.28 µs | 8.41 / 8.25 µs | +14 % |
| `state/snapshot_hash_3k_tokens` | 233 / 232 µs | 259 / 260 µs | +12 % |

The two maps (`ability_fires`, `restriction_info`) are summed entry by entry,
so the order they iterate in cannot reach the hash. The variants below were
measured in an earlier series, on the 3k board with the start-of-game bench in
brackets, where the kept version measured 271 µs (8.25 µs):

- Sorting the entries into a `Vec` first: 328 µs (8.27 µs). It allocates on
  every call and gains nothing.
- A streaming xxh3 state set up per entry: 320 µs. That setup costs more
  than a thirteen-byte entry, so `ability_fires` entries go to the one-shot
  `xxh3_64` instead, which is the kept version.
- A 64-byte write buffer in front of `Xxh3`: 616 µs (15.6 µs), twice as slow.
  `Xxh3::update` already buffers, and a copy of runtime length per write costs
  more than it saves. Not kept.

A token's definition is hashed by its content, never by its address, and on
the 3k board that was about 40 µs in the same series (13 ns a token).
Counters no longer go through a `Vec` per object; what that saves was not
measured on its own.

## A token army's combat stops being quadratic (29.09.2026)

Self-play games where an Ally deck attacks with tens of thousands of tokens
took seconds per answer, and the time was not in the payment search. Four
walks cost permanents × attackers (or permanents × effects): `Filter::Attacking`
scanned the attacker list for every object a filter walk asked about (Kor
Haven's "target attacking creature" is probed at every priority grant); the
blockers offer, and `menace_satisfiable`, asked `can_block` of every
permanent against every attacker; the duplicate checks in the two
declarations were a `contains` per entry; and `compute_legal` asked the whole
effect table per permanent for granted abilities and searched
`mana_abilities` with `contains`. `CombatState` now keeps a sorted index
beside its (private) attacker list, the offer walks `combat::ready_blockers`,
the checks are sets, and the grants are collected once
(`effects::grants`). No legal move changes; `CombatState`'s hash is the one
the derived impl gave, and replays of r001 records reach the same snapshot
hash after every input before and after.

`combat/attack_to_blocks_{100,900}` is new: seat 0 declares 100 or 900
vanilla attackers into a defender with Kor Haven and two creatures, and the
timed body runs from the declaration to the blockers offer. Both columns are
`--quick` runs on the M1 Max in one session, `66a83dd9` with the current
bench file copied in and then this change, under a load average of about
30 from other work; differences under ~10 % on unchanged paths are noise.

| Bench | Before | After | Change |
|---|---|---|---|
| `combat/attack_to_blocks_100` | 61.5 µs | 31.0 µs | −50 % |
| `combat/attack_to_blocks_900` | 2.52 ms | 297.6 µs | **8.5× faster** |
| `engine/priority_pass_x4` | 4.67 µs | 3.42 µs | −27 % |
| `setup/from_preset` | 10.92 µs | 11.04 µs | +1 % |
| `state/clone` | 7.28 µs | 6.35 µs | noise |
| `state/snapshot_hash` | 9.03 µs | 9.10 µs | +1 % |
| `state/snapshot_hash_3k_tokens` | 373.4 µs | 306.2 µs | noise |
| `layers/refresh_x1` | 7.80 µs | 7.37 µs | −6 % |
| `layers/refresh_x8` | 8.91 µs | 9.04 µs | +1 % |
| `layers/refresh_x32` | 15.55 µs | 16.50 µs | +6 % |
| `layers/refresh_over_20k_stack` | 10.29 ns | 10.30 ns | 0 % |
| `state/clone_3k_tokens` | 154.7 µs | 154.5 µs | 0 % |
| `layers/refresh_3k_tokens` | 264.5 µs | 269.1 µs | +2 % |
| `zones/drain_stack_100` | 251.4 ns | 253.4 ns | +1 % |
| `zones/drain_stack_20000` | 46.5 µs | 47.0 µs | +1 % |

From 100 to 900 attackers the step cost 41× before and 9.6× now.
`priority_pass_x4` is the one unchanged path that moved: a land's
`{T}: Add …` has no mana symbol in its cost, and `can_pay_mana` now answers
that without merging pools or walking the effect table.

`compute_legal` alone, in a local probe (not committed) on the shared
testkit, `ci-release`, µs per call:

| Permanents | Granted mana board, before | after | Attack into Kor Haven, before | after |
|---|---|---|---|---|
| 20 | 5.9 | 3.5 | 2.0 | 2.8 |
| 80 | 22.4 | 12.5 | 5.3 | 3.9 |
| 320 | 113.7 | 44.9 | 35.6 | 12.2 |
| 700 | 327.2 | 96.5 | 131.7 | 25.5 |
| 950 | 528.5 | 129.3 | 240.5 | 34.9 |

The granted mana board is n Forests under Great Divide Guide; the attack
board is n vanilla attackers against a defender holding Kor Haven.

Whole games, engine only (a record replayed through `Engine::apply`, nothing
else): r001 game 431 (168,000 permanents by turn 40, one declaration of
33,600 attackers) **54.6 s → 1.3 s**, of which one blockers offer alone was
18–29 s; game 685 **56.0 s → 3.2 s**. Played through `Session` with the
house AI the same games are still slow, and now for another reason: in 431
the AI's `PlayerView::object`, a linear scan over the view, called per
attacker and per blocker option, is 90 % of the main thread's samples,
where `Engine::apply` is 4 % and building the agent's view 2 %.

## What a departing permanent held (29.09.2026)

A permanent leaving the battlefield now asks the exile zones whether a card
there was held "until it leaves the battlefield" (CR 610.3,
`GameState::return_what_departed_hosts_held`): a walk over every exiled card,
once per departure. Nothing in the suite moved a permanent off the
battlefield, so `zones/wrath_60_exile_{0,800}` is new: sixty creatures moved
to the graveyard one after another, with no card or 800 cards in exile. 800
is every card at a table of eight Commander decks; a token in exile ceases to
exist (CR 704.5d), so the exile zone never holds more than the cards.

Scan off (`move_object` skips the call) and on, the whole suite `--quick`,
same machine, back to back, two rounds. The machine was not idle: other
worktrees were compiling, load average 19 to 86 over the four runs.

| Bench | Off | On |
|---|---|---|
| `zones/wrath_60_exile_0` | 11.93 / 12.62 µs | 11.79 / 12.77 µs |
| `zones/wrath_60_exile_800` | 12.33 / 12.85 µs | 106.6 / 83.4 µs |

About 2 ns per exiled card per departure: at the 800-card ceiling a
departure costs about 1.5 µs and a sixty-permanent wrath about 90 µs, once.
At a real table, a few dozen cards in exile, it is well under a tenth of a
microsecond a departure. The other benches move nothing off the battlefield,
and their spread between runs, up to three times and in both directions
between off and on, is the load. No early exit was added: a flag, a count
of live links or a mark on the host is more state to keep in step with the
riders, for a cost no game can see.

## The refresh projects its counters again (29.09.2026)

A projection that counts the board (Ashaya, Soul of the Wild: as big as the
lands you control) read every object the walk had not reached yet as the last
refresh left it, and came out short. The refresh now projects the counting
objects again once the board is done, repeating while one of them moved
(`GameState::refresh_characteristics`). `layers/refresh_x8_counting` is new:
`layers/refresh_x8` with every creature counting the creatures you control,
the repeat at its widest.

Both columns are this M1 Max under a shared load (load average 15 to 50),
"before" being `9dbd6e56` with the current bench file. The two bench binaries
ran alternately, six `--quick` rounds each, and each cell is the smallest
median of the six: a single round's median moved several times over on rows
neither commit touches. The rows below were run six rounds more; both series
are given.

| Bench | Before | After | Change |
|---|---|---|---|
| `layers/refresh_x8_counting` | 13.64 / 12.81 µs | 22.14 / 22.05 µs | **+62 / +72 %** |
| `layers/refresh_over_20k_stack` | 10.27 / 10.18 ns | 11.64 / 11.32 ns | +13 / +11 % |
| `layers/refresh_x1` | 6.29 / 6.01 µs | 6.43 / 6.26 µs | +2 / +4 % |
| `layers/refresh_x8` | 7.73 / 7.50 µs | 8.37 / 7.50 µs | +8 / 0 % |
| `layers/refresh_x32` | 14.51 / 15.18 µs | 15.22 / 14.69 µs | +5 / −3 % |
| `layers/refresh_3k_tokens` | 269.5 / 266.4 µs | 281.4 / 276.7 µs | +4 / +4 % |
| `engine/priority_pass_x4` | 3.09 / 2.82 µs | 3.36 / 2.88 µs | +9 / +2 % |

- **A board where something counts pays one more projection per counter.**
  With every creature a counter that is 1.6 to 1.7 times the refresh; a real
  board has one or two, and every other board has none and projects nothing
  twice.
- **The fixed cost is about a nanosecond.** `refresh_over_20k_stack` projects
  nothing at all, so its 1.1 to 1.4 ns is the new bookkeeping (a list that
  stays empty and never allocates) on every refresh.
- The other rows moved within what the same code moves between rounds here
  (`state/clone`, which neither commit touches: +2 %).


2026-10-02: captured token ability provenance uses `AbilityOrigin` in the
existing four-byte provenance slot (card face or token id). The footprint test
still measures `GameObject = 312 B`, `GameState = 1912 B`; neither budget was
raised. A separate two-byte field would round the object to 320 B and was
replaced with the packed representation before committing.

## 2026-10-03 — atomic decision checkpoints (Alpha integration)

`Engine::apply` now restores the entire last published decision when a numeric
capacity failure occurs after costs or automatic effects have run. The snapshot
includes the driver, journal, random state and nested mana obligations. This has
an observable cost; it is not covered by the older priority-pass timings above.

Same-machine `--quick` measurements of `engine/priority_pass_x4`:

| Current integration variant | Estimate |
|---|---:|
| Eager arena and journal snapshot | 47.65 µs |
| Copy-on-write arena | 28.51 µs |
| Copy-on-write arena and journal (retained implementation) | 27.52 µs |
| Same code with checkpoint temporarily disabled, measurement only | 5.98 µs |

The disabled-checkpoint control was restored immediately after the measurement;
it does **not** satisfy atomic refusal and is not an alternative production path.
The retained version still costs about 21.5 µs more across four passes in this
fixture. These short runs identify the tradeoff, not a statistically established
performance bound. `state/clone` with shared arena/journal measured 4.97 µs.

Arena and journal clones share immutable vectors. Their first actual mutation
copies the vector; subsequent writes within the same decision do not. Failed
lookups and no-op journal truncation do not force a copy. Hashes and serialized
journal contents remain structural; allocation identity is never rules state.
The clone-isolation, journal round-trip and numeric-refusal regressions cover
append, removal, rollback and replay. Larger mutable boards and long journals
still require measurement before claiming this overhead is acceptable there.


A later controlled follow-up tried retaining one checkpoint and refreshing its
buffers with field-complete `clone_from` implementations. All atomic-refusal,
nested target-batch and replay checks passed, but the actual priority path was
**32.50 µs**, versus **30.46 µs** for fresh complete snapshots measured immediately
afterward in the same client-running environment. The attempt was removed; it
is not a retained optimization. The earlier 27.52 µs run is not a simultaneous
control for this comparison.

The diagnostic state-only measurements were 5.20 µs for a fresh clone and
4.25 µs for the experimental refresh; names and zones alone were 0.27 µs and
0.32 µs. Reusing some allocations did not improve the measured engine path.
`basics` now exposes separate clone-from, names and zones measurements for
further investigation. The production path still takes and restores a complete
checkpoint, with the retained arena/journal copy-on-write storage above; no
numeric-capacity threshold or rollback omission was introduced.


## Server and tooling (2026-10-06, 0.1.0-beta.6)

Everything outside the rules kernel: the gateway, the engine process's
transport, the catalog, the database, xtask and the build. Same M1 Max, but
**shared**: other sessions were compiling throughout, at load averages of 10
to 150 on ten cores, and each figure below carries what the machine was doing
only as far as "busy". Read the ratios, which were taken minutes apart in one
session, rather than the absolute times. Every figure is reproducible with the
command beside it.

### How to measure

| What | Command |
|---|---|
| Gateway under load | `DATABASE_URL=… cargo test --release -p baylee-gateway --test load -- --ignored --nocapture` (knobs `LOAD_GAMES`, `LOAD_LOBBY`, `LOAD_CHURN`, `LOAD_ECHOES`, `LOAD_ECHO_HZ`, `LOAD_BLAST`, `LOAD_BYTES`) |
| One frame in the engine process | `cargo bench -p baylee-engine-server --bench frames` |
| Catalog search | `EXPLAIN (ANALYZE, BUFFERS)` of `search_sql()` (`crates/baylee-catalog/src/sql.rs`) against an ingested catalog |
| Engine-server start | spawn the dev harness with `PORT=0 BAYLEE_PORT_FILE=…` and time the port file |

The load bench runs a real gateway (own port, own schema) and a stand-in
engine that answers in-band, so the bytes it routes carry their own send time
and the gateway's own CPU (`ps`) is read beside the wall clock. Its seats and
engines share one test process with four runtime threads, so the round-trip
times include that process's own scheduling; the gateway's CPU per frame is
the cleaner number.

### The gateway (200 lobby feeds, 50 games of two, release)

| | Before | After | What changed |
|---|---|---|---|
| Gateway CPU per lobby change | 12.6 ms | 2.2–2.8 ms | names read once per account, not once per feed per change |
| … per listing pushed | 63 µs | 13 µs | (the rest is the listing's JSON, rendered per feed) |
| Open + leave a room, p50 / p99 | 30 / 80 ms | 2.4 / 8.2 ms | the feeds' 400 queries a change no longer hold the pool |
| … while 100 seats send, p50 | 142 ms | 2.9 ms | |
| 50 games opened with the feeds open | 23.3 s | 6.5 s | |
| Seat → engine → seat, p50 / p99, quiet | 2.3 / 7.1 ms | 2.0 / 3.0 ms | no lobby lock per frame |
| … beside lobby churn, p50 / p99 | 3.4 / 15.7 ms | 1.3 / 3.4 ms | |
| Fan-out, gateway CPU per 16 KiB frame | 23 µs | 16 µs | `Bytes` end to end, one channel per seat |
| Resident memory per game (two seats, engine link) | 672 KiB | 344 KiB | 8 KiB / 32 KiB read buffers instead of 128 KiB |
| Peak after a 100-frame burst to every seat | 169 MB | 84 MB | no copy per listener |
| Idle | 21.5 MB | 21.5 MB | |
| Per lobby feed | 26 KiB | 26 KiB | |

Seat round trips forwarded about 10 000 frames a second at 19–27 µs of
gateway CPU each, before and after: what a forwarded frame costs is the
socket's syscalls and the runtime's wake-ups, not the copies. The fan-out
delivered 110–145 000 frames a second (2 GB/s) bound by the bench's own
client.

Found while measuring: tungstenite zeroes its whole read buffer before every
read (`FrameCodec::read_in`), 128 KiB by default, which a sample of the
forwarding gateway showed as `bzero` beside `recvfrom`. Every websocket the
client and the seat bridge open still uses the default.

### The engine process (`frames` bench)

| Path | Time |
|---|---|
| `setup` (preset JSON to a built game) | 350–480 µs |
| `attach` (a seat's whole state: static, view, log) | 60–110 µs |
| `resync` | 28–34 µs |
| `answer_keep` (a mulligan answer that moves the game, two seats' frames) | 165–230 µs |
| Start to serving (dev harness, release) | 20 ms median (9 ms min), 8.8 MB resident |

The frames weigh 23.6 KB (attach, four envelopes), 13.8 KB (resync) and
7.7 KB (an answer). Protobuf around them is noise: 0.4 µs to encode a resync's
four envelopes, 0.4 µs to decode them, and the gateway's unwrap of a seat
frame went from 485 ns to 102 ns once the payload stayed a slice of the frame.
A sample of `answer_keep` puts about a fifth of it in
`GameState::snapshot_hash` and most of the rest in serializing the views to
JSON, both in the engine and gamehost lanes. Records are already gzipped
(`flate2`, level 6) in 32 KiB pieces at most every 30 s.

### Catalog search (542 177 printings, 41 991 search rows, warm)

| Query | Time |
|---|---|
| A name (`bolt`, `sol ring`, `goblin guide` in German) | 1–3 ms |
| A common word (`dragon`) | 6.6 ms |
| Rules text (`draw a card`, 3 753 hits) | 18 ms |
| One letter (`e`, no bigram: every row) | 262 ms |
| Cold cache | 10–110 ms, 338 ms for one letter |

No client calls `/catalog/search` today (the deck builder searches the
pool), so this was measured and left alone. The one-letter case and the text
tier's join back to `card_search` per hit are where a change would go.

### The database

`session_token` was the one table following an account away without an index
on the account. The guest purge (`DELETE … WHERE guest AND NOT EXISTS (live
session)`) cascades into it once per guest: 2 000 lapsed guests among 50 000
with two sessions each took **9.1 s** holding the rows, **76 ms** with the
index (migration 14); deleting one account 7.5 → 3.3 ms. The other hot reads
(a session by its token, decks by account, uploads, records) were already
indexed.

### xtask and the build

| | Before | After |
|---|---|---|
| `codegen --check` (release) | 63.7 s | 4.9 s |
| `codegen --tables` | 1.0 s | |
| `validate` | 0.5 s | |
| `cargo build --workspace`, nothing changed, in a worktree | 23.7 s | 0.8 s |
| … after touching the gateway | | 4.4 s |
| … after touching `baylee-protocol` | | 24.9 s (21 units; `baylee-client` 8 s + its binary 14 s) |
| … from clean (debug) | 3 min 46 s | |

Codegen spent all but a second spawning rustfmt once per machine-owned card,
one after another; it now runs one rustfmt per core over a share of them
(`format_rust_many`, held to formatting each alone by a test). The no-change
build was `baylee-build`'s script watching `../../.git/HEAD`, which in a
linked worktree does not exist, and cargo calls a missing watched path
changed on every build, so every agent's worktree rebuilt the gateway, the
engine-server, the client and xtask each time.
