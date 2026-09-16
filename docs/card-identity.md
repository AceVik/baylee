# What a Card Is Called

**Status: [Implemented]** for every handle described here; the open questions
are marked as such where they come up. This file is normative on card
identity — `docs/card-dsl.md` points here rather than repeating it, and
`CLAUDE.md` summarises it.

A card has four names in this system and they are not interchangeable. Most of
the bugs this document exists to prevent are one of them standing in for
another: a deck saved under a name that later changed, a client asking for a
card by a number that meant something else last week, a stack entry labelled
with a handle that dies when the game does.

## The four names

| | What it is | Who assigns it | What it survives |
|---|---|---|---|
| `CardIndex` | rules identity, a `u32` | `cargo xtask ledger`, once, for good | everything |
| `oracle_id` | Scryfall's rules identity, a UUID string | Scryfall | reprints, errata, renames |
| `scryfall_id` | one printing, a UUID string | Scryfall | nothing — it *is* the printing |
| the printed name | what a player types and a deck line says | Wizards of the Coast | reprints, but **not** a rename |

`CardIndex` is the only one the rules ever see. `baylee-engine` never looks a
card up by name or by any Scryfall id, which is why an engine that has never
heard of Scryfall can run a game. The only card strings it holds at all are
the ones interned behind `NameRef`, below, so that "creatures named X" can be
compared as a number.

It is an **identity and not a position**. The ledger numbers every card there
is — 33 694 of them, in first-appearance order — and this repo compiles 1365
of them, so most indices name a card no `CardDef` exists for yet. The
consequences are the point:

- Implementing an old card **inserts nothing**. Its number was assigned before
  anybody wrote the file.
- The 32 329 `None`s in `generated::BY_INDEX` are the corpus showing through,
  not cards that have left.
- A number, once handed out, is never handed to another card. `DeckEntry`
  carries one into every game, and a `GamePreset` serialized today has to mean
  the same cards when it is read back, so reusing a number would silently
  rewrite stored data.

`oracle_id` is the ledger's **key**: it is what says two rows are the same
card, because it is the one thing Scryfall guarantees across printings. It is
what `xtask ledger` matches on when it re-derives the table, and what
`generated::by_oracle_id` and `xtask cross-read` address a compiled card by.
`scryfall_id` names one piece of cardboard — the *reference printing* codegen
read — and is what the client asks the catalog for printed text with and what
resolves art. It is the wrong key for anything about rules: two copies of a
card with different finishes are the same card.

The printed name is the one identifier outside our control that a **player**
uses, which is why it is the one a deck is stored under.

It also has **two spellings** here, and they are different strings. The ledger
keeps a card's whole name — `Row::name` for 17140 is `Conqueror's Galleon //
Conqueror's Foothold` — and the name table keeps the front face alone, one row
per card, because that is what a deck line says. So a deck line typed the way
the ledger spells it does not resolve. Back faces and the `A // B` form are a
widening of `by_name` nobody has asked for yet, and they are worth knowing
about before someone imports a decklist from a source that writes them.

## Stored by name, played by index, lived by object

This is the layering worth remembering, because each step is a different
crate's problem:

1. **Stored** — a deck is `Vec<String>` in the gateway's database and the
   same lines in an export: `4 Lightning Bolt (M11) 149 [de] *F*`. Text,
   because a deck that round-trips through a file has to be the deck that
   never left (`docs/deck-format.md`).
2. **Played** — `POST /decks` resolves every line with
   `baylee_cards::decks::by_name` and refuses the deck if a line does not
   resolve. What reaches the engine is `DeckEntry { card: CardIndex, print:
   PrintRef }`, and `GamePreset::prints` is the table the second one indexes.
3. **Lived** — every entry becomes an `ObjectId` before the first turn, and
   that handle dies with the game.

So the name → index step happens **once per deck save and once per game
start**, never in a rules loop. That is also the step a Scryfall rename would
break: an index is safe from a rename and a stored name is not. Nothing
guards against that today beyond the rename showing up in the ledger's diff,
which is why `Row::name` is the one field of five the ledger does not call
frozen.

### When a card names another card

Magic prints exactly one sentence in which a card names another card by name:
`Partner with <name>` (CR 702.124j). It is stored as an index like everything
else — `PartnerKind::PartnerWith(CardIndex)` — and the resolution happens at
codegen time, in `IndexLedger::entry_named`, so the printed name never leaves
the generator.

