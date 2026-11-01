//! Shimmering Efreet — {2}{U} — Creature — Efreet
//! Oracle: Flying
//! Oracle: Phasing (This phases in or out before you untap during each of your untap steps. While it's phased out, it's treated as though it doesn't exist.)
//! Oracle: Whenever this creature phases in, target creature phases out. (It phases in before its controller untaps during their next untap step.)
//! Set: VIS #42 — Visions | Scryfall ID: 23c5704f-5856-4422-9d82-14558dbe1434 | Oracle ID: 34f52d00-301d-4065-a86b-8393b4c6e99d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SHIMMERING_EFREET,
    oracle_id = "34f52d00-301d-4065-a86b-8393b4c6e99d",
    scryfall_id = "23c5704f-5856-4422-9d82-14558dbe1434",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Shimmering Efreet",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::EFREET],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
