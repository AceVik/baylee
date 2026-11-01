//! Justice — {2}{W}{W} — Enchantment
//! Oracle: At the beginning of your upkeep, sacrifice this enchantment unless you pay {W}{W}.
//! Oracle: Whenever a red creature or spell deals damage, this enchantment deals that much damage to that creature's or spell's controller.
//! Set: 5ED #41 — Fifth Edition | Scryfall ID: bb11e16f-edf5-4e0e-862b-b055fd800682 | Oracle ID: 0586fdc7-3025-4040-8eaf-f8a57a21aff0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::JUSTICE,
    oracle_id = "0586fdc7-3025-4040-8eaf-f8a57a21aff0",
    scryfall_id = "bb11e16f-edf5-4e0e-862b-b055fd800682",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Justice",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
