//! Rakavolver — {2}{R} — Creature — Volver
//! Oracle: Kicker {1}{W} and/or {U} (You may pay an additional {1}{W} and/or {U} as you cast this spell.)
//! Oracle: If this creature was kicked with its {1}{W} kicker, it enters with two +1/+1 counters on it and with "Whenever this creature deals damage, you gain that much life."
//! Oracle: If this creature was kicked with its {U} kicker, it enters with a +1/+1 counter on it and with flying.
//! Set: APC #68 — Apocalypse | Scryfall ID: 43787e24-0b7d-4005-8db4-68544476bd34 | Oracle ID: c00adb98-81dc-490f-9e0f-7f8546ef22ce
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RAKAVOLVER,
    oracle_id = "c00adb98-81dc-490f-9e0f-7f8546ef22ce",
    scryfall_id = "43787e24-0b7d-4005-8db4-68544476bd34",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::Blue, Color::White]),
    faces = &[face!(
        name = "Rakavolver",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::VOLVER],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
