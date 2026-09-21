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