Three reasons, and the first is the one that decided it. The deckbuilder is
where the fact is used, and a `PoolCard` already carries `index`: "may these
two lead one deck" is then one integer compare, rather than a name match that
would have to pick between the card's whole name (`Sheoldred // The True
Scriptures`, which is what the ledger stores) and its front face
(`Sheoldred`, which is what the pool stores and what the printed sentence
says). Second, the ledger numbers **every card there is**, so
`index::TOOTHY_IMAGINARY_FRIEND` resolves whether or not this repo compiles a
`CardDef` for Toothy — a name would have had to survive the same trip
unchecked. Third, a misspelled name compiles and then pairs with nothing, in
silence, for as long as nobody plays that pair; a misspelled constant does
not compile. The generator emitted exactly that bug for as long as the field
was a string: the reminder text rides on the same printed line, so what it
wrote was `PartnerWith("Toothy, Imaginary Friend (When this creature enters,
…)")`.

The lookup is two-tier — whole name, then front face — and refuses on zero
matches **and** on several, which is the whole safety argument for looking a
name up at all: the corpus is append-only, and a set shipping two cards of
one name would otherwise hand one of them the other's index.

## Three lookups, and what `None` means in each

| Lookup | Cost | Answers `None` when |
|---|---|---|
| `baylee_cards::by_index(i)` | O(1), a sparse array | no `CardDef` is compiled at that index (usually: the corpus has the card, this repo does not) |
| `baylee_cards::decks::by_name(s)` | O(1), a perfect hash | no card **in the pool** prints that front-face name |
| `generated::by_oracle_id(s)` | O(log n), a binary search over `ALL` | same as `by_index` |

`by_name` is `crates/baylee-cards/src/generated_names.rs`, a compile-time
perfect hash over the pool's 1365 names: a 512-entry displacement table, a
2048-slot table of `u16` positions, and the names themselves beside the
answer. The arithmetic it shares with its generator is `baylee_core::phf`, and
it is written exactly once because two copies drift **silently** — a table
built with one hash and read with another answers `None` for every key, which
is indistinguishable from a table nobody filled in. The spelling is compared
after the slot is found for the same kind of reason: a perfect hash is perfect
only over its own keys, so every other string lands in some slot too.
Measured, release: 21 ns against the 11.4 µs the linear scan over 1365 cards
took.

The reverse direction needed nothing built — `by_index(i).map(CardDef::name)`
was already O(1) — and the table adds no strings to the binary, because every
name in it was already in `baylee-cards` as a `FaceDef::name`.

**The open question here** is that `by_name` gives one `None` for two
different facts, and the gateway reports both as `unknown card`: a name that
is no card at all, and a name that is a real card this build compiles no
`CardDef` for. Not an *unimplemented* one — a `Coverage::Unimplemented` stub
is a compiled card, has a row in this table and resolves; what is missing is
the 32 329 with no file at all. The ledger knows the difference — it has all
33 694 — so a second lookup in `baylee-cards-index` could say "that card
exists, this build has no `CardDef` for it". It would be a *second function
with different semantics*, not a widening of this one, because the pool is
what `by_name` is asked about everywhere else.

## Who may write what

One writer each, and that is what keeps the numbers still:

- **The ledger** — `cargo run -p xtask -- ledger`, and nothing else, ever by
  hand. It is `crates/baylee-cards-index/src/generated.rs`, one `Row` per
  card, and it *is* the assignment rather than a cache of one; it was
  `data/card-index.tsv` until a data file proved to be a second truth beside
  the code that nothing checks. The assigner **links the crate it writes**,
  reads the table one build old, and appends: a run over yesterday's table
  re-derives exactly the rows that table already has. What it cannot do is
  move one. A card this repo implements that the corpus filter drops is named
  in `data/corpus-keep.tsv` — a hand-kept, additive *input*, not a ledger —
  and is admitted whole, in its own chronological place. The crate is its own
  because the rows carry about 2.7 MB of `oracle_id`s and names the rules
  engine has no use for, and a Cargo feature would not have kept them out of
  it — features unify across a workspace build, a crate the engine does not
  link does not.
