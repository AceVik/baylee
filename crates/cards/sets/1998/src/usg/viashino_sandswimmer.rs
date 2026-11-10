//! Viashino Sandswimmer — {2}{R}{R} — Creature — Lizard
//! Oracle: {R}: Flip a coin. If you win the flip, return this creature to its owner's hand. If you lose the flip, sacrifice this creature.
//! Set: USG #225 — Urza's Saga | Scryfall ID: 0790607f-51b7-40ff-80f9-4f7f5cd2d63c | Oracle ID: 1d82f01a-29be-4756-8745-e2c04bf77c61
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VIASHINO_SANDSWIMMER,
    oracle_id = "1d82f01a-29be-4756-8745-e2c04bf77c61",
    scryfall_id = "0790607f-51b7-40ff-80f9-4f7f5cd2d63c",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Viashino Sandswimmer",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::LIZARD],
        power = Some(3),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        cost!("{R}"),
        &[Effect::FlipCoin {
            won: &[Effect::ReturnToHand {
                target: TargetSpec::ThisObject,
            }],
            lost: &[Effect::SacrificeSelf],
        }],
    )],
);
