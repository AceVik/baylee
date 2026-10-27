//! Forcemage Advocate — {1}{G} — Creature — Centaur Shaman
//! Oracle: {T}: Return target card from an opponent's graveyard to their hand. Put a +1/+1 counter on target creature.
//! Set: JUD #116 — Judgment | Scryfall ID: 1ad217fe-9309-4c67-8a6a-cfb8b1ce91f1 | Oracle ID: c8c5cd28-4f27-424d-92bc-e7d6f15a5f9f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FORCEMAGE_ADVOCATE,
    oracle_id = "c8c5cd28-4f27-424d-92bc-e7d6f15a5f9f",
    scryfall_id = "1ad217fe-9309-4c67-8a6a-cfb8b1ce91f1",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Forcemage Advocate",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::CENTAUR, subtypes::creature::SHAMAN],
        power = Some(2),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
