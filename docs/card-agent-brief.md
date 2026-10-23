# Card agent brief

The brief for an agent that implements cards or writes their tests,
whatever model it runs on. It is the contract a card batch is held to and
the defects earlier batches shipped. Read it whole before the first edit.

## 0. Read first, in this order

1. `AGENTS.md`: the map of the repository and what always applies.
2. `docs/cards.md`, all of it: codegen, layout, ownership and writing a
   card. It binds card work; where it and this brief differ, it wins, and
   you say so in your report.
3. `docs/card-dsl.md` §"File standard", §"Where a card's file lives",
   §"Only state what the card prints", §"Coverage", §"Who owns a card
   file" and §"Worked examples". Read the rest of it as the vocabulary, and
   search it before you write anything new: it is a contract, and most
   sentences already have a piece.
4. `docs/card-identity.md` §"Who may write what".
5. The last three sections of `docs/llm-learnings.md`.
6. One finished card and its test as the model to copy:
   `crates/baylee-cards/src/cards/enchantments/auras/mv_1/brainwash.rs` and
   `crates/baylee-engine/src/engine/card_tests/enchantments/auras/mv_1/brainwash.rs`.
   For a pattern the card needs, one of these: an activated ability with a
   composite cost, `lands/fetch/polluted_delta.rs`; a trigger with filter
   algebra, `creatures/mv_2/ondu_cleric.rs`; an alternative (pitch) cost,
   `instants/mv_5/force_of_will.rs`; a modal spell (overload),
   `instants/mv_2/cyclonic_rift.rs`; evoke, `creatures/mv_5/mulldrifter.rs`;
   a static through the layers, `artifacts/mv_4/maskwood_nexus.rs`; trigger
   multiplication, `creatures/mv_5/elesh_norn_mother_of_machines.rs`; a
   clone, `creatures/mv_4/spark_double.rs`; a planeswalker,
   `planeswalkers/mv_4/jace_the_mind_sculptor.rs` (all under
   `crates/baylee-cards/src/cards/`).

## 1. The rules of a card batch

- **Sets go in release order** by first printing, one main release at a
  time (`docs/mechanics-roadmap.md` §E8). Unplayable cards (ante,
  dexterity, subgames, un-sets) go in `data/unplayable.tsv` and are never
  built.
- **The reader first.** A card that codegen can transcode is not
  hand-written: fix `scriptgen` so it reads the sentence, and every card
  that prints it comes out right. Wrong generated output is fixed in the
  reader, never in the card. Hand-write only what the reader cannot reach,
  through `xtask adopt --name "<card>"`.
- **Never run plain `codegen` without the card-script corpus.** It
  rewrites every machine-owned card as a stub. Point `BAYLEE_CARD_SCRIPTS`
  at a clean upstream Card-Forge checkout, never at a fork. After a
  hand-edited card, `codegen --tables` is enough. Codegen is two-phase: a
  full run is run twice.
- **Never copy the corpus** into the repository, not even a line in a
  comment (`docs/legal.md` §"Guardrails for every change").

## 2. Writing a card

- Edit only `coverage`, `keywords` and `abilities`. The header, names,
  costs, types and P/T are generated or checked against Scryfall by
  `validate`. Never hand-type header oracle text; run `xtask refresh-oracle`.
- **A single-faced card writes `abilities` at card level**, in `card!(…)`,
  never inside `face!(…)`. The engine falls back to the card's list either
  way, so the card plays. But the testkit, the lints and every pool walk
  read `def.abilities` and see nothing.
- Never restate a default (CR 117.1b, 113.6, 605.1). Use the macros
  (`card!`, `face!`, `activated!`, `triggered!`, `static_ability!`,
  `cost!`, `mana_ability!`), `field = value` in parentheses.
- Filters: the generic ones on `Filter`, this pool's in
  `crates/baylee-cards/src/filters.rs`. No per-card filter statics.
