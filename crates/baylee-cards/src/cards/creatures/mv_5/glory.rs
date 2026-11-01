//! Glory — {3}{W}{W} — Creature — Incarnation
//! Oracle: Flying
//! Oracle: {2}{W}: Choose a color. Creatures you control gain protection from the chosen color until end of turn. Activate only if this card is in your graveyard.
//! Set: DMR #7 — Dominaria Remastered | Scryfall ID: 6b01ebcf-8451-486e-84f6-8e6e1bb9d6e3 | Oracle ID: 3853ef6a-6771-4708-8ed5-d667f646de5a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GLORY,
    oracle_id = "3853ef6a-6771-4708-8ed5-d667f646de5a",
    scryfall_id = "6b01ebcf-8451-486e-84f6-8e6e1bb9d6e3",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Glory",
        mana_cost = mana!("{3}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::INCARNATION],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
