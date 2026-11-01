//! Pardic Firecat — {3}{R} — Creature — Elemental Cat
//! Oracle: Haste
//! Oracle: If this card is in a graveyard, effects from spells named Flame Burst count it as a card named Flame Burst.
//! Set: ODY #211 — Odyssey | Scryfall ID: 16858395-c742-4657-88c2-5af20c92718d | Oracle ID: c6498ec5-fbbc-472a-9b40-5481e49efe79
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PARDIC_FIRECAT,
    oracle_id = "c6498ec5-fbbc-472a-9b40-5481e49efe79",
    scryfall_id = "16858395-c742-4657-88c2-5af20c92718d",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Pardic Firecat",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEMENTAL, subtypes::creature::CAT],
        power = Some(2),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
