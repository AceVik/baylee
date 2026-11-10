//! Wizard Mentor — {2}{U} — Creature — Human Wizard
//! Oracle: {T}: Return this creature and target creature you control to their owner's hand.
//! Set: USG #112 — Urza's Saga | Scryfall ID: 49805401-9bd9-48a3-9b99-0120a8bb1fb5 | Oracle ID: a1fb1b77-a49f-4447-bbe0-156407f192e2
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WIZARD_MENTOR,
    oracle_id = "a1fb1b77-a49f-4447-bbe0-156407f192e2",
    scryfall_id = "49805401-9bd9-48a3-9b99-0120a8bb1fb5",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Wizard Mentor",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        Cost::TAP,
        &[
            Effect::ReturnToHand {
                target: TargetSpec::ThisObject,
            },
            Effect::ReturnToHand {
                target: TargetSpec::Object(&Filter::YOUR_CREATURE),
            },
        ],
        target = Some(TargetSpec::Object(&Filter::YOUR_CREATURE)),
    )],
);
