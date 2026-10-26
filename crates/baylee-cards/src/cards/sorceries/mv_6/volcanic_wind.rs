//! Volcanic Wind — {4}{R}{R} — Sorcery
//! Oracle: Volcanic Wind deals X damage divided as you choose among any number of target creatures, where X is the number of creatures on the battlefield as you cast this spell.
//! Set: MMQ #223 — Mercadian Masques | Scryfall ID: 3c69dd00-46ce-42b2-a2ed-43b4cf04a975 | Oracle ID: baa85b14-1c9f-4a19-9f07-93e3192c597d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::VOLCANIC_WIND,
    oracle_id = "baa85b14-1c9f-4a19-9f07-93e3192c597d",
    scryfall_id = "3c69dd00-46ce-42b2-a2ed-43b4cf04a975",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Volcanic Wind",
        mana_cost = mana!("{4}{R}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