- **Read every sentence for its subject.** Earlier batches lost exactly
  these words, and every gate passed:
  - "you cast" / "you control" / "your": Derelor taxed every black spell
    instead of "black spells you cast". Soul Exchange and Vodalian War
    Machine took any creature or Merfolk where the card says "you control"
    (`Filter::And(&[Filter::ControlledByYou, …])`).
  - "each opponent" vs "target opponent" vs "defending player".
  - "this turn", "until end of turn", "your next untap step", "at the
    beginning of your upkeep" vs "each upkeep".
  - "target" vs "choose": a target is chosen on the stack (CR 601.2c),
    a choice on resolution.
  - "up to N", "any number", "exactly".
  - Which object a "this way" or "that creature" points back to.
- **If the DSL cannot say a sentence**, the card is
  `Coverage::Partial("<the sentence it gets wrong, and why>")` with a
  `// NOT SUPPORTED:` comment saying the same. Never `Implemented` while
  any printed sentence is played wrong or not at all. Brainwash was
  `Implemented` while its {3} could not be paid with lands.

- **Partial is playable, so it must never gain an unprinted advantage.**
  Never omit a printed cost, restriction, condition, drawback, opposing choice
  or the opponent's half of a symmetric instruction while keeping the benefit.
  Paying an additional cost on resolution is not paying it on casting: it must
  already be paid when a spell is countered. Search the vocabulary first. If
  the exact rule is unavailable, remove an optional positive ability in full,
  or restore the card to a stub with `codegen --hold-back` and record why.
  A blank noncreature, or a 0/0 creature without its missing size rule, is not
  useful playable coverage. A test that casts it and observes nothing does not
  make it useful.

## 3. When the DSL or the engine needs a new piece

Prefer extending the DSL to a `Partial`. A new piece is held to this:

- **It is named for the rule, never for a card.** Earlier batches added
  `Effect::RaidingParty`, `Effect::WandOfIth`, `Effect::ManaClash` and
  `Effect::FlipCoinsDamageTailsRepeatUntilBothHeads`. That is card text
  in the vocabulary, usable by one card. Find the general sentence
  ("flip coins until …", "reveal a card at random from …") and build that,
  or compose what exists.
- **No card text in `baylee-core` or `baylee-engine`.** Abilities are
  `AbilityRef { card, index }`, and the engine knows no card names.
  Dungeons, rooms and similar data belong to the cards crate.
- It gets its own entry in `docs/card-dsl.md`, and every `match` over the
  enum handles it. A `_ =>` arm hides a missed one from the compiler: grep
  for them.
- **An engine change gets a test beside the rule** (an `engine/*_tests.rs`
  or a module test, not a card test) that fails on the old code. Prove it:
  break the change on purpose, see the test go red, restore. Say so in the
  commit.
- **New state that lives for a turn is reset where the turn resets.**
  `PerTurn::reset` names every field without `..`, so a new field does not
  compile until it is reset. `tapped_for_ability` was never cleared, and
  Vodalian War Machine destroyed Merfolk tapped turns earlier.
- Engine and DSL changes go in their own commit, before the cards that use
  them, and the commit message says what rule changed.

## 4. The test of a card

The author writes the tests too, in a pass of their own after the cards
of a batch are committed: decide what a test expects from the oracle text
and this section, not by reopening the card file. A different model reviews
both afterwards (§7).

- **One file per card, at the card's own path:** `cards/<type>/<dir>/<slug>.rs`
  is tested in `crates/baylee-engine/src/engine/card_tests/<type>/<dir>/<slug>.rs`,
  with a `mod` line in the parent module in alphabetical order. Never a
  file per set, never several cards in one file (`combo_tests/` is for
  interactions only).
- **Name the subject with a literal `card_index("<oracle id>")`.** Not
  `index::…`, not a `const`, not a helper's argument: the
  `require-card-tests` hook counts only the literal, and a card it cannot
  see counts as untested.
