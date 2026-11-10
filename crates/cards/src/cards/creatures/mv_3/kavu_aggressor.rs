//! Kavu Aggressor — {2}{R} — Creature — Kavu
//! Oracle: Kicker {4} (You may pay an additional {4} as you cast this spell.)
//! Oracle: This creature can't block.
//! Oracle: If this creature was kicked, it enters with a +1/+1 counter on it.
//! Set: INV #148 — Invasion | Scryfall ID: a2832ad3-ce7f-44d2-beb2-c95d982905a6 | Oracle ID: 84c00644-d0e2-418b-b931-4e972ecb3366
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KAVU_AGGRESSOR,
    oracle_id = "84c00644-d0e2-418b-b931-4e972ecb3366",
    scryfall_id = "a2832ad3-ce7f-44d2-beb2-c95d982905a6",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Kavu Aggressor",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::KAVU],
        power = Some(3),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
