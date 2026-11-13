//! Llanowar Elite — {G} — Creature — Elf
//! Oracle: Kicker {8} (You may pay an additional {8} as you cast this spell.)
//! Oracle: Trample
//! Oracle: If this creature was kicked, it enters with five +1/+1 counters on it.
//! Set: INV #196 — Invasion | Scryfall ID: 3e207863-de68-47e1-8c63-413b5fa48943 | Oracle ID: 9e112393-962b-461e-a605-9f8d6a1ebaa4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LLANOWAR_ELITE,
    oracle_id = "9e112393-962b-461e-a605-9f8d6a1ebaa4",
    scryfall_id = "3e207863-de68-47e1-8c63-413b5fa48943",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Llanowar Elite",
        mana_cost = mana!("{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
