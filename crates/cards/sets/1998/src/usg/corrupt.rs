//! Corrupt — {5}{B} — Sorcery
//! Oracle: Corrupt deals damage to any target equal to the number of Swamps you control. You gain life equal to the damage dealt this way.
//! Set: USG #124 — Urza's Saga | Scryfall ID: 32e1c65c-ced6-484d-b3b4-db913c6bf84b | Oracle ID: a88a1c53-7de3-423a-a95a-24ed5666ab96
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CORRUPT,
    oracle_id = "a88a1c53-7de3-423a-a95a-24ed5666ab96",
    scryfall_id = "32e1c65c-ced6-484d-b3b4-db913c6bf84b",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Corrupt",
        mana_cost = mana!("{5}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[
            Effect::DealDamage {
                amount: Amount::CountOf {
                    filter: &Filter::And(&[
                        Filter::HasSubtype(subtypes::land::SWAMP),
                        Filter::ControlledByYou,
                    ]),
                    zone: ZoneSel::Battlefield,
                },
                target: TargetSpec::AnyTarget,
            },
            Effect::GainLife {
                amount: Amount::ThisWay(ThisWay::DamageDealt),
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
    )],
);
