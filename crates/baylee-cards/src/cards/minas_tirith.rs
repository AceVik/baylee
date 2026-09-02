//! Minas Tirith — (no cost) — Legendary Land
//! Oracle: Minas Tirith enters tapped unless you control a legendary creature.
//! Oracle: {T}: Add {W}.
//! Oracle: {1}{W}, {T}: Draw a card. Activate only if you attacked with two or more creatures this turn.
//! Set: LTR #256 — The Lord of the Rings: Tales of Middle-earth | Scryfall ID: b38b6760-616f-4b11-8ce7-ac1223c7fd53 | Oracle ID: 7b0d7e62-0287-454a-8702-b0bfa7b41245
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 759,
    oracle_id: "7b0d7e62-0287-454a-8702-b0bfa7b41245",
    scryfall_id: "b38b6760-616f-4b11-8ce7-ac1223c7fd53",
    color_identity: ColorSet::from_slice(&[Color::White]),
    faces: &[
    face! {
        name: "Minas Tirith",
        types: TypeSet::LAND,
        supertypes: SupertypeSet::LEGENDARY,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
