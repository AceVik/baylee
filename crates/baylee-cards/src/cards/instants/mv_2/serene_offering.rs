//! Serene Offering — {1}{W} — Instant
//! Oracle: Destroy target enchantment. You gain life equal to its mana value.
//! Set: TMP #40 — Tempest | Scryfall ID: 6c0b3795-7f30-4c61-b5d8-f238055d6be1 | Oracle ID: 6cef0548-6010-4781-8e83-431d014dd1da

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SERENE_OFFERING,
    oracle_id = "6cef0548-6010-4781-8e83-431d014dd1da",
    scryfall_id = "6c0b3795-7f30-4c61-b5d8-f238055d6be1",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Serene Offering",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::destroy(TargetSpec::Object(&Filter::ENCHANTMENT)),
            Effect::GainLife {
                amount: Amount::TargetCmc,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::ENCHANTMENT))),
    )],
);
