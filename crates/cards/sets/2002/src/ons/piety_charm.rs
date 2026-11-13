//! Piety Charm — {W} — Instant
//! Oracle: Choose one —
//! Oracle: • Destroy target Aura attached to a creature.
//! Oracle: • Target Soldier creature gets +2/+2 until end of turn.
//! Oracle: • Creatures you control gain vigilance until end of turn.
//! Set: ONS #49 — Onslaught | Scryfall ID: 1bc2da43-c0e1-4fbf-b309-a75e105c29c1 | Oracle ID: 3d4ec57a-8541-42ed-9790-7933f96c8adf
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PIETY_CHARM,
    oracle_id = "3d4ec57a-8541-42ed-9790-7933f96c8adf",
    scryfall_id = "1bc2da43-c0e1-4fbf-b309-a75e105c29c1",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Piety Charm",
        mana_cost = mana!("{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
