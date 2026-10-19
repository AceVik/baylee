//! Mana Clash — {R} — Sorcery
//! Oracle: You and target opponent each flip a coin. Mana Clash deals 1 damage to each player whose coin comes up tails. Repeat this process until both players' coins come up heads on the same flip.
//! Set: 9ED #203 — Ninth Edition | Scryfall ID: bd41352f-617f-40f6-b612-11022180c18b | Oracle ID: a2f5a5fd-14dc-48e8-bdb0-984a83288023

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MANA_CLASH,
    oracle_id = "a2f5a5fd-14dc-48e8-bdb0-984a83288023",
    scryfall_id = "bd41352f-617f-40f6-b612-11022180c18b",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mana Clash",
        mana_cost = mana!("{R}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::FlipCoinsDamageTailsRepeatUntilBothHeads {
            damage: Amount::Fixed(1),
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent))
    )],
);
