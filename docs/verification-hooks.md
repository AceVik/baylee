# Card-verification hooks

Three test-only hooks let a checker outside the engine grade the pool's
cards without reading engine code:

| rung | question                                                         | hook                                    |
|------|------------------------------------------------------------------|-----------------------------------------|
| L4   | has every ability a card prints fired in some test?              | `BAYLEE_ABILITY_LOG=<dir>`              |
| L5   | does taking any one ability away make some test of the card fail? | `BAYLEE_MUTATE=<card>:<index>`          |
| —    | is the engine's cached layer projection what the layers say now? | `Engine::projection_is_fresh` (`fuzz`)  |

This document is normative for the environment variables, the file formats,
the exit statuses and the feature names below. None of the three is in a
build that ships (see [Not in a shipped build](#not-in-a-shipped-build)).

| hook | code |
|------|------|
| firing recorder | `crates/baylee-engine/src/ability_log.rs` (`#[cfg(test)]`) and one `#[cfg(test)]` call at each door named below |
| mutation switch | `crates/baylee-cards/src/mutate.rs`, applied in `baylee_cards::{by_index, by_oracle_id, all}` |
| projection check | `crates/baylee-engine/src/engine/invariants.rs` |
| their own tests | `crates/baylee-engine/src/engine/verification_tests.rs`, `invariants::tests`, `baylee_cards::mutate::tests` |

Both variables are read once per process, at the first card lookup or the
first firing; an empty value is the same as unset. Neither is read at
compile time, so changing one never rebuilds anything.

## L4: the firing recorder

### Running it

```bash
BAYLEE_ABILITY_LOG=/abs/path/to/fresh/dir cargo test -p baylee-engine --lib
BAYLEE_ABILITY_LOG=/abs/path/to/fresh/dir cargo nextest run -p baylee-engine --lib
```

Only `baylee-engine`'s library test binary records; the variable does
nothing anywhere else. The directory is created if missing. **Use a fresh
directory per run**: a test's file is started afresh by that test's first
line, but files of tests that did not run (or fired nothing) this time are
left as they were. Filtering the run (`-- card_tests::instants`) is fine; the
files then cover only the tests that ran.

### What it writes

- `<dir>/<test>.jsonl`, one file per test that saw at least one firing.
  `<test>` is the test's full name as libtest and nextest print it, the module
  path from the crate root, for example
  `engine::card_tests::instants::lightning_bolt_deals_three_to_a_creature_or_a_player.jsonl`.
  On Windows `:` becomes `_` in the file name; the `test` field inside is
  always the real name and is the one to key on.
- `<dir>/unnamed-thread.jsonl`, for firings on a thread no test named: a test
  that spawns threads and does not start them with `testkit::spawn_named`.
  Every sweep in the suite names its threads today; this file is not written
  by a full run (measured 2026-09-29). If it appears, a new test needs
  `spawn_named`.
- `<dir>/pool-inventory.json`, written by the test
  `engine::verification_tests::pool_inventory` (see [The inventory](#the-inventory)).
  A full run includes it; a filtered run must name it too if it needs the file.

Each line of a `.jsonl` file is one object, fields in this order:

```json
{"test":"engine::card_tests::instants::lightning_bolt_deals_three_to_a_creature_or_a_player","card":144,"index":4294967295,"kind":"spell"}
```

| field   | type   | meaning |
|---------|--------|---------|
| `test`  | string | the test's full name, as above |
| `card`  | u32    | the card's `CardIndex` (the ledger in `crates/baylee-cards-index`; stable forever) |
| `index` | u32    | the ability: its position in the card's ability list (`CardDef::abilities_for_face`), or `4294967295` (`AbilityRef::SPELL`) for a spell ability |
| `kind`  | string | one of `spell`, `activated`, `triggered`, `static`, `replacement`, `mana` |

A spell ability is always logged under `4294967295`, never under its
position. Every other kind is logged under its position. A line is written
once per test for each `(card, index, kind)`, in the order they first fired.
The recorder only reads the game, by construction rather than by a test:
every door takes the state by `&GameState` and writes only its files and
thread-locals, so a game's state, journal and `snapshot_hash` are the same
with or without it.

### What counts as fired

| `kind`        | fired when | where |
|---------------|------------|-------|
| `spell`       | a spell whose card lists an `AbilityDef::Spell` or `ModalSpell` finished resolving, by either door a resolving spell leaves the stack through | `Engine::finish_resolution`, and `resolve_stack_top` before `finalize_spell` (a spell with no effects to run) |
| `activated`   | an `Activated`, `ActivatedConditional` or `Loyalty` ability that is not a mana ability finished resolving off the stack | `Engine::finish_resolution` |
| `triggered`   | a `Triggered`, `ModalTriggered` or `SagaChapter` ability finished resolving; a triggered mana ability (CR 605.1b, Badgermole Cub's "add an additional {G}") produced its mana off the stack | `Engine::finish_resolution`, `resolve_triggered_mana_abilities` |
| `mana`        | a mana ability (`mana_ability: true`) had its cost paid and produced its mana, including after the colour question it may ask; a land tapped for mana through CR 305.6 credits its printed `{T}: Add …` entry for that colour | `start_activation`'s mana branch, `mana_finished`, the two CR 305.6 taps in `actions.rs` |
| `static`      | the layer projection applied that `AbilityDef::Static`'s effect to some object | `layers::recompute_with` |
| `replacement` | an `AbilityDef::Replacement`'s rule changed an event (every `ReplacementRule`: exile instead of a graveyard, both doublers, trigger multiplier, trigger suppression), or a `CopyOnEnter` / `CopyOnEnterUntilEot` permanent entered as a copy | `replacement.rs`, `trigger.rs`, `apply_copy_choice` |

Not fired: an ability put on the stack that never resolved (countered, or
removed by CR 608.2b because every target became illegal), a spell that
never resolved, an activation whose cost was never paid, a static whose
filter matched nothing, a replacement whose event never happened.

A permanent spell (a creature, an artifact) lists no spell ability, so
casting one logs nothing; its own abilities are logged when they fire.

### Which card is credited

- **Only cards of the compiled pool.** A test that plays a card it made up
  under a pool index (the synthetic cards of `m2_tests`) credits nothing:
  the recorder compares the lookup's card with `baylee_cards::by_index`.
- **The card whose list the ability came from.** A copy fires the copied
  card's ability: a Clone that copied Llanowar Elves and tapped for mana
  logs the Elves' `0 mana`; the Clone's own line is its `replacement` for
  entering as a copy.
- **Not logged at all**, because they have no `(card, index)`: a token's or
  an emblem's ability, an ability granted by another object (the granting
  static is logged as `static` when it applies), and a synthesised one
  (prowess, ward's counter trigger, a reflexive trigger, the reserved
  indices of `AbilityRef`).
- **Variants no door logs**: `Ward`, `Toxic`, `Prepared`, `Echo`, `Suspend`
  and `Unimplemented`. The inventory lists them with `"kind": null`; L4
  needs other evidence for them.

### The inventory

`pool-inventory.json` is a JSON array with one row per card of the pool
(`baylee_cards::all()`), which is the set L4 is computed against:

```json
{"card":163,"name":"Mountain","oracle_id":"a3fb7228-e76b-4e96-a40e-20b5fed75685","implemented":true,"room":false,
 "abilities":[{"index":0,"position":0,"variant":"Activated","kind":"mana","intrinsic":true,"faces":[0]}]}
```

| field | meaning |
|-------|---------|
| `card`, `name`, `oracle_id` | the card |
| `implemented` | `Coverage::Implemented` |
| `room` | a Room, which numbers its abilities by its unlocked doors ([Rooms](#rooms)) |
| `abilities[].index` | what the recorder logs: the position, or `4294967295` for a spell ability |
| `abilities[].position` | the position in the list, also for a spell ability |
| `abilities[].variant` | the `AbilityDef` variant's name |
| `abilities[].kind` | the `kind` the recorder logs it as, or `null` if no door logs that variant |
| `abilities[].intrinsic` | the entry is the mana ability CR 305.6 gives the land for its basic land types (see [Equivalent mutants](#equivalent-mutants)) |
| `abilities[].faces` | the faces (`0` = front) whose list holds this entry |

An `AbilityRef` names no face. Face 0 reads its own list, or the card-level
list when it has none; every other face reads its own. An entry is one
`(index, kind, variant)`, listed once with every face that holds it, so a
transforming card whose back face has a different ability at the same
position has two entries under that index. Key an L4 check on
`(card, index, kind)`: every line the recorder writes is one of the
inventory's entries, except a Room's with both doors unlocked
([Rooms](#rooms)).

A card is L4 when every entry with a non-null `kind` appears as a line in
some test's file.

Generate the inventory in a run **without** `BAYLEE_MUTATE`: the pool it
reads goes through the mutation switch too.

### Rooms

A Room (`"room": true`; Walk-In Closet is the pool's one) numbers its
abilities by the doors that are unlocked (CR 709.5, `CardDef::door_abilities`):
with only the left door unlocked an index is a position in face 0's list,
with only the right door in face 1's list, and with both in the card-level
list, which is the left half's entries followed by the right half's. The
recorder logs the index of the list in force when the ability fired. So a
right-half entry at `position` p fires as index `p` with the right door
alone and as index `len(left entries) + p` with both doors open; the second
spelling is not an inventory entry, and is the one kind of line a full run
writes outside the inventory (measured: `(Walk-In Closet, 1, triggered)`).
Map it back through the left half's length.

The mutation switch replaces position k in the card-level list and in both
faces' lists, so for a Room a mutant of k may take an ability from each
half while one door is unlocked, and a mutant of `len(left) + p` takes the
right half's p-th ability only while both doors are unlocked.

### Measured on 2026-09-29

A full `BAYLEE_ABILITY_LOG` run of `cargo test -p baylee-engine --lib` at
this commit: 3487 tests passed, 2928 test files plus the inventory, 12960
lines, 3523 distinct `(card, index, kind)` over 3492 `(card, index)`, no
`unnamed-thread.jsonl`. The inventory holds 2749 cards and 3652 distinct
loggable entries (108 of them `intrinsic`); 3522 of them fired, the one
line outside them is the Room case above, and 2353 cards had every loggable
entry fire.

## L5: the mutation switch

### Running it

Build the engine's test binary once:

```bash
cargo test -p baylee-engine --lib --no-run
# Executable unittests src/lib.rs (target/debug/deps/baylee_engine-<hash>)
```

(`--message-format=json` gives the same path as the `executable` of the
`compiler-artifact` message whose `target.name` is `baylee_engine` and whose
`profile.test` is `true`.)

Then one process per mutant, naming the tests to run:

```bash
cd crates/baylee-engine    # cargo runs the binary here; do the same
BAYLEE_MUTATE=<card>:<index> ../../target/debug/deps/baylee_engine-<hash> \
    --exact <test> [<test> …] --test-threads=1
```

or, through cargo, which finds the binary and rebuilds nothing:

```bash
BAYLEE_MUTATE=<card>:<index> cargo test -p baylee-engine --lib -- --exact <test> [<test> …]
```

The tests worth running for a card are the ones whose L4 files name it.
`--test-threads=1` is not needed for correctness (the switch is per
process); it keeps the output in order.

### What it does

`<card>` is a `CardIndex`, `<index>` a position in the card's ability list or
`4294967295` (`AbilityRef::SPELL`), both decimal. The named entry is
replaced by `AbilityDef::Unimplemented`, the placeholder every reader passes
over. It is replaced rather than removed, so every other ability keeps its
position and its name.

- `SPELL` replaces every `Spell` or `ModalSpell` entry of the card; a
  position replaces that position.
- The position is replaced in the card-level list and in every face's own
  list long enough to have it, because an index names no face (the
  inventory's `faces` says which faces that touches).
- The replacement is made at the registry's doors, `baylee_cards::by_index`,
  `by_oracle_id` and `all`, which every lookup a test builds an engine with
  ends in. So every reader of the card sees the mutant: the engine's
  ability lists, the places that read `CardDef::abilities` straight, a copy
  taking the card's copiable values, an ability the card grants from inside
  one of its statics, and the test kit's own helpers.
- The other reserved indices (`ENTERS`, `ADDITIONAL_COST`, …) name questions
  a card raises, not entries, and are refused.

Every mutated process writes one line to standard error, past libtest's
capture, at its first card lookup, so a survivor's output shows that the
mutant was in place:

```
BAYLEE_MUTATE: Lightning Bolt (card 144) has ability 4294967295 (SPELL) replaced by Unimplemented
```

### Reading the result

| exit status | meaning |
|-------------|---------|
| `0`   | every test named passed: the mutant **survived**. Check that libtest's summary says `N passed` with `N > 0`: a filter that matches no test also exits `0` |
| `101` | at least one test failed: the mutant was **killed** |
| `3`   | the mutant is not one (`baylee_cards::mutate::INVALID_MUTANT_EXIT`): a malformed value, a card the registry does not have, an index naming no entry, an entry already `Unimplemented`, a reserved index other than `SPELL`. The process stops at its first card lookup, which is inside the first test that builds a game, and says why on standard error. Neither survived nor killed |

A card is L5 when, for every entry of its inventory row, the mutant of that
entry is killed by at least one of the card's tests.

### Equivalent mutants

An entry with `"intrinsic": true` in the inventory is the mana ability CR
305.6 gives a land for each of its basic land types, printed as reminder
text ("({T}: Add {R}.)"). The engine taps a land for its basic types
through that rule, whether the entry is there or not
(`casting::intrinsic_mana_offer`), so its mutant
survives by the rules, not by a hole in the tests: with Mountain's entry
replaced, Lightning Bolt's test still passes. `verification_tests` holds
that survivor as a test. Exclude these entries from L5; L4 still credits
them when the land taps.

## `Engine::projection_is_fresh` (feature `fuzz`)

```toml
# in a fuzzer's Cargo.toml
baylee-engine = { workspace = true, features = ["fuzz"] }
```

```rust
pub fn projection_is_fresh(&self) -> bool   // impl<L: CardLookup> Engine<L>
```

The same feature carries `Engine::fingerprint` and `Fingerprint`, the
engine's full state for a fuzzer's equality checks (`c42/engine-karn-targets`).
This check asks a narrower question and does not use them: whether the
cached projection is what a refresh would make of the state.

The engine keeps the layered projection (CR 613) cached behind one `u64`
generation compare, so a change to a characteristic's input that skips its
invalidation leaves every reader on the old value while the generation
says all is well. This asks the content, not the number: a copy of the
state is invalidated and refreshed by the engine's own
`GameState::refresh_characteristics`, and every object's characteristics
and controller are compared with what the object answers now. `true` means
a refresh would change nothing anyone can read. Using the engine's refresh
rather than a second statement of which objects it projects keeps the check
from drifting away from it: that set has grown (spells on the stack, every
object under a cross-zone effect, cards defining their own power and
toughness in every zone).

It writes nothing in the engine, neither a cache nor a generation. It costs
a clone of the state (journal included) and a full refresh, so call it
between answers (after `apply` returns), not in a hot loop. Its test,
`invariants::tests::a_skipped_invalidation_is_a_stale_projection`, puts a
+1/+1 counter on a creature without invalidating and gets `false`, still
`false` once the invalidation is made and the refresh has not run, and
`true` after the refresh.

A `false` has two readings, and the public state tells them apart:

| `state().characteristics_generation == state().effects.generation` | reading |
|------|---------|
| yes  | nothing told the cache it is old: a skipped invalidation, or a refresh whose single walk is not yet a fixpoint. The engine will not correct it by itself |
| no   | the engine knows and refreshes at the top of its driving loop; `apply` returned before that (a question asked in the middle of a resolution, or the mulligan window). A reader of the state in between still reads the old value |

Measured on 2026-09-29 with a probe (not committed) calling it after every
`apply` of the full engine suite. The first run gave `false` 96 times in 8
tests and found two defects in the engine, both fixed since:

- A copy of a spell is a new object put straight onto the stack (CR 707.10),
  and nothing told the projection: under Maskwood Nexus the copy answered
  the Elf Druid it prints while a refresh made it every creature type
  (`GameState::put_new_spell_on_stack`; test
  `combo_tests::filters::the_nexus_makes_a_copied_creature_spell_every_type`).
- A refresh projects one object at a time, and a count read the objects the
  walk had not reached yet as the last refresh left them, against CR 613.1.
  Ashaya, Soul of the Wild counted an Elf that had just entered as an Elf
  and not yet as a Forest, and stayed a land short (21/21 where a refresh
  gives 22/22 in `offer_tests`, 24/24 in `target_tests`: two boards, not
  two answers for one). A projection that counts now says so
  (`layers::Projection::read_board`), and the refresh projects those again
  once the board is done (test
  `card_tests::creatures::ashaya_counts_a_creature_the_moment_it_enters`).

The second run, after both fixes: `false` 18 times in 7 tests. Seven were
the harness, in
`combo_tests::doubling::bristly_bills_doubling_is_doubled_again_by_a_doubling_season`,
which set Bill's counters through `Engine::dev_state_mut` with nothing
invalidated, so Bill answered 2/2 until the next change moved the
generation; that door now invalidates as it opens.

The third run, after that: `false` 11 times in 6 tests, every one with the
generation moved and a refresh due, and every one in the mulligan window,
before the driving loop has refreshed once: cards defining their own power
and toughness in hand (Ashaya, Pyrogoyf) and the counters the harness plants
on Walking Ballista and Arcbound Ravager. Each of them counts or wears
something only a seeded board has; a game dealt from its decks has nothing
on the battlefield and in the graveyards yet, and so nothing to be behind
on.

## Not in a shipped build

| hook | compiled when |
|------|---------------|
| recorder | `#[cfg(test)]` in `baylee-engine`: only its own test binary |
| mutation switch | `cfg(any(test, feature = "mutate"))` in `baylee-cards`; only `baylee-engine`'s `[dev-dependencies]` turn `mutate` on |
| projection check | `cfg(any(test, feature = "fuzz"))` in `baylee-engine`; nothing in the workspace turns `fuzz` on |

A build without dev-dependencies (every `cargo build`, every binary, every
library a dependent links) names neither feature:

```bash
cargo tree --workspace --target all -e features,normal,build | grep -cE 'feature "(mutate|fuzz)"'   # 0
cargo tree -p baylee-engine --target all -e features | grep 'baylee-cards feature "mutate"'          # the dev edge
```

`scripts/gate-features.sh` runs both (`verification-hooks-shipped`), the
second as the control that proves the first can match, and runs clippy on
each library with its feature on and `cfg(test)` off (`fuzz`, `mutate`). Within one
`cargo test --workspace`, feature unification also compiles `mutate` into
the other crates' test binaries; it stays inert there without the variable,
and none of them is shipped.
