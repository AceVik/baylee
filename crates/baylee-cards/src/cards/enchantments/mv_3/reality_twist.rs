//! Reality Twist — {U}{U}{U} — Enchantment
//! Oracle: Cumulative upkeep {1}{U}{U} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: If tapped for mana, Plains produce {R}, Swamps produce {G}, Mountains produce {W}, and Forests produce {B} instead of any other type.
//! Set: ICE #94 — Ice Age | Scryfall ID: 1b7e955c-3de2-430c-93b9-0b39ccea5420 | Oracle ID: 2c6f9fdb-ee27-4cd8-9d4d-1ea8b2830f73
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::REALITY_TWIST,
    oracle_id = "2c6f9fdb-ee27-4cd8-9d4d-1ea8b2830f73",
    scryfall_id = "1b7e955c-3de2-430c-93b9-0b39ccea5420",
    color_identity = ColorSet::from_slice(&[
        Color::Black,
        Color::Green,
        Color::Red,
        Color::Blue,
        Color::White
    ]),
    faces = &[face!(
        name = "Reality Twist",
        mana_cost = mana!("{U}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
