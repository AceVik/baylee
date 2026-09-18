//! Krosan Grip — {2}{G} — Instant
//! Oracle: Split second (As long as this spell is on the stack, players can't cast spells or activate abilities that aren't mana abilities.)
//! Oracle: Destroy target artifact or enchantment.
//! Set: C21 #198 — Commander 2021 | Scryfall ID: d3571dee-7b90-4c0c-abc7-59b515ffa129 | Oracle ID: 3e39224c-72ce-4ecc-aa17-12c071ea1f3e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::KROSAN_GRIP,
    oracle_id = "3e39224c-72ce-4ecc-aa17-12c071ea1f3e",
    scryfall_id = "d3571dee-7b90-4c0c-abc7-59b515ffa129",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Krosan Grip",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
