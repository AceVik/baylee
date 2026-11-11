//! Nowhere to Run — {1}{B} — Enchantment
//! Oracle: Flash
//! Oracle: When this enchantment enters, target creature an opponent controls gets -3/-3 until end of turn.
//! Oracle: Creatures your opponents control can be the targets of spells and abilities as though they didn't have hexproof. Ward abilities of those creatures don't trigger.
//! Set: DSK #111 — Duskmourn: House of Horror | Scryfall ID: fee60e9d-9ee7-444a-88f3-c1929e1888fb | Oracle ID: da82ca96-a613-4d00-9e6b-1fece4fc23d0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NOWHERE_TO_RUN,
    oracle_id = "da82ca96-a613-4d00-9e6b-1fece4fc23d0",
    scryfall_id = "fee60e9d-9ee7-444a-88f3-c1929e1888fb",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Nowhere to Run",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
