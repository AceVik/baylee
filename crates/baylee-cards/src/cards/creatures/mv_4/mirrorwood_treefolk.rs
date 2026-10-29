//! Mirrorwood Treefolk — {3}{G} — Creature — Treefolk
//! Oracle: {2}{R}{W}: The next time damage would be dealt to this creature this turn, that damage is dealt to any target instead.
//! Set: PLS #83 — Planeshift | Scryfall ID: ba9a1c94-2b7f-4df7-8517-a122616d9ae4 | Oracle ID: fec51ab9-484f-46a8-b2b0-772a61d89e41
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MIRRORWOOD_TREEFOLK,
    oracle_id = "fec51ab9-484f-46a8-b2b0-772a61d89e41",
    scryfall_id = "ba9a1c94-2b7f-4df7-8517-a122616d9ae4",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red, Color::White]),
    faces = &[face!(
        name = "Mirrorwood Treefolk",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::TREEFOLK],
        power = Some(2),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
