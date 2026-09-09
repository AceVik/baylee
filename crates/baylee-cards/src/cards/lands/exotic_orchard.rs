//! Exotic Orchard — (no cost) — Land
//! Oracle: {T}: Add one mana of any color that a land an opponent controls could produce.
//! Set: MBC #79 — Mystery Booster Commander Edition | Scryfall ID: d11c5fe0-1528-4c94-a8cc-42bcab9d7487 | Oracle ID: 27b047e3-0d41-45e2-98e9-9391d7923a1e
// IMPLEMENTED — color choice from opponents' lands' producible mana
// (precomputed on the lands' characteristics at creation).

use baylee_cards_dsl::prelude::*;

card! {
    index: 48,
    oracle_id: "27b047e3-0d41-45e2-98e9-9391d7923a1e",
    scryfall_id: "d11c5fe0-1528-4c94-a8cc-42bcab9d7487",
    faces: &[face! {
        name: "Exotic Orchard",
        types: TypeSet::LAND,
    }],
    coverage: Coverage::Implemented,
    abilities: &[mana_ability!(&[Effect::mana_land_color(false)])],
}
