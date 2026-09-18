//! Thrun, the Last Troll — {2}{G}{G} — Legendary Creature — Troll Shaman
//! Oracle: This spell can't be countered.
//! Oracle: Hexproof (This creature can't be the target of spells or abilities your opponents control.)
//! Oracle: {1}{G}: Regenerate Thrun.
//! Set: MBS #92 — Mirrodin Besieged | Scryfall ID: 5d393da0-4cb6-4ae8-b747-8e6d0fa7f55a | Oracle ID: 1149e5ac-554a-41b1-84ae-bac42579c1aa
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THRUN_THE_LAST_TROLL,
    oracle_id = "1149e5ac-554a-41b1-84ae-bac42579c1aa",
    scryfall_id = "5d393da0-4cb6-4ae8-b747-8e6d0fa7f55a",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Thrun, the Last Troll",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::TROLL, subtypes::creature::SHAMAN],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
