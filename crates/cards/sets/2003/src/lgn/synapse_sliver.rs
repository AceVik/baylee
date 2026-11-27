//! Synapse Sliver — {4}{U} — Creature — Sliver
//! Oracle: Whenever a Sliver deals combat damage to a player, its controller may draw a card.
//! Set: CMM #857 — Commander Masters | Scryfall ID: 7c458a0f-0e63-417f-b51e-8491822835b0 | Oracle ID: b76f123d-7b78-42a1-a66e-0dee6f773687
// IMPLEMENTED — Sliver combat-damage-to-player trigger for its controller to optionally draw a card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static SLIVER: Filter = Filter::HasSubtype(subtypes::creature::SLIVER);

card!(
    index = index::SYNAPSE_SLIVER,
    oracle_id = "b76f123d-7b78-42a1-a66e-0dee6f773687",
    scryfall_id = "7c458a0f-0e63-417f-b51e-8491822835b0",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Synapse Sliver",
        mana_cost = mana!("{4}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SLIVER],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[triggered!(
        Trigger::DealsCombatDamageToPlayer(&SLIVER),
        &[Effect::MayDoFor {
            player: PlayerRel::ControllerOfEvent,
            effects: &[Effect::DrawCardsFor {
                amount: Amount::Fixed(1),
                who: PlayerRel::ControllerOfEvent,
            }],
        }],
    )],
);
