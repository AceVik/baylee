//! Twigwalker — {2}{G} — Creature — Insect
//! Oracle: {1}{G}, Sacrifice this creature: Two target creatures each get +2/+2 until end of turn.
//! Set: ODY #279 — Odyssey | Scryfall ID: 90b8ba27-c3bb-48cf-b831-518f5d255c2e | Oracle ID: ee90b4c5-f646-47a9-8aa8-393c86e35536
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TWIGWALKER,
    oracle_id = "ee90b4c5-f646-47a9-8aa8-393c86e35536",
    scryfall_id = "90b8ba27-c3bb-48cf-b831-518f5d255c2e",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Twigwalker",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::INSECT],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
