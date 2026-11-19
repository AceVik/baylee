//! Stubborn Denial — {U} — Instant
//! Oracle: Counter target noncreature spell unless its controller pays {1}.
//! Oracle: Ferocious — If you control a creature with power 4 or greater, counter that spell instead.
//! Set: KTK #56 — Khans of Tarkir | Scryfall ID: 6f8626c4-306f-4e9d-8840-2bb73fe87e87 | Oracle ID: 3adc3af3-5086-4d61-b38c-c1dddeaeebe6
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::STUBBORN_DENIAL,
    oracle_id = "3adc3af3-5086-4d61-b38c-c1dddeaeebe6",
    scryfall_id = "6f8626c4-306f-4e9d-8840-2bb73fe87e87",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Stubborn Denial",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
