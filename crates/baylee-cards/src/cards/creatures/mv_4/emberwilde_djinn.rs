//! Emberwilde Djinn — {2}{R}{R} — Creature — Djinn
//! Oracle: Flying
//! Oracle: At the beginning of each player's upkeep, that player may pay {R}{R} or 2 life. If the player does, they gain control of this creature.
//! Set: MIR #172 — Mirage | Scryfall ID: c9e8c9f5-61dd-4dcc-bd40-8f366374ea18 | Oracle ID: 3e1a7146-75d5-499f-8718-303bc6ea139b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::EMBERWILDE_DJINN,
    oracle_id = "3e1a7146-75d5-499f-8718-303bc6ea139b",
    scryfall_id = "c9e8c9f5-61dd-4dcc-bd40-8f366374ea18",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Emberwilde Djinn",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DJINN],
        power = Some(5),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
