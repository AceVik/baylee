# Card DSL Cookbook

**Status: frozen at M2.S8 — this is the authoring contract for LLM card
implementation batches.** Until then it tracks `baylee-cards-dsl`.

## Per-card file standard

One file per card in `crates/baylee-cards/src/cards/`, generated stub +
manual implementation. Mandatory header (human-verifiable):

```rust
//! <Name> — <mana cost> — <type line>
//! Oracle: <exact oracle text>
//! Types: ... | Subtypes: ... | Colors: ... | CMC: ...
//! Set: <SET> #<number> | Scryfall ID: <uuid> | Oracle ID: <uuid>
```

## Rules for authors (human or LLM)

1. Every oracle sentence maps to an ability/effect or an explicit
   `// NOT SUPPORTED: <reason>` flag.
2. Declare layer + duration explicitly for continuous effects; the
   framework deregisters them — never hand-roll removal.
3. Use the filter/cost/effect algebra; reach for `custom()` only when no
   primitive fits, and always declare a legality probe.
4. Every card ships with `#[cfg(test)] mod tests`: resolution, targeting,
   edge cases, and one interaction test per non-trivial mechanic.
5. If the DSL cannot express the card: stop, flag it, do not hack around.
