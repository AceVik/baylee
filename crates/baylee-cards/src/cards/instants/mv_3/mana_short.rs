//! Mana Short — {2}{U} — Instant
//! Oracle: Tap all lands target player controls and that player loses all unspent mana.
//! Set: 7ED #86 — Seventh Edition | Scryfall ID: a0486784-de03-47a7-949d-550fd23492bc | Oracle ID: 48207d1c-448a-4e1b-974a-642dfea75933
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MANA_SHORT,
    oracle_id = "48207d1c-448a-4e1b-974a-642dfea75933",
    scryfall_id = "a0486784-de03-47a7-949d-550fd23492bc",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Mana Short",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
