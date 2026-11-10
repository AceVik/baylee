//! Impulse — {1}{U} — Instant
//! Oracle: Look at the top four cards of your library. Put one of them into your hand and the rest on the bottom of your library in any order.
//! Set: DMR #56 — Dominaria Remastered | Scryfall ID: 3ec50625-5e14-4fd0-9a96-86f195342695 | Oracle ID: f6bd2902-7f8b-419e-bbc7-bcab0c1b7e01
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::IMPULSE,
    oracle_id = "f6bd2902-7f8b-419e-bbc7-bcab0c1b7e01",
    scryfall_id = "3ec50625-5e14-4fd0-9a96-86f195342695",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Impulse",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::LookAtTopPick {
        count: Amount::Fixed(4),
        pick: 1,
        random: false,
    }])],
);
