//! Breaking Wave — {2}{U}{U} — Sorcery
//! Oracle: You may cast this spell as though it had flash if you pay {2} more to cast it. (You may cast it any time you could cast an instant.)
//! Oracle: Simultaneously untap all tapped creatures and tap all untapped creatures.
//! Set: INV #48 — Invasion | Scryfall ID: 1b39cd77-97aa-4099-8405-366f82079758 | Oracle ID: f455b731-6ff9-4acb-9447-f524faa49b27
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BREAKING_WAVE,
    oracle_id = "f455b731-6ff9-4acb-9447-f524faa49b27",
    scryfall_id = "1b39cd77-97aa-4099-8405-366f82079758",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Breaking Wave",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
