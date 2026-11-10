//! Time Spiral — {4}{U}{U} — Sorcery
//! Oracle: Exile Time Spiral. Each player shuffles their hand and graveyard into their library, then draws seven cards. You untap up to six lands.
//! Set: USG #103 — Urza's Saga | Scryfall ID: f3d62dbd-63db-4ac9-950f-9852627f23f2 | Oracle ID: cac55e46-b730-4569-b92c-a4b5922fc20c
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TIME_SPIRAL,
    oracle_id = "cac55e46-b730-4569-b92c-a4b5922fc20c",
    scryfall_id = "f3d62dbd-63db-4ac9-950f-9852627f23f2",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Time Spiral",
        mana_cost = mana!("{4}{U}{U}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[
        Effect::ExileSource,
        Effect::ShuffleIntoLibrary {
            who: PlayerRel::EachPlayer,
            hand: true,
            graveyard: true,
        },
        Effect::DrawCardsFor {
            amount: Amount::Fixed(7),
            who: PlayerRel::EachPlayer,
        },
        Effect::UntapChosen {
            filter: &Filter::LAND,
            count: 6,
        },
    ]),],
);
