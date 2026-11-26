//! Brood Sliver — {4}{G} — Creature — Sliver
//! Oracle: Whenever a Sliver deals combat damage to a player, its controller may create a 1/1 colorless Sliver creature token.
//! Set: CMM #887 — Commander Masters | Scryfall ID: 441a387b-e18a-41c7-84bb-bb7e354f4f2d | Oracle ID: 6b1d178f-9713-4dcf-920b-ddf2c94cc427
// IMPLEMENTED — Sliver combat-damage-to-player trigger for its controller to optionally create a 1/1 Sliver token.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static SLIVER: Filter = Filter::HasSubtype(subtypes::creature::SLIVER);

card!(
    index = index::BROOD_SLIVER,
    oracle_id = "6b1d178f-9713-4dcf-920b-ddf2c94cc427",
    scryfall_id = "441a387b-e18a-41c7-84bb-bb7e354f4f2d",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Brood Sliver",
        mana_cost = mana!("{4}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SLIVER],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[triggered!(
        Trigger::DealsCombatDamageToPlayer(&SLIVER),
        &[Effect::MayDoFor {
            player: PlayerRel::ControllerOfEvent,
            effects: &[Effect::CreateTokensFor {
                player: PlayerRel::ControllerOfEvent,
                token: &crate::generated_tokens::SLIVER_1_1,
                amount: Amount::Fixed(1),
            }],
        }],
    )],
);
