//! Thicket Elemental — {3}{G}{G} — Creature — Elemental
//! Oracle: Kicker {1}{G} (You may pay an additional {1}{G} as you cast this spell.)
//! Oracle: When this creature enters, if it was kicked, you may reveal cards from the top of your library until you reveal a creature card. If you do, put that card onto the battlefield and shuffle all other cards revealed this way into your library.
//! Set: DDR #21 — Duel Decks: Nissa vs. Ob Nixilis | Scryfall ID: c5a462ca-0632-410d-9999-a096b71fc3ea | Oracle ID: a85390b6-e90b-4d0c-9573-eb67ce01a0aa
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THICKET_ELEMENTAL,
    oracle_id = "a85390b6-e90b-4d0c-9573-eb67ce01a0aa",
    scryfall_id = "c5a462ca-0632-410d-9999-a096b71fc3ea",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Thicket Elemental",
        mana_cost = mana!("{3}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEMENTAL],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
