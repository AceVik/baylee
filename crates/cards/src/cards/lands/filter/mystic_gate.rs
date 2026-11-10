//! Mystic Gate — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {W/U}, {T}: Add {W}{W}, {W}{U}, or {U}{U}.
//! Set: CMM #1013 — Commander Masters | Scryfall ID: 6f99714f-43bc-4048-b650-97dfef4c10fe | Oracle ID: e9f5feb2-2c1a-46ce-885a-4f378d7d10af
// IMPLEMENTED — filter land: a colorless tap, and a hybrid {W/U} that turns
// one mana into two, each of its own colour.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MYSTIC_GATE,
    oracle_id = "e9f5feb2-2c1a-46ce-885a-4f378d7d10af",
    scryfall_id = "6f99714f-43bc-4048-b650-97dfef4c10fe",
    faces = &[face!(name = "Mystic Gate", types = TypeSet::LAND,)],
    color_identity = ColorSet::from_slice(&[Color::White, Color::Blue]),
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(
            cost!("{W/U}", TapSelf),
            &[Effect::mana_combination(
                &[ManaColor::White, ManaColor::Blue],
                Amount::Fixed(2),
            )]
        ),
    ],
);
