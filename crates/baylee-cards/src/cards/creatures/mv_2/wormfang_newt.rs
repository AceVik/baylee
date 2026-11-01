//! Wormfang Newt — {1}{U} — Creature — Nightmare Salamander Beast
//! Oracle: When this creature enters, exile a land you control.
//! Oracle: When this creature leaves the battlefield, return the exiled card to the battlefield under its owner's control.
//! Set: JUD #59 — Judgment | Scryfall ID: df8012c1-76ec-4c36-8b38-5bc41ce5e156 | Oracle ID: be014a79-5550-4f79-8789-b4d6783d7ec8
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WORMFANG_NEWT,
    oracle_id = "be014a79-5550-4f79-8789-b4d6783d7ec8",
    scryfall_id = "df8012c1-76ec-4c36-8b38-5bc41ce5e156",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Wormfang Newt",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::NIGHTMARE,
            subtypes::creature::SALAMANDER,
            subtypes::creature::BEAST
        ],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
