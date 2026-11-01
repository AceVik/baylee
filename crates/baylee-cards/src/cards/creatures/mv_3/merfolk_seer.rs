//! Merfolk Seer — {2}{U} — Creature — Merfolk Wizard
//! Oracle: When this creature dies, you may pay {1}{U}. If you do, draw a card.
//! Set: MIR #76 — Mirage | Scryfall ID: 63139890-e860-4791-9bb6-b79bb361ef8b | Oracle ID: 2351a9fa-17b5-41fc-93d0-34731d54f915
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MERFOLK_SEER,
    oracle_id = "2351a9fa-17b5-41fc-93d0-34731d54f915",
    scryfall_id = "63139890-e860-4791-9bb6-b79bb361ef8b",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Merfolk Seer",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::MERFOLK, subtypes::creature::WIZARD],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::Dies(&Filter::This),
        &[Effect::PlayerMayPayManaThen {
            player: PlayerRel::You,
            cost: mana!("{1}{U}"),
            effects: &[Effect::DrawCards {
                amount: Amount::Fixed(1),
            }],
        }],
    )],
);
