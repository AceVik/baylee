//! Mystic Gate — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {W/U}, {T}: Add {W}{W}, {W}{U}, or {U}{U}.
//! {1}, {T}: Add two mana in any combination of {White} and/or {Blue}.
//! Set: CMM #1013 — Commander Masters | Scryfall ID: 6f99714f-43bc-4048-b650-97dfef4c10fe | Oracle ID: e9f5feb2-2c1a-46ce-885a-4f378d7d10af
// IMPLEMENTED — filter land (colorless tap + {1},{T} for two combination mana).

use baylee_cards_dsl::prelude::*;

card!(
    index = 101,
    oracle_id = "e9f5feb2-2c1a-46ce-885a-4f378d7d10af",
    scryfall_id = "6f99714f-43bc-4048-b650-97dfef4c10fe",
    faces = &[face!(name = "Mystic Gate", types = TypeSet::LAND,)],
    color_identity = ColorSet::from_slice(&[Color::White, Color::Blue]),
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(
            cost!("{1}", TapSelf),
            &[Effect::mana_combination(
                &[ManaColor::White, ManaColor::Blue],
                Amount::Fixed(2),
            )]
        ),
    ],
);
