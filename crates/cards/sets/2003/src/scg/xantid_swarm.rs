//! Xantid Swarm — {G} — Creature — Insect
//! Oracle: Flying
//! Oracle: Whenever this creature attacks, defending player can't cast spells this turn.
//! Set: EMA #193 — Eternal Masters | Scryfall ID: 022ab408-3292-40b6-b35e-ac1b7f06dffa | Oracle ID: 13ba9ef8-2010-4d3f-8c62-85b7c5620031
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::XANTID_SWARM,
    oracle_id = "13ba9ef8-2010-4d3f-8c62-85b7c5620031",
    scryfall_id = "022ab408-3292-40b6-b35e-ac1b7f06dffa",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Xantid Swarm",
        mana_cost = mana!("{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::INSECT],
        power = Some(0),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
