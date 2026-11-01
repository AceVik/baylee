//! Krovikan Vampire — {3}{B}{B} — Creature — Vampire
//! Oracle: At the beginning of each end step, if a creature dealt damage by this creature this turn died, put that card onto the battlefield under your control. Sacrifice it when you lose control of this creature.
//! Set: ME2 #102 — Masters Edition II | Scryfall ID: fa1e779a-7075-49f0-b62e-936ad554d036 | Oracle ID: 8fa93c01-f9ea-4d48-a040-418d1ca6be19
// NOT SUPPORTED: missing end-step trigger returning creatures dealt damage by this creature this turn that died and sacrificing them when you lose control of this creature; plays as a 3/3 Vampire with no abilities.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KROVIKAN_VAMPIRE,
    oracle_id = "8fa93c01-f9ea-4d48-a040-418d1ca6be19",
    scryfall_id = "fa1e779a-7075-49f0-b62e-936ad554d036",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "missing end-step trigger returning creatures dealt damage by this creature this turn that died and sacrificing them when you lose control of this creature; plays as a 3/3 Vampire with no abilities",
    ),
    faces = &[face!(
        name = "Krovikan Vampire",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::VAMPIRE],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[],
);
