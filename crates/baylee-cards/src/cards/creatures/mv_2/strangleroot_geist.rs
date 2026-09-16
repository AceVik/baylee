//! Strangleroot Geist — {G}{G} — Creature — Spirit
//! Oracle: Haste
//! Oracle: Undying (When this creature dies, if it had no +1/+1 counters on it, return it to the battlefield under its owner's control with a +1/+1 counter on it.)
//! Set: DKA #127 — Dark Ascension | Scryfall ID: bf1fb137-205c-480f-b6dc-dfa137793ae3 | Oracle ID: af12758f-4a7b-4156-8942-de4716aa0623
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::STRANGLEROOT_GEIST,
    oracle_id = "af12758f-4a7b-4156-8942-de4716aa0623",
    scryfall_id = "bf1fb137-205c-480f-b6dc-dfa137793ae3",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Strangleroot Geist",
        mana_cost = mana!("{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPIRIT],
        power = Some(2),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
