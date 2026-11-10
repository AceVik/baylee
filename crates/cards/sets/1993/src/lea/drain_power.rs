//! Drain Power — {U}{U} — Sorcery
//! Oracle: Target player activates a mana ability of each land they control. Then that player loses all unspent mana and you add the mana lost this way.
//! Set: ME4 #46 — Masters Edition IV | Scryfall ID: 380a7357-8c07-4ee5-83a0-10672d7e85d4 | Oracle ID: 0669172d-396b-4f5a-9703-129c5c849b55

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DRAIN_POWER,
    oracle_id = "0669172d-396b-4f5a-9703-129c5c849b55",
    scryfall_id = "380a7357-8c07-4ee5-83a0-10672d7e85d4",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Drain Power",
        mana_cost = mana!("{U}{U}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::ActivateLandsAndTakeMana {
            player: PlayerRel::Chosen
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer))
    )],
);
