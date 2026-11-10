//! Anavolver — {3}{G} — Creature — Volver
//! Oracle: Kicker {1}{U} and/or {B} (You may pay an additional {1}{U} and/or {B} as you cast this spell.)
//! Oracle: If this creature was kicked with its {1}{U} kicker, it enters with two +1/+1 counters on it and with flying.
//! Oracle: If this creature was kicked with its {B} kicker, it enters with a +1/+1 counter on it and with "Pay 3 life: Regenerate this creature."
//! Set: APC #75 — Apocalypse | Scryfall ID: 5e685a8c-fba6-495f-ac0f-1ff5456b22d0 | Oracle ID: 03a4cbe2-cf35-4c2c-b035-666f994ffd09
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ANAVOLVER,
    oracle_id = "03a4cbe2-cf35-4c2c-b035-666f994ffd09",
    scryfall_id = "5e685a8c-fba6-495f-ac0f-1ff5456b22d0",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green, Color::Blue]),
    faces = &[face!(
        name = "Anavolver",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::VOLVER],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
