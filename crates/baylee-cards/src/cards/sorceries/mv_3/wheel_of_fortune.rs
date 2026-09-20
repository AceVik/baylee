//! Wheel of Fortune — {2}{R} — Sorcery
//! Oracle: Each player discards their hand, then draws seven cards.
//! Set: VMA #192 — Vintage Masters | Scryfall ID: 2597050f-6b1b-474e-aa16-33fd154628ca | Oracle ID: a8abd966-de7b-46a3-8ac7-8747ab35653a
// PARTIAL — each player draws seven cards (DrawCardsFor / EachPlayer); the
// printed discard is not expressible and is not run.
// NOT SUPPORTED: "Each player discards their hand" — Effect::DiscardForPlayers
// carries a fixed count, and no variant discards a whole hand.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WHEEL_OF_FORTUNE,
    oracle_id = "a8abd966-de7b-46a3-8ac7-8747ab35653a",
    scryfall_id = "2597050f-6b1b-474e-aa16-33fd154628ca",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Wheel of Fortune",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::SORCERY,
    ),],
    coverage =
        Coverage::Partial("each player discards their hand: no variant discards a whole hand",),
    abilities = &[spell!(&[Effect::DrawCardsFor {
        amount: Amount::Fixed(7),
        who: PlayerRel::EachPlayer,
    }])],
);
