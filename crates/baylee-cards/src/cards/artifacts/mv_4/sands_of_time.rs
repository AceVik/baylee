//! Sands of Time — {4} — Artifact
//! Oracle: Each player skips their untap step.
//! Oracle: At the beginning of each player's upkeep, that player simultaneously untaps each tapped artifact, creature, and land they control and taps each untapped artifact, creature, and land they control.
//! Set: VIS #153 — Visions | Scryfall ID: a782ee95-bde4-41f4-a947-b073cc4c1e7c | Oracle ID: c8f69b48-a3b2-4324-852d-11fad19d6432
// PARTIAL: each player skips their untap step; simultaneously untapping tapped artifacts, creatures, and lands and tapping untapped ones on upkeep is not yet supported.

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: At the beginning of each player's upkeep, that player simultaneously untaps each tapped artifact, creature, and land they control and taps each untapped artifact, creature, and land they control.
card!(
    index = index::SANDS_OF_TIME,
    oracle_id = "c8f69b48-a3b2-4324-852d-11fad19d6432",
    scryfall_id = "a782ee95-bde4-41f4-a947-b073cc4c1e7c",
    coverage = Coverage::Partial(
        "each player skips their untap step; simultaneously untapping tapped artifacts, creatures, and lands and tapping untapped ones on upkeep is not yet supported"
    ),
    faces = &[face!(
        name = "Sands of Time",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[static_ability!(
        Filter::Any,
        Modifier::SkipUntapStep {
            who: PlayerRel::EachPlayer,
        }
    )],
);
