//! Soldevi Machinist — {1}{U} — Creature — Human Wizard Artificer
//! Oracle: {T}: Add {C}{C}. Spend this mana only to activate abilities of artifacts.
//! Set: ME4 #63 — Masters Edition IV | Scryfall ID: 87174f2d-bd11-4a40-b1c7-79040be04716 | Oracle ID: 06600efe-1718-4cc1-b1c0-59c82e2efd73

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SOLDEVI_MACHINIST,
    oracle_id = "06600efe-1718-4cc1-b1c0-59c82e2efd73",
    scryfall_id = "87174f2d-bd11-4a40-b1c7-79040be04716",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Soldevi Machinist",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::HUMAN,
            subtypes::creature::WIZARD,
            subtypes::creature::ARTIFICER,
        ],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[
        // CR 605.1a / CR 106.12: {T}: Add {C}{C}. Spend this mana only to activate abilities of artifacts.
        mana_ability!(
            cost!(TapSelf),
            &[Effect::mana(ManaColor::Colorless, 2).restricted_for(
                &Filter::ARTIFACT,
                SpendRider::None,
                ManaUse::Abilities,
            )],
        ),
    ],
);
