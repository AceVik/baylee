//! Thornscape Battlemage — {2}{G} — Creature — Elf Wizard
//! Oracle: Kicker {R} and/or {W} (You may pay an additional {R} and/or {W} as you cast this spell.)
//! Oracle: When this creature enters, if it was kicked with its {R} kicker, it deals 2 damage to any target.
//! Oracle: When this creature enters, if it was kicked with its {W} kicker, destroy target artifact.
//! Set: MM3 #142 — Modern Masters 2017 | Scryfall ID: d286f930-28ca-4c59-8a53-99686ac6030e | Oracle ID: 8d4d6806-cb01-49e3-91cc-fb0e7f7a8684
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THORNSCAPE_BATTLEMAGE,
    oracle_id = "8d4d6806-cb01-49e3-91cc-fb0e7f7a8684",
    scryfall_id = "d286f930-28ca-4c59-8a53-99686ac6030e",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red, Color::White]),
    faces = &[face!(
        name = "Thornscape Battlemage",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF, subtypes::creature::WIZARD],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