- **Play every printed sentence, through `apply`, from the pending
  question**, on the shared testkit. A test that only puts the card on the
  battlefield and reads its power, toughness or keywords tests Scryfall,
  which `validate` already does. Don't write it. Earlier batches did, and
  those tests were deleted.
- **Test the card, not its neighbours.** A control that only shows a
  Lightning Bolt killing a 2/2 tests Lightning Bolt. A control belongs in a
  test when without it the card's assertion could pass on a broken
  board. Then make it the first half of the card's own test.
- **Assert the boundaries** the sentence draws: the opponent's creature
  and yours, this turn and the next, one counter and none, the player who
  pays and the one who does not.
- Facts the testkit has bitten people with:
  - A printed mana ability is in `legal.abilities` and pressed with
    `ActivateAbility { source, ability_index }`. `ActivateManaAbility` is
    only a basic land type's intrinsic mana (`legal.mana_abilities`).
  - Targets are chosen before costs are paid (CR 602.2b → 601.2c, 601.2h):
    a sacrificed cost is gone after the target answer, not after
    `ActivateAbility`.
  - A permanent on the dev board still asks its "as this enters" question
    first. Answer it in the fixture.
  - `bury` through `dev_state_mut` does not refresh a conditional static.
    Remove the permanent with a real spell when the condition is the test.
  - The testkit deals no opening hand, and a planeswalker placed on the dev
    board has no loyalty.
  - A mana tax (Brainwash) opens a payment window after the declaration:
    `Pending::Priority` is the window. Tap there, then `PassPriority`.

## 5. Before you say a batch is done

Run, in this order, and read the output yourself:

```bash
cargo fmt --all
cargo dev gate rules
cargo run -p xtask -- codegen --tables
cargo run -p xtask -- validate
cargo tools hooks require-card-tests
```

Then the full gate (`DATABASE_URL=… cargo dev gate`). A test you did
not run is not evidence, and a filtered run is not the suite.

- `validate` prints the hand-owned / machine-owned / stub split. Report it
  before and after.
- After the tests land, run `codegen` once more. It promotes the tested
  `Generated` cards to `Implemented`.
- Add a dated section to `docs/llm-learnings.md`: what the batch found,
  in the cards, the reader, the DSL and the tools.
- Commit only your own files, selectively (`git add <paths>`), never
  `git add -A`. Never stash, reset or switch branches: other sessions
  share the checkout.
- The commit message claims only what the diff does. A "visual UI" that
  is a placeholder is a placeholder.

- Every commit of card work ends with the trailers
  `Card-Author: <model>` and `Review: pending`, beside any
  `Co-Authored-By:` line. A commit that touches only DSL or engine code
  carries them too: the review reads it the same way.

## 6. The report

End with a list a reviewer can check without asking:

- Cards per state: `Implemented`, `Partial` (each with its reason),
  `Unimplemented`, skipped as unplayable.
- Every DSL or engine piece added, with the rule test that proves it and
  the proof it fails without the change.
- Every reader (`scriptgen`) change, and how many cards it moved.
- The gate commands you ran and their results, verbatim, including red
  ones you did not fix.

## 7. The review

A model other than the author reads every `Review: pending` commit after
it lands on the author's branch, before it reaches `main`.

- Find them: `git log --format='%h %s' --grep='^Review: pending'`, minus
  those a later commit names in a `Reviewed:` trailer
  (`git log --format=%B | grep '^Reviewed:'`).
- Read each card sentence by sentence against its oracle text (§2's
  list), each test against §4, each DSL or engine piece against §3.
- Fix in commits of your own, never by amending the author's. Each fix
  names its defect and gets a test that fails without it. A defect in a
  rule rather than a card goes beside the rule (§3).
- Close the review with the trailer `Reviewed: <sha> <sha> …` naming
  every commit read, defects or not. Note in `docs/llm-learnings.md` what
  the author got wrong, so the brief can learn it.
