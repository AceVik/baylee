//! Teferi's Protection — {2}{W} — Instant
//! Oracle: Until your next turn, your life total can't change and you gain protection from everything. All permanents you control phase out. (While they're phased out, they're treated as though they don't exist. They phase in before you untap during your untap step.)
//! Oracle: Exile Teferi's Protection.
//! Set: 2X2 #32 — Double Masters 2022 | Scryfall ID: 483fa1cb-1e35-44f2-a143-98c0f107f5ca | Oracle ID: 0d4ecdb1-ec90-497f-a7a4-1c68092b8757
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TEFERI_S_PROTECTION,
    oracle_id = "0d4ecdb1-ec90-497f-a7a4-1c68092b8757",
    scryfall_id = "483fa1cb-1e35-44f2-a143-98c0f107f5ca",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Teferi's Protection",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
