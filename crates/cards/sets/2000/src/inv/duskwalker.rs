//! Duskwalker — {B} — Creature — Human Minion
//! Oracle: Kicker {3}{B} (You may pay an additional {3}{B} as you cast this spell.)
//! Oracle: If this creature was kicked, it enters with two +1/+1 counters on it and with fear. (It can't be blocked except by artifact creatures and/or black creatures.)
//! Set: INV #104 — Invasion | Scryfall ID: 39a4a026-f44e-40e1-9942-a3d8448aca70 | Oracle ID: 689f8696-dd13-4e1e-bdbc-3794c524f423
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DUSKWALKER,
    oracle_id = "689f8696-dd13-4e1e-bdbc-3794c524f423",
    scryfall_id = "39a4a026-f44e-40e1-9942-a3d8448aca70",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Duskwalker",
        mana_cost = mana!("{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::MINION],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
