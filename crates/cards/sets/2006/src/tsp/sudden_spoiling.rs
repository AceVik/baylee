//! Sudden Spoiling — {1}{B}{B} — Instant
//! Oracle: Split second (As long as this spell is on the stack, players can't cast spells or activate abilities that aren't mana abilities.)
//! Oracle: Until end of turn, creatures target player controls lose all abilities and have base power and toughness 0/2.
//! Set: TSR #144 — Time Spiral Remastered | Scryfall ID: 73514b51-6c19-4b3c-9cf9-2cf5028d7708 | Oracle ID: dce202c7-fe8e-462a-858e-7a5a69bd5b6b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SUDDEN_SPOILING,
    oracle_id = "dce202c7-fe8e-462a-858e-7a5a69bd5b6b",
    scryfall_id = "73514b51-6c19-4b3c-9cf9-2cf5028d7708",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Sudden Spoiling",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
