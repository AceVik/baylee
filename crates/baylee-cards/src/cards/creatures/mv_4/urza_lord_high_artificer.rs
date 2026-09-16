//! Urza, Lord High Artificer — {2}{U}{U} — Legendary Creature — Human Artificer
//! Oracle: When Urza enters, create a 0/0 colorless Construct artifact creature token with "This token gets +1/+1 for each artifact you control."
//! Oracle: Tap an untapped artifact you control: Add {U}.
//! Oracle: {5}: Shuffle your library, then exile the top card. Until end of turn, you may play that card without paying its mana cost.
//! Set: CMM #130 — Commander Masters | Scryfall ID: 7b7a348a-51f7-4dc5-8fe7-1c70fea5e050 | Oracle ID: e87906d2-db1a-4e19-b910-adb4eb339945
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::URZA_LORD_HIGH_ARTIFICER,
    oracle_id = "e87906d2-db1a-4e19-b910-adb4eb339945",
    scryfall_id = "7b7a348a-51f7-4dc5-8fe7-1c70fea5e050",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Urza, Lord High Artificer",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::ARTIFICER],
        power = Some(1),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
