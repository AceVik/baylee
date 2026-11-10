//! Sphere of Resistance — {2} — Artifact
//! Oracle: Spells cost {1} more to cast.
//! Set: VMA #284 — Vintage Masters | Scryfall ID: 9ea2ae1d-91b4-44a0-b2e6-61839d8f4468 | Oracle ID: 09c96077-3804-4f12-a613-5bebc5e0413f
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SPHERE_OF_RESISTANCE,
    oracle_id = "09c96077-3804-4f12-a613-5bebc5e0413f",
    scryfall_id = "9ea2ae1d-91b4-44a0-b2e6-61839d8f4468",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Sphere of Resistance",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[static_ability!(Filter::Any, Modifier::SpellsCostMore(1))],
);
