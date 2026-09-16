//! Ragavan, Nimble Pilferer — {R} — Legendary Creature — Monkey Pirate
//! Oracle: Whenever Ragavan deals combat damage to a player, create a Treasure token and exile the top card of that player's library. Until end of turn, you may cast that card.
//! Oracle: Dash {1}{R} (You may cast this spell for its dash cost. If you do, it gains haste, and it's returned from the battlefield to its owner's hand at the beginning of the next end step.)
//! Set: MH2 #138 — Modern Horizons 2 | Scryfall ID: a9738cda-adb1-47fb-9f4c-ecd930228c4d | Oracle ID: 37108cd4-bbab-4ce3-9ed6-f60e8422e703
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RAGAVAN_NIMBLE_PILFERER,
    oracle_id = "37108cd4-bbab-4ce3-9ed6-f60e8422e703",
    scryfall_id = "a9738cda-adb1-47fb-9f4c-ecd930228c4d",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Ragavan, Nimble Pilferer",
        mana_cost = mana!("{R}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::MONKEY, subtypes::creature::PIRATE],
        power = Some(2),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
