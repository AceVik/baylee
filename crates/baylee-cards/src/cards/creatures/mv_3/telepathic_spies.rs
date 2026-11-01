//! Telepathic Spies — {2}{U} — Creature — Human Wizard
//! Oracle: When this creature enters, look at target opponent's hand.
//! Set: 7ED #101 — Seventh Edition | Scryfall ID: 1aeb39c9-7853-4032-882a-83d4863dcbc5 | Oracle ID: 8162730e-2573-4bc0-827f-5b7b3dbf3bff

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TELEPATHIC_SPIES,
    oracle_id = "8162730e-2573-4bc0-827f-5b7b3dbf3bff",
    scryfall_id = "1aeb39c9-7853-4032-882a-83d4863dcbc5",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Telepathic Spies",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::ETB,
        &[Effect::LookAtChosenHand],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent)),
    )],
);
