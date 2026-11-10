//! Anurid Swarmsnapper — {2}{G} — Creature — Frog Beast
//! Oracle: Reach (This creature can block creatures with flying.)
//! Oracle: {1}{G}: This creature can block an additional creature this turn.
//! Set: JUD #105 — Judgment | Scryfall ID: 3636a9f8-d1d7-4452-8a53-788b514fdb97 | Oracle ID: fd275ea7-9ccc-4148-9b33-392a612486dd
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ANURID_SWARMSNAPPER,
    oracle_id = "fd275ea7-9ccc-4148-9b33-392a612486dd",
    scryfall_id = "3636a9f8-d1d7-4452-8a53-788b514fdb97",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Anurid Swarmsnapper",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::FROG, subtypes::creature::BEAST],
        power = Some(1),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
