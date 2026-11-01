//! Thieves' Auction — {4}{R}{R}{R} — Sorcery
//! Oracle: Exile all nontoken permanents. Starting with you, each player chooses one of the exiled cards and puts it onto the battlefield tapped under their control. Repeat this process until all cards exiled this way have been chosen.
//! Set: 8ED #227 — Eighth Edition | Scryfall ID: c9e8ad32-a701-4b77-bb7e-0e440e4072da | Oracle ID: 230ee0b4-eb75-457f-b171-72d585cfb5aa
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::THIEVES_AUCTION,
    oracle_id = "230ee0b4-eb75-457f-b171-72d585cfb5aa",
    scryfall_id = "c9e8ad32-a701-4b77-bb7e-0e440e4072da",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Thieves' Auction",
        mana_cost = mana!("{4}{R}{R}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
