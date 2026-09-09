//! Fellwar Stone — {2} — Artifact
//! Oracle: {T}: Add one mana of any color that a land an opponent controls could produce.
//! Set: MBC #74 — Mystery Booster Commander Edition | Scryfall ID: 3ef87948-0ad9-4757-a692-2262c8e24367 | Oracle ID: 95560508-7ac9-4be9-8a3f-3c7d5b52807b
// IMPLEMENTED — `ManaSource::LandColor { mine: false }`: the colours are read
// off the *opponents'* lands on resolution, so a Fellwar Stone facing nothing
// but Wastes produces nothing. The client's mana planner counts it for zero
// for the same reason: `simple_mana` refuses a `LandColor` source outright
// (`baylee-cards-dsl/src/manaread.rs`), because what it makes is a fact about
// the rest of the board and not about this card.

use baylee_cards_dsl::prelude::*;

card! {
    index: 1350,
    oracle_id: "95560508-7ac9-4be9-8a3f-3c7d5b52807b",
    scryfall_id: "3ef87948-0ad9-4757-a692-2262c8e24367",
    coverage: Coverage::Implemented,
    faces: &[
    face! {
        name: "Fellwar Stone",
        mana_cost: baylee_core::mana!("{2}"),
        types: TypeSet::ARTIFACT,
    },
    ],
    abilities: &[mana_ability!(&[Effect::mana_land_color(false)])],
}
