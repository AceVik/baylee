//! Imaginary Pet — {1}{U} — Creature — Illusion
//! Oracle: At the beginning of your upkeep, if you have a card in hand, return this creature to its owner's hand.
//! Set: USG #81 — Urza's Saga | Scryfall ID: 1aa19ecb-146e-4109-b4ef-74675b35d8c4 | Oracle ID: afee3422-5490-411b-a36e-82e447c1bd0f
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::IMAGINARY_PET,
    oracle_id = "afee3422-5490-411b-a36e-82e447c1bd0f",
    scryfall_id = "1aa19ecb-146e-4109-b4ef-74675b35d8c4",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Imaginary Pet",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ILLUSION],
        power = Some(4),
        toughness = Some(4),
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::ReturnToHand {
            target: TargetSpec::ThisObject,
        }],
        condition = Some(Condition::Not(&Condition::HandSizeAtMost(0))),
    )],
);
