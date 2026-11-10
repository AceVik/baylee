//! Brood of Cockroaches — {1}{B} — Creature — Insect
//! Oracle: When this creature is put into your graveyard from the battlefield, at the beginning of the next end step, you lose 1 life and return this card to your hand.
//! Set: VIS #53 — Visions | Scryfall ID: 30b6150e-7d0c-4361-b99b-79de96dfc53a | Oracle ID: 7a22d694-29d1-4e7b-8fca-d7008aede489
// IMPLEMENTED — when it dies, a delayed trigger at the beginning of the next end step makes you lose 1 life and returns this card from the graveyard to your hand.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BROOD_OF_COCKROACHES,
    oracle_id = "7a22d694-29d1-4e7b-8fca-d7008aede489",
    scryfall_id = "30b6150e-7d0c-4361-b99b-79de96dfc53a",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Brood of Cockroaches",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::INSECT],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::Dies(&Filter::And(&[Filter::This, Filter::OwnedByYou])),
        &[Effect::AtNextEndStep {
            effects: &[
                Effect::LoseLife {
                    amount: Amount::Fixed(1),
                    target: PlayerRel::You,
                },
                Effect::ReturnToHand {
                    target: TargetSpec::EventObject,
                },
            ],
        }],
    )],
);
