//! Defense Grid — {2} — Artifact
//! Oracle: Each spell costs {3} more to cast except during its controller's turn.
//! Set: 9ED #293 — Ninth Edition | Scryfall ID: f939dac5-ad71-4163-adde-4f435412477c | Oracle ID: 22ba474e-f6e6-450d-a4cf-0739f0b51079
// IMPLEMENTED — taxes every spell whose controller is not the active player.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DEFENSE_GRID,
    oracle_id = "22ba474e-f6e6-450d-a4cf-0739f0b51079",
    scryfall_id = "f939dac5-ad71-4163-adde-4f435412477c",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Defense Grid",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[static_ability!(
        Filter::Not(&Filter::ControlledByActivePlayer),
        Modifier::SpellsCostMore(3),
    )],
);
