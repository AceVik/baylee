//! Head Games — {3}{B}{B} — Sorcery
//! Oracle: Target opponent puts the cards from their hand on top of their library. Search that player's library for that many cards. The player puts those cards into their hand, then shuffles.
//! Set: 10E #148 — Tenth Edition | Scryfall ID: 73ab3037-7e62-46a7-800e-8bfa7d33f237 | Oracle ID: 671a8509-3b6e-499c-a744-ff8b3c2b6e6a
// IMPLEMENTED — target opponent puts hand on library, controller searches for that many cards for them, then shuffle.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HEAD_GAMES,
    oracle_id = "671a8509-3b6e-499c-a744-ff8b3c2b6e6a",
    scryfall_id = "73ab3037-7e62-46a7-800e-8bfa7d33f237",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Head Games",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[
            Effect::PutHandOnTop {
                player: PlayerRel::Chosen,
            },
            Effect::SearchLibraryFor {
                library: PlayerRel::Chosen,
                receiver: PlayerRel::Chosen,
                count: Amount::ThisWay(ThisWay::PutIntoLibraryFromHand),
                filter: &Filter::Any,
                find: &Find::HAND,
                optional: false,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent)),
    )],
);
