//! Tar Pit Warrior — {2}{B} — Creature — Cyclops Warrior
//! Oracle: When this creature becomes the target of a spell or ability, sacrifice it.
//! Set: VIS #70 — Visions | Scryfall ID: e1283190-094e-4a9f-bf67-f9fd05778744 | Oracle ID: 05a7ca83-e820-433f-b9e9-151e817d3708
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TAR_PIT_WARRIOR,
    oracle_id = "05a7ca83-e820-433f-b9e9-151e817d3708",
    scryfall_id = "e1283190-094e-4a9f-bf67-f9fd05778744",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Tar Pit Warrior",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::CYCLOPS, subtypes::creature::WARRIOR],
        power = Some(3),
        toughness = Some(4),
    ),],
    abilities = &[triggered!(Trigger::BecomesTarget, &[Effect::SacrificeSelf])],
);
