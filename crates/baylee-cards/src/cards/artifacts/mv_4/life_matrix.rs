//! Life Matrix — {4} — Artifact
//! Oracle: {4}, {T}: Put a matrix counter on target creature and that creature gains "Remove a matrix counter from this creature: Regenerate this creature." Activate only during your upkeep.
//! Set: LEG #284 — Legends | Scryfall ID: c99a3abc-e2a3-4eee-8f72-b1b25dcd1d0b | Oracle ID: 0fd1ddc3-65cc-489f-81b9-6c144843fc67
// PARTIAL — the whole ability is off the card: the grant to the target and
// the matrix counter are not expressible, see the NOT SUPPORTED line above
// `abilities`.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LIFE_MATRIX,
    oracle_id = "0fd1ddc3-65cc-489f-81b9-6c144843fc67",
    scryfall_id = "c99a3abc-e2a3-4eee-8f72-b1b25dcd1d0b",
    faces = &[face!(
        name = "Life Matrix",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Partial(
        "no effect grants an activated ability to the effect's target \
         (`Modifier::GrantActivated` rides `Effect::continuous`, whose Filter \
         names board objects and never the effect's target; `Effect::PumpTarget` \
         carries keywords only), and the matrix counter has no assigned \
         `counters` id"
    ),
    // NOT SUPPORTED: "{4}, {T}: Put a matrix counter on target creature and
    // that creature gains \"Remove a matrix counter from this creature:
    // Regenerate this creature.\" Activate only during your upkeep." — the
    // ability's target, the counter once it exists, the upkeep window
    // (`Condition::All(&[YourTurn, DuringStep(StepKind::Upkeep)])`) and the
    // regenerate are each sayable, and `Modifier::GrantActivated` carries the
    // quoted ability with `cost!(RemoveCounterSelf { kind, n: 1 })`. But the
    // grant reaches the battlefield through `Effect::continuous`, whose
    // `Filter` names characteristics (`Filter::This` is the artifact itself),
    // and no `Filter` names the effect's target; `Effect::PumpTarget` applies
    // to the target but carries a `KeywordSet` and no ability. "Matrix" also
    // has no assigned `counters` id — ids live in `baylee_cards_dsl::counters`,
    // outside this file, and a bare `CounterKind::Custom(n)` is the collision
    // the registry exists to prevent.
);
