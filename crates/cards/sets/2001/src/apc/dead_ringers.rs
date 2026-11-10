//! Dead Ringers — {4}{B} — Sorcery
//! Oracle: Destroy two target nonblack creatures unless either one is a color the other isn't. They can't be regenerated.
//! Set: APC #37 — Apocalypse | Scryfall ID: 9b78028c-3ebd-432d-b628-e1fa284f08f3 | Oracle ID: fb34e487-f7a9-48ad-8f42-00295a80a160
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DEAD_RINGERS,
    oracle_id = "fb34e487-f7a9-48ad-8f42-00295a80a160",
    scryfall_id = "9b78028c-3ebd-432d-b628-e1fa284f08f3",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Dead Ringers",
        mana_cost = mana!("{4}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
