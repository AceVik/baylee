//! King Crab — {4}{U}{U} — Creature — Crab
//! Oracle: {1}{U}, {T}: Put target green creature on top of its owner's library.
//! Set: ULG #34 — Urza's Legacy | Scryfall ID: aedea953-b5f1-4ec7-bd9f-b7827f9d40fe | Oracle ID: 24982bfe-5276-44a8-99a5-43b497ebf51f
// PARTIAL

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const GREEN_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::of(Color::Green)),
]);

card!(
    index = index::KING_CRAB,
    oracle_id = "24982bfe-5276-44a8-99a5-43b497ebf51f",
    scryfall_id = "aedea953-b5f1-4ec7-bd9f-b7827f9d40fe",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "putting target green creature unconditionally on top of its owner's library from the battlefield is not yet a dedicated Effect variant; uses OwnerPutsOnTopOrBottom so its owner puts it on top or bottom of their library",
    ),
    faces = &[face!(
        name = "King Crab",
        mana_cost = mana!("{4}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::CRAB],
        power = Some(4),
        toughness = Some(5),
    ),],
    abilities = &[activated!(
        cost!("{1}{U}", TapSelf),
        &[Effect::OwnerPutsOnTopOrBottom {
            target: TargetSpec::Object(&GREEN_CREATURE),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&GREEN_CREATURE))),
    )],
);
