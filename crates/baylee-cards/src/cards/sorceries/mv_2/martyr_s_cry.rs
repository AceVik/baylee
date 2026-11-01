//! Martyr's Cry — {W}{W} — Sorcery
//! Oracle: Exile all white creatures. For each creature exiled this way, its controller draws a card.
//! Set: ME4 #19 — Masters Edition IV | Scryfall ID: c9fb6ef0-0a57-4521-9401-92ad6c614af2 | Oracle ID: b636f17d-b54a-4a72-9a5e-ce13e7b4f00a

use baylee_cards_dsl::prelude::*;

static WHITE_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::White])),
]);

card!(
    index = index::MARTYR_S_CRY,
    oracle_id = "b636f17d-b54a-4a72-9a5e-ce13e7b4f00a",
    scryfall_id = "c9fb6ef0-0a57-4521-9401-92ad6c614af2",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Martyr's Cry",
        mana_cost = mana!("{W}{W}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[
        Effect::ExileAll {
            filter: &WHITE_CREATURE,
            linked: false,
        },
        Effect::DrawCardsFor {
            who: PlayerRel::EachPlayer,
            amount: Amount::ThisWayBy {
                what: ThisWay::ExiledFromBattlefield(&WHITE_CREATURE),
                who: PlayerRel::Recipient,
            },
        },
    ])],
);
