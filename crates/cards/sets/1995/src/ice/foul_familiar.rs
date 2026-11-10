//! Foul Familiar — {2}{B} — Creature — Spirit
//! Oracle: This creature can't block.
//! Oracle: {B}, Pay 1 life: Return this creature to its owner's hand.
//! Set: ME2 #90 — Masters Edition II | Scryfall ID: 61b54f73-0a7b-4aa4-8792-24b6f8ae35c5 | Oracle ID: 90767f52-fa6d-4ef3-ae54-d8e3881971c1
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FOUL_FAMILIAR,
    oracle_id = "90767f52-fa6d-4ef3-ae54-d8e3881971c1",
    scryfall_id = "61b54f73-0a7b-4aa4-8792-24b6f8ae35c5",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::CANT_BLOCK,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Foul Familiar",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPIRIT],
        power = Some(3),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{B}", PayLife(1)),
        &[Effect::ReturnToHand {
            target: TargetSpec::ThisObject,
        }],
    )],
);
