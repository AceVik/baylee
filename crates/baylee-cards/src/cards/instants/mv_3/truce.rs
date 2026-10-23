//! Truce — {2}{W} — Instant
//! Oracle: Each player may draw up to two cards. For each card less than two a player draws this way, that player gains 2 life.
//! Set: 5ED #65 — Fifth Edition | Scryfall ID: de4f53fc-69d7-49fb-885b-4cc02bf5facc | Oracle ID: fe4aa72b-f176-461a-aca0-1519721609b2

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: "up to two cards" and "for each card less than two … that player gains 2 life". `Effect` has no per-player count of cards to draw, so each of you and one opponent either draws two cards or none, and the 2 life per card not drawn is never gained. The opponent is asked as `PlayerRel::Opponent`, so at a table of three or more only one opponent gets to draw.
card!(
    index = index::TRUCE,
    oracle_id = "fe4aa72b-f176-461a-aca0-1519721609b2",
    scryfall_id = "de4f53fc-69d7-49fb-885b-4cc02bf5facc",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "'draw up to two cards' is all or nothing: you and one opponent may each draw two cards or none (drawing exactly one is not in Effect), 'for each card less than two … gains 2 life' is not applied (a player who draws fewer than two gains nothing), and at a table of three or more only one opponent is asked"
    ),
    faces = &[face!(
        name = "Truce",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[
        Effect::MayDo {
            effects: &[Effect::DrawCardsFor {
                who: PlayerRel::You,
                amount: Amount::Fixed(2),
            }],
        },
        Effect::MayDoFor {
            player: PlayerRel::Opponent,
            effects: &[Effect::DrawCardsFor {
                who: PlayerRel::Opponent,
                amount: Amount::Fixed(2),
            }],
        },
    ])],
);
