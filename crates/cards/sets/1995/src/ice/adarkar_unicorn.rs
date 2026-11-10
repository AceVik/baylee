//! Adarkar Unicorn — {1}{W}{W} — Creature — Unicorn
//! Oracle: {T}: Add {U} or {C}{U}. Spend this mana only to pay cumulative upkeep costs.
//! Set: ICE #1 — Ice Age | Scryfall ID: 0ba7526f-dba8-4483-b925-946164fc0ae9 | Oracle ID: 287cd5e8-51e2-42a4-a139-c112ff89706d

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ADARKAR_UNICORN,
    oracle_id = "287cd5e8-51e2-42a4-a139-c112ff89706d",
    scryfall_id = "0ba7526f-dba8-4483-b925-946164fc0ae9",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Adarkar Unicorn",
        mana_cost = mana!("{1}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::UNICORN],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[
        mana_ability!(
            cost!(TapSelf),
            &[Effect::mana(ManaColor::Blue, 1).restricted_for(
                &Filter::Any,
                SpendRider::None,
                ManaUse::CumulativeUpkeep,
            )]
        ),
        mana_ability!(
            cost!(TapSelf),
            &[
                Effect::mana(ManaColor::Colorless, 1).restricted_for(
                    &Filter::Any,
                    SpendRider::None,
                    ManaUse::CumulativeUpkeep,
                ),
                Effect::mana(ManaColor::Blue, 1).restricted_for(
                    &Filter::Any,
                    SpendRider::None,
                    ManaUse::CumulativeUpkeep,
                ),
            ]
        ),
    ],
);
