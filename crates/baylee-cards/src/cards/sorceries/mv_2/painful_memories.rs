//! Painful Memories — {1}{B} — Sorcery
//! Oracle: Look at target opponent's hand and choose a card from it. Put that card on top of that player's library.
//! Set: 6ED #147 — Classic Sixth Edition | Scryfall ID: bdec2ddc-3a91-4872-b4c0-e75af6cbb184 | Oracle ID: 27a3faa1-4f1a-4581-9e19-9265faa9c10b
// PARTIAL: The engine has no effect to put a chosen card from a player's hand on top of their library; casting this spell reveals target opponent's hand and makes them discard a chosen card instead

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PAINFUL_MEMORIES,
    oracle_id = "27a3faa1-4f1a-4581-9e19-9265faa9c10b",
    scryfall_id = "bdec2ddc-3a91-4872-b4c0-e75af6cbb184",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "The engine has no effect to put a chosen card from a player's hand on top of their library; casting this spell reveals target opponent's hand and makes them discard a chosen card instead"
    ),
    faces = &[face!(
        name = "Painful Memories",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::RevealHandDiscard {
            filter: &Filter::Any,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent)),
    )],
);
