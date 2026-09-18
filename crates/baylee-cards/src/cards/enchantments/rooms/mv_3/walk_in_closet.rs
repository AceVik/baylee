//! Walk-In Closet // Forgotten Cellar — {2}{G} — Enchantment — Room // Enchantment — Room
//! Oracle: You may play lands from your graveyard.
//! Oracle: (You may cast either half. That door unlocks on the battlefield. As a sorcery, you may pay the mana cost of a locked door to unlock it.)
//! Oracle: When you unlock this door, you may cast spells from your graveyard this turn, and if a card would be put into your graveyard from anywhere this turn, exile it instead.
//! Oracle: (You may cast either half. That door unlocks on the battlefield. As a sorcery, you may pay the mana cost of a locked door to unlock it.)
//! Set: DSK #205 — Duskmourn: House of Horror | Scryfall ID: 0adcd4e5-d542-4293-8774-ace2305ef820 | Oracle ID: 52e77cc3-f8e9-4a20-811b-fe1e46a96ad7
//! Face: Walk-In Closet — {2}{G} — Enchantment — Room
//! Face: Forgotten Cellar — {3}{G}{G} — Enchantment — Room
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WALK_IN_CLOSET,
    oracle_id = "52e77cc3-f8e9-4a20-811b-fe1e46a96ad7",
    scryfall_id = "0adcd4e5-d542-4293-8774-ace2305ef820",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[
        face!(
            name = "Walk-In Closet",
            mana_cost = mana!("{2}{G}"),
            types = TypeSet::ENCHANTMENT,
            subtypes = &[subtypes::enchantment::ROOM],
        ),
        face!(
            name = "Forgotten Cellar",
            mana_cost = mana!("{3}{G}{G}"),
            types = TypeSet::ENCHANTMENT,
            subtypes = &[subtypes::enchantment::ROOM],
        ),
    ],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
