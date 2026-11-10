//! Necravolver — {2}{B} — Creature — Volver
//! Oracle: Kicker {1}{G} and/or {W} (You may pay an additional {1}{G} and/or {W} as you cast this spell.)
//! Oracle: If this creature was kicked with its {1}{G} kicker, it enters with two +1/+1 counters on it and with trample.
//! Oracle: If this creature was kicked with its {W} kicker, it enters with a +1/+1 counter on it and with "Whenever this creature deals damage, you gain that much life."
//! Set: APC #46 — Apocalypse | Scryfall ID: 232c32d9-9b0c-458d-b1b3-e4219bd34c82 | Oracle ID: 639d16ed-daa4-4c8d-8550-5c9a29f4d9fe
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::NECRAVOLVER,
    oracle_id = "639d16ed-daa4-4c8d-8550-5c9a29f4d9fe",
    scryfall_id = "232c32d9-9b0c-458d-b1b3-e4219bd34c82",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green, Color::White]),
    faces = &[face!(
        name = "Necravolver",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::VOLVER],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
