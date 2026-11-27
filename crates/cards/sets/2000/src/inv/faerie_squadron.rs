//! Faerie Squadron — {U} — Creature — Faerie
//! Oracle: Kicker {3}{U} (You may pay an additional {3}{U} as you cast this spell.)
//! Oracle: If this creature was kicked, it enters with two +1/+1 counters on it and with flying.
//! Set: INV #58 — Invasion | Scryfall ID: 4c707c81-dbbd-43be-a79a-7bc92a584839 | Oracle ID: 109f7a7d-dc08-4c8f-8065-93c620a9194e
// IMPLEMENTED — kicker {3}{U} puts two +1/+1 counters and flying on entry.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FAERIE_SQUADRON,
    oracle_id = "109f7a7d-dc08-4c8f-8065-93c620a9194e",
    scryfall_id = "4c707c81-dbbd-43be-a79a-7bc92a584839",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Faerie Squadron",
        mana_cost = mana!("{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::FAERIE],
        power = Some(1),
        toughness = Some(1),
        additional_costs = &[cost!("{3}{U}")],
        enter_modifiers = &[EnterModifier::IfKicked {
            index: None,
            counters: 2,
            modifiers: &[Modifier::AddKeyword(KeywordSet::FLYING)],
        }],
    ),],
);
