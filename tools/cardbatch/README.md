# tools/cardbatch — cards implemented by a model, one at a time

792 of the 1344 card files are still `// GENERATED STUB`. The two readers in
`baylee-cards-codegen` have already taken everything they can read *in full*;
what is left is the part that needs a reader who can be wrong. So this is
built around that: a model produces volume, and nothing it produces reaches
`main` without being read.

## Running it

```bash
cargo run -p xtask -- card-batch --out target/card-batch   # every stub, one package each
tools/cardbatch/run.sh target/card-batch 20                # twenty of them
tools/cardbatch/run.sh target/card-batch 20 gemini-3.1-pro-high   # a harder batch
```

`agy models` lists what the account can reach. The suffix is reasoning effort,
not a different model.

## What a package is

One directory per card, written by `xtask card-batch`:

| File | What it is |
|---|---|
| `STUB.rs` | the generated stub, as a reference copy |
| `FORGE.txt` | the forge-reference script — rules ground truth, read never copied |
| `SCRYFALL.json` | printing metadata |
| `EXEMPLAR.rs` | an implemented card of the same type, to match in style |
| `PROMPT.md` | the instructions, pointing at all of the above |

The agent works **in the repository**, not on pasted text: it reads
`crates/baylee-cards/AGENTS.md` and `docs/card-dsl.md` itself and edits the
card file in place. `SCRYFALL.json` alone would otherwise dominate the budget.

## What the driver guarantees

**One commit per card.** A bad card has to be removable on its own. A night's
work as one commit makes reverting one card a revert of the night.

**Nothing is kept that does not pass the narrow gate** — `cargo check -p
baylee-cards`, `cargo test -p baylee-cards`, `xtask validate`. The full
workspace gate runs once on the branch, before a merge; per card it would turn
a night into a week.

**A card that touched anything but its own file is reverted**, even when it
compiles. Otherwise a DSL change nobody read arrives inside a card commit.

**A verdict that cannot be parsed counts as a refusal.** An unreadable answer
is not evidence that a card is good.

## What the driver does *not* guarantee

The gate does not check whether the card is **right**. A wrong `Filter` or
`TargetSpec` compiles cleanly and passes the data tests — that has already
happened here once. Every batch therefore still needs, by hand:

- `Filter`, `TargetSpec` and `Amount` read on a sample of the batch;
- one *played* engine test for every entry a verdict listed under
  `new_mechanics`;
- `cargo run -p xtask -- pool-dump` before and after, diffed, to prove no
  card that was already implemented has changed.

Only then does a slice get merged.

## The refusal file

`data/card-refusals.tsv`, appended to as cards are refused. Two of its columns
are the reason it exists:

- **`cannot_say`** — what the DSL cannot *express*, in the DSL's own
  vocabulary. "No `Effect` variant names the target of the ability being
  resolved" is an answer. "Pump is not supported" is not.
- **`nearest_existing`** — the closest variant that *does* exist, and what it
  falls short of.

The second column is there to make a particular mistake hard. When `Pump` was
the top blocker in `forge-report`, it was read as a missing subsystem; the
subsystem was already complete, and what was missing was one variant that
could say "the target" without overloading a `Filter` to mean it. A refusal
list that names absent mechanics sends the next round of engine work in the
wrong direction, which is worse than no list.

Refusals are the input to extending the DSL — the standing instruction here is
that an inexpressible card is a gap in the engine, not a card to skip.
