//! Ashen Powder — {2}{B}{B} — Sorcery
//! Oracle: Put target creature card from an opponent's graveyard onto the battlefield under your control.
//! Set: 6ED #112 — Classic Sixth Edition | Scryfall ID: d6fa11f5-546b-4a14-afd0-80866d4968ab | Oracle ID: d906965a-0f33-4d75-8abe-66f2cf4817b8
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ASHEN_POWDER,
    oracle_id = "d906965a-0f33-4d75-8abe-66f2cf4817b8",
    scryfall_id = "d6fa11f5-546b-4a14-afd0-80866d4968ab",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Ashen Powder",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::GraveyardToBattlefield {
            target: TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::Opponent),
            owner_control: false,
            counters: None
        }],
        targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &Filter::CREATURE,
            PlayerRel::Opponent
        )))
    ),],
);
