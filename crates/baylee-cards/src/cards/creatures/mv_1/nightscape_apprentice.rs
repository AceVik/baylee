//! Nightscape Apprentice — {B} — Creature — Zombie Wizard
//! Oracle: {U}, {T}: Put target creature you control on top of its owner's library.
//! Oracle: {R}, {T}: Target creature gains first strike until end of turn.
//! Set: INV #112 — Invasion | Scryfall ID: 7498ca4c-614a-4776-8886-0a6ed58520f6 | Oracle ID: 429de3c3-3f2e-4d55-a04a-f414912aff3a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::NIGHTSCAPE_APPRENTICE,
    oracle_id = "429de3c3-3f2e-4d55-a04a-f414912aff3a",
    scryfall_id = "7498ca4c-614a-4776-8886-0a6ed58520f6",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red, Color::Blue]),
    faces = &[face!(
        name = "Nightscape Apprentice",
        mana_cost = mana!("{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
