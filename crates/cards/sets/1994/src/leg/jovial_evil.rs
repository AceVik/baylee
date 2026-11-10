//! Jovial Evil — {2}{B} — Sorcery
//! Oracle: Jovial Evil deals X damage to target opponent, where X is twice the number of white creatures that player controls.
//! Set: LEG #109 — Legends | Scryfall ID: c993c74c-a574-423b-81c8-96b0a7a6e529 | Oracle ID: 274c5367-f02e-44a0-be8a-5ed03c831bda

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::JOVIAL_EVIL,
    oracle_id = "274c5367-f02e-44a0-be8a-5ed03c831bda",
    scryfall_id = "c993c74c-a574-423b-81c8-96b0a7a6e529",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Jovial Evil",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::SORCERY,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[Effect::DealDamage {
            amount: Amount::Times {
                base: &Amount::ControlledCount {
                    who: PlayerRel::Chosen,
                    filter: &Filter::And(&[
                        Filter::CREATURE,
                        Filter::HasColor(ColorSet::of(Color::White))
                    ])
                },
                factor: 2
            },
            target: TargetSpec::AnyOpponent
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent))
    )],
);
