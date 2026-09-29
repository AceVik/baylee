# Card-verification hooks

Four test-only hooks let a checker outside the engine grade the pool's
cards without reading engine code:

| rung | question                                                         | hook                                    |
|------|------------------------------------------------------------------|-----------------------------------------|
| L4   | has every ability a card prints fired in some test?              | `BAYLEE_ABILITY_LOG=<dir>`              |
| L4   | does a permanent's every ability stop when it leaves play?       | `BAYLEE_LEAVE_LOG=<dir>`                |
| L5   | does taking any one ability away make some test of the card fail? | `BAYLEE_MUTATE=<card>:<index>`          |
| —    | is the engine's cached layer projection what the layers say now? | `Engine::projection_is_fresh` (`fuzz`)  |

This document is normative for the environment variables, the file formats,
the exit statuses and the feature names below. None of the four is in a
build that ships (see [Not in a shipped build](#not-in-a-shipped-build)).

| hook | code |
|------|------|
| firing recorder | `crates/baylee-engine/src/ability_log.rs` (`#[cfg(test)]`) and one `#[cfg(test)]` call at each door named below |
| mutation switch | `crates/baylee-cards/src/mutate.rs`, applied in `baylee_cards::{by_index, by_oracle_id, all}` |
| projection check | `crates/baylee-engine/src/engine/invariants.rs` |
| leave probe | `crates/baylee-engine/src/engine/leave_probe_tests.rs` (`#[cfg(test)]`) |
| their own tests | `crates/baylee-engine/src/engine/verification_tests.rs`, `invariants::tests`, `baylee_cards::mutate::tests`, `leave_probe_tests::representative_cards_leave_cleanly_by_every_route` |

The first two variables are read once per process, at the first card lookup
or the first firing; `BAYLEE_LEAVE_LOG` once, by the sweep, when it has
finished. An empty value is the same as unset. None is read at
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
| no   | the engine knows and refreshes at the top of its driving loop; `apply` returned before that (a question asked in the middle of a resolution; the mulligan window refreshes as it opens and after every answer). A reader of the state in between still reads the old value |

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
something only a seeded board has.

The fourth run, over the tree merged with `c42/night-decks`: `false` 118
times in 10 tests, in three classes, each a defect in the engine and each
fixed since:

- 100 in `refusal_tests::a_dual_land_listed_for_the_lanterns_grant_is_taken_when_pressed`,
  with the generation current: under Maskwood Nexus, creature cards drawn
  into a hand kept the printed subtypes a move clears a cache to. A move
  invalidated only when it touched the battlefield, the stack, a graveyard
  or exile, and a draw touches none. While a cross-zone effect is registered
  every move now invalidates (`GameState::move_object`; test
  `combo_tests::filters::the_nexus_makes_a_drawn_creature_card_every_type`).
  The other moves between hidden zones go through `move_object` as well: a
  tutor, a wish, a card put back on a library, a mulligan's hand, a
  commander put in the command zone.
- 2 in `refusal_tests::a_refused_answer_changes_nothing_and_every_question_has_an_answer`,
  with the generation current: a Pyrogoyf kept the size a departed player's
  graveyard gave it. `sba::eliminate_player` takes the leaver's objects out
  without `move_object`, and now invalidates the projection itself (test
  `leave_tests::a_pyrogoyf_shrinks_as_the_graveyard_it_counted_leaves_the_game`).
- 16 in 8 tests, each with a refresh due and in the mulligan window: the
  third run's class. Not the harness's alone. A starting battlefield is
  dealt outside the tests too, by a `dev-table` gateway's
  `BAYLEE_DEV_SEAT_BOARD` and the client's `dev-control` board (both
  `baylee_cards::decks::deal_named`), and a hosted engine gets it in its
  game setup; every seat is shown its hand and the board with its mulligan
  question. `Engine::new` now ends with a refresh, and so does every answer
  in the window (`settle_mulligans`; test
  `house_rules_tests::a_hand_is_shown_as_it_projects_while_the_mulligans_are_open`).

The fifth run, after those three: no `false` at all. Putting the departed
graveyard's defect back made the probe report the one `false` its test
names, so the probe was running.

## L4: the leave probe

Does every ability of a permanent stop when the permanent is no longer in
play, and follow it when it changes sides? The probe takes each card off
the battlefield along seven routes, and an Aura, Equipment or Fortification
along an eighth, and reads what is left of it.

### Running it

```bash
BAYLEE_LEAVE_LOG=/abs/path/to/dir cargo test -p baylee-engine --lib -- --ignored --exact engine::leave_probe_tests::leave_probe_sweep --nocapture
```

The sweep is `#[ignore]`d and asserts nothing about the pool. Without the
variable it still probes and prints, and writes nothing. With it, the
directory is created if missing and `leave.jsonl` in it is overwritten. It
prints one totals line and one `walk:` line per thin trigger walk (see
[The trigger walk](#the-trigger-walk)). A full sweep runs one thread per core
and took 8 to 22 s here (2026-09-29, 2013 cards). It silences the panic hook
while it runs, since a panic is a `skipped` line: run it with `--exact`, as
above, so that no other test shares the process.

The gate runs `leave_probe_tests::representative_cards_leave_cleanly_by_every_route`
instead: seven cards, every route `ok`, every walk full. They are Glorious
Anthem (an anthem), Doubling Season (replacement effects), Soul Warden (a
triggered ability), Exploration (a rules static; the pool has no cost
reducer, see the test's comment), Kongming, "Sleeping Dragon" (a static
whose "you" must follow its controller), and Holy Strength and Bonesplitter
(an Aura and an Equipment, for `phase_host`).

### What it writes

`<dir>/leave.jsonl`, one line per (card, route), sorted by card, then by
route in the order of the table below; `phase_host` only for a card whose
front face is an Aura, Equipment or Fortification. Fields in this order:

```json
{"card": 111, "route": "phase_out", "verdict": "ok", "detail": ""}
```

| field     | type   | meaning |
|-----------|--------|---------|
| `card`    | u32    | the card's `CardIndex` |
| `route`   | string | one of `destroy`, `exile`, `bounce`, `library`, `sacrifice`, `phase_out`, `phase_host`, `control` |
| `verdict` | string | one of `ok`, `lingers`, `stale`, `skipped` |
| `detail`  | string | empty exactly when `ok`; otherwise the findings, joined by `; `, cut at 400 bytes and ended with `…` if longer |

| verdict   | meaning |
|-----------|---------|
| `ok`      | every check below passed |
| `lingers` | some part of the card was still in force, offered or triggering where it must not be; wins over `stale` |
| `stale`   | only `Engine::projection_is_fresh` failed, after settling or at the end ("already stale before the route" when the board was stale before anything left) |
| `skipped` | the card could not be set up, or the route did not happen to it: `setup: …` (it was not on the battlefield under seat 0 when the walk reached seat 0's main phase, an Aura with nothing to enchant), `did not leave the battlefield` (indestructible), `came back to the battlefield` (persist, undying), `did not phase out`, `its host did not phase out` (the host was gone before the route: Skullclamp's +1/-1 kills its 1/1), `control did not change`, `the card's object is gone`, `route: …` (the route could not be run: its effect asked a question, seat 1 had nothing left to be its source, or the card is attached to nothing), `unaffected: …` (the check's own control failed), or `panic: …`; never a panic of the sweep |

### The cards and the board

Every card whose coverage is `Implemented` and whose front face is a
permanent (`testkit::is_permanent`: land, creature, artifact, enchantment,
planeswalker). Only the front face is probed.

A two-player game at seed 4211, seat 0 going first. Each seat has a
Llanowar Elves, a Forest and one other basic land on the battlefield, and a
Llanowar Elves and a Forest in hand; the card is on seat 0's battlefield. An
Aura enchants the first of seat 0's Elf, seat 1's Elf and the lands that its
spell's `AttachSelf` target allows; an Equipment is attached to seat 0's
Elf; a planeswalker gets its printed loyalty (#304). The kit then walks to
seat 0's first main phase, answering every question with its defaults.

### The routes

Each route is one effect resolved by `resolve::run`, the function every
resolving spell and ability runs, and the engine then settles by itself (a
priority pass: statics synced, state-based actions, triggers put on the
stack and resolved with the kit's answers).

| route       | effect                                              | controlled by |
|-------------|-----------------------------------------------------|---------------|
| `destroy`   | `Effect::Destroy`, the card its target              | seat 1        |
| `exile`     | `Effect::Exile`                                     | seat 1        |
| `bounce`    | `Effect::ReturnToHand`, to its owner's hand         | seat 1        |
| `library`   | `Effect::PutSourceOnTopOfLibrary`                   | seat 0        |
| `sacrifice` | `Effect::SacrificeSelf`                             | seat 0        |
| `phase_out` | `Effect::PhaseOut` (CR 702.26)                      | seat 0        |
| `phase_host`| `Effect::PhaseOut` on the permanent the card is attached to (CR 702.26g) | seat 0 |
| `control`   | `Effect::ChangeController`: the layer-2 effect of `resolve::gain_control` | seat 1 |

### The checks

- **Residue.** No effect from the card with origin `Static` is in force, and
  no replacement rule from it is registered. After a leaving route (the
  first five) also no effect of any origin lasting while its source is on
  the battlefield (CR 611.2b). An effect a resolution made that lasts a turn
  is left alone (CR 611.2a), as are one-shot results and leave-the-battlefield
  triggers. For `phase_out` and `phase_host` a resolution's effect is left
  alone too: CR 702.26f ends only a "for as long as" duration that tracks
  the permanent, and the table does not tell one from an indefinite effect
  on the permanent itself (`docs/engine-internals.md` §"Phasing").
- **Activation.** Seat 0 is offered none of the card's battlefield abilities
  (`compute_legal`), mana abilities included; an ability that works from
  where the card now is (cycling from a hand) is not counted.
- **Triggers.** None of the card's own triggered abilities triggers during
  the trigger walk. For the two phasing routes the window closes when it
  phases in; for `control` an ability that triggers under seat 1 is fine,
  one under seat 0 is not.
- **Fresh.** `Engine::projection_is_fresh` after settling and after the walk.
- **Unaffected** (the two phasing routes). While the card is phased out,
  seat 1 resolves `CreateContinuousEffect`: every permanent gets +0/+0 for
  the rest of the game. The effect fixes its set as it begins (CR 611.2c),
  one entry per object, and the card must not be in it (CR 702.26e). A
  bystander of seat 1's that is phased in must be, or the line is skipped.
  +0/+0 changes no number the rest of the probe reads.
- **`phase_out`, after.** At seat 0's next untap step the card phases in,
  and its unconditional statics and its replacement rules are back. A card
  that phased in and then left by its own rules (a Saga's last chapter, an
  upkeep's "sacrifice unless") is checked as a leaving route instead.
- **`phase_host`.** Right after the route the card is phased out with its
  host (CR 702.26g). After the walk it phased in no sooner than its host did
  (it does not phase in by itself), is still attached to it (CR 702.26d),
  and everything `phase_out` asks after holds. The host is phased in at its
  controller's untap step: seat 0's for an Equipment, seat 1's for an Aura
  on seat 1's Elf.
- **`control`.** Every static and replacement of the card answers to seat 1
  (CR 109.5). For a card attached to nothing, what its statics do to the two
  sides' identical bystanders is swapped: what seat 0's got before, seat 1's
  gets now, and the other way round.

### The trigger walk

From the route to seat 0's first main phase two turns later (seat 1's turn,
then seat 0's):

- both turns begin every step: upkeep, draw (a card is drawn), beginning of
  combat, end step;
- each active player plays a land (a land played, a land entering), taps a
  Forest for mana and casts Llanowar Elves (a spell cast, a creature
  entering), once per turn;
- seat 1 attacks with everything it may, at a player (attackers declared,
  combat damage to a player);
- in seat 1's second main phase both original Elves are destroyed (a
  creature dies on each side), except the card's host for an Aura or an
  Equipment, which is spared: a host that dies takes a phased-out Aura with
  it when the Aura phases in (CR 702.26i, 704.5m), and that says nothing
  about the Aura's statics.

A trigger on anything else (a noncreature spell, a target, counters, a
discard, a token, life gained as such, a later turn) is not exercised.

A walk is full at 2 lands played, 2 spells cast, 4 permanents entering,
1 creature dying, 1 attack, 2 upkeeps, 2 draws and 2 end steps. A thinner
walk (the card's own rules can stop a cast or an attack: The Tabernacle at
Pendrell Vale, Night of Souls' Betrayal) is still judged on what happened,
and the sweep prints it. Every question on the way is answered with the
kit's `answer_one`; an answer the engine refuses stops the walk short.
Until `28698c03` the engine refused Metamorphosis Fanatic's miracle "yes"
after spending the offer, and the probe carried on past it; that "yes" is
now taken and a cast it cannot pay is reversed, and the sweep after the
merge stopped no walk short on a refused answer.

### A run

2026-09-29, 2013 cards × 7 routes = 14091 lines, this probe both times;
"before" is the engine at `9133d843`, before the phasing fixes.

| | ok | lingers | stale | skipped |
|---|---|---|---|---|
| before the phasing fixes | 12618 | 1242 | 0 | 231 |
| after | 13860 | 0 | 0 | 231 |

Every `lingers` line was `phase_out`; the other six routes were clean on
the old engine too. A phased-out permanent's abilities were offered (1054
cards, 26 of them only a land's intrinsic mana), its statics kept applying
(167) and so did its replacement rules (8), and its triggered abilities
triggered (43); two of the lines also found the projection stale. The
fixes are in `docs/engine-internals.md` §"Phasing". The 231 skipped lines
are 28 cards not on the battlefield once set up (×7: copies that copied
nothing, Walking Ballista with no counters, Auras that kill their 1/1 host,
upkeep costs the kit declines), one Aura with nothing to enchant (Inertia
Bubble, ×7), 18 indestructible cards (`destroy`) and 5 with persist or
undying (`destroy`, `sacrifice`). 31 walks were thin, none stopped short.

A second run the same day, with `phase_host`, the unaffected check and the
engine's indirect phasing (`docs/engine-internals.md` §"Phasing"): 2013
cards, 14158 lines, 67 of them `phase_host`; 13918 `ok`, 0 `lingers`, 0
`stale`, 240 skipped. The 9 new skips are `phase_host` lines: 7 of the 28
cards not on the battlefield once set up, Inertia Bubble, and Skullclamp,
whose Elf died before the route. 31 walks were thin, none stopped short.

### Proving it can fail

Two faults were put into the engine by hand and taken out again
(2026-09-29), each with the representative test as the only judge:

- The departure sweep in `sync_static_effects` removed nothing, so a static
  outlived its source: the test failed with 15 `lingers` lines, the five
  leaving routes of the three cards with statics (`static ModifyPT still
  applies after it left; … a turn cycle later`, and `ExtraLandDrops` for
  Exploration).
- `EffectTable::follow_phasing` parked nothing: 3 `lingers` lines, the
  `phase_out` route of the same three cards (`static … still applies while
  phased out`).

Three more for the phasing checks, the rule tests in `phasing_tests` run
alongside:

- `GameState::phase_out` took nothing attached along: 2 `lingers` lines,
  Holy Strength's and Bonesplitter's `phase_host` (`stayed phased in when
  the permanent it is attached to phased out`).
- The untap step phased in what had phased out indirectly: the same 2
  lines, `phased in by itself, before the permanent it phased out with`.
- `bound_now` walked the raw battlefield again: 9 `lingers` lines, every
  phasing route of the seven cards (`an effect that began while it was
  phased out took it into its set`).

## Not in a shipped build

| hook | compiled when |
|------|---------------|
| recorder | `#[cfg(test)]` in `baylee-engine`: only its own test binary |
| mutation switch | `cfg(any(test, feature = "mutate"))` in `baylee-cards`; only `baylee-engine`'s `[dev-dependencies]` turn `mutate` on |
| projection check | `cfg(any(test, feature = "fuzz"))` in `baylee-engine`; nothing in the workspace turns `fuzz` on |
| leave probe | `#[cfg(test)]` in `baylee-engine`: only its own test binary |

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
