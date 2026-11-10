//! Devoted Caretaker — {W} — Creature — Human Cleric
//! Oracle: {W}, {T}: Target permanent you control gains protection from instant spells and from sorcery spells until end of turn.
//! Set: ODY #18 — Odyssey | Scryfall ID: 8d400b63-de3e-4805-a51d-c4c0dfbb7033 | Oracle ID: 654bcf7f-9325-4a4b-8a50-41fdd2b7e514
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DEVOTED_CARETAKER,
    oracle_id = "654bcf7f-9325-4a4b-8a50-41fdd2b7e514",
    scryfall_id = "8d400b63-de3e-4805-a51d-c4c0dfbb7033",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Devoted Caretaker",
        mana_cost = mana!("{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
