//! Superior Numbers — {G}{G} — Sorcery
//! Oracle: Superior Numbers deals damage to target creature equal to the number of creatures you control in excess of the number of creatures target opponent controls.
//! Set: MIR #244 — Mirage | Scryfall ID: 43d90914-ddfc-49c2-8e58-fdc3693040f2 | Oracle ID: b98177b5-599f-41b0-aa1e-4769d7005bbd
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SUPERIOR_NUMBERS,
    oracle_id = "b98177b5-599f-41b0-aa1e-4769d7005bbd",
    scryfall_id = "43d90914-ddfc-49c2-8e58-fdc3693040f2",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Superior Numbers",
        mana_cost = mana!("{G}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
