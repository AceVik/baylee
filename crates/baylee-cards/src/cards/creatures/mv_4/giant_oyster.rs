//! Giant Oyster — {2}{U}{U} — Creature — Oyster
//! Oracle: You may choose not to untap this creature during your untap step.
//! Oracle: {T}: For as long as this creature remains tapped, target tapped creature doesn't untap during its controller's untap step, and at the beginning of each of your draw steps, put a -1/-1 counter on that creature. When this creature leaves the battlefield or becomes untapped, remove all -1/-1 counters from the creature.
//! Set: TSB #22 — Time Spiral Timeshifted | Scryfall ID: 2dab7498-c574-4ea6-8dee-e3e0991af018 | Oracle ID: 062b4d84-5ba3-417b-b8e1-65afce7b05e3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GIANT_OYSTER,
    oracle_id = "062b4d84-5ba3-417b-b8e1-65afce7b05e3",
    scryfall_id = "2dab7498-c574-4ea6-8dee-e3e0991af018",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Giant Oyster",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::OYSTER],
        power = Some(0),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
