//! Callous Oppressor — {1}{U}{U} — Creature — Octopus
//! Oracle: You may choose not to untap this creature during your untap step.
//! Oracle: As this creature enters, an opponent chooses a creature type.
//! Oracle: {T}: Gain control of target creature that isn't of the chosen type for as long as this creature remains tapped.
//! Set: ONS #72 — Onslaught | Scryfall ID: b3dd3ce7-e0e3-4412-9983-ff933584f59b | Oracle ID: de14f6a9-1c62-42e9-b174-3e8b1080e0dc
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CALLOUS_OPPRESSOR,
    oracle_id = "de14f6a9-1c62-42e9-b174-3e8b1080e0dc",
    scryfall_id = "b3dd3ce7-e0e3-4412-9983-ff933584f59b",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Callous Oppressor",
        mana_cost = mana!("{1}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::OCTOPUS],
        power = Some(1),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
