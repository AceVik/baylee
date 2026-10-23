//! Painful Memories — {1}{B} — Sorcery
//! Oracle: Look at target opponent's hand and choose a card from it. Put that card on top of that player's library.
//! Set: 6ED #147 — Classic Sixth Edition | Scryfall ID: bdec2ddc-3a91-4872-b4c0-e75af6cbb184 | Oracle ID: 27a3faa1-4f1a-4581-9e19-9265faa9c10b
// PARTIAL: only the look at target opponent's hand is there; choosing a card from it and putting it on top of that player's library is not
// NOT SUPPORTED: "choose a card from it. Put that card on top of that player's library": PutFromHandOnTop puts cards from the caster's own hand; the old card made the opponent discard the chosen card instead, which loses it for good (stronger than printed).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PAINFUL_MEMORIES,
    oracle_id = "27a3faa1-4f1a-4581-9e19-9265faa9c10b",
    scryfall_id = "bdec2ddc-3a91-4872-b4c0-e75af6cbb184",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "looks at target opponent's hand; choosing a card from it and putting it on top of that player's library is not in the DSL (PutFromHandOnTop is the caster's own hand), so no card moves"
    ),
    faces = &[face!(
        name = "Painful Memories",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::LookAtChosenHand],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent)),
    )],
);
