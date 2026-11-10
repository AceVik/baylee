//! Day of the Dragons — {4}{U}{U}{U} — Enchantment
//! Oracle: When this enchantment enters, exile all creatures you control. Then create that many 5/5 red Dragon creature tokens with flying.
//! Oracle: When this enchantment leaves the battlefield, sacrifice all Dragons you control. Then return the exiled cards to the battlefield under your control.
//! Set: KHC #38 — Kaldheim Commander | Scryfall ID: 76e4dd1d-b0bd-4fcf-b5c5-fe203af2fe8e | Oracle ID: c4df259a-cdd4-420a-8e3d-7b69da5c9f6f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DAY_OF_THE_DRAGONS,
    oracle_id = "c4df259a-cdd4-420a-8e3d-7b69da5c9f6f",
    scryfall_id = "76e4dd1d-b0bd-4fcf-b5c5-fe203af2fe8e",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Day of the Dragons",
        mana_cost = mana!("{4}{U}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