- **The constants** — `cargo run -p xtask -- codegen` writes
  `crates/baylee-core/src/generated/index/`, the same assignment as one
  `pub const` per card, one `set_<code>.rs` per first-appearance set, globbed
  into one namespace so a caller writes `index::LIGHTNING_BOLT` and never
  learns which set that was. Every module carries the `set_` prefix because
  three set codes (`2x2`, `40k`, `5dn`) begin with a digit, and a prefix
  applied to only those three is a rule somebody has to remember. Both ways
  such a tree fails are invisible — a module missing from `mod.rs` is merely
  unreachable, two sets exporting one name are a glob ambiguity rustc reports
  at the *use site* — so `mod.rs` ends in a generated test naming one constant
  from every set module.
- **The card file** — `index`, `oracle_id` and `scryfall_id` are generated and
  stay as generated. Codegen does **not** write the ledger: it reads the row a
  card needs and fails loudly if there is none.
- **The name table** — codegen again, from the pool it compiles, which makes
  it two-phase like the ability-line table: a card added by the run that
  writes the table gets its row on the *next* run, and `decks::name_table_tests`
  is red in between. `decks::name_table_tests` is what turns a forgotten second
  run into a build failure, and it is a test rather than `codegen --check`
  because codegen does not run in CI: a runner has no card-script reference,
  so the generator there writes different files than the machine that
  committed them.

A card file **names** its index — `index = index::MOX_OPAL` — and the `card!`
macro takes that field as a `path` rather than a literal, so a bare `11391` is
refused by the matcher before the type system is reached. The reason is the
one this whole page is about: a number says only that somebody typed a number,
and a digit typed wrong names a different card that compiles. The constant is
read out of the ledger row and never re-derived from the name, which is what
the frozen `const` column is for — `render_stub` takes the whole
`LedgerEntry`, so the generator cannot spell a constant the ledger did not
freeze.

## What is not card identity

Four handles sit close enough to be mistaken for it. Three of them are minted
per game and must never be stored:

- **`ObjectId`** — `slot:24 | generation:8`, a generational arena handle. The
  generation is what distinguishes an object from an earlier one that used the
  same slot, which is what a zone change into a hidden zone produces. It means
  nothing in another game.
- **`PrintRef`** — a `u16` index into *this* game's `GamePreset::prints`,
  presentation only; the rules never look inside one. `PrintRef::UNKNOWN` is
  the printing a game does not have, which is what a card the rules conjure
  gets — an index picked out of the air would be some other card's art, and a
  seat shown it would earn a printing out of an opponent's deck.
- **`NameRef`** — a per-game interner (`state::Names`) giving rules identity to
  a *name string*, which is what "creatures named Ondu Cleric" needs. It is
  not for display and it is not stable between games.
- **`AbilityRef`** — the one in this list that **does** survive a game:
  `(CardIndex, index into that card's ability list)`. That is what lets a
  client label a stack entry and what lets an account's standing answer —
  "always yes for this one" — be replayed into every future game. Indices
  count **down from `u32::MAX`** for the card-scoped questions that are not
  entries in the ability list at all (`SPELL`, `ENTERS`, `ADDITIONAL_COST`,
  `MIRACLE`, `UPKEEP_COST`, `SYNTHETIC`, `COMMANDER_ZONE`,
  `COMMANDER_REPLACE`), so a real ability index and a reserved one can never
  collide.

And one that is stored and is **not** stable, which is worth saying plainly:

- **`SubtypeId`** — generated from Scryfall's catalogs, **sorted
  alphabetically and numbered sequentially per kind**. A subtype landing
  before an existing one therefore renumbers every subtype after it, and a
  `SubtypeSet` is serialized on the wire, in `PublicObject::subtypes`. Today
  that is survivable because both ends are built together and `VIEW_VERSION`
  guards the shape, not because the numbers are safe. Giving subtypes a
  first-appearance ledger of their own, the way cards have one, is a queued
  change.

Beyond the rules there is a fourth world that shares none of this: accounts,
decks and games are UUID strings in the gateway's database, minted by
Postgres. Nothing in `baylee-core` knows they exist.

## The rule, in one sentence

A number may leave a process only if something froze it — `CardIndex` and the
`AbilityRef` built on it — and everything else is minted for one game and dies
with it.
