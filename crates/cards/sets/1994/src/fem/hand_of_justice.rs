//! Hand of Justice — {5}{W} — Creature — Avatar
//! Oracle: {T}, Tap three untapped white creatures you control: Destroy target creature.
//! Set: ME1 #15 — Masters Edition | Scryfall ID: 7f371a5d-0e79-46d9-ae6c-a9eada48cd55 | Oracle ID: af8eb20a-dfa2-49a4-892b-26d7e276fcd2
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const UNTAPPED_WHITE_CREATURE_YOU_CONTROL: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::White])),
    Filter::Untapped,
    Filter::ControlledByYou,
]);

card!(
    index = index::HAND_OF_JUSTICE,
    oracle_id = "af8eb20a-dfa2-49a4-892b-26d7e276fcd2",
    scryfall_id = "7f371a5d-0e79-46d9-ae6c-a9eada48cd55",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Hand of Justice",
        mana_cost = mana!("{5}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::AVATAR],
        power = Some(2),
        toughness = Some(6),
    ),],
    abilities = &[activated!(
        cost!(
            TapSelf,
            TapN {
                filter: &UNTAPPED_WHITE_CREATURE_YOU_CONTROL,
                count: 3,
            }
        ),
        &[Effect::destroy(TargetSpec::Object(&Filter::CREATURE))],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
    ),],
);
