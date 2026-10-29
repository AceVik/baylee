//! Blind Seer — {2}{U}{U} — Legendary Creature — Human Wizard
//! Oracle: {1}{U}: Target spell or permanent becomes the color of your choice until end of turn.
//! Set: INV #47 — Invasion | Scryfall ID: 5c54ec26-c7f1-4258-9cc9-1709987f293c | Oracle ID: f905b72d-734a-44e8-b118-c8de889994a9
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BLIND_SEER,
    oracle_id = "f905b72d-734a-44e8-b118-c8de889994a9",
    scryfall_id = "5c54ec26-c7f1-4258-9cc9-1709987f293c",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Blind Seer",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
