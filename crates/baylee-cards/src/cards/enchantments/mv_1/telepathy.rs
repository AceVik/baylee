//! Telepathy — {U} — Enchantment
//! Oracle: Your opponents play with their hands revealed.
//! Set: USG #102 — Urza's Saga | Scryfall ID: 51729f36-0a0c-47fb-a3bf-22afc78df7a4 | Oracle ID: d2e647ac-717f-46e6-baa8-6a793e7bdc32
// PARTIAL: revealing only opponents' hands is not in Modifier; reveals all players' hands.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TELEPATHY,
    oracle_id = "d2e647ac-717f-46e6-baa8-6a793e7bdc32",
    scryfall_id = "51729f36-0a0c-47fb-a3bf-22afc78df7a4",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "revealing only opponents' hands is not in Modifier; reveals all players' hands",
    ),
    faces = &[face!(
        name = "Telepathy",
        mana_cost = mana!("{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[static_ability!(Filter::Any, Modifier::RevealAllHands),],
);
