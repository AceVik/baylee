//! Witch Engine — {5}{B} — Creature — Horror
//! Oracle: Swampwalk (This creature can't be blocked as long as defending player controls a Swamp.)
//! Oracle: {T}: Add {B}{B}{B}{B}. Target opponent gains control of this creature. (Activate only as an instant.)
//! Set: USG #169 — Urza's Saga | Scryfall ID: ef749290-58e2-4b40-a141-5fe294f9b995 | Oracle ID: 48adccf6-adb4-49b2-b013-4dc5e0fc7bcb
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WITCH_ENGINE,
    oracle_id = "48adccf6-adb4-49b2-b013-4dc5e0fc7bcb",
    scryfall_id = "ef749290-58e2-4b40-a141-5fe294f9b995",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Witch Engine",
        mana_cost = mana!("{5}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HORROR],
        power = Some(4),
        toughness = Some(4),
        keywords = KeywordSet::SWAMPWALK,
    ),],
    abilities = &[activated!(
        Cost::TAP,
        &[
            Effect::mana(ManaColor::Black, 4),
            Effect::GainControlAll {
                filter: &Filter::This,
                to: PlayerRel::TargetPlayerOrController,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent)),
    )],
);
