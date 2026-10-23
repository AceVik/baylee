//! Argothian Elder — {3}{G} — Creature — Elf Druid
//! Oracle: {T}: Untap two target lands.
//! Set: USG #233 — Urza's Saga | Scryfall ID: 453e7cb4-bc37-4932-85b7-3a4e160b73dc | Oracle ID: b57f06e4-da97-4473-9fbf-f66e9129502a
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ARGOTHIAN_ELDER,
    oracle_id = "b57f06e4-da97-4473-9fbf-f66e9129502a",
    scryfall_id = "453e7cb4-bc37-4932-85b7-3a4e160b73dc",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Argothian Elder",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF, subtypes::creature::DRUID],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        Cost::TAP,
        &[Effect::UntapTarget],
        targets = Some(TargetReq::exactly(TargetSpec::Object(&Filter::LAND), 2)),
    )],
);
