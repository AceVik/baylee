//! Abyssal Gatekeeper — {1}{B} — Creature — Horror
//! Oracle: When this creature dies, each player sacrifices a creature of their choice.
//! Set: DVD #31 — Duel Decks Anthology: Divine vs. Demonic | Scryfall ID: 6d2a66f0-1afe-41e6-a597-ec2e95685191 | Oracle ID: 4954181c-cb46-4cac-adc3-2c8b7693a211
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ABYSSAL_GATEKEEPER,
    oracle_id = "4954181c-cb46-4cac-adc3-2c8b7693a211",
    scryfall_id = "6d2a66f0-1afe-41e6-a597-ec2e95685191",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Abyssal Gatekeeper",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HORROR],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::Dies(&Filter::This),
        &[Effect::SacrificeFilter {
            who: PlayerRel::EachPlayer,
            filter: &Filter::CREATURE,
        }],
    )],
);
