//! Saproling Symbiosis — {3}{G} — Sorcery
//! Oracle: You may cast this spell as though it had flash if you pay {2} more to cast it. (You may cast it any time you could cast an instant.)
//! Oracle: Create a 1/1 green Saproling creature token for each creature you control.
//! Set: DMR #175 — Dominaria Remastered | Scryfall ID: 64ae774a-c1de-4c9e-a2ec-86d71cb2d1db | Oracle ID: 69d22037-fbf7-44d2-81a7-7ce8e32f06ec
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SAPROLING_SYMBIOSIS,
    oracle_id = "69d22037-fbf7-44d2-81a7-7ce8e32f06ec",
    scryfall_id = "64ae774a-c1de-4c9e-a2ec-86d71cb2d1db",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Saproling Symbiosis",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
