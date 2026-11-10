//! Churning Eddy — {3}{U} — Sorcery
//! Oracle: Return target creature and target land to their owners' hands.
//! Set: TOR #32 — Torment | Scryfall ID: bd28bc8e-842f-4788-92a0-3019e3c2385f | Oracle ID: 774e53b2-5aec-43e1-b4fe-8812b8430401
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CHURNING_EDDY,
    oracle_id = "774e53b2-5aec-43e1-b4fe-8812b8430401",
    scryfall_id = "bd28bc8e-842f-4788-92a0-3019e3c2385f",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Churning Eddy",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
