//! Truce — {2}{W} — Instant
//! Oracle: Each player may draw up to two cards. For each card less than two a player draws this way, that player gains 2 life.
//! Set: 5ED #65 — Fifth Edition | Scryfall ID: de4f53fc-69d7-49fb-885b-4cc02bf5facc | Oracle ID: fe4aa72b-f176-461a-aca0-1519721609b2

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: per-player choice of how many cards up to two to draw and gaining 2 life per card not drawn this way is not in Effect, so each player draws two cards.
card!(
    index = index::TRUCE,
    oracle_id = "fe4aa72b-f176-461a-aca0-1519721609b2",
    scryfall_id = "de4f53fc-69d7-49fb-885b-4cc02bf5facc",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "per-player choice of how many cards up to two to draw and gaining 2 life per card not drawn this way is not in Effect; each player draws two cards"
    ),
    faces = &[face!(
        name = "Truce",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::DrawCardsFor {
        who: PlayerRel::EachPlayer,
        amount: Amount::Fixed(2),
    }])],
);
