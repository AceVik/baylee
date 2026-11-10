//! Kjeldoran Outpost — (no cost) — Land
//! Oracle: If this land would enter, sacrifice a Plains instead. If you do, put this land onto the battlefield. If you don't, put it into its owner's graveyard.
//! Oracle: {T}: Add {W}.
//! Oracle: {1}{W}, {T}: Create a 1/1 white Soldier creature token.
//! Set: VMA #301 — Vintage Masters | Scryfall ID: db90c357-26e3-4ab2-a280-0d8d74b95048 | Oracle ID: 8b370db5-dfb9-4ea0-9017-bae3e767b041

use crate::tokens::SOLDIER_1_1_WHITE;
use baylee_cards_dsl::prelude::*;

card!(
    index = index::KJELDORAN_OUTPOST,
    oracle_id = "8b370db5-dfb9-4ea0-9017-bae3e767b041",
    scryfall_id = "db90c357-26e3-4ab2-a280-0d8d74b95048",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Kjeldoran Outpost",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::SacrificeOrGraveyard {
            filter: &Filter::And(&[
                Filter::ControlledByYou,
                Filter::HasSubtype(baylee_core::generated::subtypes::land::PLAINS)
            ]),
            count: 1
        }],
    )],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::White, 1)]),
        activated!(
            cost!("{1}{W}", TapSelf),
            &[Effect::CreateToken {
                token: &SOLDIER_1_1_WHITE,
            }]
        ),
    ],
);
