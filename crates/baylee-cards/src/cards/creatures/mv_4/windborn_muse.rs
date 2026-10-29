//! Windborn Muse — {3}{W} — Creature — Spirit
//! Oracle: Flying
//! Oracle: Creatures can't attack you unless their controller pays {2} for each creature they control that's attacking you.
//! Set: MKC #92 — Murders at Karlov Manor Commander | Scryfall ID: 3ce8b722-7bc0-4053-8627-7846f9d17eda | Oracle ID: 75e09ac6-d789-4c4d-9aca-845d68f3899f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WINDBORN_MUSE,
    oracle_id = "75e09ac6-d789-4c4d-9aca-845d68f3899f",
    scryfall_id = "3ce8b722-7bc0-4053-8627-7846f9d17eda",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Windborn Muse",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPIRIT],
        power = Some(2),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
