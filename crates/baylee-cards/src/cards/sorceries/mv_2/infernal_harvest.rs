//! Infernal Harvest — {1}{B} — Sorcery
//! Oracle: As an additional cost to cast this spell, return X Swamps you control to their owner's hand.
//! Oracle: Infernal Harvest deals X damage divided as you choose among any number of target creatures.
//! Set: VIS #62 — Visions | Scryfall ID: ccf85ac9-f5d8-4a36-aa6c-3a31427a0348 | Oracle ID: b8c8cde9-34db-4185-86f5-4289969880c5
// PARTIAL — Returning X Swamps you control to their owner's hand as a variable additional cost and dividing X damage among any number of target creatures from that count is not yet supported by cast_wizard::paid_as_a_mandatory_additional_cost; in play, Infernal Harvest returns a Swamp you control to its owner's hand and deals 1 damage to target creature on resolution.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static YOUR_SWAMP: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::HasSubtype(subtypes::land::SWAMP),
]);

card!(
    index = index::INFERNAL_HARVEST,
    oracle_id = "b8c8cde9-34db-4185-86f5-4289969880c5",
    scryfall_id = "ccf85ac9-f5d8-4a36-aa6c-3a31427a0348",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "Returning X Swamps you control to their owner's hand as a variable additional cost and dividing X damage among any number of target creatures from that count is not yet supported by cast_wizard::paid_as_a_mandatory_additional_cost; in play, Infernal Harvest returns a Swamp you control to its owner's hand and deals 1 damage to target creature on resolution.",
    ),
    faces = &[face!(
        name = "Infernal Harvest",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[
            Effect::ReturnChosenToHand {
                who: PlayerRel::You,
                filter: &YOUR_SWAMP,
            },
            Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::Object(&Filter::CREATURE),
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
