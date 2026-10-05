//! Voodoo Doll — {6} — Artifact
//! Oracle: At the beginning of your upkeep, put a pin counter on this artifact.
//! Oracle: At the beginning of your end step, if this artifact is untapped, destroy this artifact and it deals damage to you equal to the number of pin counters on it.
//! Oracle: {X}{X}, {T}: This artifact deals damage equal to the number of pin counters on it to any target. X is the number of pin counters on this artifact.
//! Set: ME3 #203 — Masters Edition III | Scryfall ID: c60ea64d-0209-4ca4-bee6-f9eb63784c9e | Oracle ID: 4d330c40-3d72-4528-a254-d036683958d3
// PARTIAL — all three sentences are off the card: pin counters have no
// assigned CounterKind id, and every clause counts or places one.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::VOODOO_DOLL,
    oracle_id = "4d330c40-3d72-4528-a254-d036683958d3",
    scryfall_id = "c60ea64d-0209-4ca4-bee6-f9eb63784c9e",
    faces = &[face!(
        name = "Voodoo Doll",
        mana_cost = mana!("{6}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Partial(
        "pin counters have no assigned CounterKind id (`baylee_cards_dsl::counters` \
         has no PIN and a card file may not spell a raw custom-counter id), so \
         the upkeep trigger, the end-step trigger and the damage ability are \
         all off the card"
    ),
    // NOT SUPPORTED: "At the beginning of your upkeep, put a pin counter on
    // this artifact." — a custom counter's id is assigned in
    // `baylee_cards_dsl::counters`, which has no `PIN`, and a card file may
    // not write a bare `CounterKind::Custom(n)` itself.
    // NOT SUPPORTED: "At the beginning of your end step, if this artifact is
    // untapped, destroy this artifact and it deals damage to you equal to the
    // number of pin counters on it." — pin counters as above; the trigger,
    // its `Condition::SourceMatches(&Filter::Untapped)` gate, the
    // `Effect::destroy(TargetSpec::ThisObject)` and the damage to you are all
    // sayable, but there is no pin counter to count.
    // NOT SUPPORTED: "{X}{X}, {T}: This artifact deals damage equal to the
    // number of pin counters on it to any target. X is the number of pin
    // counters on this artifact." — pin counters as above.
);
