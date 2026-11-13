//! Greater Good — {2}{G}{G} — Enchantment
//! Oracle: Sacrifice a creature: Draw cards equal to the sacrificed creature's power, then discard three cards.
//! Set: USG #257 — Urza's Saga | Scryfall ID: 12befd35-2dc6-4852-a153-75b553042643 | Oracle ID: dc0593c2-ccb4-4648-a592-c5bcd121dc72
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GREATER_GOOD,
    oracle_id = "dc0593c2-ccb4-4648-a592-c5bcd121dc72",
    scryfall_id = "12befd35-2dc6-4852-a153-75b553042643",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Greater Good",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[activated!(
        cost!(Sacrifice(&Filter::YOUR_CREATURE)),
        &[
            Effect::DrawCards {
                amount: Amount::SacrificedPower,
            },
            Effect::DiscardForPlayers {
                who: PlayerRel::You,
                count: 3,
            },
        ],
    )],
);
