//! Thoughtseize — {B} — Sorcery
//! Oracle: Target player reveals their hand. You choose a nonland card from it. That player discards that card. You lose 2 life.
//! Set: 2XM #109 — Double Masters | Scryfall ID: b281a308-ab6b-47b6-bec7-632c9aaecede | Oracle ID: edd8d1e8-be43-4c38-bb3a-83081fbaf0b5

use baylee_cards_dsl::prelude::*;

card!(
    index = index::THOUGHTSEIZE,
    oracle_id = "edd8d1e8-be43-4c38-bb3a-83081fbaf0b5",
    scryfall_id = "b281a308-ab6b-47b6-bec7-632c9aaecede",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Thoughtseize",
        mana_cost = mana!("{B}"),
        types = TypeSet::SORCERY,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[
            Effect::RevealHandDiscard {
                filter: &Filter::LacksType(TypeSet::LAND),
            },
            Effect::LoseLife {
                amount: Amount::Fixed(2),
                target: PlayerRel::You,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer))
    )],
);
