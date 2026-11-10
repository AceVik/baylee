//! Dwarven Thaumaturgist — {2}{R} — Creature — Dwarf Shaman
//! Oracle: {T}: Switch target creature's power and toughness until end of turn.
//! Set: WTH #98 — Weatherlight | Scryfall ID: 8e68aa29-9f38-48a2-b00a-39aef9d91f6d | Oracle ID: 019d0013-52fd-4261-9cc8-7efcee5a4616
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DWARVEN_THAUMATURGIST,
    oracle_id = "019d0013-52fd-4261-9cc8-7efcee5a4616",
    scryfall_id = "8e68aa29-9f38-48a2-b00a-39aef9d91f6d",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Dwarven Thaumaturgist",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DWARF, subtypes::creature::SHAMAN],
        power = Some(1),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        Cost::TAP,
        &[Effect::ModifyTargets {
            modifier: Modifier::SwitchPT,
            duration: Duration::UntilEndOfTurn,
        }],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
    )],
);
