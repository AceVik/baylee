//! Blossoming Wreath — {G} — Instant
//! Oracle: You gain life equal to the number of creature cards in your graveyard.
//! Set: WTH #120 — Weatherlight | Scryfall ID: 2f944ad9-c9ce-47b2-80fa-d0f7fcf0fd5d | Oracle ID: fbd99a88-8926-455e-8e23-58fed6d14719
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BLOSSOMING_WREATH,
    oracle_id = "fbd99a88-8926-455e-8e23-58fed6d14719",
    scryfall_id = "2f944ad9-c9ce-47b2-80fa-d0f7fcf0fd5d",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Blossoming Wreath",
        mana_cost = mana!("{G}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::GainLife {
        amount: Amount::CountOf {
            filter: &Filter::CREATURE,
            zone: ZoneSel::GraveyardYou,
        },
    }])],
);
