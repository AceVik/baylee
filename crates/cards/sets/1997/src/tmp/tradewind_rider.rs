//! Tradewind Rider — {3}{U} — Creature — Spirit
//! Oracle: Flying
//! Oracle: {T}, Tap two untapped creatures you control: Return target permanent to its owner's hand.
//! Set: TPR #75 — Tempest Remastered | Scryfall ID: df626c4c-75f4-4250-9057-c8c383a5a93d | Oracle ID: d0314da0-da24-4f6e-b1f6-0d50617bcaaa

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TRADEWIND_RIDER,
    oracle_id = "d0314da0-da24-4f6e-b1f6-0d50617bcaaa",
    scryfall_id = "df626c4c-75f4-4250-9057-c8c383a5a93d",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Tradewind Rider",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPIRIT],
        power = Some(1),
        toughness = Some(4),
    ),],
    abilities = &[activated!(
        cost!(
            TapSelf,
            TapOther(&Filter::YOUR_CREATURE),
            TapOther(&Filter::YOUR_CREATURE)
        ),
        &[Effect::bounce(TargetSpec::Object(&Filter::Any))],
        target = Some(TargetSpec::Object(&Filter::Any)),
    )],
);
