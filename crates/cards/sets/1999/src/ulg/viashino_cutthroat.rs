//! Viashino Cutthroat — {2}{R}{R} — Creature — Lizard
//! Oracle: Haste
//! Oracle: At the beginning of the end step, return this creature to its owner's hand.
//! Set: ULG #94 — Urza's Legacy | Scryfall ID: bcbab69d-3259-40f4-a588-ab550858a178 | Oracle ID: 6dc1305a-d6ca-4280-ba5d-5ed91f78ced0
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VIASHINO_CUTTHROAT,
    oracle_id = "6dc1305a-d6ca-4280-ba5d-5ed91f78ced0",
    scryfall_id = "bcbab69d-3259-40f4-a588-ab550858a178",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    keywords = KeywordSet::HASTE,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Viashino Cutthroat",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::LIZARD],
        power = Some(5),
        toughness = Some(3),
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::End,
            whose: PlayerRel::EachPlayer
        },
        &[Effect::bounce(TargetSpec::ThisObject)]
    ),],
);
