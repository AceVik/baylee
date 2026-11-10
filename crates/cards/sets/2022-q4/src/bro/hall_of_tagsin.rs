//! Hall of Tagsin — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: Add one mana of any color.
//! Oracle: {4}, {T}: Create a tapped Powerstone token. (It's an artifact with "{T}: Add {C}. This mana can't be spent to cast a nonartifact spell.")
//! Set: BRO #263 — The Brothers' War | Scryfall ID: a8007012-39c5-4247-ba77-1cfcaade37fa | Oracle ID: fd809587-773c-47ec-8676-cefef6d1e38f

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HALL_OF_TAGSIN,
    oracle_id = "fd809587-773c-47ec-8676-cefef6d1e38f",
    scryfall_id = "a8007012-39c5-4247-ba77-1cfcaade37fa",
    faces = &[face!(name = "Hall of Tagsin", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(cost!("{1}", TapSelf), &[Effect::mana_of_any_color()]),
        activated!(
            cost!("{4}", TapSelf),
            &[Effect::CreateTokenModified {
                token: &crate::tokens::POWERSTONE,
                amount: Amount::Fixed(1),
                tapped: true,
                haste: false,
            }]
        ),
    ],
);
