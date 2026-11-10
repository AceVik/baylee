//! Balduvian Horde — {2}{R}{R} — Creature — Human Barbarian
//! Oracle: When this creature enters, sacrifice it unless you discard a card at random.
//! Set: A25 #120 — Masters 25 | Scryfall ID: fe32f853-4afb-4a91-8b4c-0d647ce9c216 | Oracle ID: fd2a94d3-901d-4b46-aa9b-a30ed6dea6bd
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BALDUVIAN_HORDE,
    oracle_id = "fd2a94d3-901d-4b46-aa9b-a30ed6dea6bd",
    scryfall_id = "fe32f853-4afb-4a91-8b4c-0d647ce9c216",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Balduvian Horde",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::BARBARIAN],
        power = Some(5),
        toughness = Some(5),
    ),],
    abilities = &[triggered!(
        Trigger::ETB,
        &[
            // "Sacrifice it unless you discard a card at random": the discard is the controller's
            // to make or not, and the sacrifice follows unless a card was discarded this way (an
            // empty hand cannot pay it).
            Effect::MayDo {
                effects: &[Effect::DiscardRandom {
                    who: PlayerRel::You,
                    count: Amount::Fixed(1),
                }],
            },
            Effect::IfCondition {
                condition: Condition::AmountAtLeast(Amount::ThisWay(ThisWay::Discarded), 1),
                then: &[],
                otherwise: &[Effect::SacrificeSelf],
            },
        ],
    )],
);
