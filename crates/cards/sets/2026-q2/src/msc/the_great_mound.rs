//! The Great Mound — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {3}, {T}: Create a tapped Vibranium token. (It's an artifact with indestructible and "{T}: Add {C}. This mana can't be spent to cast a nonartifact spell.")
//! Oracle: {6}, {T}: Draw a card.
//! Set: MSC #120 — Marvel Super Heroes Commander | Scryfall ID: 11b3f96c-138e-431e-a1d6-5ebae4cb6b2f | Oracle ID: 6b78e417-44f8-4ab6-9d3c-e71704fc648e

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::THE_GREAT_MOUND,
    oracle_id = "6b78e417-44f8-4ab6-9d3c-e71704fc648e",
    scryfall_id = "11b3f96c-138e-431e-a1d6-5ebae4cb6b2f",
    faces = &[face!(name = "The Great Mound", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(cost!("{6}", TapSelf), &[Effect::draw(1)]),
        activated!(
            cost!("{3}", TapSelf),
            &[Effect::CreateTokenModified {
                token: &crate::tokens::VIBRANIUM,
                amount: Amount::Fixed(1),
                tapped: true,
                haste: false,
            }]
        ),
    ],
);
